use super::{capability::Context, http::checked_url};
use crate::model::AppResult;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    id: u64,
    state: String,
    bytes: u64,
    total: Option<u64>,
    error: Option<String>,
}
struct Task {
    plugin: String,
    generation: u64,
    started: std::time::Instant,
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    snapshot: Arc<Mutex<Snapshot>>,
}
const MAX_COMPLETED: usize = 128;
fn parse_max_bytes(args: &Value) -> AppResult<Option<u64>> {
    args.get("maxBytes")
        .map(|value| {
            value
                .as_u64()
                .filter(|bytes| *bytes > 0)
                .ok_or_else(|| "maxBytes 必须为正整数".to_string())
        })
        .transpose()
}
fn running(task: &Task) -> bool {
    task.snapshot
        .lock()
        .map(|snapshot| matches!(snapshot.state.as_str(), "running" | "paused"))
        .unwrap_or(true)
}
fn prune_completed(tasks: &mut BTreeMap<u64, Task>, plugin: &str, generation: u64) {
    tasks.retain(|_, task| {
        running(task) || (task.plugin == plugin && task.generation == generation)
    });
    let mut completed: Vec<_> = tasks
        .iter()
        .filter(|(_, task)| !running(task))
        .map(|(id, task)| (*id, task.started))
        .collect();
    completed.sort_by_key(|(id, started)| (*started, *id));
    let excess = completed.len().saturating_sub(MAX_COMPLETED);
    for (id, _) in completed.into_iter().take(excess) {
        tasks.remove(&id);
    }
}
#[derive(Default)]
pub struct Transfers {
    tasks: Mutex<BTreeMap<u64, Task>>,
}
impl Transfers {
    pub fn call(&self, context: Context, operation: &str, args: &Value) -> AppResult<Value> {
        let mut tasks = self
            .tasks
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.transfers.call.lock_tasks",
                    error,
                );
            })
            .map_err(|_| "传输管理器不可用")?;
        if operation != "transfers.start" {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or("缺少传输 ID")?;
            let task = tasks
                .get(&id)
                .filter(|t| t.plugin == context.id && t.generation == context.generation)
                .ok_or("传输任务已失效")?;
            if operation == "transfers.cancel" {
                task.cancel.store(true, Ordering::SeqCst);
            }
            if matches!(operation, "transfers.pause" | "transfers.resume") {
                let mut snapshot = task
                    .snapshot
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "plugins.transfers.pause.lock_snapshot",
                            error,
                        );
                    })
                    .map_err(|_| "传输状态不可用")?;
                if matches!(snapshot.state.as_str(), "running" | "paused") {
                    let paused = operation == "transfers.pause";
                    task.paused.store(paused, Ordering::SeqCst);
                    snapshot.state = if paused { "paused" } else { "running" }.into();
                }
            }
            return serde_json::to_value(
                task.snapshot
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "plugins.transfers.snapshot.lock",
                            error,
                        );
                    })
                    .map_err(|_| "传输状态不可用")?
                    .clone(),
            )
            .map_err(|e| e.to_string());
        }
        let url = args.get("url").and_then(Value::as_str).ok_or("缺少 URL")?;
        let url = checked_url(url)?;
        let root = args
            .get("root")
            .and_then(Value::as_str)
            .ok_or("缺少目录")?
            .to_string();
        context.check(Some(if root == "data" {
            "files:data"
        } else {
            "files:selected"
        }))?;
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or("缺少文件路径")?
            .to_string();
        let max_bytes = parse_max_bytes(args)?;
        prune_completed(&mut tasks, &context.id, context.generation);
        let id = fastrand::u64(1..(1u64 << 53));
        let snapshot = Arc::new(Mutex::new(Snapshot {
            id,
            state: "running".into(),
            bytes: 0,
            total: None,
            error: None,
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        tasks.insert(
            id,
            Task {
                plugin: context.id.clone(),
                generation: context.generation,
                started: std::time::Instant::now(),
                cancel: cancel.clone(),
                paused: paused.clone(),
                snapshot: snapshot.clone(),
            },
        );
        let context = TransferContext::from(&context);
        tauri::async_runtime::spawn(async move {
            let result = run(
                &context,
                url,
                &root,
                &path,
                max_bytes,
                &snapshot,
                (&cancel, &paused),
            )
            .await;
            if let Ok(mut state) = (snapshot.lock()).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.transfers.complete.lock_snapshot",
                    error,
                );
            }) {
                state.state = if result.is_ok() {
                    "completed"
                } else if cancel.load(Ordering::SeqCst) || !context.active.load(Ordering::SeqCst) {
                    "cancelled"
                } else {
                    "failed"
                }
                .into();
                state.error = result.err();
            }
        });
        Ok(json!({"id":id}))
    }
}
struct TransferContext {
    unrestricted: bool,
    id: String,
    generation: u64,
    files: Arc<super::files::Files>,
    http: Arc<super::http::Http>,
    active: Arc<AtomicBool>,
}
impl From<&Context> for TransferContext {
    fn from(context: &Context) -> Self {
        Self {
            unrestricted: context.file_roots.iter().any(|r| r == "*"),
            id: context.id.clone(),
            generation: context.generation,
            files: context.files.clone(),
            http: context.http.clone(),
            active: context.active.clone(),
        }
    }
}
async fn run(
    context: &TransferContext,
    url: url::Url,
    root: &str,
    path: &str,
    max: Option<u64>,
    snapshot: &Arc<Mutex<Snapshot>>,
    controls: (&AtomicBool, &AtomicBool),
) -> AppResult<()> {
    let (cancel, paused) = controls;
    let io = |operation: &str, args: Value| {
        let files = context.files.clone();
        let plugin = context.id.clone();
        let generation = context.generation;
        let active = context.active.clone();
        let operation = operation.to_string();
        let unrestricted = context.unrestricted;
        tauri::async_runtime::spawn_blocking(move || {
            files.call_with_access(
                &plugin,
                generation,
                &operation,
                &args,
                &active,
                unrestricted,
            )
        })
    };
    let opened = io(
        "files.open",
        json!({"root":root,"path":path,"mode":"create"}),
    )
    .await
    .inspect_err(|error| {
        crate::infrastructure::diagnostics::warn(
            "plugins.transfers.download.create_file.worker_join",
            error,
        );
    })
    .map_err(|_| "创建文件失败")??;
    let handle = opened["handle"].as_u64().ok_or("文件句柄无效")?;
    let transfer = async {
        let mut response = context
            .http
            .client_with_redirects(&url, false)
            .get(url)
            .send()
            .await
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "plugins.transfers.download.http_send",
                    error,
                );
            })
            .map_err(|_| "连接资源失败")?;
        if !response.status().is_success() {
            log::warn!(target: "nons::plugins", "operation=plugins.transfers.download stage=http_status plugin={} generation={} status={}", context.id, context.generation, response.status().as_u16());
            return Err("资源服务器拒绝传输或返回重定向".into());
        }
        let total = response.content_length();
        if max.is_some_and(|max| total.is_some_and(|total| total > max)) {
            log::warn!(target: "nons::plugins", "operation=plugins.transfers.download stage=content_length plugin={} generation={} declared_bytes={} limit_bytes={}", context.id, context.generation, total.unwrap_or_default(), max.unwrap_or_default());
            return Err("资源超过传输大小上限".into());
        }
        snapshot
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.transfers.download.set_total.lock_snapshot",
                    error,
                );
            })
            .map_err(|_| "传输状态不可用")?
            .total = total;
        let mut bytes = 0u64;
        loop {
            while paused.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let chunk = response
                .chunk()
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn_error(
                        "plugins.transfers.download.read_chunk",
                        error,
                    );
                })
                .map_err(|_| "读取资源失败")?;
            let Some(chunk) = chunk else {
                break;
            };
            bytes += chunk.len() as u64;
            if max.is_some_and(|max| bytes > max) {
                log::warn!(target: "nons::plugins", "operation=plugins.transfers.download stage=body_limit plugin={} generation={} received_bytes={bytes} limit_bytes={}", context.id, context.generation, max.unwrap_or_default());
                return Err("资源超过传输大小上限".into());
            }
            while paused.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let data = chunk.to_vec();
            let files = context.files.clone();
            let plugin = context.id.clone();
            let generation = context.generation;
            let active = context.active.clone();
            tauri::async_runtime::spawn_blocking(move || {
                files.write_transfer(&plugin, generation, handle, &data, &active)
            })
            .await
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.transfers.download.write_file.worker_join",
                    error,
                );
            })
            .map_err(|_| "文件写入失败")??;
            snapshot
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.transfers.download.set_progress.lock_snapshot",
                        error,
                    );
                })
                .map_err(|_| "传输状态不可用")?
                .bytes = bytes;
        }
        if bytes == 0 || total.is_some_and(|n| n != bytes) {
            return Err("资源内容不完整".into());
        }
        let files = context.files.clone();
        let plugin = context.id.clone();
        let generation = context.generation;
        let active = context.active.clone();
        tauri::async_runtime::spawn_blocking(move || {
            files.sync_transfer(&plugin, generation, handle, &active)
        })
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn(
                "plugins.transfers.download.sync_file.worker_join",
                error,
            );
        })
        .map_err(|_| "文件保存失败")??;
        Ok(())
    };
    let result = tokio::select! {
        result = transfer => result,
        _ = async { while context.active.load(Ordering::SeqCst) && !cancel.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        } } => Err("传输已取消".into()),
    };
    let _ = (io("files.close", json!({"handle":handle})).await).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("io", error);
    });
    if result.is_err() {
        let _ = (io("files.remove", json!({"root":root,"path":path})).await).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("io", error);
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        path::Path,
    };

    fn fixture(
        body: Vec<u8>,
        delay: Duration,
    ) -> (Arc<super::super::http::Http>, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request);
            let _ = write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            for chunk in body.chunks(16384) {
                std::thread::sleep(delay);
                if socket.write_all(chunk).is_err() {
                    break;
                }
            }
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .resolve("example.org", address)
            .build()
            .unwrap();
        (Arc::new(super::super::http::Http::for_test(client)), server)
    }
    fn generated_fixture(
        total: u64,
    ) -> (Arc<super::super::http::Http>, std::thread::JoinHandle<()>) {
        const CHUNK: usize = 1024 * 1024;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request);
            let _ = write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
            );
            let chunk = vec![0x5a; CHUNK];
            let mut remaining = total;
            while remaining > 0 {
                let size = remaining.min(CHUNK as u64) as usize;
                if socket.write_all(&chunk[..size]).is_err() {
                    break;
                }
                remaining -= size as u64;
            }
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .resolve("example.org", address)
            .build()
            .unwrap();
        (Arc::new(super::super::http::Http::for_test(client)), server)
    }
    fn task(plugin: &str, generation: u64, state: &str, started: std::time::Instant) -> Task {
        let snapshot = Arc::new(Mutex::new(Snapshot {
            id: 1,
            state: state.into(),
            bytes: 0,
            total: None,
            error: None,
        }));
        Task {
            plugin: plugin.into(),
            generation,
            started,
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            snapshot,
        }
    }
    #[test]
    fn max_bytes_is_optional_and_must_be_positive_when_present() {
        assert_eq!(parse_max_bytes(&json!({})).unwrap(), None);
        assert_eq!(parse_max_bytes(&json!({"maxBytes":1})).unwrap(), Some(1));
        assert_eq!(
            parse_max_bytes(&json!({"maxBytes":0})).unwrap_err(),
            "maxBytes 必须为正整数"
        );
        assert_eq!(
            parse_max_bytes(&json!({"maxBytes":"1"})).unwrap_err(),
            "maxBytes 必须为正整数"
        );
    }
    #[test]
    fn completed_history_is_lru_capped_without_evicting_running_tasks() {
        let mut tasks = BTreeMap::new();
        let started = std::time::Instant::now();
        for id in 0..130 {
            tasks.insert(
                id,
                task("other", 1, "completed", started + Duration::from_millis(id)),
            );
        }
        for id in 0..130 {
            tasks.insert(
                1000 + id,
                task("current", 2, "running", started + Duration::from_millis(id)),
            );
        }
        for id in 0..3 {
            tasks.insert(
                2000 + id,
                task(
                    "current",
                    2,
                    "completed",
                    started + Duration::from_millis(id),
                ),
            );
        }
        prune_completed(&mut tasks, "current", 2);
        assert_eq!(tasks.len(), 133);
        assert!((1000..1130).all(|id| tasks.contains_key(&id)));
        assert!((2000..2003).all(|id| tasks.contains_key(&id)));

        for id in 0..130 {
            tasks.insert(
                3000 + id,
                task(
                    "current",
                    2,
                    "completed",
                    started + Duration::from_millis(10 + id),
                ),
            );
        }
        prune_completed(&mut tasks, "current", 2);
        assert_eq!(
            tasks.values().filter(|task| !running(task)).count(),
            MAX_COMPLETED
        );
        assert_eq!(tasks.values().filter(|task| running(task)).count(), 130);
        assert!(!tasks.contains_key(&3000));
        assert!(!tasks.contains_key(&3001));
        assert!(tasks.contains_key(&3002));
    }
    #[test]
    fn wildcard_transfer_writes_selected_absolute_path_without_directory_grant() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let tmp = tempfile::tempdir().unwrap();
                let db = Arc::new(
                    super::super::database::Database::open(Path::new(":memory:")).unwrap(),
                );
                let files = Arc::new(super::super::files::Files::new(tmp.path().join("data"), db));
                let (http, server) = fixture(vec![42; 1000], Duration::ZERO);
                let context = TransferContext {
                    id: "one".into(),
                    generation: 1,
                    unrestricted: true,
                    files,
                    http,
                    active: Arc::new(AtomicBool::new(true)),
                };
                let state = Arc::new(Mutex::new(Snapshot {
                    id: 1,
                    state: "running".into(),
                    bytes: 0,
                    total: None,
                    error: None,
                }));
                let path = tmp.path().join("absolute.part");
                run(
                    &context,
                    url::Url::parse("http://example.org/file").unwrap(),
                    "*",
                    path.to_str().unwrap(),
                    Some(1000),
                    &state,
                    (&AtomicBool::new(false), &AtomicBool::new(false)),
                )
                .await
                .unwrap();
                server.join().unwrap();
                assert_eq!(std::fs::read(path).unwrap(), vec![42; 1000]);
            });
    }
    #[test]
    fn pause_preserves_partial_file_then_resumes_and_cancel_cleans_paused_transfer() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let tmp = tempfile::tempdir().unwrap();
                let db = Arc::new(
                    super::super::database::Database::open(Path::new(":memory:")).unwrap(),
                );
                let files = Arc::new(super::super::files::Files::new(tmp.path().join("data"), db));
                let grant = files.grant("one", tmp.path().into(), true).unwrap();
                for cancel_while_paused in [false, true] {
                    let (http, server) = fixture(vec![42; 100_000], Duration::from_millis(5));
                    let context = TransferContext {
                        unrestricted: false,
                        id: "one".into(),
                        generation: 1,
                        files: files.clone(),
                        http,
                        active: Arc::new(AtomicBool::new(true)),
                    };
                    let snapshot = Arc::new(Mutex::new(Snapshot {
                        id: 1,
                        state: "paused".into(),
                        bytes: 0,
                        total: None,
                        error: None,
                    }));
                    let paused = Arc::new(AtomicBool::new(true));
                    let cancel = Arc::new(AtomicBool::new(false));
                    let pause_toggle = paused.clone();
                    let cancel_toggle = cancel.clone();
                    let tmp_path = tmp.path().join("pause.part");
                    let toggle = tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        assert_eq!(std::fs::metadata(tmp_path).unwrap().len(), 0);
                        if cancel_while_paused {
                            cancel_toggle.store(true, Ordering::SeqCst);
                        } else {
                            pause_toggle.store(false, Ordering::SeqCst);
                        }
                    });
                    let result = run(
                        &context,
                        url::Url::parse("http://example.org/file").unwrap(),
                        &grant.id,
                        "pause.part",
                        Some(100_000),
                        &snapshot,
                        (&cancel, &paused),
                    )
                    .await;
                    toggle.await.unwrap();
                    server.join().unwrap();
                    if cancel_while_paused {
                        assert!(result.is_err());
                        assert!(!tmp.path().join("pause.part").exists());
                    } else {
                        result.unwrap();
                        assert_eq!(
                            std::fs::read(tmp.path().join("pause.part")).unwrap(),
                            vec![42; 100_000]
                        );
                        std::fs::remove_file(tmp.path().join("pause.part")).unwrap();
                    }
                }
            });
    }
    #[test]
    fn unlimited_transfer_streams_past_the_old_512_mib_limit() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                const TOTAL: u64 = 513 * 1024 * 1024;
                let tmp = tempfile::tempdir().unwrap();
                let db = Arc::new(
                    super::super::database::Database::open(Path::new(":memory:")).unwrap(),
                );
                let files = Arc::new(super::super::files::Files::new(tmp.path().join("data"), db));
                let grant = files.grant("one", tmp.path().into(), true).unwrap();
                let (http, server) = generated_fixture(TOTAL);
                let context = TransferContext {
                    unrestricted: false,
                    id: "one".into(),
                    generation: 1,
                    files,
                    http,
                    active: Arc::new(AtomicBool::new(true)),
                };
                let snapshot = Arc::new(Mutex::new(Snapshot {
                    id: 1,
                    state: "running".into(),
                    bytes: 0,
                    total: None,
                    error: None,
                }));
                let result = tokio::time::timeout(
                    Duration::from_secs(120),
                    run(
                        &context,
                        url::Url::parse("http://example.org/large").unwrap(),
                        &grant.id,
                        "large.bin",
                        None,
                        &snapshot,
                        (&AtomicBool::new(false), &AtomicBool::new(false)),
                    ),
                )
                .await
                .unwrap();
                result.unwrap();
                server.join().unwrap();
                assert_eq!(
                    std::fs::metadata(tmp.path().join("large.bin"))
                        .unwrap()
                        .len(),
                    TOTAL
                );
                assert_eq!(snapshot.lock().unwrap().bytes, TOTAL);
            });
    }
    #[test]
    fn streams_bounded_files_and_cleans_cancelled_and_oversized_resources() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let tmp = tempfile::tempdir().unwrap();
                let db = Arc::new(
                    super::super::database::Database::open(Path::new(":memory:")).unwrap(),
                );
                let files = Arc::new(super::super::files::Files::new(tmp.path().join("data"), db));
                let grant = files.grant("one", tmp.path().into(), true).unwrap();
                let active = Arc::new(AtomicBool::new(true));
                let state = Arc::new(Mutex::new(Snapshot {
                    id: 1,
                    state: "running".into(),
                    bytes: 0,
                    total: None,
                    error: None,
                }));
                let (http, server) = fixture(vec![42; 100_000], Duration::ZERO);
                let mut context = TransferContext {
                    unrestricted: false,
                    id: "one".into(),
                    generation: 1,
                    files,
                    http,
                    active,
                };
                let url = url::Url::parse("http://example.org/file").unwrap();
                run(
                    &context,
                    url.clone(),
                    &grant.id,
                    "good.part",
                    Some(100_000),
                    &state,
                    (&AtomicBool::new(false), &AtomicBool::new(false)),
                )
                .await
                .unwrap();
                server.join().unwrap();
                assert_eq!(
                    std::fs::read(tmp.path().join("good.part")).unwrap(),
                    vec![42; 100_000]
                );
                assert_eq!(state.lock().unwrap().bytes, 100_000);
                let (http, server) = fixture(vec![42; 100], Duration::ZERO);
                context.http = http;
                assert!(run(
                    &context,
                    url.clone(),
                    &grant.id,
                    "large.part",
                    Some(99),
                    &state,
                    (&AtomicBool::new(false), &AtomicBool::new(false))
                )
                .await
                .unwrap_err()
                .contains("上限"));
                server.join().unwrap();
                assert!(!tmp.path().join("large.part").exists());
                let (http, server) = fixture(vec![42; 100_000], Duration::from_millis(30));
                context.http = http;
                let cancel = Arc::new(AtomicBool::new(false));
                let cancelled = cancel.clone();
                let toggle = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    cancelled.store(true, Ordering::SeqCst);
                });
                assert!(run(
                    &context,
                    url.clone(),
                    &grant.id,
                    "cancel.part",
                    Some(100_000),
                    &state,
                    (&cancel, &AtomicBool::new(false))
                )
                .await
                .is_err());
                toggle.await.unwrap();
                server.join().unwrap();
                assert!(!tmp.path().join("cancel.part").exists());
                let (http, server) = fixture(vec![42; 100_000], Duration::from_millis(30));
                context.http = http;
                let files = context.files.clone();
                let root = grant.id.clone();
                let revoke = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    files.revoke("one", Some(&root)).unwrap();
                });
                assert!(run(
                    &context,
                    url,
                    &grant.id,
                    "revoke.part",
                    Some(100_000),
                    &state,
                    (&AtomicBool::new(false), &AtomicBool::new(false))
                )
                .await
                .is_err());
                revoke.await.unwrap();
                server.join().unwrap();
                assert!(
                    std::fs::metadata(tmp.path().join("revoke.part"))
                        .unwrap()
                        .len()
                        < 100_000
                );
            });
    }
}
