//! FFmpeg decoding and shared-mode device output, controlled by the playback actor.

mod buffer;
mod decoder;
mod mixer;

use crate::model::{AppResult, PlayerSnapshot, ResolvedTrack};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample,
};
use decoder::Decoder;
use mixer::{Mixer, Voice};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

pub(crate) use super::web_audio::{Control, Feedback, FeedbackKind};
use super::{
    engine::Resource,
    web_audio::{NextSource, WebAudio},
};

struct Session {
    stream: Option<cpal::Stream>,
    mixer: Arc<Mutex<Mixer>>,
    decoders: Vec<Decoder>,
    output_failed: Arc<AtomicBool>,
    latency_ms: Arc<AtomicU64>,
    drained_at: Option<Instant>,
}

impl Drop for Session {
    fn drop(&mut self) {
        for decoder in &self.decoders {
            decoder.buffer.stop();
        }
        self.stream.take();
        self.decoders.clear();
    }
}

pub(crate) struct AudioEngine {
    control: Control,
    transport: WebAudio,
    session: Option<Session>,
    current: Option<(ResolvedTrack, Resource, u64)>,
    prepared: Option<(ResolvedTrack, Resource, u64)>,
    sequence: u64,
    last_sequence: u64,
    last_status: Option<FeedbackKind>,
    last_progress: Instant,
    recovering_device: bool,
}

impl AudioEngine {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            control: Control::default(),
            transport: WebAudio::new()?,
            session: None,
            current: None,
            prepared: None,
            sequence: 0,
            last_sequence: 0,
            last_status: None,
            last_progress: Instant::now(),
            recovering_device: false,
        })
    }

    pub fn control(&self) -> Control {
        self.control.clone()
    }

    pub fn configure(&mut self, state: &PlayerSnapshot) -> AppResult<()> {
        let previous = self.control.device_id.clone();
        let changed = previous != state.device_id
            || self
                .session
                .as_ref()
                .is_some_and(|session| session.output_failed.load(Ordering::Acquire));
        if changed {
            // Validate saved endpoint IDs even when nothing is playing.
            select_device(state.device_id.as_deref())?;
        }
        self.control.fades = state.playback_options.fades;
        self.control.crossfade_ms = if state.playback_options.crossfade {
            state.playback_options.crossfade_duration_ms
        } else {
            0
        };
        if changed {
            let position = self
                .session
                .as_ref()
                .map(|session| {
                    let mixer = session.mixer.lock().unwrap();
                    mixer
                        .current
                        .position_ms()
                        .saturating_sub(mixer.current.cue_start_ms)
                })
                .unwrap_or(self.control.position_ms);
            self.control.device_id = state.device_id.clone();
            if let Some((resolved, resource, generation)) = self.current.clone() {
                if let Err(error) = self.load_internal(
                    resolved.clone(),
                    resource.clone(),
                    generation,
                    position,
                    self.control.playing,
                    self.control.volume,
                ) {
                    self.control.device_id = previous;
                    let _ = self.load_internal(
                        resolved,
                        resource,
                        generation,
                        position,
                        self.control.playing,
                        self.control.volume,
                    );
                    return Err(error);
                }
            }
        }
        if let Some(session) = &self.session {
            session
                .mixer
                .lock()
                .unwrap()
                .configure(self.control.fades, self.control.crossfade_ms);
        }
        Ok(())
    }

    pub fn emit(&mut self, _app: &tauri::AppHandle) -> AppResult<()> {
        Ok(())
    }

    pub fn cancel_preload(&mut self, _app: &tauri::AppHandle) -> AppResult<()> {
        self.cancel_prepared()
    }

    fn cancel_prepared(&mut self) -> AppResult<()> {
        let mut retain_outgoing = false;
        if let Some(session) = &mut self.session {
            let mut mixer = session.mixer.lock().unwrap();
            if let Some(next) = mixer.next.take() {
                next.buffer.stop();
            }
            // A queue edit may arrive after the callback switched voices, before actor promotion.
            if self.control.next.is_some() && mixer.transition_started {
                if let Some(outgoing) = mixer.outgoing.take() {
                    let cancelled = std::mem::replace(&mut mixer.current, outgoing);
                    cancelled.buffer.stop();
                }
                mixer.transition_started = false;
                mixer.transition_finished = false;
            }
            retain_outgoing = mixer.outgoing.is_some();
            drop(mixer);
            session
                .decoders
                .retain(|decoder| !decoder.buffer.stop.load(Ordering::Acquire));
        }
        self.prepared = None;
        self.control.next = None;
        if retain_outgoing {
            Ok(())
        } else {
            self.transport.clear_secondary()
        }
    }

    pub fn prepare(
        &mut self,
        app: &tauri::AppHandle,
        resolved: ResolvedTrack,
        resource: Resource,
        generation: u64,
    ) -> AppResult<()> {
        self.cancel_preload(app)?;
        if !self.ready_to_prepare() {
            return Err("音频过渡尚未完成".into());
        }
        if let Some(resource) = &resource {
            resource
                .check(generation)
                .map_err(|error| error.to_string())?;
        }
        let session = self.session.as_mut().ok_or("当前音频尚未启动")?;
        let uri = self
            .transport
            .install_source(resolved.clone(), resource.clone(), true)?;
        let (rate, channels) = {
            let mixer = session.mixer.lock().unwrap();
            (mixer.current.buffer.rate, mixer.current.buffer.channels)
        };
        let decoder = Decoder::spawn(resolved.clone(), uri.clone(), rate, channels, 0)?;
        let start_ms = resolved.track.cue().map_or(0, |cue| cue.start_ms);
        session.mixer.lock().unwrap().next = Some(Voice::new(
            decoder.buffer.clone(),
            start_ms,
            start_ms,
            uri.clone(),
        ));
        self.control.next = Some(NextSource {
            source: uri,
            start_ms,
            end_ms: resolved.track.cue().map(|cue| cue.end_ms),
        });
        session.decoders.push(decoder);
        self.prepared = Some((resolved, resource, generation));
        Ok(())
    }

    pub fn promote(&mut self, _app: &tauri::AppHandle, position_ms: u64) -> AppResult<()> {
        let prepared = self.prepared.take().ok_or("下一首音频尚未准备")?;
        let next = self.control.next.take().ok_or("下一首音频尚未准备")?;
        self.transport.promote_source()?;
        self.current = Some(prepared);
        self.control.source = Some(next.source);
        self.control.start_ms = next.start_ms;
        self.control.end_ms = next.end_ms;
        self.control.position_ms = position_ms;
        self.last_status = None;
        if let Some(session) = &mut self.session {
            session.drained_at = None;
        }
        Ok(())
    }

    pub fn retire(&mut self) -> AppResult<()> {
        if let Some(session) = &mut self.session {
            let mut mixer = session.mixer.lock().unwrap();
            if let Some(outgoing) = mixer.outgoing.take() {
                outgoing.buffer.stop();
            }
            mixer.transition_started = false;
            mixer.transition_finished = false;
            drop(mixer);
            session
                .decoders
                .retain(|decoder| !decoder.buffer.stop.load(Ordering::Acquire));
        }
        self.transport.clear_secondary()
    }

    pub fn ready_to_prepare(&mut self) -> bool {
        self.control.next.is_none()
            && self.session.as_ref().is_some_and(|session| {
                let mixer = session.mixer.lock().unwrap();
                mixer.outgoing.is_none() && mixer.next.is_none()
            })
    }

    pub fn stop(&mut self, _app: &tauri::AppHandle) -> AppResult<()> {
        self.control.serial = self.control.serial.wrapping_add(1);
        self.session.take();
        self.prepared = None;
        self.current = None;
        self.control.source = None;
        self.control.next = None;
        self.control.playing = false;
        self.transport.clear_current()?;
        self.transport.clear_secondary()
    }

    pub fn load(
        &mut self,
        _app: &tauri::AppHandle,
        resolved: ResolvedTrack,
        resource: Resource,
        generation: u64,
        position_ms: u64,
        playing: bool,
        volume: f64,
    ) -> AppResult<()> {
        if let Some(resource) = &resource {
            resource
                .check(generation)
                .map_err(|error| error.to_string())?;
        }
        self.recovering_device = false;
        self.load_internal(resolved, resource, generation, position_ms, playing, volume)
    }

    pub fn intent(
        &mut self,
        _app: &tauri::AppHandle,
        playing: bool,
        volume: f64,
        seek: Option<u64>,
    ) -> AppResult<()> {
        if let Some(position) = seek {
            if let Some((resolved, resource, generation)) = self.current.clone() {
                return self
                    .load_internal(resolved, resource, generation, position, playing, volume);
            }
        }
        self.control.playing = playing;
        self.control.volume = volume;
        if let Some(session) = &mut self.session {
            session.mixer.lock().unwrap().intent(playing, volume as f32);
            session.drained_at = None;
        }
        Ok(())
    }

    pub fn accepts(&mut self, feedback: &Feedback) -> bool {
        if feedback.serial != self.control.serial || feedback.sequence <= self.last_sequence {
            return false;
        }
        self.last_sequence = feedback.sequence;
        true
    }

    pub fn poll(&mut self) -> Option<Feedback> {
        let session = self.session.as_mut()?;
        let mixer = session.mixer.lock().unwrap();
        let position_ms = mixer.current.position_ms();
        let duration_ms = mixer
            .current
            .cue_start_ms
            .saturating_add(mixer.current.duration_ms());
        let mut next_source = None;
        let mut error = None;
        let kind = if session.output_failed.load(Ordering::Acquire) {
            if self.recovering_device {
                error = Some("系统默认音频设备不可用".into());
                FeedbackKind::DeviceError
            } else {
                self.recovering_device = true;
                FeedbackKind::DeviceDefault
            }
        } else if mixer.transition_started && self.control.next.is_some() {
            next_source = Some(mixer.current.token.clone());
            FeedbackKind::Transition
        } else if mixer.transition_finished {
            FeedbackKind::TransitionEnd
        } else if mixer
            .next
            .as_ref()
            .is_some_and(|voice| voice.buffer.failed.load(Ordering::Acquire))
        {
            let next = mixer.next.as_ref().unwrap();
            next_source = Some(next.token.clone());
            FeedbackKind::PreloadError
        } else if mixer.current.buffer.failed.load(Ordering::Acquire) {
            error = mixer.current.buffer.error.lock().unwrap().clone();
            FeedbackKind::Error
        } else if mixer.playing && mixer.current.finished() && mixer.outgoing.is_none() {
            let drained_at = session.drained_at.get_or_insert_with(Instant::now);
            if drained_at.elapsed()
                < Duration::from_millis(session.latency_ms.load(Ordering::Relaxed))
            {
                return None;
            }
            FeedbackKind::Ended
        } else {
            let status = if !mixer.playing {
                FeedbackKind::Paused
            } else if mixer.buffering || mixer.current.buffer.consumed.load(Ordering::Acquire) == 0
            {
                FeedbackKind::Buffering
            } else {
                FeedbackKind::Playing
            };
            if self.last_status == Some(status) {
                if self.last_progress.elapsed() < Duration::from_millis(100) {
                    return None;
                }
                self.last_progress = Instant::now();
                FeedbackKind::Progress
            } else {
                self.last_status = Some(status);
                status
            }
        };
        self.sequence = self.sequence.wrapping_add(1);
        Some(Feedback {
            serial: self.control.serial,
            sequence: self.sequence,
            kind,
            position_ms,
            duration_ms,
            error,
            next_source,
        })
    }

    fn load_internal(
        &mut self,
        resolved: ResolvedTrack,
        resource: Resource,
        generation: u64,
        position_ms: u64,
        playing: bool,
        volume: f64,
    ) -> AppResult<()> {
        let device = select_device(self.control.device_id.as_deref())?;
        let supported = device
            .default_output_config()
            .map_err(|error| format!("读取音频设备配置失败：{error}"))?;
        if supported.channels() == 0 || supported.channels() > 32 || supported.sample_rate() == 0 {
            return Err("不支持的音频设备配置".into());
        }
        self.session.take();
        self.prepared = None;
        self.transport.clear_secondary()?;
        let uri = self
            .transport
            .install_source(resolved.clone(), resource.clone(), false)?;
        let cue_start_ms = resolved.track.cue().map_or(0, |cue| cue.start_ms);
        let decoder = Decoder::spawn(
            resolved.clone(),
            uri.clone(),
            supported.sample_rate(),
            supported.channels() as usize,
            position_ms,
        )?;
        let mixer = Arc::new(Mutex::new(Mixer::new(
            Voice::new(
                decoder.buffer.clone(),
                cue_start_ms.saturating_add(position_ms),
                cue_start_ms,
                uri.clone(),
            ),
            playing,
            volume as f32,
            self.control.fades,
            self.control.crossfade_ms,
        )));
        let output_failed = Arc::new(AtomicBool::new(false));
        let latency_ms = Arc::new(AtomicU64::new(100));
        let stream = build_stream(
            &device,
            &supported,
            mixer.clone(),
            output_failed.clone(),
            latency_ms.clone(),
        )?;
        stream
            .play()
            .map_err(|error| format!("启动音频输出失败：{error}"))?;
        self.session = Some(Session {
            stream: Some(stream),
            mixer,
            decoders: vec![decoder],
            output_failed,
            latency_ms,
            drained_at: None,
        });
        self.control.serial = self.control.serial.wrapping_add(1);
        self.last_sequence = 0;
        self.last_status = None;
        self.control.source = Some(uri);
        self.control.next = None;
        self.control.position_ms = position_ms;
        self.control.start_ms = cue_start_ms;
        self.control.end_ms = resolved.track.cue().map(|cue| cue.end_ms);
        self.control.playing = playing;
        self.control.volume = volume;
        self.current = Some((resolved, resource, generation));
        self.recovering_device = false;
        Ok(())
    }
}

fn select_device(selected: Option<&str>) -> AppResult<cpal::Device> {
    let host = cpal::default_host();
    if let Some(selected) = selected {
        host.output_devices()
            .map_err(|error| format!("枚举音频设备失败：{error}"))?
            .find(|device| device.id().map(|id| id.to_string()).ok().as_deref() == Some(selected))
            .ok_or_else(|| "所选音频输出设备不可用".into())
    } else {
        host.default_output_device()
            .ok_or_else(|| "没有可用的音频输出设备".into())
    }
}

pub(crate) fn output_devices() -> AppResult<Vec<super::engine::OutputDevice>> {
    Ok(cpal::default_host()
        .output_devices()
        .map_err(|error| format!("枚举音频设备失败：{error}"))?
        .filter_map(|device| {
            Some(super::engine::OutputDevice {
                id: device.id().ok()?.to_string(),
                name: device.description().ok()?.name().to_owned(),
            })
        })
        .collect())
}

fn build_stream(
    device: &cpal::Device,
    supported: &cpal::SupportedStreamConfig,
    mixer: Arc<Mutex<Mixer>>,
    failed: Arc<AtomicBool>,
    latency: Arc<AtomicU64>,
) -> AppResult<cpal::Stream> {
    let config = supported.config();
    macro_rules! build {
        ($sample:ty) => {
            build_typed::<$sample>(device, config, mixer, failed, latency)
        };
    }
    match supported.sample_format() {
        SampleFormat::I8 => build!(i8),
        SampleFormat::I16 => build!(i16),
        SampleFormat::I24 => build!(cpal::I24),
        SampleFormat::I32 => build!(i32),
        SampleFormat::I64 => build!(i64),
        SampleFormat::U8 => build!(u8),
        SampleFormat::U16 => build!(u16),
        SampleFormat::U32 => build!(u32),
        SampleFormat::U64 => build!(u64),
        SampleFormat::F32 => build!(f32),
        SampleFormat::F64 => build!(f64),
        _ => Err("不支持的音频输出格式".into()),
    }
}

fn build_typed<T: SizedSample + Sample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mixer: Arc<Mutex<Mixer>>,
    failed: Arc<AtomicBool>,
    latency: Arc<AtomicU64>,
) -> AppResult<cpal::Stream> {
    let rate = config.sample_rate;
    let channels = config.channels as usize;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], info| {
                let timestamp = info.timestamp();
                let delay = timestamp.playback.duration_since(timestamp.callback);
                latency.store(
                    output_drain_ms(delay, data.len(), rate, channels),
                    Ordering::Relaxed,
                );
                if let Ok(mut mixer) = mixer.try_lock() {
                    mixer.render(data);
                } else {
                    data.fill(T::EQUILIBRIUM);
                }
            },
            move |error| {
                if !matches!(
                    error.kind(),
                    cpal::ErrorKind::Xrun
                        | cpal::ErrorKind::RealtimeDenied
                        | cpal::ErrorKind::DeviceChanged
                ) {
                    failed.store(true, Ordering::Release);
                }
            },
            None,
        )
        .map_err(|error| format!("创建音频输出失败：{error}"))
}

fn output_drain_ms(delay: Duration, samples: usize, rate: u32, channels: usize) -> u64 {
    // The playback timestamp refers to the first frame in this callback, not its end.
    let frames = samples / channels;
    let block_ms = (frames as u64 * 1000).div_ceil(u64::from(rate));
    (delay.as_millis().min(u64::MAX as u128) as u64).saturating_add(block_ms)
}

#[cfg(test)]
mod tests;
