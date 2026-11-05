use crate::{
    infrastructure::{http::Http, storage::Store},
    model::AppResult,
    music::{identity::SourceId, manager::ResourceTicket},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::State;
use tauri_plugin_opener::OpenerExt;
use tokio::io::AsyncWriteExt;

use crate::application::Backend;

const SAVE_ENABLED_KEY: &str = "mvSaveEnabled";
const MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_NETWORK_WAIT: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MvSaveSettings {
    pub directory: String,
    pub enabled: bool,
}

struct FetchOutcome {
    bytes: u64,
    extension: &'static str,
}

struct FetchFailure {
    message: String,
    retryable: bool,
}

pub struct SavedMvs {
    store: Arc<Store>,
    http: Arc<Http>,
    directory: PathBuf,
    origin: String,
    jobs: Mutex<HashSet<String>>,
    permits: Arc<tokio::sync::Semaphore>,
}

impl SavedMvs {
    pub fn new(store: Arc<Store>, http: Arc<Http>, directory: PathBuf) -> AppResult<Arc<Self>> {
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        cleanup_partials(&directory);
        let origin = directory
            .canonicalize()
            .unwrap_or_else(|_| directory.clone())
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        Ok(Arc::new(Self {
            store,
            http,
            directory,
            origin,
            jobs: Mutex::new(HashSet::new()),
            permits: Arc::new(tokio::sync::Semaphore::new(2)),
        }))
    }

    pub fn settings(&self) -> AppResult<MvSaveSettings> {
        Ok(MvSaveSettings {
            directory: self.origin.clone(),
            enabled: self.enabled(),
        })
    }

    pub fn set_enabled(&self, enabled: bool) -> AppResult<MvSaveSettings> {
        self.store
            .set_setting(SAVE_ENABLED_KEY, if enabled { "true" } else { "false" })?;
        self.settings()
    }

    pub fn open_directory(&self, app: &tauri::AppHandle) -> AppResult<()> {
        app.opener()
            .open_path(self.directory.to_string_lossy(), None::<&str>)
            .map_err(|error| error.to_string())
    }

    pub fn saved_path(&self, source: &SourceId, mv_id: &str) -> AppResult<Option<String>> {
        let Some(path) = self.store.saved_mv(source.as_str(), mv_id)? else {
            return Ok(None);
        };
        if Path::new(&path).is_file() {
            return Ok(Some(path));
        }
        crate::infrastructure::diagnostics::warn(
            "mv.save.missing",
            format!(
                "保存的 MV 文件不存在 source={} id={mv_id} path={path}",
                source.as_str()
            ),
        );
        if let Err(error) = self.store.remove_saved_mv(source.as_str(), mv_id) {
            crate::infrastructure::diagnostics::warn("mv.save.remove_missing", error);
        }
        Ok(None)
    }

    pub fn start_save(
        self: &Arc<Self>,
        source: &SourceId,
        mv_id: &str,
        ticket: Arc<ResourceTicket>,
    ) {
        if !self.enabled() {
            return;
        }
        let identity = identity(source.as_str(), mv_id);
        let Ok(mut jobs) = self.jobs.lock() else {
            crate::infrastructure::diagnostics::warn("mv.save.jobs", "MV 保存任务锁不可用");
            return;
        };
        if !jobs.insert(identity.clone()) {
            return;
        }
        drop(jobs);
        let this = self.clone();
        let source = source.clone();
        let mv_id = mv_id.to_owned();
        tauri::async_runtime::spawn(async move {
            let permit = match this.permits.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    crate::infrastructure::diagnostics::warn(
                        "mv.save.permits",
                        "MV 保存并发限制不可用",
                    );
                    if let Ok(mut jobs) = this.jobs.lock() {
                        jobs.remove(&identity);
                    }
                    return;
                }
            };
            let result = this.download(&source, &mv_id, &identity, &ticket).await;
            if let Err(error) = result {
                crate::infrastructure::diagnostics::warn(
                    "mv.save",
                    format!(
                        "保存 MV 失败 source={} id={mv_id}：{error}",
                        source.as_str()
                    ),
                );
            }
            drop(permit);
            if let Ok(mut jobs) = this.jobs.lock() {
                jobs.remove(&identity);
            } else {
                crate::infrastructure::diagnostics::warn(
                    "mv.save.jobs.release",
                    "MV 保存任务锁不可用",
                );
            }
        });
    }

    async fn download(
        &self,
        source: &SourceId,
        mv_id: &str,
        identity: &str,
        ticket: &ResourceTicket,
    ) -> AppResult<()> {
        let urls = ticket
            .urls()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let headers = ticket.headers().to_vec();
        if urls.is_empty() {
            return Err("MV 缺少可下载地址".into());
        }
        let mut last_error = None;
        for (index, url) in urls.iter().enumerate() {
            let temporary = self
                .directory
                .join(format!(".nons-mv-{identity}-{}.part", uuid::Uuid::new_v4()));
            match self.fetch(url, &headers, &temporary).await {
                Ok(outcome) => {
                    if let Err(error) = self
                        .publish(source, mv_id, identity, &temporary, &outcome)
                        .await
                    {
                        let _ = tokio::fs::remove_file(&temporary).await;
                        return Err(error);
                    }
                    return Ok(());
                }
                Err(failure) => {
                    let _ = tokio::fs::remove_file(&temporary).await;
                    if failure.retryable && index + 1 < urls.len() {
                        last_error = Some(failure.message);
                        continue;
                    }
                    return Err(failure.message);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| "MV 下载失败".into()))
    }

    async fn fetch(
        &self,
        url: &str,
        headers: &[(String, String)],
        temporary: &Path,
    ) -> Result<FetchOutcome, FetchFailure> {
        if headers.len() > 32 {
            return Err(failure("MV 请求头过多", false));
        }
        let parsed =
            crate::infrastructure::http::checked_url(url).map_err(|error| failure(error, false))?;
        let mut response = self.send(parsed, headers).await?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(failure(
                format!("MV 服务器返回 HTTP {}", status.as_u16()),
                retry_status(status),
            ));
        }
        let total = response.content_length();
        if total.is_some_and(|total| total > MAX_BYTES) {
            return Err(failure("MV 文件超过 4GiB 上限", false));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary)
            .await
            .map_err(|error| failure(format!("无法创建 MV 文件：{error}"), false))?;
        let mut bytes = 0u64;
        let mut network_wait = Duration::ZERO;
        let mut head = Vec::with_capacity(16);
        loop {
            let started = std::time::Instant::now();
            let chunk = tokio::time::timeout(Duration::from_secs(30), response.chunk())
                .await
                .map_err(|_| failure("读取 MV 内容超时", true))?
                .map_err(|error| {
                    crate::infrastructure::diagnostics::warn("mv.save.http.chunk", &error);
                    failure("读取 MV 内容失败", true)
                })?;
            network_wait += started.elapsed();
            if network_wait > MAX_NETWORK_WAIT {
                return Err(failure("MV 网络等待超过 10 分钟", true));
            }
            let Some(chunk) = chunk else {
                break;
            };
            if head.len() < 16 {
                let take = (16 - head.len()).min(chunk.len());
                head.extend_from_slice(&chunk[..take]);
            }
            bytes = bytes.saturating_add(chunk.len() as u64);
            if bytes > MAX_BYTES {
                return Err(failure("MV 文件超过 4GiB 上限", false));
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| failure(format!("写入 MV 文件失败：{error}"), false))?;
        }
        file.flush()
            .await
            .map_err(|error| failure(format!("保存 MV 文件失败：{error}"), false))?;
        file.sync_all()
            .await
            .map_err(|error| failure(format!("保存 MV 文件失败：{error}"), false))?;
        drop(file);
        if bytes == 0 || total.is_some_and(|total| total != bytes) {
            return Err(failure("MV 内容不完整", true));
        }
        Ok(FetchOutcome {
            bytes,
            extension: extension(content_type.as_deref(), &head),
        })
    }

    async fn send(
        &self,
        mut url: url::Url,
        headers: &[(String, String)],
    ) -> Result<reqwest::Response, FetchFailure> {
        for _ in 0..5 {
            let mut request = self
                .http
                .client(&url)
                .get(url.clone())
                .header(reqwest::header::ACCEPT_ENCODING, "identity")
                .timeout(Duration::from_secs(24 * 60 * 60));
            for (name, value) in headers {
                let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| failure("MV 请求头无效", false))?;
                if matches!(
                    name.as_str(),
                    "host" | "content-length" | "transfer-encoding" | "connection"
                ) {
                    continue;
                }
                let value = reqwest::header::HeaderValue::from_str(value)
                    .map_err(|_| failure("MV 请求头无效", false))?;
                request = request.header(name, value);
            }
            let response = tokio::time::timeout(Duration::from_secs(30), request.send())
                .await
                .map_err(|_| failure("连接 MV 资源超时", true))?
                .map_err(|error| {
                    crate::infrastructure::diagnostics::warn("mv.save.http.send", &error);
                    failure("连接 MV 资源失败", true)
                })?;
            if !response.status().is_redirection() {
                return Ok(response);
            }
            let Some(location) = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
            else {
                return Err(failure("MV 跳转缺少地址", true));
            };
            url = url
                .join(&location)
                .map_err(|error| failure(error.to_string(), false))?;
            url = crate::infrastructure::http::checked_url(url.as_str())
                .map_err(|error| failure(error, false))?;
        }
        Err(failure("MV 跳转过多", true))
    }

    async fn publish(
        &self,
        source: &SourceId,
        mv_id: &str,
        identity: &str,
        temporary: &Path,
        outcome: &FetchOutcome,
    ) -> AppResult<()> {
        let final_path = self
            .directory
            .join(format!("{identity}.{}", outcome.extension));
        if final_path.is_file() {
            let _ = tokio::fs::remove_file(temporary).await;
        } else {
            tokio::fs::rename(temporary, &final_path)
                .await
                .map_err(|error| format!("发布 MV 文件失败：{error}"))?;
        }
        let path = final_path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        if let Err(error) = self
            .store
            .save_mv(source.as_str(), mv_id, &path, outcome.bytes)
        {
            let _ = tokio::fs::remove_file(&final_path).await;
            return Err(format!("保存 MV 记录失败：{error}"));
        }
        Ok(())
    }

    fn enabled(&self) -> bool {
        match self.store.setting(SAVE_ENABLED_KEY) {
            Ok(None) => true,
            Ok(Some(value)) => value != "false",
            Err(error) => {
                crate::infrastructure::diagnostics::warn("mv.save.setting", error);
                true
            }
        }
    }
}

fn identity(source: &str, mv_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update([0]);
    hasher.update(mv_id.as_bytes());
    hex::encode(hasher.finalize())
}

fn extension(content_type: Option<&str>, head: &[u8]) -> &'static str {
    if head.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        return "webm";
    }
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        return if &head[8..12] == b"qt  " {
            "m4v"
        } else {
            "mp4"
        };
    }
    match content_type
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("video/webm") => "webm",
        Some("video/quicktime") => "m4v",
        _ => "mp4",
    }
}

fn failure(message: impl Into<String>, retryable: bool) -> FetchFailure {
    FetchFailure {
        message: message.into(),
        retryable,
    }
}

fn retry_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 403 | 404 | 408 | 410 | 429 | 500..=599)
}

fn cleanup_partials(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(".nons-mv-") {
            let _ = std::fs::remove_file(entry.path()).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("mv.save.cleanup", error);
            });
        }
    }
}

#[tauri::command]
pub fn mv_save_settings(backend: State<'_, Backend>) -> AppResult<MvSaveSettings> {
    backend.saved_mvs.settings()
}

#[tauri::command]
pub fn set_mv_save_enabled(
    enabled: bool,
    backend: State<'_, Backend>,
) -> AppResult<MvSaveSettings> {
    backend.saved_mvs.set_enabled(enabled)
}

#[tauri::command]
pub fn open_mv_directory(app: tauri::AppHandle, backend: State<'_, Backend>) -> AppResult<()> {
    backend.saved_mvs.open_directory(&app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, response::Response, routing::get, Router};

    #[test]
    fn detects_supported_video_extensions() {
        assert_eq!(extension(None, &[0x1a, 0x45, 0xdf, 0xa3]), "webm");
        assert_eq!(extension(None, b"0000ftypqt  "), "m4v");
        assert_eq!(extension(Some("video/webm; charset=binary"), &[]), "webm");
        assert_eq!(extension(Some("video/mp4"), &[]), "mp4");
        assert_eq!(extension(None, &[]), "mp4");
    }

    #[test]
    fn save_setting_defaults_on_and_persists_off() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("mv.sqlite3")).unwrap());
        let saved = SavedMvs::new(
            store,
            Arc::new(Http::new().unwrap()),
            directory.path().join("mvs"),
        )
        .unwrap();
        assert!(saved.settings().unwrap().enabled);
        saved.set_enabled(false).unwrap();
        assert!(!saved.settings().unwrap().enabled);
    }

    #[tokio::test]
    async fn a_disabled_setting_skips_new_saves() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("mv.sqlite3")).unwrap());
        let saved = SavedMvs::new(
            store.clone(),
            Arc::new(Http::new().unwrap()),
            directory.path().join("mvs"),
        )
        .unwrap();
        saved.set_enabled(false).unwrap();
        let source = SourceId::try_from("test".to_owned()).unwrap();
        saved.start_save(
            &source,
            "video-1",
            ResourceTicket::for_audio_test("http://127.0.0.1:9/video".into(), vec![]),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(store.saved_mv("test", "video-1").unwrap().is_none());
    }

    #[tokio::test]
    async fn downloads_once_and_removes_the_record_when_the_file_is_missing() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route(
                        "/redirect",
                        get(|| async {
                            Response::builder()
                                .status(302)
                                .header("location", "/video")
                                .body(Body::empty())
                                .unwrap()
                        }),
                    )
                    .route(
                        "/video",
                        get(|| async {
                            Response::builder()
                                .header("content-type", "video/mp4")
                                .body(Body::from("0000ftypisom-video-bytes"))
                                .unwrap()
                        }),
                    ),
            )
            .await
            .unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("mv.sqlite3")).unwrap());
        let saved = SavedMvs::new(
            store.clone(),
            Arc::new(Http::for_test(reqwest::Client::new())),
            directory.path().join("mvs"),
        )
        .unwrap();
        let source = SourceId::try_from("test".to_owned()).unwrap();
        let ticket = ResourceTicket::for_audio_test(format!("http://{address}/redirect"), vec![]);
        saved.start_save(&source, "video-1", ticket);
        let path = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let Some(path) = saved.saved_path(&source, "video-1").unwrap() {
                    break path;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert!(path.ends_with(".mp4"));
        assert_eq!(
            tokio::fs::read(&path).await.unwrap(),
            b"0000ftypisom-video-bytes"
        );
        tokio::fs::remove_file(&path).await.unwrap();
        assert!(saved.saved_path(&source, "video-1").unwrap().is_none());
        assert!(store.saved_mv("test", "video-1").unwrap().is_none());
        server.abort();
    }
}
