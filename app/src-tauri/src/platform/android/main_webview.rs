//! Android keeps its Activity WebView; desktop idle destruction is not used.
use crate::model::AppResult;

#[tauri::command]
pub(crate) async fn main_webview_ready() -> AppResult<serde_json::Value> {
    Ok(serde_json::json!({"settings": false, "shortcut": null}))
}

#[tauri::command]
pub(crate) fn main_sleep_shortcuts() -> AppResult<()> {
    Err("Android 不支持桌面全局快捷键".into())
}
