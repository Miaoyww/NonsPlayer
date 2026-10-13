use crate::model::AppResult;

pub(crate) fn update(active: bool, title: &str, artist: &str) -> AppResult<()> {
    super::HANDLE
        .get()
        .ok_or("Android 播放能力未初始化")?
        .run_mobile_plugin::<serde_json::Value>(
            "playback",
            serde_json::json!({ "active": active, "title": title, "artist": artist }),
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}
