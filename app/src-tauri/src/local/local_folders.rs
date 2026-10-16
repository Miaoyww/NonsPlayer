use crate::{library, model::AppResult, storage::Store};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Emitter;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicFolder {
    pub path: String,
    pub tracks: i64,
    pub scanning: bool,
    pub error: Option<String>,
}
struct Status {
    paths: Vec<String>,
    scanning: HashSet<String>,
    errors: HashMap<String, String>,
}
pub struct LocalFolders {
    status: Mutex<Status>,
    watcher: Mutex<RecommendedWatcher>,
    scans: Mutex<()>,
    signal: mpsc::SyncSender<()>,
    store: Arc<Store>,
    covers: PathBuf,
    fallback: String,
    app: tauri::AppHandle,
}
impl LocalFolders {
    pub fn new(
        store: Arc<Store>,
        covers: PathBuf,
        fallback: String,
        app: tauri::AppHandle,
    ) -> AppResult<Arc<Self>> {
        let paths: Vec<String> = store
            .setting("musicFolders")?
            .and_then(|s| {
                serde_json::from_str(&s)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
            })
            .unwrap_or_default();
        let (signal, receiver) = mpsc::sync_channel(1);
        let events = signal.clone();
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            // Reading tags generates access notifications on some systems.
            // Only mutations (and watcher errors requiring reconciliation) rescan.
            let rescan = match event {
                Ok(event) => !event.kind.is_access(),
                Err(error) => {
                    crate::infrastructure::diagnostics::warn("local.watch", error);
                    true
                }
            };
            if rescan {
                request_scan(&events);
            }
        })
        .map_err(|e| format!("无法监听音乐文件夹：{e}"))?;
        let service = Arc::new(Self {
            status: Mutex::new(Status {
                paths: paths.into_iter().take(32).collect(),
                scanning: HashSet::new(),
                errors: HashMap::new(),
            }),
            watcher: Mutex::new(watcher),
            scans: Mutex::new(()),
            signal,
            store,
            covers,
            fallback,
            app,
        });
        {
            let mut watcher = service
                .watcher
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "目录监听锁不可用")?;
            let mut status = service
                .status
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "目录状态锁不可用")?;
            for path in status.paths.clone() {
                if let Err(error) = watcher.watch(Path::new(&path), RecursiveMode::Recursive) {
                    crate::infrastructure::diagnostics::warn("local.watch_folder", &error);
                    status.errors.insert(path, error.to_string());
                }
            }
        }
        let weak = Arc::downgrade(&service);
        std::thread::Builder::new()
            .name("local-library-scan".into())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    let burst = Instant::now();
                    while burst.elapsed() < Duration::from_secs(2)
                        && receiver.recv_timeout(Duration::from_millis(350)).is_ok()
                    {
                    }
                    let Some(service) = weak.upgrade() else {
                        break;
                    };
                    if let Err(error) = service.scan() {
                        crate::infrastructure::diagnostics::warn("local.scan", &error);
                        let _ = (service
                            .app
                            .emit("local-library-updated", serde_json::json!({"error":error})))
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("service .app .emit", error);
                        });
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        service.rescan();
        Ok(service)
    }
    pub fn list(&self) -> AppResult<Vec<MusicFolder>> {
        let status = self
            .status
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录状态锁不可用")?;
        status
            .paths
            .iter()
            .map(|path| {
                Ok(MusicFolder {
                    path: path.clone(),
                    tracks: self.store.origin_count(path)?,
                    scanning: status.scanning.contains(path),
                    error: status.errors.get(path).cloned(),
                })
            })
            .collect()
    }
    fn changed(&self) {
        let scanning = self
            .status
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("local.scan_status", error)
            })
            .map(|s| !s.scanning.is_empty())
            .unwrap_or(false);
        let _ = (self.app.emit(
            "local-library-updated",
            serde_json::json!({"scanning": scanning}),
        ))
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("self.app.emit", error);
        });
    }
    pub fn add(&self, path: String) -> AppResult<()> {
        let path = Path::new(&path)
            .canonicalize()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "音乐文件夹不存在或无法访问")?;
        if !path.is_dir() {
            return Err("请选择音乐文件夹".into());
        }
        let path = path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        let _scan = self
            .scans
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录扫描锁不可用")?;
        let mut watcher = self
            .watcher
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录监听锁不可用")?;
        let mut status = self
            .status
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录状态锁不可用")?;
        if status.paths.iter().any(|p| same_path(p, &path)) {
            return Ok(());
        }
        if status.paths.len() >= 32 {
            return Err("最多管理 32 个音乐文件夹".into());
        }
        watcher
            .watch(Path::new(&path), RecursiveMode::Recursive)
            .map_err(|e| format!("无法监听此文件夹：{e}"))?;
        let mut next = status.paths.clone();
        next.push(path.clone());
        if let Err(error) = self.store.set_setting(
            "musicFolders",
            &serde_json::to_string(&next).map_err(|e| e.to_string())?,
        ) {
            let _ = (watcher.unwatch(Path::new(&path))).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("watcher.unwatch", error);
            });
            return Err(error);
        }
        status.paths = next;
        drop(status);
        drop(watcher);
        self.rescan();
        self.changed();
        Ok(())
    }
    pub fn remove(&self, path: String) -> AppResult<()> {
        let _scan = self
            .scans
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录扫描锁不可用")?;
        let mut watcher = self
            .watcher
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录监听锁不可用")?;
        let mut status = self
            .status
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录状态锁不可用")?;
        if !status.paths.contains(&path) {
            return Err("音乐文件夹未添加".into());
        }
        let next: Vec<_> = status
            .paths
            .iter()
            .filter(|p| **p != path)
            .cloned()
            .collect();
        self.store.set_setting(
            "musicFolders",
            &serde_json::to_string(&next).map_err(|e| e.to_string())?,
        )?;
        self.store.reconcile_origin(&path, &HashSet::new())?;
        let _ = (watcher.unwatch(Path::new(&path))).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("watcher.unwatch", error);
        });
        status.paths = next;
        status.errors.remove(&path);
        drop(status);
        drop(watcher);
        self.changed();
        Ok(())
    }
    pub fn rescan(&self) {
        request_scan(&self.signal);
    }
    fn scan(&self) -> AppResult<()> {
        let _scan = self
            .scans
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录扫描锁不可用")?;
        let paths = self
            .status
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "目录状态锁不可用")?
            .paths
            .clone();
        for path in paths {
            self.status
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "目录状态锁不可用")?
                .scanning
                .insert(path.clone());
            self.changed();
            let result = library::import(
                vec![path.clone()],
                self.store.clone(),
                self.covers.clone(),
                self.fallback.clone(),
                Some(path.clone()),
            );
            let mut status = self
                .status
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "目录状态锁不可用")?;
            status.scanning.remove(&path);
            match result {
                Ok(report) if report.skipped == 0 => {
                    status.errors.remove(&path);
                }
                Ok(report) => {
                    status.errors.insert(
                        path,
                        format!(
                            "{} 个文件暂未读取成功{}",
                            report.skipped,
                            report
                                .errors
                                .first()
                                .map(|e| format!("：{e}"))
                                .unwrap_or_default()
                        ),
                    );
                }
                Err(error) => {
                    crate::infrastructure::diagnostics::warn("local.scan_folder", &error);
                    status.errors.insert(path, error);
                }
            }
            drop(status);
            self.changed();
        }
        Ok(())
    }
}
fn request_scan(signal: &mpsc::SyncSender<()>) {
    // Full means a rescan is already queued; watcher bursts intentionally coalesce.
    if let Err(mpsc::TrySendError::Disconnected(())) = signal.try_send(()) {
        crate::infrastructure::diagnostics::warn("local.request_scan", "scan worker disconnected");
    }
}
fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}
