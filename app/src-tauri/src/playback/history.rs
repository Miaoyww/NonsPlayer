use crate::{
    model::Track,
    storage::{now_seconds, Store},
};
use std::sync::{mpsc, Arc, Mutex};

pub struct PlaybackHistory {
    sender: Mutex<Option<mpsc::SyncSender<HistoryMessage>>>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

struct PlaybackRecord {
    instance_id: String,
    track: Track,
    started_at: i64,
}

enum HistoryMessage {
    Playback(PlaybackRecord),
    Listening { seconds: i64, at: i64 },
}

impl PlaybackHistory {
    pub fn start(store: Arc<Store>) -> Arc<Self> {
        let (sender, receiver) = mpsc::sync_channel::<HistoryMessage>(128);
        let thread = std::thread::Builder::new().name("nons-play-history".into()).spawn(move || {
            while let Ok(message) = receiver.recv() {
                let result = match message {
                    HistoryMessage::Playback(record) => {
                        store.record_playback(&record.instance_id, &record.track, record.started_at)
                    }
                    HistoryMessage::Listening { seconds, at } => {
                        store.add_listening_seconds(at, seconds).map(|()| false)
                    }
                };
                if let Err(error) = result {
                    log::warn!(target: "nons::playback", "playback_history_write_failed reason={error}");
                }
            }
        }).ok();
        Arc::new(Self {
            sender: Mutex::new(Some(sender)),
            thread: Mutex::new(thread),
        })
    }

    pub fn record(&self, instance_id: String, track: Track) {
        if let Ok(sender) = self.sender.lock() {
            if let Some(sender) = sender.as_ref() {
                let _ = sender.try_send(HistoryMessage::Playback(PlaybackRecord {
                    instance_id,
                    track,
                    started_at: now_seconds(),
                }));
            }
        }
    }

    pub fn record_listening(&self, seconds: i64) {
        if let Ok(sender) = self.sender.lock() {
            if let Some(sender) = sender.as_ref() {
                let _ = sender.try_send(HistoryMessage::Listening {
                    seconds,
                    at: now_seconds(),
                });
            }
        }
    }

    pub fn shutdown(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            sender.take();
        }
        if let Ok(mut guard) = self.thread.lock() {
            if let Some(thread) = guard.take() {
                let _ = thread.join();
            }
        }
    }
}
