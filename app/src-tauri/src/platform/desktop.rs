use crate::{application::Backend, model::AppResult};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Window, WindowEvent,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CloseBehavior {
    #[default]
    Undecided,
    Close,
    Minimize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CloseAction {
    Close,
    Minimize,
}

#[derive(Default)]
pub(crate) struct CloseRequests(AtomicBool);

fn read_close_behavior(store: &crate::storage::Store) -> AppResult<CloseBehavior> {
    let value = store.setting("closeBehavior").inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("window.close_behavior.read", error);
    })?;
    match value.as_deref() {
        None | Some("undecided") => Ok(CloseBehavior::Undecided),
        Some("close") => Ok(CloseBehavior::Close),
        Some("minimize") => Ok(CloseBehavior::Minimize),
        Some(_) => {
            crate::infrastructure::diagnostics::warn(
                "window.close_behavior.read",
                "关闭行为配置无效，按未定处理",
            );
            Ok(CloseBehavior::Undecided)
        }
    }
}

fn write_close_behavior(store: &crate::storage::Store, behavior: CloseBehavior) -> AppResult<()> {
    store
        .set_setting(
            "closeBehavior",
            match behavior {
                CloseBehavior::Undecided => "undecided",
                CloseBehavior::Close => "close",
                CloseBehavior::Minimize => "minimize",
            },
        )
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("window.close_behavior.save", error);
        })
}

fn remember_close_action(
    store: &crate::storage::Store,
    action: CloseAction,
    remember: bool,
) -> AppResult<Option<CloseBehavior>> {
    if !remember {
        return Ok(None);
    }
    let behavior = match action {
        CloseAction::Close => CloseBehavior::Close,
        CloseAction::Minimize => CloseBehavior::Minimize,
    };
    write_close_behavior(store, behavior)?;
    Ok(Some(behavior))
}

fn notify_close_behavior(app: &AppHandle, behavior: CloseBehavior) {
    if let Err(error) = app.emit("close-behavior-changed", behavior) {
        crate::infrastructure::diagnostics::warn("window.close_behavior.notify", error);
    }
}

#[tauri::command]
pub(crate) fn close_behavior(backend: tauri::State<'_, Backend>) -> AppResult<CloseBehavior> {
    read_close_behavior(&backend.store)
}

#[tauri::command]
pub(crate) fn set_close_behavior(
    behavior: CloseBehavior,
    backend: tauri::State<'_, Backend>,
    app: AppHandle,
) -> AppResult<()> {
    write_close_behavior(&backend.store, behavior)?;
    notify_close_behavior(&app, behavior);
    Ok(())
}

#[tauri::command]
pub(crate) fn take_main_close_request(state: tauri::State<'_, CloseRequests>) -> bool {
    state.0.swap(false, Ordering::SeqCst)
}

#[tauri::command]
pub(crate) fn confirm_main_close(
    action: CloseAction,
    remember: bool,
    window: Window,
    backend: tauri::State<'_, Backend>,
) -> AppResult<()> {
    if let Some(behavior) = remember_close_action(&backend.store, action, remember)? {
        notify_close_behavior(window.app_handle(), behavior);
    }
    match action {
        CloseAction::Close => window.app_handle().exit(0),
        CloseAction::Minimize => window
            .hide()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("window.close_behavior.hide", error);
            })
            .map_err(|error| error.to_string())?,
    }
    Ok(())
}

fn open_main(app: &AppHandle) -> AppResult<()> {
    super::main_webview::open(app, false)
}

pub(crate) fn single_instance() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_single_instance::init(|app, _args, _cwd| {
        let handle = app.clone();
        // Defer callbacks received during startup until application setup completes.
        if let Err(error) = app.run_on_main_thread(move || {
            if let Err(error) = open_main(&handle) {
                log::warn!(target: "nons::window", "唤醒已有实例失败：{error}");
            }
        }) {
            log::warn!(target: "nons::window", "唤醒已有实例失败：{error}");
        }
    })
}

fn menu_action(app: &AppHandle, action: &str) -> AppResult<()> {
    match action {
        "open" => open_main(app)?,
        "settings" => {
            super::main_webview::open(app, true)?;
        }
        "exit" => app.exit(0),
        _ => {}
    }
    Ok(())
}

pub(crate) fn setup(app: &tauri::App) -> tauri::Result<()> {
    app.manage(CloseRequests::default());
    let open = MenuItem::with_id(app, "open", "打开", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &exit])?;
    let mut tray = TrayIconBuilder::with_id("nons-tray")
        .tooltip("NonsPlayer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Err(error) = menu_action(app, event.id().as_ref()) {
                log::warn!(target: "nons::window", "托盘操作失败：{error}");
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                if let Err(error) = open_main(tray.app_handle()) {
                    log::warn!(target: "nons::window", "托盘操作失败：{error}");
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if let WindowEvent::ThemeChanged(theme) = event {
        window.state::<Backend>().player.theme_changed(*theme);
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let backend = window.state::<Backend>();
        match read_close_behavior(&backend.store).unwrap_or_default() {
            CloseBehavior::Undecided => {
                window
                    .state::<CloseRequests>()
                    .0
                    .store(true, Ordering::SeqCst);
                if let Err(error) = window.emit("main-close-requested", ()) {
                    crate::infrastructure::diagnostics::warn(
                        "window.close_behavior.request",
                        error,
                    );
                }
                if window.app_handle().get_webview("main").is_none() {
                    if let Err(error) = open_main(window.app_handle()) {
                        crate::infrastructure::diagnostics::warn(
                            "window.close_behavior.open",
                            error,
                        );
                    }
                }
            }
            CloseBehavior::Minimize => {
                let _ = (window.hide()).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("window.hide", error);
                });
            }
            CloseBehavior::Close => window.app_handle().exit(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_behavior_defaults_to_undecided_and_preserves_explicit_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.sqlite3");
        let store = crate::storage::Store::open(&path).unwrap();
        assert_eq!(
            read_close_behavior(&store).unwrap(),
            CloseBehavior::Undecided
        );
        store.set_setting("closeBehavior", "minimize").unwrap();
        drop(store);
        let store = crate::storage::Store::open(&path).unwrap();
        assert_eq!(
            read_close_behavior(&store).unwrap(),
            CloseBehavior::Minimize
        );
        store.set_setting("closeBehavior", "invalid").unwrap();
        assert_eq!(
            read_close_behavior(&store).unwrap(),
            CloseBehavior::Undecided
        );
        write_close_behavior(&store, CloseBehavior::Close).unwrap();
        assert_eq!(read_close_behavior(&store).unwrap(), CloseBehavior::Close);
        write_close_behavior(&store, CloseBehavior::Undecided).unwrap();
        drop(store);
        let store = crate::storage::Store::open(&path).unwrap();
        assert_eq!(
            read_close_behavior(&store).unwrap(),
            CloseBehavior::Undecided
        );
        assert_eq!(
            serde_json::from_str::<CloseBehavior>("\"minimize\"").unwrap(),
            CloseBehavior::Minimize
        );
        assert!(serde_json::from_str::<CloseBehavior>("\"invalid\"").is_err());
        assert_eq!(
            serde_json::from_str::<CloseBehavior>("\"undecided\"").unwrap(),
            CloseBehavior::Undecided
        );
        assert!(serde_json::from_str::<CloseAction>("\"undecided\"").is_err());
    }

    #[test]
    fn closing_without_remembering_keeps_asking_and_remembered_actions_survive_restart() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.sqlite3");
        let store = crate::storage::Store::open(&path).unwrap();
        for action in [CloseAction::Close, CloseAction::Minimize] {
            assert!(remember_close_action(&store, action, false)
                .unwrap()
                .is_none());
            assert!(store.setting("closeBehavior").unwrap().is_none());
        }
        assert_eq!(
            remember_close_action(&store, CloseAction::Minimize, true).unwrap(),
            Some(CloseBehavior::Minimize)
        );
        drop(store);
        let store = crate::storage::Store::open(&path).unwrap();
        assert_eq!(
            read_close_behavior(&store).unwrap(),
            CloseBehavior::Minimize
        );
        remember_close_action(&store, CloseAction::Close, true).unwrap();
        assert_eq!(read_close_behavior(&store).unwrap(), CloseBehavior::Close);
    }

    #[test]
    fn failing_to_remember_an_action_returns_an_error_and_preserves_the_preference() {
        let directory = tempfile::tempdir().unwrap();
        let store =
            crate::storage::Store::open(&directory.path().join("settings.sqlite3")).unwrap();
        store
            .0
            .lock()
            .unwrap()
            .execute_batch("PRAGMA query_only = ON")
            .unwrap();
        assert!(remember_close_action(&store, CloseAction::Close, true).is_err());
        assert_eq!(
            read_close_behavior(&store).unwrap(),
            CloseBehavior::Undecided
        );
    }
}
