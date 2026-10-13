use crate::model::AppResult;
#[cfg(all(debug_assertions, desktop))]
use tauri::Manager;

#[tauri::command]
pub fn open_devtools(app: tauri::AppHandle) -> AppResult<()> {
    #[cfg(all(debug_assertions, desktop))]
    {
        app.get_webview("main")
            .ok_or("主窗口不可用")?
            .open_devtools();
        Ok(())
    }
    #[cfg(not(all(debug_assertions, desktop)))]
    {
        let _ = app;
        Err("发布版不提供开发者工具".into())
    }
}
