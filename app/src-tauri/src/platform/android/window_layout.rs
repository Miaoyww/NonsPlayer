use crate::model::AppResult;

#[tauri::command]
pub(crate) async fn android_window_layout(immersive: bool, dark: bool) -> AppResult<()> {
    super::HANDLE
        .get()
        .ok_or("Android 窗口能力未初始化")?
        .run_mobile_plugin::<serde_json::Value>(
            "windowLayout",
            serde_json::json!({ "immersive": immersive, "dark": dark }),
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}
