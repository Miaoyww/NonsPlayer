use crate::model::{AppResult, ResolvedTrack, Track, TrackSource};

pub(crate) async fn resolve(track: Track) -> AppResult<ResolvedTrack> {
    if let TrackSource::Local { path, .. } = &track.source {
        let decoded_audio = if crate::encoded_audio::is_encoded(std::path::Path::new(path)) {
            let path = path.clone();
            static DECODING: std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>> =
                std::sync::OnceLock::new();
            let permit = DECODING
                .get_or_init(|| std::sync::Arc::new(tokio::sync::Semaphore::new(2)))
                .clone()
                .try_acquire_owned()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "local.playback.decode_admission",
                        error,
                    );
                })
                .map_err(|_| "音频解析繁忙，请稍后重试")?;
            Some(
                tauri::async_runtime::spawn_blocking(move || {
                    let _permit = permit;
                    crate::encoded_audio::prepare(&path)
                })
                .await
                .map_err(|e| e.to_string())??,
            )
        } else {
            None
        };
        let playable_path = decoded_audio
            .as_ref()
            .map(|f| f.path())
            .unwrap_or_else(|| std::path::Path::new(path));
        let uri = url::Url::from_file_path(playable_path)
            .inspect_err(|_| {
                crate::infrastructure::diagnostics::warn(
                    "local.file_uri",
                    "file path cannot be represented as a file URL",
                );
            })
            .map_err(|_| "本地文件路径无效")?
            .to_string();
        if !std::path::Path::new(path).is_file() {
            return Err("本地音乐文件已移动或删除，请重新导入".into());
        }
        return Ok(ResolvedTrack {
            decoded_audio,
            track,
            uri,
            quality: None,
        });
    }
    Err("本地音乐来源无效".into())
}
