use super::buffer::{Buffer, CHUNK_FRAMES};
use crate::model::{AppResult, ResolvedTrack};
use ffmpeg_audio::{AudioReader, HttpAudioSource, ResampleOptions, SeekMode};
use std::{
    fs::File,
    sync::{atomic::Ordering, Arc},
    thread::{self, JoinHandle},
    time::Duration,
};

pub(super) struct Decoder {
    pub buffer: Arc<Buffer>,
    thread: Option<JoinHandle<()>>,
}

impl Decoder {
    pub fn spawn(
        resolved: ResolvedTrack,
        uri: String,
        rate: u32,
        channels: usize,
        position_ms: u64,
    ) -> AppResult<Self> {
        let cue = resolved.track.cue();
        let start_ms = cue
            .map_or(0, |cue| cue.start_ms)
            .saturating_add(position_ms);
        let end_ms = cue.map(|cue| cue.end_ms);
        let duration_ms =
            crate::audio::track_duration(Some(&resolved.track), resolved.track.duration_ms);
        let buffer = Buffer::new(rate, channels, duration_ms);
        let shared = buffer.clone();
        let thread = thread::Builder::new()
            .name("nons-ffmpeg".into())
            .spawn(move || {
                if let Err(error) = decode(&resolved, &uri, &shared, start_ms, end_ms) {
                    if !shared.stop.load(Ordering::Acquire) {
                        shared.fail(error);
                    }
                }
                shared.done.store(true, Ordering::Release);
            })
            .map_err(|error| format!("启动音频解码失败：{error}"))?;
        Ok(Self {
            buffer,
            thread: Some(thread),
        })
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        self.buffer.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn decode(
    resolved: &ResolvedTrack,
    uri: &str,
    shared: &Arc<Buffer>,
    start_ms: u64,
    end_ms: Option<u64>,
) -> AppResult<()> {
    let mut reader = if let Some(file) = &resolved.decoded_audio {
        AudioReader::new(
            File::open(file.path()).map_err(|error| format!("打开本地音频失败：{error}"))?,
        )
    } else if let crate::model::TrackSource::Local { path, .. } = &resolved.track.source {
        AudioReader::new(
            crate::local::source::open(std::path::PathBuf::from(path))
                .map_err(|error| format!("打开本地音频失败：{error}"))?,
        )
    } else {
        AudioReader::new(
            HttpAudioSource::new_with_cancel_handle(uri, &shared.cancel)
                .map_err(|_| "打开在线音频失败".to_owned())?,
        )
    }
    .map_err(|error| format!("解析音频失败：{error}"))?;
    if end_ms.is_none() {
        if let Some(duration) = reader.duration() {
            shared.duration_ms.store(
                duration.as_millis().min(u64::MAX as u128) as u64,
                Ordering::Release,
            );
        }
    }
    if start_ms > 0 {
        reader
            .seek(Duration::from_millis(start_ms), SeekMode::Accurate)
            .map_err(|error| format!("音频定位失败：{error}"))?;
    }
    let mut resampler = reader
        .build_resampler(
            ResampleOptions::new()
                .sample_rate(shared.rate as i32)
                .channels(shared.channels as i32)
                .format::<f32>(),
        )
        .map_err(|error| format!("构建音频重采样失败：{error}"))?;
    let limit = end_ms
        .map(|end| {
            end.saturating_sub(start_ms)
                .saturating_mul(u64::from(shared.rate))
                / 1000
                * shared.channels as u64
        })
        .unwrap_or(u64::MAX);
    let mut written = 0;
    while !shared.stop.load(Ordering::Acquire) && written < limit {
        match reader
            .receive_frame()
            .map_err(|error| format!("音频解码失败：{error}"))?
        {
            Some(frame) => {
                resampler
                    .process::<f32>(Some(&frame))
                    .map_err(|error| format!("音频重采样失败：{error}"))?;
                written += push(shared, resampler.output_as::<f32>(), limit - written);
            }
            None => {
                loop {
                    resampler
                        .process::<f32>(None)
                        .map_err(|error| format!("排空音频重采样失败：{error}"))?;
                    let samples = resampler.output_as::<f32>();
                    if samples.is_empty() || shared.stop.load(Ordering::Acquire) || written >= limit
                    {
                        break;
                    }
                    written += push(shared, samples, limit - written);
                }
                break;
            }
        }
    }
    Ok(())
}

fn push(shared: &Buffer, samples: &[f32], limit: u64) -> u64 {
    let count = samples.len().min(limit.min(usize::MAX as u64) as usize);
    let count = count / shared.channels * shared.channels;
    let mut written = 0;
    for samples in samples[..count].chunks(CHUNK_FRAMES * shared.channels) {
        let mut chunk = loop {
            if shared.stop.load(Ordering::Acquire) {
                return written;
            }
            if let Some(chunk) = shared.take_chunk() {
                break chunk;
            }
            thread::sleep(Duration::from_millis(2));
        };
        chunk.extend_from_slice(samples);
        loop {
            match shared.samples.push(chunk) {
                Ok(()) => break,
                Err(returned) => chunk = returned,
            }
            if shared.stop.load(Ordering::Acquire) {
                shared.recycle(chunk);
                return written;
            }
            thread::sleep(Duration::from_millis(2));
        }
        written += samples.len() as u64;
    }
    written
}
