//! Always-on-top desktop lyric window.
//!
//! Playback state, progress and lyric bodies stay on the existing Tauri IPC
//! channels. This module only owns the native window lifecycle, click-through
//! and persisted position.
use crate::{application::Backend, model::AppResult};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tokio::time::sleep;

const LABEL: &str = "desktop-lyrics";
const WIDTH: f64 = 1100.0;
const DEFAULT_HEIGHT: f64 = 160.0;
const MIN_HEIGHT: f64 = 96.0;
const MAX_HEIGHT: f64 = 320.0;
const POSITION_KEY: &str = "desktopLyricsPosition";
const ENABLED_KEY: &str = "desktopLyricsEnabled";
const LOCKED_KEY: &str = "desktopLyricsLocked";
const EVENT: &str = "desktop-lyrics-changed";
const HOVER_EVENT: &str = "desktop-lyrics-hover-changed";
const SAVE_INTERVAL: Duration = Duration::from_millis(250);
const UNLOCK_HOVER_SIZE: f64 = 44.0;
const UNLOCK_HOVER_RIGHT: f64 = 44.0;
const UNLOCK_HOVER_TOP: f64 = 3.0;
const UNLOCK_HOVER_POLL: Duration = Duration::from_millis(40);

static WINDOW_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
struct UnlockHover {
    epoch: u64,
    window_hovered: bool,
    interactive: bool,
    revision: u64,
}

static UNLOCK_HOVER: Mutex<UnlockHover> = Mutex::new(UnlockHover {
    epoch: 0,
    window_hovered: false,
    interactive: false,
    revision: 0,
});

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct State {
    pub enabled: bool,
    pub locked: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HoverState {
    pub hovered: bool,
    pub revision: u64,
}

fn read_flag(store: &crate::storage::Store, key: &str, fallback: bool) -> bool {
    match store.setting(key) {
        Ok(Some(value)) => value == "true",
        _ => fallback,
    }
}

fn state(app: &AppHandle) -> State {
    let Some(backend) = app.try_state::<Backend>() else {
        return State {
            enabled: false,
            locked: true,
        };
    };
    State {
        enabled: read_flag(&backend.store, ENABLED_KEY, false),
        locked: read_flag(&backend.store, LOCKED_KEY, true),
    }
}

fn publish(app: &AppHandle) {
    let _ = app.emit(EVENT, state(app));
}

fn save_position(app: &AppHandle, position: PhysicalPosition<i32>) {
    let Some(backend) = app.try_state::<Backend>() else {
        return;
    };
    let value = serde_json::json!({ "x": position.x, "y": position.y });
    if let Err(error) = backend.store.set_setting(POSITION_KEY, &value.to_string()) {
        log::warn!(target: "nons::window", "保存桌面歌词位置失败：{error}");
    }
}

fn restored_position(app: &AppHandle) -> Option<PhysicalPosition<i32>> {
    let backend = app.try_state::<Backend>()?;
    let raw = backend.store.setting(POSITION_KEY).ok().flatten()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let x = value.get("x")?.as_i64()? as i32;
    let y = value.get("y")?.as_i64()? as i32;
    // Only restore positions that still fall on an attached monitor.
    app.monitor_from_point(x as f64, y as f64).ok().flatten()?;
    Some(PhysicalPosition::new(x, y))
}

fn default_position(app: &AppHandle) -> Option<PhysicalPosition<i32>> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let position = monitor.position();
    let size = monitor.size();
    let width = (WIDTH * scale) as i32;
    let height = (DEFAULT_HEIGHT * scale) as i32;
    let margin = (80.0 * scale) as i32;
    Some(PhysicalPosition::new(
        position.x + (size.width as i32 - width) / 2,
        position.y + size.height as i32 - height - margin,
    ))
}

fn cursor_zone_state(window: &WebviewWindow) -> Option<(bool, bool)> {
    let (Ok(cursor), Ok(position), Ok(size), Ok(scale)) = (
        window.cursor_position(),
        window.outer_position(),
        window.outer_size(),
        window.scale_factor(),
    ) else {
        return None;
    };
    let left = position.x as f64;
    let top = position.y as f64;
    let right = left + size.width as f64;
    let bottom = top + size.height as f64;
    let window_hovered =
        cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom;
    if !window_hovered {
        return Some((false, false));
    }
    let size = UNLOCK_HOVER_SIZE * scale;
    let zone_right = right - UNLOCK_HOVER_RIGHT * scale;
    let zone_top = top + UNLOCK_HOVER_TOP * scale;
    Some((
        true,
        cursor.x >= zone_right - size
            && cursor.x <= zone_right
            && cursor.y >= zone_top
            && cursor.y <= zone_top + size,
    ))
}

fn watch_unlock_hover(window: WebviewWindow, epoch: u64) {
    tauri::async_runtime::spawn(async move {
        loop {
            sleep(UNLOCK_HOVER_POLL).await;
            let Some((window_hovered, interactive)) = cursor_zone_state(&window) else {
                continue;
            };
            let Ok(mut hover) = UNLOCK_HOVER.lock() else {
                return;
            };
            if hover.epoch != epoch {
                return;
            }
            let hover_changed = hover.window_hovered != window_hovered;
            if hover_changed {
                hover.window_hovered = window_hovered;
                hover.revision = hover.revision.wrapping_add(1);
            }
            if hover.interactive != interactive {
                if window.set_ignore_cursor_events(!interactive).is_err() {
                    return;
                }
                hover.interactive = interactive;
            }
            let value = HoverState {
                hovered: hover.window_hovered,
                revision: hover.revision,
            };
            drop(hover);
            if hover_changed {
                let _ = window.emit(HOVER_EVENT, value);
            }
        }
    });
}

fn stop_unlock_hover() {
    if let Ok(mut hover) = UNLOCK_HOVER.lock() {
        hover.epoch = hover.epoch.wrapping_add(1);
        hover.window_hovered = false;
        hover.interactive = false;
        hover.revision = hover.revision.wrapping_add(1);
    }
}

fn set_window_lock(window: &WebviewWindow, locked: bool) -> AppResult<()> {
    let (epoch, hover_state) = {
        let mut hover = UNLOCK_HOVER
            .lock()
            .map_err(|_| "桌面歌词窗口不可用".to_string())?;
        hover.epoch = hover.epoch.wrapping_add(1);
        hover.window_hovered = false;
        hover.interactive = false;
        hover.revision = hover.revision.wrapping_add(1);
        window
            .set_ignore_cursor_events(locked)
            .map_err(|error| error.to_string())?;
        (
            hover.epoch,
            HoverState {
                hovered: false,
                revision: hover.revision,
            },
        )
    };
    let _ = window.emit(HOVER_EVENT, hover_state);
    if locked {
        watch_unlock_hover(window.clone(), epoch);
    }
    Ok(())
}

fn on_window_event(app: &AppHandle, last_save: &Mutex<Instant>, event: &WindowEvent) {
    match event {
        WindowEvent::Moved(position) => {
            let Ok(mut last) = last_save.lock() else {
                return;
            };
            if last.elapsed() >= SAVE_INTERVAL {
                *last = Instant::now();
                save_position(app, *position);
            }
        }
        WindowEvent::CloseRequested { .. } => {
            if let Some(window) = app.get_webview_window(LABEL) {
                if let Ok(position) = window.outer_position() {
                    save_position(app, position);
                }
            }
            // App exit destroys windows without CloseRequested, so an explicit
            // close is the user-visible "disable desktop lyrics" action.
            if let Some(backend) = app.try_state::<Backend>() {
                let _ = backend.store.set_setting(ENABLED_KEY, "false");
            }
            stop_unlock_hover();
            publish(app);
        }
        WindowEvent::Destroyed => {
            stop_unlock_hover();
            publish(app);
        }
        _ => {}
    }
}

fn build(app: &AppHandle) -> AppResult<WebviewWindow> {
    let _guard = WINDOW_LOCK.lock().map_err(|_| "桌面歌词窗口不可用")?;
    let locked = read_flag(&app.state::<Backend>().store, LOCKED_KEY, true);
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show().map_err(|error| error.to_string())?;
        set_window_lock(&window, locked)?;
        return Ok(window);
    }
    let mut builder =
        WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("desktop-lyrics.html".into()))
            .title("桌面歌词")
            .inner_size(WIDTH, DEFAULT_HEIGHT)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .shadow(false)
            .resizable(false)
            .focused(false)
            .visible(false)
            .background_color(tauri::webview::Color(0, 0, 0, 0))
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);
    // WebView2 reuses the environment for a data directory only when the
    // browser arguments match. Sharing the main profile keeps lyric and font
    // settings available through the existing localStorage transport.
    #[cfg(windows)]
    if let Some(args) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .and_then(|window| window.additional_browser_args.as_deref())
    {
        builder = builder.additional_browser_args(args);
    }
    if let Some(data) = crate::infrastructure::directories::portable() {
        builder = builder.data_directory(data.join("webview"));
    }
    let window = builder.build().map_err(|error| error.to_string())?;
    let event_app = app.clone();
    let last_save = Arc::new(Mutex::new(Instant::now() - SAVE_INTERVAL));
    window.on_window_event(move |event| on_window_event(&event_app, &last_save, event));
    let position = restored_position(app).or_else(|| default_position(app));
    if let Some(position) = position {
        window
            .set_position(position)
            .map_err(|error| error.to_string())?;
    }
    set_window_lock(&window, locked)?;
    window.show().map_err(|error| error.to_string())?;
    Ok(window)
}

#[tauri::command]
pub(crate) fn desktop_lyrics_state(app: AppHandle) -> State {
    state(&app)
}

#[tauri::command]
pub(crate) fn desktop_lyrics_hover_state() -> HoverState {
    UNLOCK_HOVER
        .lock()
        .map(|hover| HoverState {
            hovered: hover.window_hovered,
            revision: hover.revision,
        })
        .unwrap_or(HoverState {
            hovered: false,
            revision: 0,
        })
}

#[tauri::command]
pub(crate) async fn desktop_lyrics_set_enabled(enabled: bool, app: AppHandle) -> AppResult<State> {
    app.state::<Backend>()
        .store
        .set_setting(ENABLED_KEY, if enabled { "true" } else { "false" })?;
    if enabled {
        let handle = app.clone();
        tauri::async_runtime::spawn_blocking(move || build(&handle))
            .await
            .map_err(|error| error.to_string())??;
    } else if let Some(window) = app.get_webview_window(LABEL) {
        window.close().map_err(|error| error.to_string())?;
    }
    let value = state(&app);
    let _ = app.emit(EVENT, value);
    Ok(value)
}

#[tauri::command]
pub(crate) fn desktop_lyrics_set_locked(locked: bool, app: AppHandle) -> AppResult<State> {
    app.state::<Backend>()
        .store
        .set_setting(LOCKED_KEY, if locked { "true" } else { "false" })?;
    if let Some(window) = app.get_webview_window(LABEL) {
        set_window_lock(&window, locked)?;
    }
    let value = state(&app);
    let _ = app.emit(EVENT, value);
    Ok(value)
}

#[tauri::command]
pub(crate) fn desktop_lyrics_set_height(height: f64, window: WebviewWindow) -> AppResult<()> {
    if window.label() != LABEL {
        return Err("桌面歌词窗口不可用".into());
    }
    let height = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let outer = window.outer_size().map_err(|error| error.to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let physical_height = (height * scale).round() as u32;
    if outer.height == physical_height {
        return Ok(());
    }
    let delta = outer.height as i32 - physical_height as i32;
    window
        .set_size(PhysicalSize::new(outer.width, physical_height))
        .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition::new(position.x, position.y + delta / 2))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn desktop_lyrics_begin_drag(window: WebviewWindow) -> AppResult<()> {
    if window.label() != LABEL {
        return Err("桌面歌词窗口不可用".into());
    }
    window.start_dragging().map_err(|error| error.to_string())
}
