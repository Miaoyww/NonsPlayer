use super::{
    api::{check_queue_revision, ApiError, Provider, ProviderFuture, MAX_COVER_BYTES},
    dto::{public_track_id, CoverAsset, LyricsAsset, PlayerCommand},
};
use crate::{
    model::Track,
    playback::player::{Command, Player},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct BackendProvider {
    app: tauri::AppHandle,
    player: Arc<Player>,
    adapters: Arc<crate::music::manager::AdapterManager>,
    cover_client: reqwest::Client,
    cover_requests: Arc<tokio::sync::Semaphore>,
    online_covers: Arc<crate::infrastructure::covers::CoverCache>,
    covers: PathBuf,
}

impl BackendProvider {
    pub fn new(backend: &crate::Backend, app: tauri::AppHandle) -> Self {
        Self {
            app,
            player: backend.player.clone(),
            adapters: backend.music.adapters.clone(),
            cover_client: backend.cover_client.clone(),
            cover_requests: backend.cover_requests.clone(),
            online_covers: backend.online_covers.clone(),
            covers: backend.covers.clone(),
        }
    }

    fn map_command_error(error: String) -> ApiError {
        crate::infrastructure::diagnostics::warn("open_api.command", &error);
        if error.contains("已变化") {
            ApiError::conflict("播放队列已变化，请重新同步")
        } else if error.contains("不在播放队列") {
            ApiError::not_found("曲目不在播放队列中")
        } else if error.contains("忙碌") {
            ApiError::busy()
        } else if error.contains("不可用") {
            ApiError::unavailable()
        } else {
            ApiError::invalid("播放命令无法执行")
        }
    }

    fn mime(bytes: &[u8], path: &str) -> String {
        match image::guess_format(bytes) {
            Ok(image::ImageFormat::Jpeg) => "image/jpeg".into(),
            Ok(image::ImageFormat::Png) => "image/png".into(),
            Ok(image::ImageFormat::Gif) => "image/gif".into(),
            Ok(image::ImageFormat::WebP) => "image/webp".into(),
            Ok(image::ImageFormat::Bmp) => "image/bmp".into(),
            _ => mime_guess::from_path(path)
                .first_raw()
                .filter(|value| value.starts_with("image/"))
                .unwrap_or("application/octet-stream")
                .into(),
        }
    }
}

fn find_track<'a>(
    bootstrap: &'a crate::playback::publication::PlayerBootstrap,
    track_id: &str,
) -> Result<&'a Track, ApiError> {
    if track_id.len() != 64 || !track_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ApiError::invalid("trackId 无效"));
    }
    bootstrap
        .queue
        .queue
        .iter()
        .find(|track| public_track_id(&track.key) == track_id)
        .ok_or_else(|| ApiError::not_found("曲目不在播放队列中"))
}

impl Provider for BackendProvider {
    fn bootstrap(&self) -> Result<crate::playback::publication::PlayerBootstrap, ApiError> {
        self.player.bootstrap().map_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.player.bootstrap", error);
            ApiError::unavailable()
        })
    }

    fn track(&self, track_id: &str) -> Result<Track, ApiError> {
        find_track(&self.bootstrap()?, track_id).cloned()
    }

    fn lyrics(&self, track: Track) -> ProviderFuture<'_, Option<LyricsAsset>> {
        Box::pin(super::lyrics::load(&self.app, track))
    }

    fn cover(&self, track: Track) -> ProviderFuture<'_, Option<CoverAsset>> {
        Box::pin(async move {
            let source = track
                .cover
                .url(nons_music_adapter_sdk::cover::CoverSize::Original);
            if source.is_empty() {
                return Ok(None);
            }
            let bytes = if matches!(
                url::Url::parse(source)
                    .map(|url| url.scheme().to_owned())
                    .as_deref(),
                Ok("http" | "https")
            ) {
                let runtime = crate::infrastructure::covers::CoverRuntime::new(
                    self.adapters.clone(),
                    self.cover_client.clone(),
                    self.cover_requests.clone(),
                    self.online_covers.clone(),
                );
                runtime
                    .read(source.to_owned(), &self.app)
                    .await
                    .map_err(|error| {
                        crate::infrastructure::diagnostics::warn("open_api.cover.online", error);
                        ApiError::unavailable()
                    })?
            } else {
                let source = source.to_owned();
                let path = self.covers.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let provider = standalone_local(&path);
                    provider(&source)
                })
                .await
                .map_err(|error| {
                    crate::infrastructure::diagnostics::warn("open_api.cover.join", error);
                    ApiError::internal()
                })??
            };
            if bytes.len() > MAX_COVER_BYTES {
                return Err(ApiError::unavailable());
            }
            let content_type = Self::mime(&bytes, source);
            Ok(Some(CoverAsset {
                bytes,
                content_type,
            }))
        })
    }

    fn command(&self, command: PlayerCommand) -> ProviderFuture<'_, ()> {
        Box::pin(async move {
            CommandProvider {
                player: self.player.clone(),
            }
            .run(command)
            .await
        })
    }
}

fn standalone_local(covers: &Path) -> impl Fn(&str) -> Result<Vec<u8>, ApiError> + '_ {
    move |source| {
        let covers = covers.canonicalize().map_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.cover.root", error);
            ApiError::unavailable()
        })?;
        let canonical = Path::new(source).canonicalize().map_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.cover.local", error);
            ApiError::not_found("封面不可用")
        })?;
        if !canonical.starts_with(&covers) {
            return Err(ApiError::not_found("封面不可用"));
        }
        let metadata = canonical.metadata().map_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.cover.metadata", error);
            ApiError::not_found("封面不可用")
        })?;
        if !metadata.is_file() || metadata.len() > MAX_COVER_BYTES as u64 {
            return Err(ApiError::not_found("封面不可用"));
        }
        std::fs::read(&canonical).map_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.cover.read", error);
            ApiError::unavailable()
        })
    }
}

struct CommandProvider {
    player: Arc<Player>,
}

impl CommandProvider {
    async fn run(&self, command: PlayerCommand) -> Result<(), ApiError> {
        let command = match command {
            PlayerCommand::Resume => Command::Resume,
            PlayerCommand::Pause => Command::Pause,
            PlayerCommand::Toggle => Command::Toggle,
            PlayerCommand::Stop => Command::Stop,
            PlayerCommand::Next => Command::Next,
            PlayerCommand::Previous => Command::Previous,
            PlayerCommand::Seek { position_ms } => Command::Seek(position_ms),
            PlayerCommand::Volume { volume } => {
                if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
                    return Err(ApiError::invalid("音量必须介于 0 和 1"));
                }
                Command::Volume(volume)
            }
            PlayerCommand::Shuffle {
                queue_revision,
                shuffle,
            } => Command::SetShuffle {
                queue_revision,
                value: shuffle,
            },
            PlayerCommand::Repeat {
                queue_revision,
                repeat_mode,
            } => Command::SetRepeat {
                queue_revision,
                mode: repeat_mode,
            },
            PlayerCommand::Jump {
                queue_revision,
                track_id,
            } => {
                let bootstrap = self.player.bootstrap().map_err(|error| {
                    crate::infrastructure::diagnostics::warn("open_api.command.bootstrap", error);
                    ApiError::unavailable()
                })?;
                check_queue_revision(&bootstrap, queue_revision)?;
                let track = find_track(&bootstrap, &track_id)?;
                Command::JumpTrack {
                    queue_revision,
                    key: track.key.clone(),
                }
            }
        };
        self.player
            .send_external(command)
            .await
            .map_err(BackendProvider::map_command_error)
    }
}
