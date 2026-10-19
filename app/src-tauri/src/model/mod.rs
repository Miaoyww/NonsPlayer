#[allow(unused_imports)]
// Standalone probes/tests embed this compatibility module and use a subset.
pub use nons_music_adapter_sdk::legacy::{
    Lyrics, MusicCredit, Track, TrackSource, MAX_LYRIC_BYTES,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Loading,
    Playing,
    Paused,
    Buffering,
    Error,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AudioOutputMode {
    #[default]
    Shared,
    Exclusive,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaybackOptions {
    pub fades: bool,
    pub gapless: bool,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            fades: false,
            gapless: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshot {
    #[serde(default)]
    pub playback_options: PlaybackOptions,
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default)]
    pub shuffle_order: Arc<Vec<usize>>,
    #[serde(default)]
    pub repeat_mode: RepeatMode,
    #[serde(default, skip_deserializing)]
    pub private_fm_session: Option<u64>,
    pub revision: u64,
    pub queue: Arc<Vec<Track>>,
    pub index: Option<usize>,
    pub status: PlaybackStatus,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f64,
    pub device_id: Option<String>,
    #[serde(default)]
    pub audio_output_mode: AudioOutputMode,
    pub actual_quality: Option<String>,
    #[serde(default, skip_deserializing)]
    pub playback_source: Option<nons_music_adapter_sdk::provider::PlaybackSource>,
    pub error: Option<String>,
    pub media_error: Option<String>,
}

impl Default for PlayerSnapshot {
    fn default() -> Self {
        Self {
            playback_options: PlaybackOptions::default(),
            shuffle: false,
            shuffle_order: Arc::default(),
            repeat_mode: RepeatMode::Off,
            private_fm_session: None,
            revision: 0,
            queue: Arc::default(),
            index: None,
            status: PlaybackStatus::Stopped,
            position_ms: 0,
            duration_ms: 0,
            volume: if cfg!(target_os = "android") {
                1.0
            } else {
                0.8
            },
            device_id: None,
            audio_output_mode: AudioOutputMode::Shared,
            actual_quality: None,
            playback_source: None,
            error: None,
            media_error: None,
        }
    }
}

impl PlayerSnapshot {
    pub fn deduplicate_queue(&mut self) {
        let mut indices = std::collections::HashMap::new();
        let mut remap = Vec::with_capacity(self.queue.len());
        let mut queue = Vec::with_capacity(self.queue.len());
        for track in Arc::unwrap_or_clone(std::mem::take(&mut self.queue)) {
            let index = *indices.entry(track.key.clone()).or_insert_with(|| {
                queue.push(track);
                queue.len() - 1
            });
            remap.push(index);
        }
        self.index = self.index.and_then(|i| remap.get(i).copied());
        let mut seen = std::collections::HashSet::new();
        self.shuffle_order = self
            .shuffle_order
            .iter()
            .filter_map(|i| remap.get(*i).copied())
            .filter(|i| seen.insert(*i))
            .collect::<Vec<_>>()
            .into();
        self.queue = queue.into();
    }

    pub fn toggle_shuffle(&mut self, current: Option<usize>) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            self.repeat_mode = RepeatMode::Off;
        }
        self.reset_shuffle_order(current);
    }

    pub fn cycle_repeat(&mut self) {
        self.repeat_mode = match self.repeat_mode {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        if self.repeat_mode != RepeatMode::Off {
            self.shuffle = false;
            self.shuffle_order = Arc::default();
        }
    }

    pub fn reset_shuffle_order(&mut self, current: Option<usize>) {
        let mut order = Vec::new();
        if self.shuffle {
            order.extend((0..self.queue.len()).filter(|i| Some(*i) != current));
            fastrand::shuffle(&mut order);
            if let Some(index) = current.filter(|i| *i < self.queue.len()) {
                order.insert(0, index);
            }
        }
        self.shuffle_order = order.into();
    }

    pub fn restore_shuffle_order(&mut self) {
        // Older saved snapshots allowed shuffle and repeat at the same time.
        if self.shuffle {
            self.repeat_mode = RepeatMode::Off;
        }
        let mut sorted = self.shuffle_order.as_ref().clone();
        sorted.sort_unstable();
        if sorted != (0..self.queue.len()).collect::<Vec<_>>() {
            self.reset_shuffle_order(self.index);
        }
        if !self.shuffle {
            self.shuffle_order = Arc::default();
        }
    }

    pub fn following(&self, manual: bool) -> Option<usize> {
        let current = self.index.filter(|i| *i < self.queue.len())?;
        if !self.shuffle && !manual && self.repeat_mode == RepeatMode::One {
            return Some(current);
        }
        let mode = if self.shuffle || (manual && self.repeat_mode == RepeatMode::One) {
            RepeatMode::All
        } else {
            self.repeat_mode
        };
        if self.shuffle {
            let slot = self.shuffle_order.iter().position(|i| *i == current)?;
            following_index(Some(slot), self.shuffle_order.len(), mode)
                .map(|i| self.shuffle_order[i])
        } else {
            following_index(Some(current), self.queue.len(), mode)
        }
    }

    pub fn previous(&self) -> Option<usize> {
        let current = self.index.filter(|i| *i < self.queue.len())?;
        let slot = if self.shuffle {
            self.shuffle_order.iter().position(|i| *i == current)?
        } else {
            current
        };
        let count = if self.shuffle {
            self.shuffle_order.len()
        } else {
            self.queue.len()
        };
        let previous = slot.checked_sub(1).or_else(|| {
            (self.shuffle || self.repeat_mode != RepeatMode::Off).then_some(count - 1)
        })?;
        Some(if self.shuffle {
            self.shuffle_order[previous]
        } else {
            previous
        })
    }

    pub fn append_private_fm(
        &mut self,
        session: u64,
        queue_len: usize,
        tracks: Vec<Track>,
    ) -> AppResult<Option<usize>> {
        if self.private_fm_session != Some(session) || self.queue.len() != queue_len {
            return Ok(None);
        }
        let mut seen: std::collections::HashSet<String> =
            self.queue.iter().map(|t| t.key.clone()).collect();
        let tracks: Vec<_> = tracks
            .into_iter()
            .filter(|t| seen.insert(t.key.clone()))
            .collect();
        if tracks.is_empty() {
            return Err("暂时没有新的 FM 歌曲，请重试。".into());
        }
        let trim = (self.queue.len() + tracks.len()).saturating_sub(1000);
        if trim > self.index.unwrap_or(0) {
            return Err("私人 FM 队列最多支持 1000 首歌曲".into());
        }
        Arc::make_mut(&mut self.queue).drain(..trim);
        self.index = self.index.map(|i| i - trim);
        self.shuffle_order = self
            .shuffle_order
            .iter()
            .filter_map(|i| i.checked_sub(trim))
            .collect::<Vec<_>>()
            .into();
        let start = self.queue.len();
        Arc::make_mut(&mut self.queue).extend(tracks);
        if self.shuffle {
            Arc::make_mut(&mut self.shuffle_order).extend(start..self.queue.len());
        }
        Ok(Some(trim))
    }

    pub fn insert_next(&mut self, tracks: Vec<Track>) -> AppResult<usize> {
        // The current track stays in place; requesting it again never duplicates it.
        let current = self.current().map(|t| t.key.clone());
        let mut seen = std::collections::HashSet::new();
        let tracks: Vec<_> = tracks
            .into_iter()
            .filter(|t| Some(&t.key) != current.as_ref() && seen.insert(t.key.clone()))
            .collect();
        if tracks.is_empty() {
            return Ok(self.index.unwrap_or(0));
        }
        let existing: std::collections::HashSet<_> = self.queue.iter().map(|t| &t.key).collect();
        let new_count = tracks.iter().filter(|t| !existing.contains(&t.key)).count();
        if new_count + self.queue.len() > 10000 {
            return Err("播放队列最多支持 10000 首歌曲".into());
        }
        for index in (0..self.queue.len()).rev() {
            if seen.contains(&self.queue[index].key) {
                let key = self.queue[index].key.clone();
                self.remove_track(index, &key)?;
            }
        }
        let position = self
            .index
            .map_or(0, |index| index + 1)
            .min(self.queue.len());
        let count = tracks.len();
        if self.shuffle {
            let slot = self
                .index
                .and_then(|current| self.shuffle_order.iter().position(|i| *i == current))
                .map_or(0, |i| i + 1);
            for index in Arc::make_mut(&mut self.shuffle_order) {
                if *index >= position {
                    *index += count;
                }
            }
            Arc::make_mut(&mut self.shuffle_order).splice(slot..slot, position..position + count);
        }
        self.private_fm_session = None;
        Arc::make_mut(&mut self.queue).splice(position..position, tracks);
        if self.index.is_none() && !self.queue.is_empty() {
            self.index = Some(0);
            self.duration_ms = self.queue[0].duration_ms;
        }
        Ok(position)
    }

    pub fn remove_track(&mut self, index: usize, key: &str) -> AppResult<bool> {
        if self.queue.get(index).is_none_or(|track| track.key != key) {
            return Err("播放列表已变化，请重新选择歌曲".into());
        }
        let removed_current = self.index == Some(index);
        let shuffle_replacement = (removed_current && self.shuffle)
            .then(|| {
                self.following(true)
                    .filter(|i| *i != index)
                    .or_else(|| self.previous().filter(|i| *i != index))
            })
            .flatten();
        self.private_fm_session = None;
        Arc::make_mut(&mut self.queue).remove(index);
        Arc::make_mut(&mut self.shuffle_order).retain(|i| *i != index);
        for entry in Arc::make_mut(&mut self.shuffle_order) {
            if *entry > index {
                *entry -= 1;
            }
        }
        self.index = self.index.and_then(|current| {
            if self.queue.is_empty() {
                None
            } else if let Some(replacement) = shuffle_replacement {
                Some(if replacement > index {
                    replacement - 1
                } else {
                    replacement
                })
            } else if current > index {
                Some(current - 1)
            } else {
                Some(current.min(self.queue.len() - 1))
            }
        });
        Ok(removed_current)
    }

    pub fn current(&self) -> Option<&Track> {
        self.index.and_then(|index| self.queue.get(index))
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub revision: u64,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub status: PlaybackStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct ResolvedTrack {
    pub decoded_audio: Option<std::sync::Arc<tempfile::NamedTempFile>>,
    pub track: Track,
    pub uri: String,
    pub quality: Option<String>,
}

impl std::fmt::Debug for ResolvedTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedTrack")
            .field("track", &self.track.key)
            .field("quality", &self.quality)
            .field("access", &"[redacted]")
            .finish()
    }
}

pub type AppResult<T> = Result<T, String>;

pub fn next_index(index: Option<usize>, count: usize) -> Option<usize> {
    index.and_then(|i| i.checked_add(1)).filter(|i| *i < count)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

pub fn following_index(index: Option<usize>, count: usize, mode: RepeatMode) -> Option<usize> {
    let current = index.filter(|i| *i < count)?;
    match mode {
        RepeatMode::One => Some(current),
        RepeatMode::All => Some((current + 1) % count),
        RepeatMode::Off => next_index(index, count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn playback_modes_are_mutually_exclusive_and_restore_consistently() {
        let mut state = PlayerSnapshot {
            queue: (0..4).map(track).collect::<Vec<_>>().into(),
            index: Some(2),
            ..Default::default()
        };
        for repeat in [RepeatMode::All, RepeatMode::One] {
            state.repeat_mode = repeat;
            state.toggle_shuffle(Some(2));
            assert!(state.shuffle);
            assert_eq!(state.repeat_mode, RepeatMode::Off);
            assert_eq!(state.shuffle_order[0], 2);
            state.cycle_repeat();
            assert!(!state.shuffle);
            assert!(state.shuffle_order.is_empty());
            assert_eq!(state.repeat_mode, RepeatMode::All);
            assert_eq!(state.following(false), Some(3));
            state.cycle_repeat();
            assert_eq!(state.repeat_mode, RepeatMode::One);
            assert_eq!(state.following(false), Some(2));
            state.cycle_repeat();
            assert_eq!(state.repeat_mode, RepeatMode::Off);
        }
        state.toggle_shuffle(Some(2));
        state.toggle_shuffle(Some(2));
        assert!(!state.shuffle);
        assert!(state.shuffle_order.is_empty());
        assert_eq!(state.repeat_mode, RepeatMode::Off);
        state.toggle_shuffle(Some(2));
        state.repeat_mode = RepeatMode::One;
        let mut restored: PlayerSnapshot =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        restored.restore_shuffle_order();
        assert!(restored.shuffle);
        assert_eq!(restored.repeat_mode, RepeatMode::Off);
        assert_eq!(restored.shuffle_order, state.shuffle_order);
    }

    #[test]
    fn shuffle_navigation_visits_each_entry_and_respects_repeat() {
        let mut state = PlayerSnapshot {
            shuffle: true,
            queue: (0..5).map(track).collect::<Vec<_>>().into(),
            index: Some(2),
            ..Default::default()
        };
        state.reset_shuffle_order(state.index);
        assert_eq!(state.shuffle_order[0], 2);
        let last = *state.shuffle_order.last().unwrap();
        assert_eq!(state.previous(), Some(last));
        let mut visited = vec![2];
        for _ in 1..state.queue.len() {
            let next = state.following(false).unwrap();
            assert_eq!(
                state.following(true),
                Some(next),
                "manual next and preload agree"
            );
            let previous = state.index;
            state.index = Some(next);
            assert_eq!(state.previous(), previous);
            visited.push(next);
        }
        assert_eq!(
            state.following(false),
            Some(2),
            "shuffle repeats the whole list by default"
        );
        assert_eq!(state.following(true), Some(2));
        assert_eq!(
            state.repeat_mode,
            RepeatMode::Off,
            "repeat button remains unselected"
        );
        visited.sort_unstable();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
        state.cycle_repeat();
        assert!(!state.shuffle);
        state.index = Some(2);
        assert_eq!(state.following(true), Some(3));
        state.cycle_repeat();
        assert_eq!(state.following(false), Some(2));
        assert_eq!(state.following(true), Some(3));
    }

    #[test]
    fn shuffle_edits_preserve_next_insertion_and_valid_indices() {
        let mut state = PlayerSnapshot {
            shuffle: true,
            queue: (0..4).map(track).collect::<Vec<_>>().into(),
            index: Some(2),
            shuffle_order: vec![2, 0, 3, 1].into(),
            ..Default::default()
        };
        state.insert_next(vec![track(8), track(9)]).unwrap();
        assert_eq!(state.following(true), Some(3));
        state.index = Some(3);
        assert_eq!(state.following(true), Some(4));
        state.remove_track(0, "0").unwrap();
        assert_eq!(state.index, Some(2));
        assert_eq!(state.following(true), Some(3));
        state.remove_track(2, "8").unwrap();
        let mut sorted = state.shuffle_order.as_ref().clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..state.queue.len()).collect::<Vec<_>>());
        state.shuffle_order = vec![99, 99].into();
        state.restore_shuffle_order();
        assert_eq!(state.shuffle_order[0], state.index.unwrap());
        let restored: PlayerSnapshot =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(restored.shuffle_order, state.shuffle_order);
    }

    #[test]
    fn shuffle_handles_empty_and_single_track_queues() {
        let mut state = PlayerSnapshot {
            shuffle: true,
            ..Default::default()
        };
        assert_eq!(state.following(false), None);
        assert_eq!(state.previous(), None);
        state.insert_next(vec![track(1)]).unwrap();
        assert_eq!(state.repeat_mode, RepeatMode::Off);
        assert_eq!(state.following(false), Some(0));
        assert_eq!(state.previous(), Some(0));
    }

    #[test]
    fn removing_current_in_shuffle_uses_play_order_instead_of_list_position() {
        let mut state = PlayerSnapshot {
            shuffle: true,
            queue: (0..4).map(track).collect::<Vec<_>>().into(),
            index: Some(1),
            shuffle_order: vec![1, 3, 0, 2].into(),
            ..Default::default()
        };
        state.remove_track(1, "1").unwrap();
        assert_eq!(state.current().unwrap().key, "3");
        assert_eq!(state.following(true), Some(0));
        state.index = Some(1);
        state.remove_track(1, "2").unwrap();
        assert_eq!(
            state.current().unwrap().key,
            "3",
            "last shuffled entry wraps to the first remaining entry"
        );
    }
    fn track(id: u64) -> Track {
        Track {
            key: id.to_string(),
            title: id.to_string(),
            aliases: vec![],
            artists: vec![],
            album_id: None,
            artist: String::new(),
            album: String::new(),
            duration_ms: 1000,
            cover: Default::default(),
            source: TrackSource::Netease { id },
        }
    }
    #[test]
    fn next_insertion_moves_existing_tracks_and_keeps_current_and_shuffle_order() {
        for shuffle in [false, true] {
            let mut state = PlayerSnapshot {
                queue: (1..=5).map(track).collect::<Vec<_>>().into(),
                index: Some(2),
                shuffle,
                shuffle_order: (if shuffle { vec![2, 4, 0, 3, 1] } else { vec![] }).into(),
                position_ms: 500,
                status: PlaybackStatus::Playing,
                ..Default::default()
            };
            assert_eq!(
                state
                    .insert_next(vec![track(1), track(5), track(1), track(3)])
                    .unwrap(),
                2
            );
            assert_eq!(
                state
                    .queue
                    .iter()
                    .map(|t| t.key.as_str())
                    .collect::<Vec<_>>(),
                vec!["2", "3", "1", "5", "4"]
            );
            assert_eq!(state.index, Some(1));
            assert_eq!(state.current().unwrap().key, "3");
            assert_eq!(state.position_ms, 500);
            assert_eq!(state.status, PlaybackStatus::Playing);
            assert_eq!(state.following(true), Some(2));
            if shuffle {
                assert_eq!(*state.shuffle_order, vec![1, 2, 3, 4, 0]);
            }
            state.insert_next(vec![track(3)]).unwrap();
            assert_eq!(state.queue.len(), 5);
            assert_eq!(state.index, Some(1));
        }
    }

    #[test]
    fn ten_thousand_track_queue_shuffles_cycles_and_restores_every_entry() {
        let mut state = PlayerSnapshot {
            queue: (0..10000).map(track).collect::<Vec<_>>().into(),
            index: Some(9999),
            ..Default::default()
        };
        state.shuffle = true;
        state.reset_shuffle_order(state.index);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10000 {
            seen.insert(state.index.unwrap());
            state.index = state.following(false);
        }
        assert_eq!(seen.len(), 10000);
        assert_eq!(state.index, Some(9999));
        let mut restored: PlayerSnapshot =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        restored.restore_shuffle_order();
        assert_eq!(restored.shuffle_order, state.shuffle_order);
        restored.shuffle = false;
        restored.repeat_mode = RepeatMode::All;
        assert_eq!(restored.following(false), Some(0));
        restored.repeat_mode = RepeatMode::One;
        assert_eq!(restored.following(false), Some(9999));
        restored.repeat_mode = RepeatMode::Off;
        assert_eq!(restored.following(false), None);
    }
    #[test]
    fn full_queue_allows_moves_and_checks_unique_additions_atomically() {
        let mut state = PlayerSnapshot {
            queue: (0..10000).map(track).collect::<Vec<_>>().into(),
            index: Some(500),
            ..Default::default()
        };
        state
            .insert_next(vec![track(1), track(999), track(1)])
            .unwrap();
        assert_eq!(state.queue.len(), 10000);
        assert_eq!(state.current().unwrap().key, "500");
        assert_eq!(state.queue[state.index.unwrap() + 1].key, "1");
        let before = state
            .queue
            .iter()
            .map(|t| t.key.clone())
            .collect::<Vec<_>>();
        assert!(state.insert_next(vec![track(2), track(10001)]).is_err());
        assert_eq!(
            state
                .queue
                .iter()
                .map(|t| t.key.clone())
                .collect::<Vec<_>>(),
            before
        );
        let mut empty = PlayerSnapshot::default();
        empty.insert_next(vec![track(1); 1001]).unwrap();
        assert_eq!(empty.queue.len(), 1);
    }

    #[test]
    fn queue_deduplication_remaps_selected_duplicate_and_shuffle_indices() {
        let mut state = PlayerSnapshot {
            queue: vec![track(1), track(2), track(1), track(3)].into(),
            index: Some(2),
            shuffle: true,
            shuffle_order: vec![2, 3, 0, 1].into(),
            position_ms: 500,
            ..Default::default()
        };
        state.deduplicate_queue();
        assert_eq!(state.queue.len(), 3);
        assert_eq!(state.index, Some(0));
        assert_eq!(state.current().unwrap().key, "1");
        assert_eq!(*state.shuffle_order, vec![0, 2, 1]);
        assert_eq!(state.position_ms, 500);
    }

    #[test]
    fn next_insertion_preserves_current_playback_and_batch_order() {
        let mut state = PlayerSnapshot {
            queue: vec![track(1), track(2), track(3)].into(),
            index: Some(1),
            position_ms: 500,
            status: PlaybackStatus::Playing,
            ..Default::default()
        };
        assert_eq!(state.insert_next(vec![track(4), track(5)]).unwrap(), 2);
        assert_eq!(
            state
                .queue
                .iter()
                .map(|t| t.key.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "2", "4", "5", "3"]
        );
        assert_eq!(state.current().unwrap().key, "2");
        assert_eq!(state.position_ms, 500);
        assert_eq!(state.status, PlaybackStatus::Playing);
        state.insert_next(vec![track(6)]).unwrap();
        assert_eq!(state.queue[2].key, "6");
    }
    #[test]
    fn next_insertion_supports_empty_and_last_track_queues_and_enforces_limit() {
        let mut state = PlayerSnapshot::default();
        state.insert_next(vec![track(1)]).unwrap();
        assert_eq!(state.index, Some(0));
        assert_eq!(state.status, PlaybackStatus::Stopped);
        state.insert_next(vec![track(2)]).unwrap();
        assert_eq!(state.queue[1].key, "2");
        assert!(state.insert_next((3..10002).map(track).collect()).is_err());
        assert_eq!(state.queue.len(), 2);
    }
    #[test]
    fn removing_queue_entries_preserves_current_playback_and_checks_identity() {
        let mut state = PlayerSnapshot {
            queue: vec![track(1), track(2), track(1), track(3)].into(),
            index: Some(2),
            position_ms: 500,
            status: PlaybackStatus::Playing,
            ..PlayerSnapshot::default()
        };
        assert!(state.remove_track(1, "wrong").is_err());
        assert_eq!(state.queue.len(), 4);
        assert!(!state.remove_track(0, "1").unwrap());
        assert_eq!(state.index, Some(1));
        assert_eq!(state.current().unwrap().key, "1");
        assert_eq!(state.position_ms, 500);
        assert_eq!(state.status, PlaybackStatus::Playing);
        assert!(!state.remove_track(2, "3").unwrap());
        assert_eq!(state.index, Some(1));
    }
    #[test]
    fn removing_current_selects_next_then_previous_and_supports_empty_queue() {
        let mut state = PlayerSnapshot {
            queue: vec![track(1), track(2), track(3)].into(),
            index: Some(1),
            ..PlayerSnapshot::default()
        };
        assert!(state.remove_track(1, "2").unwrap());
        assert_eq!(state.current().unwrap().key, "3");
        assert!(state.remove_track(1, "3").unwrap());
        assert_eq!(state.current().unwrap().key, "1");
        assert!(state.remove_track(0, "1").unwrap());
        assert!(state.queue.is_empty());
        assert_eq!(state.index, None);
        assert!(state.remove_track(0, "1").is_err());
    }
    #[test]
    fn repeat_respects_queue_boundaries() {
        assert_eq!(following_index(Some(1), 2, RepeatMode::All), Some(0));
        assert_eq!(following_index(Some(1), 2, RepeatMode::One), Some(1));
        assert_eq!(following_index(Some(0), 1, RepeatMode::All), Some(0));
        assert_eq!(following_index(None, 0, RepeatMode::All), None);
        assert_eq!(following_index(Some(usize::MAX), 2, RepeatMode::One), None);
    }
    #[test]
    fn a_finished_queue_does_not_repeat_or_wrap() {
        assert_eq!(next_index(Some(0), 2), Some(1));
        assert_eq!(next_index(Some(1), 2), None);
        assert_eq!(next_index(None, 2), None);
        assert_eq!(next_index(Some(usize::MAX), 2), None);
    }
}
