use crate::{
    model::{AppResult, PlayerSnapshot},
    player::Command,
};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::SyncSender;
use tauri::{
    ipc::{Channel, InvokeResponseBody},
    Manager,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub sequence: u64,
    pub key: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: String,
    pub status: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub revision: u64,
    pub can_next: bool,
    pub can_previous: bool,
    pub shuffle: bool,
    pub error: Option<String>,
}
impl Snapshot {
    pub fn from_player(state: &PlayerSnapshot) -> Self {
        let track = state.current();
        Self {
            sequence: {
                static SEQUENCE: std::sync::atomic::AtomicU64 =
                    std::sync::atomic::AtomicU64::new(1);
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            },
            key: track.map_or_else(String::new, |t| t.key.clone()),
            title: track.map_or_else(String::new, |t| t.title.clone()),
            artist: track.map_or_else(String::new, |t| t.artist.clone()),
            album: track.map_or_else(String::new, |t| t.album.clone()),
            cover: track.map_or_else(String::new, |t| {
                t.cover
                    .url(nons_music_adapter_sdk::cover::CoverSize::Exlarge)
                    .to_owned()
            }),
            status: serde_json::to_value(state.status)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned(),
            position_ms: state.position_ms,
            duration_ms: state.duration_ms,
            revision: state.revision,
            can_next: state.following(true).is_some(),
            can_previous: state.previous().is_some(),
            shuffle: state.shuffle,
            error: state.error.clone(),
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    id: u64,
    action: String,
    #[serde(default)]
    position_ms: u64,
    #[serde(default)]
    key: String,
    #[serde(default)]
    favorite_sequence: u64,
    #[serde(default)]
    liked: Option<bool>,
}
fn command(request: &Request) -> AppResult<Command> {
    match request.action.as_str() {
        "play" => Ok(Command::Resume),
        "pause" => Ok(Command::Pause),
        "stop" => Ok(Command::Stop),
        "next" => Ok(Command::Next),
        "previous" => Ok(Command::Previous),
        "seek" => Ok(Command::Seek(request.position_ms)),
        "shuffle" => Ok(Command::Shuffle),
        _ => Err("不支持的媒体命令".into()),
    }
}
pub(crate) fn call(method: &str, args: serde_json::Value) -> AppResult<()> {
    super::HANDLE
        .get()
        .ok_or("Android 播放能力未初始化")?
        .run_mobile_plugin::<serde_json::Value>(method, args)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
pub(crate) fn initialize(sender: SyncSender<Command>) -> AppResult<()> {
    let favorites = super::favorites::initialize();
    let channel: Channel = Channel::new(move |body| {
        let InvokeResponseBody::Json(json) = body else {
            return Ok(());
        };
        let Ok(request) = serde_json::from_str::<Request>(&json) else {
            return Ok(());
        };
        if request.action == "favorite" {
            if favorites
                .try_send(super::favorites::Request {
                    id: request.id,
                    key: request.key,
                    sequence: request.favorite_sequence,
                    liked: request.liked,
                })
                .is_err()
            {
                let _ = (call(
                    "mediaCommandResult",
                    serde_json::json!({"id": request.id, "error": "收藏操作忙碌"}),
                ))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("call", error);
                });
            }
            return Ok(());
        }
        let (reply, receive) = tokio::sync::oneshot::channel();
        let result = command(&request).and_then(|command| {
            sender
                .try_send(Command::Media {
                    command: Box::new(command),
                    reply,
                })
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "播放核心忙碌或已退出".to_owned())
        });
        tauri::async_runtime::spawn(async move {
            let result = match result {
                Ok(()) => tokio::time::timeout(std::time::Duration::from_secs(5), receive)
                    .await
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "媒体命令超时".to_owned())
                    .and_then(|r| {
                        r.inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "播放核心已退出".to_owned())
                    })
                    .and_then(|r| r),
                Err(e) => Err(e),
            };
            let (snapshot, error) = match result {
                Ok(snapshot) => (Some(snapshot), None),
                Err(error) => (None, Some(error)),
            };
            let _ = (call(
                "mediaCommandResult",
                serde_json::json!({"id": request.id, "snapshot": snapshot, "error": error}),
            ))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("call", error);
            });
        });
        Ok(())
    });
    call("mediaInitialize", serde_json::json!({"commands": channel}))
}
pub(crate) fn update(snapshot: &Snapshot) -> AppResult<()> {
    call(
        "playback",
        serde_json::to_value(snapshot).map_err(|e| e.to_string())?,
    )
}
pub(crate) fn shutdown() {
    let _ = (call("mediaShutdown", serde_json::json!({}))).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("call", error);
    });
}
pub(crate) fn debug_status() -> AppResult<serde_json::Value> {
    super::HANDLE
        .get()
        .ok_or("Android 媒体能力未初始化")?
        .run_mobile_plugin("mediaStatus", serde_json::json!({}))
        .map_err(|e| e.to_string())
}
pub(crate) fn artwork(key: String, url: String) {
    let Some(app) = super::APP.get().cloned() else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        let Some(backend) = app.try_state::<crate::Backend>() else {
            return;
        };
        let data = if url.starts_with("http://") || url.starts_with("https://") {
            crate::infrastructure::covers::fetch(url.clone(), &backend)
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_result", error);
                })
                .ok()
        } else {
            let path = std::path::PathBuf::from(&url);
            match (
                tokio::fs::canonicalize(&path).await,
                tokio::fs::canonicalize(&backend.covers).await,
            ) {
                (Ok(path), Ok(root)) if path.starts_with(&root) => {
                    if tokio::fs::metadata(&path)
                        .await
                        .is_ok_and(|m| m.len() <= 4 * 1024 * 1024)
                    {
                        use base64::Engine;
                        tokio::fs::read(path)
                            .await
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("optional_result", error);
                            })
                            .ok()
                            .map(|b| {
                                format!(
                                    "data:image/png;base64,{}",
                                    base64::engine::general_purpose::STANDARD.encode(b)
                                )
                            })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        };
        if let Some(data) = data {
            let _ = (call(
                "mediaArtwork",
                serde_json::json!({"key": key, "cover": url, "data": data}),
            ))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("call", error);
            });
        }
    });
}
