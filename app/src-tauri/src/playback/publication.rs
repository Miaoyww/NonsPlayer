//! Versioned transport snapshots. Queue storage is shared until an edit uses Arc::make_mut.
use crate::model::{PlaybackOptions, PlaybackStatus, PlayerSnapshot, Progress, RepeatMode, Track};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub playback_engine: crate::model::PlaybackEngine,
    pub sequence: u64,
    pub queue_revision: u64,
    pub revision: u64,
    pub playback_options: PlaybackOptions,
    pub private_fm_session: Option<u64>,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub index: Option<usize>,
    pub status: PlaybackStatus,
    pub volume: f64,
    pub device_id: Option<String>,
    pub actual_quality: Option<String>,
    pub playback_source: Option<crate::music::manager::PlaybackSource>,
    pub error: Option<String>,
    pub media_error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerQueue {
    pub sequence: u64,
    pub queue_revision: u64,
    pub queue: Arc<Vec<Track>>,
    pub shuffle_order: Arc<Vec<usize>>,
}

#[derive(Clone, Serialize)]
pub struct PlayerProgress {
    pub sequence: u64,
    #[serde(flatten)]
    pub value: Progress,
}

#[derive(Clone, Serialize)]
pub struct PlayerBootstrap {
    pub state: PlayerState,
    pub queue: PlayerQueue,
    pub progress: PlayerProgress,
}

pub(super) struct Publication {
    pub snapshot: PlayerSnapshot,
    sequence: u64,
    queue_revision: u64,
    progress: PlayerProgress,
}

impl Publication {
    pub fn new(snapshot: PlayerSnapshot) -> Self {
        let progress = PlayerProgress {
            sequence: 0,
            value: Progress {
                revision: snapshot.revision,
                position_ms: snapshot.position_ms,
                duration_ms: snapshot.duration_ms,
                status: snapshot.status,
            },
        };
        Self {
            snapshot,
            sequence: 0,
            queue_revision: 0,
            progress,
        }
    }

    pub fn bootstrap(&self) -> PlayerBootstrap {
        let s = &self.snapshot;
        PlayerBootstrap {
            state: PlayerState {
                playback_engine: s.playback_engine,
                sequence: self.sequence,
                queue_revision: self.queue_revision,
                revision: s.revision,
                playback_options: s.playback_options,
                private_fm_session: s.private_fm_session,
                shuffle: s.shuffle,
                repeat_mode: s.repeat_mode,
                index: s.index,
                status: s.status,
                volume: s.volume,
                device_id: s.device_id.clone(),
                actual_quality: s.actual_quality.clone(),
                playback_source: s.playback_source.clone(),
                error: s.error.clone(),
                media_error: s.media_error.clone(),
            },
            queue: PlayerQueue {
                sequence: self.sequence,
                queue_revision: self.queue_revision,
                queue: Arc::clone(&s.queue),
                shuffle_order: Arc::clone(&s.shuffle_order),
            },
            progress: self.progress.clone(),
        }
    }

    pub fn update(
        &mut self,
        state: &PlayerSnapshot,
    ) -> (PlayerState, Option<PlayerQueue>, Option<PlayerProgress>) {
        self.sequence += 1;
        let queue_changed = !Arc::ptr_eq(&self.snapshot.queue, &state.queue)
            || !Arc::ptr_eq(&self.snapshot.shuffle_order, &state.shuffle_order);
        if queue_changed {
            self.queue_revision += 1;
        }
        let p = &self.progress.value;
        let timeline_changed = p.revision != state.revision
            || p.position_ms != state.position_ms
            || p.duration_ms != state.duration_ms
            || p.status != state.status;
        // Cloning these snapshots retains queue allocations; only small fields are copied.
        self.snapshot = state.clone();
        if timeline_changed {
            self.sample(state);
        }
        let bootstrap = self.bootstrap();
        (
            bootstrap.state,
            queue_changed.then_some(bootstrap.queue),
            timeline_changed.then_some(bootstrap.progress),
        )
    }

    fn sample(&mut self, state: &PlayerSnapshot) {
        self.progress = PlayerProgress {
            sequence: self.sequence,
            value: Progress {
                revision: state.revision,
                position_ms: state.position_ms,
                duration_ms: state.duration_ms,
                status: state.status,
            },
        };
    }

    pub fn tick(&mut self, state: &PlayerSnapshot) -> PlayerProgress {
        self.sequence += 1;
        self.snapshot.position_ms = state.position_ms;
        self.snapshot.duration_ms = state.duration_ms;
        self.sample(state);
        self.progress.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: u64) -> Track {
        Track::from_music(nons_music_adapter_sdk::identity::MusicTrack {
            title: format!("歌曲 {id}"),
            aliases: vec![],
            artist: "歌手".into(),
            artists: vec![],
            album: "专辑".into(),
            album_reference: None,
            duration_ms: 180_000,
            cover: "https://example.com/cover.jpg".into(),
            reference: nons_music_adapter_sdk::identity::EntityRef {
                source: "netease".to_string().try_into().unwrap(),
                kind: nons_music_adapter_sdk::identity::EntityKind::Track,
                id: (id).to_string().try_into().unwrap(),
            },
            associations: vec![],
            lyric_identifiers: vec![],
            unavailable_reason: None,
        })
        .unwrap()
    }

    #[test]
    fn volume_and_pause_do_not_copy_or_transmit_queue_and_volume_does_not_publish_progress() {
        let mut state = PlayerSnapshot {
            queue: (0..1000).map(track).collect::<Vec<_>>().into(),
            shuffle_order: (0..1000).collect::<Vec<_>>().into(),
            ..Default::default()
        };
        let mut publication = Publication::new(state.clone());
        let original = publication.snapshot.queue.clone();
        let original_order = publication.snapshot.shuffle_order.clone();
        for volume in [0.1, 0.3, 0.5] {
            state.volume = volume;
            let (event, queue, progress) = publication.update(&state);
            assert!(queue.is_none());
            assert!(progress.is_none());
            assert!(Arc::ptr_eq(&original, &publication.snapshot.queue));
            assert!(Arc::ptr_eq(
                &original_order,
                &publication.snapshot.shuffle_order
            ));
            let json = serde_json::to_value(event).unwrap();
            assert!(json.get("queue").is_none());
            assert!(json.get("shuffleOrder").is_none());
            assert!(json.get("positionMs").is_none());
        }
        state.status = PlaybackStatus::Paused;
        let (_, queue, progress) = publication.update(&state);
        assert!(queue.is_none());
        assert_eq!(progress.unwrap().value.status, PlaybackStatus::Paused);
    }

    #[test]
    fn edits_publish_matching_versions_and_leave_previous_snapshots_immutable() {
        let state = PlayerSnapshot {
            queue: vec![track(1)].into(),
            ..Default::default()
        };
        let mut publication = Publication::new(state.clone());
        let previous = publication.bootstrap();
        let mut edited = state;
        Arc::make_mut(&mut edited.shuffle_order).push(0);
        let (event, queue, _) = publication.update(&edited);
        let queue = queue.unwrap();
        assert_eq!(event.queue_revision, queue.queue_revision);
        assert_eq!(event.sequence, queue.sequence);
        assert!(previous.queue.shuffle_order.is_empty());
        assert_eq!(queue.shuffle_order.len(), 1);
        Arc::make_mut(&mut edited.queue)[0].title = "更新后的标题".into();
        let (event, queue, _) = publication.update(&edited);
        assert_eq!(queue.unwrap().queue[0].title, "更新后的标题");
        assert_eq!(previous.queue.queue[0].title, "歌曲 1");
        let tick = publication.tick(&edited);
        assert!(tick.sequence > event.sequence);
    }
}
