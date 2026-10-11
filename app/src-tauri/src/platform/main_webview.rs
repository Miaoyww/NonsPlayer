//! The native main window (and its SMTC HWND) outlives the replaceable WebView.
//! All blocking WebView creation runs on this worker, never on a tray/UI callback.
use crate::{application::Backend, model::AppResult, plugins::PluginManager};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewBuilder};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const IDLE_TIMEOUT: Duration = Duration::from_secs(180);

enum Request {
    Open { show: bool, settings: bool },
    Ready(tokio::sync::oneshot::Sender<ReadyState>),
    Shortcut(String),
    Stop,
}

pub(crate) struct MainWebview(Sender<Request>);

#[derive(Clone, Deserialize)]
pub(crate) struct SleepShortcut {
    action: String,
    binding: String,
}
#[derive(Default)]
struct SleepShortcuts(Mutex<Vec<SleepShortcut>>);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadyState {
    settings: bool,
    shortcut: Option<String>,
}

pub(crate) fn setup(app: &tauri::App) -> AppResult<()> {
    if let Some(webview) = app.get_webview("main") {
        configure_browser_shortcuts(&webview)?;
    }
    let (sender, receiver) = mpsc::channel();
    app.manage(MainWebview(sender));
    app.manage(SleepShortcuts::default());
    let app = app.handle().clone();
    std::thread::Builder::new()
        .name("nons-main-webview".into())
        .spawn(move || {
            let mut hidden_since = None;
            let mut ready = false;
            let mut pending_settings = false;
            let mut pending_shortcut = None;
            loop {
                let request = receiver.recv_timeout(Duration::from_secs(1));
                let stopping = matches!(
                    &request,
                    Ok(Request::Stop) | Err(mpsc::RecvTimeoutError::Disconnected)
                );
                let result: AppResult<()> = (|| {
                    match request {
                        Ok(Request::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return Ok(())
                        }
                        Ok(Request::Open { show, settings }) => {
                            pending_settings |= settings;
                            if ensure_webview(&app)? {
                                ready = false;
                            }
                            if show {
                                let window = app.get_window("main").ok_or("主窗口不可用")?;
                                window.unminimize().map_err(|e| e.to_string())?;
                                window.show().map_err(|e| e.to_string())?;
                                window.set_focus().map_err(|e| e.to_string())?;
                                hidden_since = None;
                            }
                            if ready && pending_settings {
                                app.emit_to("main", "open-settings", ())
                                    .map_err(|e| e.to_string())?;
                                pending_settings = false;
                            }
                        }
                        Ok(Request::Ready(reply)) => {
                            ready = true;
                            let _ = reply.send(ReadyState {
                                settings: std::mem::take(&mut pending_settings),
                                shortcut: pending_shortcut.take(),
                            });
                        }
                        Ok(Request::Shortcut(action)) => {
                            if action == "like" {
                                pending_shortcut = Some(action);
                                if ensure_webview(&app)? {
                                    ready = false;
                                }
                                hidden_since = Some(Instant::now());
                            } else {
                                background_control(&app, &action)?;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    let window = app.get_window("main").ok_or("主窗口不可用")?;
                    let hidden = !window.is_visible().map_err(|e| e.to_string())?
                        || window.is_minimized().map_err(|e| e.to_string())?;
                    if !hidden {
                        hidden_since = None;
                        if ensure_webview(&app)? {
                            ready = false;
                        }
                        return Ok(());
                    }
                    let since = *hidden_since.get_or_insert_with(Instant::now);
                    // Resume through SMTC can reactivate a paused FM while the UI sleeps.
                    if app.state::<Backend>().player.needs_private_fm_frontend()? {
                        if ensure_webview(&app)? {
                            ready = false;
                        }
                    } else if since.elapsed() >= IDLE_TIMEOUT {
                        if let Some(webview) = app.get_webview("main") {
                            webview.close().map_err(|e| e.to_string())?;
                            app.state::<std::sync::Arc<PluginManager>>()
                                .frontend_detached()?;
                            ready = false;
                            eprintln!("主面板已隐藏 3 分钟，WebView 已释放");
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    eprintln!("主面板休眠操作失败：{error}");
                }
                if stopping {
                    break;
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn ensure_webview(app: &AppHandle) -> AppResult<bool> {
    if app.get_webview("main").is_some() {
        return Ok(false);
    }
    let window = app.get_window("main").ok_or("主窗口不可用")?;
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .ok_or("主窗口配置不可用")?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    let webview = window
        .add_child(
            WebviewBuilder::from_config(config).auto_resize(),
            PhysicalPosition::new(0, 0),
            size,
        )
        .map_err(|e| e.to_string())?;
    configure_browser_shortcuts(&webview)?;
    Ok(true)
}

fn configure_browser_shortcuts(webview: &tauri::Webview) -> AppResult<()> {
    #[cfg(windows)]
    if !cfg!(debug_assertions) {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
        use windows::core::Interface;

        webview
            .with_webview(|platform| {
                // WebView2 browser accelerators can run before DOM keydown handlers.
                let result = (|| -> windows::core::Result<()> {
                    unsafe {
                        let settings = platform.controller().CoreWebView2()?.Settings()?;
                        settings
                            .cast::<ICoreWebView2Settings3>()?
                            .SetAreBrowserAcceleratorKeysEnabled(false)?;
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    eprintln!("禁用浏览器快捷键失败：{error}");
                }
            })
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    let _ = webview;
    Ok(())
}

pub(crate) fn open(app: &AppHandle, settings: bool) -> AppResult<()> {
    app.state::<MainWebview>()
        .0
        .send(Request::Open {
            show: true,
            settings,
        })
        .map_err(|e| e.to_string())
}

pub(crate) fn stop(app: &AppHandle) {
    if let Some(state) = app.try_state::<MainWebview>() {
        let _ = state.0.send(Request::Stop);
    }
}

#[tauri::command]
pub(crate) async fn main_webview_ready(app: AppHandle) -> AppResult<ReadyState> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.state::<MainWebview>()
        .0
        .send(Request::Ready(sender))
        .map_err(|e| e.to_string())?;
    receiver.await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn main_sleep_shortcuts(bindings: Vec<SleepShortcut>, app: AppHandle) -> AppResult<()> {
    if bindings.len() > 6
        || bindings.iter().any(|s| {
            s.binding.len() > 100
                || !matches!(
                    s.action.as_str(),
                    "toggle" | "previous" | "next" | "like" | "volumeUp" | "volumeDown"
                )
        })
    {
        return Err("快捷键配置无效".into());
    }
    *app.state::<SleepShortcuts>()
        .0
        .lock()
        .map_err(|_| "快捷键配置不可用")? = bindings;
    Ok(())
}

pub(crate) fn background_shortcut(
    app: &AppHandle,
    shortcut: &tauri_plugin_global_shortcut::Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state != tauri_plugin_global_shortcut::ShortcutState::Pressed
        || app.get_webview("main").is_some()
    {
        return;
    }
    let Some(bindings) = app.try_state::<SleepShortcuts>() else {
        return;
    };
    let Ok(bindings) = bindings.0.lock() else {
        return;
    };
    if let Some(binding) = bindings.iter().find(|binding| {
        binding
            .binding
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .is_ok_and(|registered| registered.id() == shortcut.id())
    }) {
        let _ = app
            .state::<MainWebview>()
            .0
            .send(Request::Shortcut(binding.action.clone()));
    }
}

fn background_control(app: &AppHandle, action: &str) -> AppResult<()> {
    use crate::player::Command;
    let player = &app.state::<Backend>().player;
    let snapshot = player.snapshot()?;
    let command = match action {
        "toggle" => Command::Toggle,
        "previous" => Command::Previous,
        "next" => Command::Next,
        "volumeUp" => Command::Volume((snapshot.volume + 0.05).min(1.0)),
        "volumeDown" => Command::Volume((snapshot.volume - 0.05).max(0.0)),
        _ => return Ok(()),
    };
    player.send(command)
}
