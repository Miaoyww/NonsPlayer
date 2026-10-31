use crossbeam_queue::ArrayQueue;
use ffmpeg_audio::HttpCancelHandle;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

pub(super) const CHUNK_FRAMES: usize = 1024;
const BUFFER_MS: usize = 500;

pub(super) struct Buffer {
    pub samples: ArrayQueue<Vec<f32>>,
    recycled: ArrayQueue<Vec<f32>>,
    pub stop: AtomicBool,
    pub done: AtomicBool,
    pub failed: AtomicBool,
    pub consumed: AtomicU64,
    pub duration_ms: AtomicU64,
    pub error: Mutex<Option<String>>,
    pub cancel: HttpCancelHandle,
    pub rate: u32,
    pub channels: usize,
}

impl Buffer {
    pub fn new(rate: u32, channels: usize, duration_ms: u64) -> Arc<Self> {
        let capacity = (rate as usize * BUFFER_MS / 1000)
            .div_ceil(CHUNK_FRAMES)
            .max(2);
        let recycled = ArrayQueue::new(capacity + 2);
        for _ in 0..capacity + 2 {
            recycled
                .push(Vec::with_capacity(CHUNK_FRAMES * channels))
                .unwrap();
        }
        Arc::new(Self {
            samples: ArrayQueue::new(capacity),
            recycled,
            stop: AtomicBool::new(false),
            done: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            consumed: AtomicU64::new(0),
            duration_ms: AtomicU64::new(duration_ms),
            error: Mutex::new(None),
            cancel: HttpCancelHandle::new(),
            rate,
            channels,
        })
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        self.cancel.cancel();
    }

    pub fn fail(&self, error: String) {
        *self.error.lock().unwrap() = Some(error);
        self.failed.store(true, Ordering::Release);
    }

    pub fn take_chunk(&self) -> Option<Vec<f32>> {
        self.recycled.pop()
    }

    pub fn recycle(&self, mut samples: Vec<f32>) {
        if samples.capacity() == 0 {
            return;
        }
        samples.clear();
        // There are exactly capacity + 2 buffers, including both in-flight chunks.
        assert!(self.recycled.push(samples).is_ok());
    }

    pub fn position_ms(&self) -> u64 {
        self.consumed.load(Ordering::Acquire) / self.channels as u64 * 1000 / self.rate as u64
    }
}
