//! Host queue/account migration; shared track conversion keeps legacy data stable.
use super::{
    identity::{MusicTrack, OpaqueId, SourceId},
    ErrorCode, MusicResult, CONTRACT_VERSION,
};
use crate::model::PlayerSnapshot;
use nons_music_adapter_sdk::migration::convert_track;
pub use nons_music_adapter_sdk::migration::{legacy_track, LocalBinding, MigratedTrack};
use serde::{Deserialize, Serialize};
pub const LEGACY_KEYRING_SERVICE: &str = "NonsPlayer";
pub const LEGACY_KEYRING_USER: &str = "netease-session";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQueue {
    pub version: u32,
    pub tracks: Vec<MusicTrack>,
    pub index: Option<usize>,
    pub shuffle: bool,
    pub shuffle_order: Vec<usize>,
    pub repeat_mode: QueueRepeatMode,
    pub volume: f64,
    pub device_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueueRepeatMode {
    Off,
    All,
    One,
}

/// The caller persists bindings and key mappings in the same transaction as the queue.
pub struct MigratedQueue {
    pub queue: SavedQueue,
    pub tracks: Vec<MigratedTrack>,
}

pub fn legacy_queue(json: &str) -> MusicResult<MigratedQueue> {
    if json.len() > 8 * 1024 * 1024 {
        return Err(ErrorCode::InvalidData.into());
    }
    let mut state: PlayerSnapshot =
        serde_json::from_str(json).map_err(|_| ErrorCode::InvalidData)?;
    if state.queue.len() > 1000
        || state.index.is_some_and(|index| index >= state.queue.len())
        || !state.volume.is_finite()
        || !(0.0..=1.0).contains(&state.volume)
    {
        return Err(ErrorCode::InvalidData.into());
    }
    state.restore_shuffle_order();
    let tracks = std::sync::Arc::unwrap_or_clone(state.queue)
        .into_iter()
        .map(convert_track)
        .collect::<MusicResult<Vec<_>>>()?;
    let queue = SavedQueue {
        version: CONTRACT_VERSION,
        tracks: tracks.iter().map(|t| t.track.clone()).collect(),
        index: state.index,
        shuffle: state.shuffle,
        shuffle_order: std::sync::Arc::unwrap_or_clone(state.shuffle_order),
        repeat_mode: match state.repeat_mode {
            crate::model::RepeatMode::Off => QueueRepeatMode::Off,
            crate::model::RepeatMode::All => QueueRepeatMode::All,
            crate::model::RepeatMode::One => QueueRepeatMode::One,
        },
        volume: state.volume,
        device_id: state.device_id,
    };
    Ok(MigratedQueue { queue, tracks })
}

/// Pure account identity conversion; credentials are read/moved by AccountManager later.
/// Missing/invalid profile stays pending, never mapped to a made-up account ID.
pub fn legacy_account(profile: &str) -> MusicResult<super::account::AccountRecord> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LegacyProfile {
        user_id: u64,
        nickname: String,
        avatar_url: String,
    }
    if profile.len() > 64 * 1024 {
        return Err(ErrorCode::InvalidData.into());
    }
    let value: LegacyProfile = serde_json::from_str(profile).map_err(|_| ErrorCode::InvalidData)?;
    if value.user_id == 0 {
        return Err(ErrorCode::InvalidData.into());
    }
    Ok(super::account::AccountRecord {
        reference: super::account::AccountRef {
            source: SourceId::try_from("netease".to_owned())?,
            id: OpaqueId::try_from(value.user_id.to_string())?,
        },
        display_name: value.nickname,
        avatar: value.avatar_url,
    })
}
