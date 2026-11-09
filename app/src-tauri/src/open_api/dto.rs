use crate::{
    model::{PlaybackMode, PlaybackStatus, RepeatMode, Track, TrackSource},
    playback::publication::{PlayerBootstrap, PlayerProgress, PlayerQueue, PlayerState},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub source: String,
    pub kind: String,
    pub id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub track_id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: u64,
    pub source: String,
    pub reference: Reference,
    pub cover_url: Option<String>,
}

impl TrackSummary {
    pub fn from_track(track: &Track) -> Self {
        let reference = match &track.source {
            TrackSource::Online { track } => Reference {
                source: track.reference.source.as_str().into(),
                kind: match track.reference.kind {
                    nons_music_adapter_sdk::identity::EntityKind::Track => "track",
                    nons_music_adapter_sdk::identity::EntityKind::Playlist => "playlist",
                    nons_music_adapter_sdk::identity::EntityKind::Album => "album",
                    nons_music_adapter_sdk::identity::EntityKind::Artist => "artist",
                }
                .into(),
                id: Some(track.reference.id.as_str().into()),
            },
            TrackSource::Local { .. } => Reference {
                source: "local".into(),
                kind: "track".into(),
                id: None,
            },
        };
        let artists = if track.artists.is_empty() {
            (!track.artist.is_empty())
                .then(|| track.artist.clone())
                .into_iter()
                .collect()
        } else {
            track
                .artists
                .iter()
                .map(|artist| artist.name.clone())
                .collect()
        };
        let track_id = public_track_id(&track.key);
        Self {
            track_id: track_id.clone(),
            title: track.title.clone(),
            artists,
            album: track.album.clone(),
            duration_ms: track.duration_ms,
            source: track.source_id().to_owned(),
            reference,
            cover_url: (!track.cover.is_empty()).then(|| format!("/v1/cover?trackId={track_id}")),
        }
    }
}

pub fn public_track_id(key: &str) -> String {
    format!("{:x}", Sha256::digest(key.as_bytes()))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub sequence: u64,
    pub queue_revision: u64,
    pub revision: u64,
    pub status: PlaybackStatus,
    pub index: Option<usize>,
    pub queue_length: usize,
    pub current: Option<TrackSummary>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub actual_quality: Option<String>,
    pub playback_mode: Option<PlaybackMode>,
    pub error: Option<String>,
}

impl Player {
    pub fn from_bootstrap(bootstrap: &PlayerBootstrap) -> Self {
        let current = bootstrap
            .state
            .index
            .and_then(|index| bootstrap.queue.queue.get(index))
            .map(TrackSummary::from_track);
        Self {
            sequence: bootstrap.state.sequence,
            queue_revision: bootstrap.state.queue_revision,
            revision: bootstrap.state.revision,
            status: bootstrap.state.status,
            index: bootstrap.state.index,
            queue_length: bootstrap.queue.queue.len(),
            current,
            position_ms: bootstrap.progress.value.position_ms,
            duration_ms: bootstrap.progress.value.duration_ms,
            volume: bootstrap.state.volume,
            shuffle: bootstrap.state.shuffle,
            repeat_mode: bootstrap.state.repeat_mode,
            actual_quality: bootstrap.state.actual_quality.clone(),
            playback_mode: bootstrap.state.playback_mode,
            error: bootstrap
                .state
                .error
                .as_ref()
                .map(|_| "playback_error".into()),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub sequence: u64,
    pub queue_revision: u64,
    pub items: Vec<TrackSummary>,
    pub shuffle_order: Vec<usize>,
}

impl Queue {
    pub fn from_public(value: &PlayerQueue) -> Self {
        Self {
            sequence: value.sequence,
            queue_revision: value.queue_revision,
            items: value.queue.iter().map(TrackSummary::from_track).collect(),
            shuffle_order: value.shuffle_order.as_ref().clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub sequence: u64,
    pub queue_revision: u64,
    pub revision: u64,
    pub status: PlaybackStatus,
    pub index: Option<usize>,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub actual_quality: Option<String>,
    pub playback_mode: Option<PlaybackMode>,
    pub error: Option<String>,
}

impl State {
    pub fn from_public(value: &PlayerState) -> Self {
        Self {
            sequence: value.sequence,
            queue_revision: value.queue_revision,
            revision: value.revision,
            status: value.status,
            index: value.index,
            volume: value.volume,
            shuffle: value.shuffle,
            repeat_mode: value.repeat_mode,
            actual_quality: value.actual_quality.clone(),
            playback_mode: value.playback_mode,
            error: value.error.as_ref().map(|_| "playback_error".into()),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub sequence: u64,
    pub revision: u64,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub status: PlaybackStatus,
}

impl Progress {
    pub fn from_public(value: &PlayerProgress) -> Self {
        Self {
            sequence: value.sequence,
            revision: value.value.revision,
            position_ms: value.value.position_ms,
            duration_ms: value.value.duration_ms,
            status: value.value.status,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PlayerCommand {
    Resume,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    Seek {
        position_ms: u64,
    },
    Volume {
        volume: f64,
    },
    Shuffle {
        queue_revision: u64,
        shuffle: bool,
    },
    Repeat {
        queue_revision: u64,
        repeat_mode: RepeatMode,
    },
    Jump {
        queue_revision: u64,
        track_id: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Accepted {
    pub accepted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub version: &'static str,
    pub protocol_version: u32,
    pub capabilities: [&'static str; 7],
}

#[derive(Clone)]
pub struct LyricsAsset {
    pub body: String,
    pub source: String,
    pub profile: String,
    pub effective_offset_ms: i64,
    pub etag: String,
}

#[derive(Clone)]
pub struct CoverAsset {
    pub bytes: Vec<u8>,
    pub content_type: String,
}
