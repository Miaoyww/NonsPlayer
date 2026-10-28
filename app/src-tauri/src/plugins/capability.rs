use super::{database::Database, music::SongService};
use crate::{
    model::AppResult,
    player::{Command, Player},
};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::Emitter;

pub const MAX_JSON: usize = 64 * 1024;
#[derive(Clone)]
pub struct Context {
    pub id: String,
    pub generation: u64,
    pub permissions: HashSet<String>,
    pub http_hosts: Vec<String>,
    pub file_roots: Vec<String>,
    pub http: Arc<super::http::Http>,
    pub transfers: Arc<super::transfers::Transfers>,
    pub processes: Arc<super::processes::Processes>,
    pub windows: Arc<super::windows::Windows>,
    pub window_exports: std::collections::BTreeMap<String, String>,
    pub active: Arc<AtomicBool>,
    pub database: Arc<Database>,
    pub configurations: Arc<super::configuration::Configurations>,
    pub files: Arc<super::files::Files>,
    pub songs: Arc<SongService>,
    pub player: Arc<Player>,
    pub app: tauri::AppHandle,
    pub event_budget: Arc<std::sync::Mutex<(Instant, usize)>>,
    pub diagnostic_budget: Arc<std::sync::Mutex<(Instant, usize)>>,
    pub requests: Arc<tokio::sync::Semaphore>,
}
impl Context {
    async fn file_location(&self, args: Value) -> AppResult<Value> {
        let context = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            context.files.call_with_access(
                &context.id,
                context.generation,
                "files.resolve",
                &args,
                &context.active,
                context.file_roots.iter().any(|r| r == "*"),
            )
        })
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.files.resolve.worker_join", error);
        })
        .map_err(|_| "文件路径解析失败")?
    }
    pub fn check(&self, permission: Option<&str>) -> AppResult<()> {
        if !self.active.load(Ordering::SeqCst) {
            return Err("插件已卸载或禁用".into());
        }
        if permission.is_some_and(|p| !self.permissions.contains(p)) {
            return Err("插件缺少所需权限".into());
        }
        Ok(())
    }
    pub async fn call(&self, operation: &str, args: &str) -> AppResult<String> {
        if args.len() > MAX_JSON {
            return Err("插件请求过大".into());
        }
        let args: Value = serde_json::from_str(args)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.call.parse_arguments", error);
            })
            .map_err(|_| "插件参数必须为 JSON")?;
        let result = match operation {
            op if op.starts_with("process.") => {
                self.check(Some("process:spawn"))?;
                self.processes.call(self, operation, &args).await?
            }
            op if op.starts_with("windows.") => {
                self.check(Some("window:manage"))?;
                self.windows.call(self, operation, &args).await?
            }
            "diagnostic.report" => {
                self.check(None)?;
                super::diagnostics::report(&self.id, &self.diagnostic_budget, args)?;
                Value::Null
            }
            "files.pick-audio" => super::audio_files::pick(self, &args).await?,
            "accounts.credential-read" => {
                self.check(Some("account:credentials"))?;
                self.songs.account_credentials(&args).await?
            }
            "clipboard.read-text" => {
                self.check(Some("clipboard:read"))?;
                let app = self.app.clone();
                let text =
                    tauri::async_runtime::spawn_blocking(move || super::clipboard::read_text(&app))
                        .await
                        .map_err(|e| e.to_string())??;
                self.check(Some("clipboard:read"))?;
                Value::String(text)
            }
            "http.request" => {
                self.check(Some("http:request"))?;
                self.http
                    .request(&self.http_hosts, &args, &self.active)
                    .await?
            }
            op if op.starts_with("secrets.") => {
                self.check(Some("secrets"))?;
                let id = self.id.clone();
                let op = op.to_string();
                let active = self.active.clone();
                let database = self.database.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if !active.load(Ordering::SeqCst) {
                        return Err("插件已停用".into());
                    }
                    super::secrets::call(&id, &op, &args, &database, &active)
                })
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.secrets.call.worker_join",
                        error,
                    );
                })
                .map_err(|_| "凭据操作失败")??
            }
            "transfers.start" | "transfers.get" | "transfers.cancel" | "transfers.pause"
            | "transfers.resume" => {
                self.check(Some("http:transfer"))?;
                self.transfers.call(self.clone(), operation, &args)?
            }
            "config.get" | "config.update" | "config.reset" => {
                self.check(Some("config"))?;
                let snapshot = if operation == "config.get" {
                    self.configurations.snapshot(&self.id)?
                } else {
                    let revision = args
                        .get("revision")
                        .and_then(Value::as_u64)
                        .ok_or("缺少配置修订号")?;
                    let patch = if operation == "config.update" {
                        Some(
                            args.get("patch")
                                .and_then(Value::as_object)
                                .ok_or("配置修改必须为对象")?
                                .clone(),
                        )
                    } else {
                        None
                    };
                    let keys = args
                        .get("keys")
                        .map(|v| {
                            serde_json::from_value(v.clone())
                                .inspect_err(|error| {
                                    crate::infrastructure::diagnostics::warn(
                                        "plugin.configuration.keys_decode",
                                        error,
                                    );
                                })
                                .map_err(|_| "配置键列表无效")
                        })
                        .transpose()?;
                    self.configurations
                        .change(&self.id, revision, patch, keys)?
                };
                serde_json::to_value(snapshot).map_err(|e| e.to_string())?
            }
            op if op.starts_with("files.") => {
                self.check(None)?;
                if matches!(
                    op,
                    "files.read" | "files.write" | "files.close" | "files.truncate" | "files.roots"
                ) {
                    if !self.permissions.contains("files:data")
                        && !self.permissions.contains("files:selected")
                    {
                        return Err("插件缺少文件权限".into());
                    }
                } else {
                    self.check(Some(
                        if args.get("root").and_then(Value::as_str) == Some("data") {
                            "files:data"
                        } else {
                            "files:selected"
                        },
                    ))?;
                }
                if op == "files.play" {
                    self.check(Some("player:control"))?;
                    let location = self.file_location(args.clone()).await?;
                    let mut track: crate::model::Track =
                        serde_json::from_value(args.get("track").ok_or("缺少歌曲信息")?.clone())
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn(
                                    "plugins.files.play.parse_track",
                                    error,
                                );
                            })
                            .map_err(|_| "歌曲信息无效")?;
                    let path = location["path"]
                        .as_str()
                        .ok_or("文件路径不可用")?
                        .to_string();
                    use sha2::{Digest, Sha256};
                    track.key = format!("local:{:x}", Sha256::digest(path.as_bytes()));
                    track.source = crate::model::TrackSource::Local {
                        path,
                        netease_id: None,
                        cue: None,
                    };
                    self.check(Some("player:control"))?;
                    self.player.send(Command::Queue(vec![track], 0))?;
                    Value::Null
                } else if op == "files.open-directory" {
                    let location = self
                        .file_location(json!({"root":args.get("root"),"path":if args.get("root").and_then(Value::as_str) == Some("*") { args.get("path").and_then(Value::as_str).ok_or("任意目录访问须提供绝对路径")? } else { "" }}))
                        .await?;
                    self.check(None)?;
                    tauri_plugin_opener::OpenerExt::opener(&self.app)
                        .open_path(location["path"].as_str().ok_or("目录不可用")?, None::<&str>)
                        .map_err(|e| e.to_string())?;
                    Value::Null
                } else if op == "files.roots" {
                    let mut roots = Vec::new();
                    if self.permissions.contains("files:data") {
                        roots.push(serde_json::json!({"id":"data","writable":true}));
                    }
                    if self.file_roots.iter().any(|r| r == "*") {
                        roots.push(json!({"id":"*","writable":true}));
                    }
                    if self.permissions.contains("files:selected") {
                        roots.extend(
                            self.files
                                .grants(&self.id)?
                                .into_iter()
                                .map(|g| serde_json::json!({"id":g.id,"writable":g.writable})),
                        );
                    }
                    serde_json::to_value(roots).map_err(|e| e.to_string())?
                } else {
                    let files = self.files.clone();
                    let id = self.id.clone();
                    let generation = self.generation;
                    let active = self.active.clone();
                    let operation = op.to_string();
                    let unrestricted = self.file_roots.iter().any(|r| r == "*");
                    tauri::async_runtime::spawn_blocking(move || {
                        files.call_with_access(
                            &id,
                            generation,
                            &operation,
                            &args,
                            &active,
                            unrestricted,
                        )
                    })
                    .await
                    .map_err(|e| e.to_string())??
                }
            }
            "music.sources" => {
                self.check(Some("music:metadata"))?;
                serde_json::to_value(
                    self.songs
                        .music
                        .adapters
                        .descriptors()
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
            }
            "music.read-track" => {
                self.check(Some("music:metadata"))?;
                let reference: crate::music::identity::EntityRef =
                    serde_json::from_value(args.get("reference").ok_or("缺少曲目引用")?.clone())
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.music.read_track.parse_reference",
                                error,
                            );
                        })
                        .map_err(|_| "曲目引用无效")?;
                serde_json::to_value(self.songs.read_track(&reference).await?)
                    .map_err(|e| e.to_string())?
            }
            "music.query" | "music.write" => {
                let source: crate::music::identity::SourceId =
                    serde_json::from_value(args.get("source").ok_or("缺少音乐来源")?.clone())
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.music.query.parse_source",
                                error,
                            );
                        })
                        .map_err(|_| "音乐来源无效")?;
                let request: crate::music::business::BusinessRequest =
                    serde_json::from_value(args.get("request").ok_or("缺少音乐请求")?.clone())
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.music.query.parse_request",
                                error,
                            );
                        })
                        .map_err(|_| "音乐请求无效")?;
                self.check(Some(if request.is_write() {
                    "music:write"
                } else if request.requires_account() {
                    "music:library"
                } else {
                    "music:metadata"
                }))?;
                if request.is_write() != (operation == "music.write") {
                    return Err("音乐操作类型无效".into());
                }
                let response = self
                    .songs
                    .music
                    .query(
                        &source,
                        &request,
                        args.get("refresh")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                self.check(None)?;
                if request.is_write() {
                    let _ = (self.app.emit("music-changed", &source)).inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("self.app.emit", error);
                    });
                }
                serde_json::to_value(response).map_err(|e| e.to_string())?
            }
            "events.emit" => {
                self.check(None)?;
                let name = text(&args, "event")?;
                if name.len() > 64
                    || name.is_empty()
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                {
                    return Err("事件名称无效".into());
                }
                let mut budget = self
                    .event_budget
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "plugins.events.emit.lock_budget",
                            error,
                        );
                    })
                    .map_err(|_| "事件限制不可用")?;
                if budget.0.elapsed() >= Duration::from_secs(1) {
                    *budget = (Instant::now(), 0);
                }
                budget.1 += 1;
                if budget.1 > 32 {
                    return Err("插件事件发送过于频繁".into());
                }
                self.app.emit("plugin-event", json!({"pluginId":self.id,"generation":self.generation,"namespace":format!("plugin:{}:{name}",self.id),"event":name,"payload":args.get("payload").unwrap_or(&Value::Null)})).map_err(|e| e.to_string())?;
                Value::Null
            }
            "storage.get" | "storage.set" | "storage.delete" => {
                self.check(Some("storage"))?;
                let key = text(&args, "key")?;
                match operation {
                    "storage.get" => self
                        .database
                        .get(&self.id, key)?
                        .map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
                        .transpose()?
                        .unwrap_or(Value::Null),
                    "storage.set" => {
                        self.database.set(
                            &self.id,
                            key,
                            &serde_json::to_string(args.get("value").ok_or("缺少存储值")?)
                                .map_err(|e| e.to_string())?,
                        )?;
                        Value::Null
                    }
                    _ => {
                        self.database.delete(&self.id, key)?;
                        Value::Null
                    }
                }
            }
            "player.read" => {
                self.check(Some("player:read"))?;
                let state = self.player.snapshot()?;
                json!({"status":state.status,"positionMs":state.position_ms,"durationMs":state.duration_ms,"volume":state.volume,"track":state.current().map(|t| json!({"title":t.title,"artist":t.artist,"album":t.album,"durationMs":t.duration_ms,"cover":if t.cover.starts_with("http") { &*t.cover } else { "" }}))})
            }
            "music.play-track" => {
                self.check(Some("player:control"))?;
                self.check(Some("music:metadata"))?;
                let next = match text(&args, "mode")? {
                    "now" => false,
                    "next" => true,
                    _ => return Err("不支持的播放方式".into()),
                };
                let reference =
                    serde_json::from_value(args.get("reference").ok_or("缺少曲目引用")?.clone())
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.player.enqueue.parse_reference",
                                error,
                            );
                        })
                        .map_err(|_| "曲目引用无效")?;
                let track =
                    crate::model::Track::from_music(self.songs.read_track(&reference).await?)
                        .map_err(|e| e.to_string())?;
                self.check(Some("player:control"))?;
                self.player.send(if next {
                    Command::PlayNext(vec![track])
                } else {
                    Command::Queue(vec![track], 0)
                })?;
                Value::Null
            }
            "player.control" => {
                self.check(Some("player:control"))?;
                let command = match text(&args, "action")? {
                    "pause" => Command::Pause,
                    "resume" => Command::Resume,
                    "next" => Command::Next,
                    "previous" => Command::Previous,
                    "stop" => Command::Stop,
                    _ => return Err("不支持的播放动作".into()),
                };
                self.player.send(command)?;
                Value::Null
            }
            _ => return Err("未知 Host Capability".into()),
        };
        self.check(None)?;
        let result = serde_json::to_string(&result).map_err(|e| e.to_string())?;
        let limit = if operation.starts_with("config.") {
            MAX_JSON * 2
        } else {
            MAX_JSON
        };
        if result.len() > limit {
            return Err("插件响应过大".into());
        }
        Ok(result)
    }
}
fn text<'a>(value: &'a Value, key: &str) -> AppResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("缺少参数 {key}"))
}
