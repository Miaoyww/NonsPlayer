use super::buffer::{Buffer, CHUNK_FRAMES};
use crate::model::{AppResult, ResolvedTrack};
use crate::playback::engine::Resource;
use ffmpeg_audio::{AudioReader, HttpAudioSource, ResampleOptions, Resampler, SeekMode};
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

#[derive(Default)]
struct DecodeStats {
    decoded_samples: u64,
    source_rate: u64,
    declared_end_ms: Option<u64>,
}

impl Decoder {
    #[cfg(test)]
    pub fn spawn(
        resolved: ResolvedTrack,
        uri: String,
        rate: u32,
        channels: usize,
        position_ms: u64,
    ) -> AppResult<Self> {
        Self::spawn_with_resource(resolved, uri, rate, channels, position_ms, None)
    }

    pub(super) fn spawn_with_resource(
        resolved: ResolvedTrack,
        uri: String,
        rate: u32,
        channels: usize,
        position_ms: u64,
        resource: Resource,
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
                let mut stats = DecodeStats::default();
                if let Err(error) = decode(&resolved, &uri, &shared, start_ms, end_ms, &mut stats) {
                    if !shared.stop.load(Ordering::Acquire) {
                        let source =
                            crate::infrastructure::diagnostics::field(resolved.track.source_id());
                        let track_id = match &resolved.track.source {
                            crate::model::TrackSource::Online { track } => {
                                crate::infrastructure::diagnostics::field(
                                    track.reference.id.as_str(),
                                )
                            }
                            crate::model::TrackSource::Local { .. } => "local".into(),
                        };
                        let retryable = is_retryable_playback_error(&error);
                        let decoded_ms = if stats.source_rate == 0 {
                            0
                        } else {
                            stats.decoded_samples.saturating_mul(1000) / stats.source_rate
                        };
                        let resource_age_ms =
                            resource.as_ref().map_or(0, |resource| resource.age_ms());
                        crate::infrastructure::diagnostics::warn(
                            "playback.decoder",
                            format_args!(
                                "source={source} track_id={track_id} position_ms={start_ms} decoded_samples={} decoded_ms={decoded_ms} source_rate={} declared_end_ms={} resource_age_ms={resource_age_ms} retryable={retryable} {error}",
                                stats.decoded_samples,
                                stats.source_rate,
                                stats
                                    .declared_end_ms
                                    .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                            ),
                        );
                        shared.fail(error, retryable);
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
    stats: &mut DecodeStats,
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
                .map_err(|error| format!("打开在线音频失败：{error}"))?,
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
    let declared_end_ms = reader
        .duration()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64);
    let source_rate = u64::try_from(reader.source_info().sample_rate).unwrap_or(0);
    stats.declared_end_ms = declared_end_ms;
    stats.source_rate = source_rate;
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
        match reader.receive_frame() {
            Ok(Some(frame)) => {
                stats.decoded_samples += frame.samples() as u64;
                resampler
                    .process::<f32>(Some(&frame))
                    .map_err(|error| format!("音频重采样失败：{error}"))?;
                written += push(shared, resampler.output_as::<f32>(), limit - written);
            }
            Ok(None) => {
                drain(&mut resampler, shared, &mut written, limit)?;
                break;
            }
            // Some formats leave non-audio tags at the end of the stream, which the
            // parser forwards as a junk frame. Once every declared sample is decoded,
            // a decode error can only come from those trailing bytes.
            Err(_)
                if reached_declared_end(
                    declared_end_ms,
                    start_ms,
                    source_rate,
                    stats.decoded_samples,
                ) =>
            {
                drain(&mut resampler, shared, &mut written, limit)?;
                break;
            }
            Err(error) => return Err(format!("音频解码失败：{error}")),
        }
    }
    Ok(())
}

pub(super) fn is_retryable_playback_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("http error status: 403")
        || error.contains("http error status: 410")
        || error.contains("invalid data found when processing input")
}

fn drain(
    resampler: &mut Resampler,
    shared: &Buffer,
    written: &mut u64,
    limit: u64,
) -> AppResult<()> {
    loop {
        resampler
            .process::<f32>(None)
            .map_err(|error| format!("排空音频重采样失败：{error}"))?;
        let samples = resampler.output_as::<f32>();
        if samples.is_empty() || shared.stop.load(Ordering::Acquire) || *written >= limit {
            break;
        }
        *written += push(shared, samples, limit - *written);
    }
    Ok(())
}

pub(super) fn reached_declared_end(
    declared_end_ms: Option<u64>,
    start_ms: u64,
    source_rate: u64,
    decoded: u64,
) -> bool {
    if source_rate == 0 {
        return false;
    }
    declared_end_ms.is_some_and(|end| {
        decoded >= end.saturating_sub(start_ms).saturating_mul(source_rate) / 1000
    })
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

#[cfg(test)]
mod tests {
    use super::is_retryable_playback_error;

    #[test]
    fn retryable_playback_errors_cover_stale_and_invalid_streams() {
        assert!(is_retryable_playback_error(
            "打开在线音频失败：HTTP error: HTTP error status: 403"
        ));
        assert!(is_retryable_playback_error(
            "打开在线音频失败：HTTP error: HTTP error status: 410"
        ));
        assert!(is_retryable_playback_error(
            "音频解码失败：FFmpeg error -1094995529: Invalid data found when processing input"
        ));
        assert!(!is_retryable_playback_error(
            "打开在线音频失败：HTTP error: HTTP error status: 404"
        ));
    }
}
