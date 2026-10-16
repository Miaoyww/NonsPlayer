//! Signed Windows installer updates, with independent mirror and GitHub attempts.
use crate::model::AppResult;
use serde::Serialize;
use std::{
    io::Write,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

const MIRROR: &str = "https://download.miaoyww.top/nons/releases/latest.json";
const GITHUB: &str = "https://github.com/Miaoyww/Nons/releases/latest/download/latest.json";
const MAX_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    phase: &'static str,
    version: Option<String>,
    notes: Option<String>,
    source: &'static str,
    portable: bool,
    supported: bool,
    downloaded: u64,
    total: Option<u64>,
    error: Option<String>,
}
struct Inner {
    snapshot: Snapshot,
    update: Option<Update>,
    package: Option<tempfile::TempPath>,
}
pub struct Updates {
    operation: tokio::sync::Mutex<()>,
    inner: Mutex<Inner>,
}
impl Updates {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            operation: tokio::sync::Mutex::new(()),
            inner: Mutex::new(Inner {
                snapshot: Snapshot {
                    phase: "idle",
                    version: None,
                    notes: None,
                    source: "r2",
                    portable: crate::infrastructure::directories::portable().is_some(),
                    supported: cfg!(all(windows, target_arch = "x86_64")),
                    downloaded: 0,
                    total: None,
                    error: None,
                },
                update: None,
                package: None,
            }),
        })
    }
    fn snapshot(&self) -> AppResult<Snapshot> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| "更新状态锁不可用")?
            .snapshot
            .clone())
    }
    pub fn cleanup(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.package.take();
        }
    }
    fn change(
        &self,
        app: &tauri::AppHandle,
        change: impl FnOnce(&mut Inner),
    ) -> AppResult<Snapshot> {
        let mut inner = self.inner.lock().map_err(|_| "更新状态锁不可用")?;
        change(&mut inner);
        let snapshot = inner.snapshot.clone();
        let _ = app.emit("app-update", &snapshot);
        Ok(snapshot)
    }
    fn fail(&self, app: &tauri::AppHandle, error: &str) {
        let _ = self.change(app, |inner| {
            inner.snapshot.phase = if inner.package.is_some() {
                "ready"
            } else if inner.update.is_some() {
                "available"
            } else {
                "idle"
            };
            inner.snapshot.error = Some(error.into());
        });
    }
    pub async fn check(&self, app: &tauri::AppHandle) -> AppResult<Snapshot> {
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "正在处理更新，请稍后重试")?;
        if !self.snapshot()?.supported {
            return self.snapshot();
        }
        if self.snapshot()?.phase == "ready" {
            return self.snapshot();
        }
        self.change(app, |inner| {
            inner.snapshot.phase = "checking";
            inner.snapshot.error = None;
        })?;
        let result = prefer_mirror(fetch(app, MIRROR), || fetch(app, GITHUB)).await;
        match result {
            Ok((update, source)) => self.change(app, |inner| {
                inner.snapshot.phase = if update.is_some() {
                    "available"
                } else {
                    "current"
                };
                inner.snapshot.version = update.as_ref().map(|u| u.version.clone());
                inner.snapshot.notes = update.as_ref().and_then(|u| u.body.clone());
                inner.snapshot.source = source;
                inner.snapshot.downloaded = 0;
                inner.snapshot.total = None;
                inner.update = update;
            }),
            Err(_) => {
                let message = "检查更新失败：下载镜像和 GitHub 均不可用，请稍后重试。";
                self.fail(app, message);
                Err(message.into())
            }
        }
    }
    async fn download(&self, app: &tauri::AppHandle) -> AppResult<Snapshot> {
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "正在处理更新，请稍后重试")?;
        let (mut update, source) = {
            let inner = self.inner.lock().map_err(|_| "更新状态锁不可用")?;
            if inner.snapshot.portable {
                return Err("便携版请下载新版压缩包".into());
            }
            if inner.package.is_some() {
                return Ok(inner.snapshot.clone());
            }
            (
                inner.update.clone().ok_or("请先检查更新")?,
                inner.snapshot.source,
            )
        };
        self.change(app, |inner| {
            inner.snapshot.phase = "downloading";
            inner.snapshot.error = None;
        })?;
        let result = async {
            let bytes = match self.download_from(app, &mut update).await {
                Ok(bytes) => bytes,
                Err(_) if source == "r2" => {
                    let fallback = fetch(app, GITHUB).await?.ok_or("GitHub 未提供该更新")?;
                    if update.version != fallback.version {
                        return Err("GitHub 与镜像的版本不一致，请重新检查更新".into());
                    }
                    update = fallback;
                    self.change(app, |inner| {
                        inner.snapshot.source = "github";
                    })?;
                    self.download_from(app, &mut update).await?
                }
                Err(_) => return Err("更新下载或签名验证失败，请稍后重试。".into()),
            };
            let package =
                tauri::async_runtime::spawn_blocking(move || -> AppResult<tempfile::TempPath> {
                    let mut file = tempfile::NamedTempFile::new_in(
                        crate::infrastructure::directories::temporary(),
                    )
                    .map_err(|e| e.to_string())?;
                    file.write_all(&bytes).map_err(|e| e.to_string())?;
                    file.flush().map_err(|e| e.to_string())?;
                    Ok(file.into_temp_path())
                })
                .await
                .map_err(|_| "更新文件写入任务失败")??;
            self.change(app, |inner| {
                inner.package = Some(package);
                inner.update = Some(update);
                inner.snapshot.phase = "ready";
            })
        }
        .await;
        if let Err(error) = &result {
            self.fail(app, error);
        }
        result
    }
    async fn download_from(
        &self,
        app: &tauri::AppHandle,
        update: &mut Update,
    ) -> AppResult<Vec<u8>> {
        if update.download_url.scheme() != "https" {
            return Err("更新包必须使用 HTTPS".into());
        }
        update.timeout = Some(Duration::from_secs(900));
        self.change(app, |inner| {
            inner.snapshot.downloaded = 0;
            inner.snapshot.total = None;
        })?;
        let limit = tokio::sync::Notify::new();
        let mut downloaded = 0u64;
        let mut last = Instant::now() - Duration::from_secs(1);
        let download = update.download(
            |size, total| {
                downloaded = downloaded.saturating_add(size as u64);
                if downloaded > MAX_BYTES || total.is_some_and(|v| v > MAX_BYTES) {
                    limit.notify_one();
                }
                if last.elapsed() >= Duration::from_millis(200) {
                    let _ = self.change(app, |inner| {
                        inner.snapshot.downloaded = downloaded;
                        inner.snapshot.total = total;
                    });
                    last = Instant::now();
                }
            },
            || {},
        );
        let bytes = tokio::select! {
            biased;
            _ = limit.notified() => return Err("更新包超过 512MiB 上限".into()),
            result = download => result.map_err(|_| "更新下载或签名验证失败")?
        };
        if bytes.len() as u64 > MAX_BYTES {
            return Err("更新包超过 512MiB 上限".into());
        }
        self.change(app, |inner| {
            inner.snapshot.downloaded = bytes.len() as u64;
            inner.snapshot.total = Some(bytes.len() as u64);
        })?;
        Ok(bytes)
    }
}
async fn fetch(app: &tauri::AppHandle, endpoint: &str) -> AppResult<Option<Update>> {
    let exit_app = app.clone();
    app.updater_builder()
        .endpoints(vec![endpoint.parse().map_err(|_| "更新地址无效")?])
        .map_err(|e| e.to_string())?
        .timeout(Duration::from_secs(8))
        .on_before_exit(move || {
            // The updater exits Windows directly; flush playback before that exit.
            crate::platform::main_webview::stop(&exit_app);
            exit_app
                .state::<Arc<crate::plugins::PluginManager>>()
                .stop();
            exit_app.state::<super::Backend>().player.shutdown();
            log::logger().flush();
        })
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())
}

async fn prefer_mirror<T, P, F, G>(primary: P, fallback: F) -> AppResult<(Option<T>, &'static str)>
where
    P: std::future::Future<Output = AppResult<Option<T>>>,
    F: FnOnce() -> G,
    G: std::future::Future<Output = AppResult<Option<T>>>,
{
    match primary.await {
        Ok(value) => Ok((value, "r2")),
        Err(_) => fallback().await.map(|value| (value, "github")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mirror_success_and_no_update_do_not_contact_github() {
        for value in [Some("1.4.0"), None] {
            let result = prefer_mirror(async { Ok(value) }, || async {
                panic!("unexpected fallback")
            })
            .await
            .unwrap();
            assert_eq!(result, (value, "r2"));
        }
    }
    #[tokio::test]
    async fn mirror_errors_fall_back_and_both_failures_are_reported() {
        let result = prefer_mirror(
            async { Err::<Option<&str>, _>("mirror error".into()) },
            || async { Ok(Some("1.4.0")) },
        )
        .await
        .unwrap();
        assert_eq!(result, (Some("1.4.0"), "github"));
        assert!(prefer_mirror(
            async { Err::<Option<&str>, _>("mirror error".into()) },
            || async { Err("github error".into()) }
        )
        .await
        .is_err());
    }
}

#[tauri::command]
pub fn update_status(updates: State<'_, Arc<Updates>>) -> AppResult<Snapshot> {
    updates.snapshot()
}
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    updates: State<'_, Arc<Updates>>,
) -> AppResult<Snapshot> {
    updates.check(&app).await
}
#[tauri::command]
pub async fn update_download(
    app: tauri::AppHandle,
    updates: State<'_, Arc<Updates>>,
) -> AppResult<Snapshot> {
    updates.download(&app).await
}
#[tauri::command]
pub async fn update_install(
    app: tauri::AppHandle,
    updates: State<'_, Arc<Updates>>,
) -> AppResult<()> {
    let _guard = updates
        .operation
        .try_lock()
        .map_err(|_| "正在处理更新，请稍后重试")?;
    let (update, package) = {
        let mut inner = updates.inner.lock().map_err(|_| "更新状态锁不可用")?;
        (
            inner.update.clone().ok_or("请先下载更新")?,
            inner.package.take().ok_or("请先下载更新")?,
        )
    };
    updates.change(&app, |inner| {
        inner.snapshot.phase = "installing";
        inner.snapshot.error = None;
    })?;
    let result = tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
        let bytes = std::fs::read(&package).map_err(|e| e.to_string())?;
        // Windows install exits the process directly and does not run destructors.
        drop(package);
        update.install(bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "安装任务失败".to_owned())
    .and_then(|result| result);
    if let Err(error) = &result {
        updates.fail(&app, error);
    }
    Ok(result?)
}
#[tauri::command]
pub async fn update_open_download(
    app: tauri::AppHandle,
    updates: State<'_, Arc<Updates>>,
) -> AppResult<()> {
    let snapshot = updates.snapshot()?;
    let version = snapshot.version.ok_or("请先检查更新")?;
    semver::Version::parse(&version).map_err(|_| "更新版本无效")?;
    let name = format!("Nons_{version}_windows_x64_portable.zip");
    let mirror = format!("https://download.miaoyww.top/nons/releases/v{version}/{name}");
    let github = format!("https://github.com/Miaoyww/Nons/releases/download/v{version}/{name}");
    let reachable = if snapshot.source == "r2" {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?
            .head(&mirror)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
    } else {
        false
    };
    app.opener()
        .open_url(if reachable { mirror } else { github }, None::<&str>)
        .map_err(|e| e.to_string())
}
