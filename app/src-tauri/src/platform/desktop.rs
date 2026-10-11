use crate::{application::Backend, model::AppResult};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Window, WindowEvent,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CloseBehavior {
    #[default]
    Close,
    Minimize,
}

fn read_close_behavior(store: &crate::storage::Store) -> AppResult<CloseBehavior> {
    Ok(
        if store.setting("closeBehavior")?.as_deref() == Some("minimize") {
            CloseBehavior::Minimize
        } else {
            CloseBehavior::Close
        },
    )
}

#[tauri::command]
pub(crate) fn close_behavior(backend: tauri::State<'_, Backend>) -> AppResult<CloseBehavior> {
    read_close_behavior(&backend.store)
}

#[tauri::command]
pub(crate) fn set_close_behavior(
    behavior: CloseBehavior,
    backend: tauri::State<'_, Backend>,
) -> AppResult<()> {
    backend.store.set_setting(
        "closeBehavior",
        match behavior {
            CloseBehavior::Close => "close",
            CloseBehavior::Minimize => "minimize",
        },
    )
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
                eprintln!("唤醒已有实例失败：{error}");
            }
        }) {
            eprintln!("唤醒已有实例失败：{error}");
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
                eprintln!("托盘操作失败：{error}");
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
                    eprintln!("托盘操作失败：{error}");
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
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let backend = window.state::<Backend>();
        match read_close_behavior(&backend.store).unwrap_or_default() {
            CloseBehavior::Minimize => {
                let _ = window.hide();
            }
            CloseBehavior::Close => window.app_handle().exit(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_behavior_persists_and_defaults_to_close() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.sqlite3");
        let store = crate::storage::Store::open(&path).unwrap();
        assert_eq!(read_close_behavior(&store).unwrap(), CloseBehavior::Close);
        store.set_setting("closeBehavior", "minimize").unwrap();
        drop(store);
        let store = crate::storage::Store::open(&path).unwrap();
        assert_eq!(
            read_close_behavior(&store).unwrap(),
            CloseBehavior::Minimize
        );
        store.set_setting("closeBehavior", "invalid").unwrap();
        assert_eq!(read_close_behavior(&store).unwrap(), CloseBehavior::Close);
        assert_eq!(
            serde_json::from_str::<CloseBehavior>("\"minimize\"").unwrap(),
            CloseBehavior::Minimize
        );
        assert!(serde_json::from_str::<CloseBehavior>("\"invalid\"").is_err());
    }
}
