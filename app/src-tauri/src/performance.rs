//! Opt-in Release measurements against the real WebView and playback actor.
//! The feature and an explicit absolute workspace are both required.
use crate::{
    application::Backend,
    model::{AppResult, Track, TrackSource},
    player::Command,
};
use serde_json::{json, Value};
use std::{
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        OnceLock,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};

static ORIGIN: OnceLock<Instant> = OnceLock::new();
static ARMED: AtomicBool = AtomicBool::new(false);
static REQUEST: AtomicU64 = AtomicU64::new(0);
static FIRST_OUTPUT: AtomicU64 = AtomicU64::new(0);
static READY: AtomicBool = AtomicBool::new(false);
pub fn startup_ready() {
    READY.store(true, Ordering::Release);
}

pub fn root() -> Option<PathBuf> {
    if !cfg!(all(windows, feature = "performance-probe")) {
        return None;
    }
    std::env::var_os("NONS_PERFORMANCE_ROOT")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}
pub fn start_clock() {
    ORIGIN.get_or_init(Instant::now);
}
fn micros() -> u64 {
    ORIGIN.get_or_init(Instant::now).elapsed().as_micros() as u64
}
#[cfg(feature = "performance-probe")]
pub fn output_buffer() {
    // No allocation, lock, file IO or IPC on the audio thread.
    if ARMED.swap(false, Ordering::AcqRel) {
        FIRST_OUTPUT.store(micros(), Ordering::Release);
    }
}
fn record(event: &str, value: Value) -> AppResult<()> {
    let directory = root().ok_or("未开启性能探测")?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("events.jsonl"))
        .map_err(|e| e.to_string())?;
    writeln!(file, "{}", json!({"event": event, "nativeMs": micros() as f64 / 1000.0,
        "epochMs": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
        "value": value})).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn performance_probe(
    action: String,
    value: Value,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Value> {
    let root = root().ok_or("未开启性能探测")?;
    if cfg!(debug_assertions) || tauri::is_dev() {
        return Err("性能基线必须使用 Release 和嵌入的生产前端".into());
    }
    match action.as_str() {
        "ready" => Ok(json!(READY.load(Ordering::Acquire))),
        "config" => {
            let text =
                std::fs::read_to_string(root.join("config.json")).map_err(|e| e.to_string())?;
            let config: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .map_err(|e| e.to_string())?;
            if !config["sampleSeconds"]
                .as_u64()
                .is_some_and(|v| (5..=120).contains(&v))
                || !config["audioRuns"]
                    .as_u64()
                    .is_some_and(|v| (3..=30).contains(&v))
            {
                return Err("无效测量配置".into());
            }
            Ok(config)
        }
        "mark" => {
            let event = value["event"]
                .as_str()
                .filter(|v| v.len() <= 64)
                .ok_or("无效测量事件")?;
            record(event, value.clone())?;
            Ok(Value::Null)
        }
        "prepare" => {
            let count = value["count"]
                .as_u64()
                .filter(|v| [1, 1000, 10000].contains(v))
                .ok_or("无效队列规模")?;
            let path = root.join("fixture.wav");
            let path = path.to_str().ok_or("无效音频路径")?;
            let tracks: Vec<_> = (0..count)
                .map(|i| Track {
                    key: format!("local:performance:{i}"),
                    title: format!("Baseline {:05}", i + 1),
                    artist: "Nons performance fixture".into(),
                    album: "Deterministic PCM".into(),
                    duration_ms: 300000,
                    cover: String::new().into(),
                    source: TrackSource::Local {
                        path: path.into(),
                        netease_id: None,
                        cue: None,
                    },
                    aliases: vec![],
                    artists: vec![],
                })
                .collect();
            backend.store.save_tracks(&tracks)?;
            backend.store.set_setting(
                "lyricSources",
                r#"{"localSource":"local","amll":false,"qq":false}"#,
            )?;
            backend.store.set_setting("updateAutoCheck", "false")?;
            FIRST_OUTPUT.store(0, Ordering::Release);
            REQUEST.store(micros(), Ordering::Release);
            ARMED.store(true, Ordering::Release);
            backend.player.send(Command::Queue(tracks, 0))?;
            record("play_requested", json!({"count": count}))?;
            Ok(Value::Null)
        }
        "audio" => {
            let output = FIRST_OUTPUT.load(Ordering::Acquire);
            Ok(if output == 0 {
                Value::Null
            } else {
                json!({"firstOutputBufferMs": output.saturating_sub(REQUEST.load(Ordering::Acquire)) as f64 / 1000.0})
            })
        }
        "hide" => {
            app.get_webview_window("main")
                .ok_or("主窗口不可用")?
                .hide()
                .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "show" => {
            app.get_webview_window("main")
                .ok_or("主窗口不可用")?
                .show()
                .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "finish" => {
            record("finished", value)?;
            app.exit(0);
            Ok(Value::Null)
        }
        _ => Err("无效测量操作".into()),
    }
}
