use std::sync::Mutex;

use crate::models::song::Song;

#[derive(Clone, Debug, PartialEq)]
pub enum PlayMode {
    /// Play list in order, stop after the last track.
    Sequential,
    /// Shuffle the list and play through.
    Shuffle,
    /// Repeat the current track indefinitely.
    SingleLoop,
    /// Loop the entire list.
    ListLoop,
}

/// A queue entry that identifies a song and its source adapter.
#[derive(Clone, Debug)]
pub struct QueueItem {
    pub song: Song,
    pub adapter_slug: String,
}

/// Thread-safe play queue with playlist management and play-mode state.
pub struct PlayQueue {
    items: Mutex<Vec<QueueItem>>,
    current_index: Mutex<usize>,
    mode: Mutex<PlayMode>,
    /// Shuffled permutation of indices; None when mode != Shuffle.
    shuffle_order: Mutex<Option<Vec<usize>>>,
    shuffle_pos: Mutex<usize>,
}

impl PlayQueue {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(Vec::new()),
            current_index: Mutex::new(0),
            mode: Mutex::new(PlayMode::Sequential),
            shuffle_order: Mutex::new(None),
            shuffle_pos: Mutex::new(0),
        }
    }

    /// Replace the queue with a new list and reset position.
    pub fn set_list(&self, items: Vec<QueueItem>) {
        let mut current = self.current_index.lock().unwrap();
        let mut shuffle = self.shuffle_order.lock().unwrap();
        let mut shuffle_pos = self.shuffle_pos.lock().unwrap();

        *shuffle = None;
        *shuffle_pos = 0;
        *current = 0;
        *self.items.lock().unwrap() = items;
    }

    /// Add songs to the end of the queue.
    pub fn append(&self, items: Vec<QueueItem>) {
        self.items.lock().unwrap().extend(items);
        // Invalidate shuffle order so it will be regenerated.
        *self.shuffle_order.lock().unwrap() = None;
    }

    /// Get the current track. Returns None if the queue is empty.
    pub fn current(&self) -> Option<QueueItem> {
        let items = self.items.lock().unwrap();
        let idx = *self.current_index.lock().unwrap();

        if items.is_empty() {
            return None;
        }

        let mode = self.mode.lock().unwrap().clone();
        match mode {
            PlayMode::Shuffle => {
                let order = self.shuffle_order.lock().unwrap();
                if let Some(ref order) = *order {
                    let pos = *self.shuffle_pos.lock().unwrap();
                    if pos < order.len() {
                        return items.get(order[pos]).cloned();
                    }
                }
                None
            }
            _ => items.get(idx).cloned(),
        }
    }

    /// Move to the next track and return it. Returns None when the queue ends.
    pub fn next(&self) -> Option<QueueItem> {
        let mode = self.mode.lock().unwrap().clone();
        let items = self.items.lock().unwrap();

        if items.is_empty() {
            return None;
        }

        match mode {
            PlayMode::Sequential => {
                let mut idx = self.current_index.lock().unwrap();
                *idx += 1;
                if *idx >= items.len() {
                    return None; // end of queue
                }
                items.get(*idx).cloned()
            }
            PlayMode::Shuffle => {
                let mut pos = self.shuffle_pos.lock().unwrap();
                *pos += 1;

                let order = self.shuffle_order.lock().unwrap();
                if let Some(ref order) = *order {
                    if *pos >= order.len() {
                        return None;
                    }
                    let real_idx = order[*pos];
                    *self.current_index.lock().unwrap() = real_idx;
                    return items.get(real_idx).cloned();
                }
                None
            }
            PlayMode::SingleLoop => items.get(*self.current_index.lock().unwrap()).cloned(),
            PlayMode::ListLoop => {
                let mut idx = self.current_index.lock().unwrap();
                *idx = (*idx + 1) % items.len();
                items.get(*idx).cloned()
            }
        }
    }

    /// Move to the previous track and return it.
    pub fn prev(&self) -> Option<QueueItem> {
        let mode = self.mode.lock().unwrap().clone();
        let items = self.items.lock().unwrap();

        if items.is_empty() {
            return None;
        }

        match mode {
            PlayMode::Shuffle => {
                let mut pos = self.shuffle_pos.lock().unwrap();
                if *pos == 0 {
                    return None;
                }
                *pos -= 1;
                let order = self.shuffle_order.lock().unwrap();
                if let Some(ref order) = *order {
                    let real_idx = order[*pos];
                    *self.current_index.lock().unwrap() = real_idx;
                    return items.get(real_idx).cloned();
                }
                None
            }
            _ => {
                let mut idx = self.current_index.lock().unwrap();
                if *idx == 0 {
                    if mode == PlayMode::ListLoop {
                        *idx = items.len() - 1;
                    }
                    // else: stay at first track
                } else {
                    *idx -= 1;
                }
                items.get(*idx).cloned()
            }
        }
    }

    /// Jump to a specific index (0-based) in the queue.
    pub fn jump_to(&self, index: usize) -> Option<QueueItem> {
        let items = self.items.lock().unwrap();
        if index >= items.len() {
            return None;
        }
        *self.current_index.lock().unwrap() = index;
        items.get(index).cloned()
    }

    /// Set the play mode.
    pub fn set_mode(&self, mode: PlayMode) {
        let is_shuffle = mode == PlayMode::Shuffle;
        *self.mode.lock().unwrap() = mode;
        // Invalidate shuffle when switching modes — regenerated lazily.
        if !is_shuffle {
            *self.shuffle_order.lock().unwrap() = None;
            *self.shuffle_pos.lock().unwrap() = 0;
        }
    }

    /// Get the current play mode.
    pub fn get_mode(&self) -> PlayMode {
        self.mode.lock().unwrap().clone()
    }

    /// Generate a shuffled index permutation.
    /// Called once when shuffle mode is first used with the current queue.
    pub fn reshuffle(&self) {
        use rand::seq::SliceRandom;
        let items = self.items.lock().unwrap();
        let mut indices: Vec<usize> = (0..items.len()).collect();
        let mut rng = rand::thread_rng();
        indices.shuffle(&mut rng);
        *self.shuffle_order.lock().unwrap() = Some(indices);
        *self.shuffle_pos.lock().unwrap() = 0;
        *self.current_index.lock().unwrap() = 0;
    }

    /// Get the current index (0-based) into the queue.
    pub fn current_index(&self) -> usize {
        *self.current_index.lock().unwrap()
    }

    /// Get the total number of items in the queue.
    pub fn len(&self) -> usize {
        self.items.lock().unwrap().len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.items.lock().unwrap().is_empty()
    }

    /// Get a copy of all queue items.
    pub fn get_all(&self) -> Vec<QueueItem> {
        self.items.lock().unwrap().clone()
    }
}

impl Default for PlayQueue {
    fn default() -> Self {
        Self::new()
    }
}
