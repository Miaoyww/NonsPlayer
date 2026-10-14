//! Collection preparation belongs to the playback intent, not to the IPC completion order.
use crate::model::{AppResult, PlayerSnapshot, Track};

#[derive(Default)]
pub(super) struct CollectionRequest {
    serial: u64,
    pending: Option<(u64, Option<String>)>,
}

impl CollectionRequest {
    pub fn begin(&mut self, key: Option<String>) -> u64 {
        self.serial += 1;
        self.pending = Some((self.serial, key));
        self.serial
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }

    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn take(&mut self, serial: u64) -> Option<Option<String>> {
        if self.pending.as_ref()?.0 != serial {
            return None;
        }
        self.pending.take().map(|(_, key)| key)
    }
}

pub(super) fn hydrate(state: &mut PlayerSnapshot, key: &str, tracks: Vec<Track>) -> AppResult<()> {
    if state.current().is_none_or(|track| track.key != key) {
        return Err("播放选择已变化".into());
    }
    if tracks.is_empty() || tracks.len() > 1000 {
        return Err("播放队列无效，最多支持 1000 首歌曲".into());
    }
    let index = tracks
        .iter()
        .position(|track| track.key == key)
        .ok_or("歌曲已不在收藏中，请刷新")?;
    state.queue = tracks.into();
    state.index = Some(index);
    state.deduplicate_queue();
    state.reset_shuffle_order(state.index);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PlaybackStatus, TrackSource};

    fn track(key: &str) -> Track {
        Track {
            key: key.into(),
            title: key.into(),
            aliases: vec![],
            artist: String::new(),
            artists: vec![],
            album: String::new(),
            album_id: None,
            duration_ms: 180_000,
            cover: Default::default(),
            source: TrackSource::Netease { id: 1 },
        }
    }

    #[test]
    fn new_selection_and_cancellation_reject_late_collection_results() {
        let mut request = CollectionRequest::default();
        let a = request.begin(Some("A".into()));
        let b = request.begin(Some("B".into()));
        assert!(request.take(a).is_none());
        assert_eq!(request.take(b), Some(Some("B".into())));
        assert!(request.take(b).is_none());
        let c = request.begin(None);
        request.cancel();
        assert!(request.take(c).is_none());
    }

    #[test]
    fn only_latest_selection_can_hydrate_for_every_completion_order() {
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let mut request = CollectionRequest::default();
            let mut serials = vec![];
            let mut state = PlayerSnapshot::default();
            for key in ["A", "B", "C"] {
                state.queue = vec![track(key)].into();
                state.index = Some(0);
                serials.push(request.begin(Some(key.into())));
            }
            let mut accepted = 0;
            for index in order {
                if let Some(Some(key)) = request.take(serials[index]) {
                    hydrate(&mut state, &key, vec![track("A"), track("B"), track("C")]).unwrap();
                    accepted += 1;
                }
                assert_eq!(state.current().unwrap().key, "C");
            }
            assert_eq!(accepted, 1);
        }
    }

    #[test]
    fn cancelled_preparation_cannot_overwrite_user_queue_edits() {
        let mut request = CollectionRequest::default();
        let serial = request.begin(Some("A".into()));
        let mut state = PlayerSnapshot {
            queue: vec![track("A"), track("custom")].into(),
            index: Some(0),
            ..Default::default()
        };
        request.cancel();
        if let Some(Some(key)) = request.take(serial) {
            hydrate(&mut state, &key, vec![track("A"), track("B")]).unwrap();
        }
        assert_eq!(state.queue[1].key, "custom");
    }

    #[test]
    fn hydration_repositions_current_track_without_restarting_audio() {
        let mut state = PlayerSnapshot {
            queue: vec![track("B")].into(),
            index: Some(0),
            revision: 7,
            position_ms: 4200,
            duration_ms: 180_000,
            status: PlaybackStatus::Playing,
            actual_quality: Some("lossless".into()),
            ..Default::default()
        };
        hydrate(&mut state, "B", vec![track("A"), track("B"), track("C")]).unwrap();
        assert_eq!(state.index, Some(1));
        assert_eq!(state.current().unwrap().key, "B");
        assert_eq!(state.revision, 7);
        assert_eq!(state.position_ms, 4200);
        assert_eq!(state.status, PlaybackStatus::Playing);
        assert_eq!(state.actual_quality.as_deref(), Some("lossless"));
    }

    #[test]
    fn missing_or_changed_selection_preserves_the_playable_queue() {
        let mut state = PlayerSnapshot {
            queue: vec![track("B")].into(),
            index: Some(0),
            ..Default::default()
        };
        assert!(hydrate(&mut state, "B", vec![track("A")]).is_err());
        assert!(hydrate(&mut state, "A", vec![track("A")]).is_err());
        assert_eq!(state.current().unwrap().key, "B");
        assert_eq!(state.queue.len(), 1);
    }
}
