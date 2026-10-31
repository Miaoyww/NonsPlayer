use super::buffer::Buffer;
use std::sync::{atomic::Ordering, Arc};

const MAX_CHANNELS: usize = 32;

pub(super) struct Voice {
    pub buffer: Arc<Buffer>,
    pub start_ms: u64,
    pub cue_start_ms: u64,
    pub token: String,
    samples: Vec<f32>,
    offset: usize,
}

impl Voice {
    pub fn new(buffer: Arc<Buffer>, start_ms: u64, cue_start_ms: u64, token: String) -> Self {
        Self {
            buffer,
            start_ms,
            cue_start_ms,
            token,
            samples: Vec::new(),
            offset: 0,
        }
    }

    pub fn position_ms(&self) -> u64 {
        self.start_ms.saturating_add(self.buffer.position_ms())
    }

    pub fn duration_ms(&self) -> u64 {
        self.buffer.duration_ms.load(Ordering::Acquire)
    }

    pub fn finished(&self) -> bool {
        self.buffer.done.load(Ordering::Acquire)
            && self.offset >= self.samples.len()
            && self.buffer.samples.is_empty()
    }

    fn frame(&mut self, output: &mut [f32]) -> bool {
        if self.offset >= self.samples.len() {
            let Some(samples) = self.buffer.samples.pop() else {
                return false;
            };
            let previous = std::mem::replace(&mut self.samples, samples);
            self.buffer.recycle(previous);
            self.offset = 0;
        }
        output.copy_from_slice(&self.samples[self.offset..self.offset + output.len()]);
        self.offset += output.len();
        self.buffer
            .consumed
            .fetch_add(output.len() as u64, Ordering::Release);
        true
    }

    fn ready(&self) -> bool {
        !self.buffer.failed.load(Ordering::Acquire)
            && (self.offset < self.samples.len() || !self.buffer.samples.is_empty())
    }
}

#[derive(Clone, Copy)]
struct Ramp {
    value: f32,
    target: f32,
    remaining: u64,
}

impl Ramp {
    fn new(value: f32) -> Self {
        Self {
            value,
            target: value,
            remaining: 0,
        }
    }
    fn to(&mut self, target: f32, frames: u64) {
        self.target = target;
        self.remaining = frames;
        if frames == 0 {
            self.value = target;
        }
    }
    fn next(&mut self) -> f32 {
        if self.remaining > 0 {
            self.value += (self.target - self.value) / self.remaining as f32;
            self.remaining -= 1;
        }
        self.value
    }
}

pub(super) struct Mixer {
    pub current: Voice,
    pub next: Option<Voice>,
    pub outgoing: Option<Voice>,
    pub transition_started: bool,
    pub transition_finished: bool,
    pub buffering: bool,
    pub playing: bool,
    pub fades: bool,
    pub crossfade_ms: u32,
    envelope: Ramp,
    volume: Ramp,
    fade_frame: u64,
    fade_frames: u64,
}

impl Mixer {
    pub fn new(current: Voice, playing: bool, volume: f32, fades: bool, crossfade_ms: u32) -> Self {
        let mut envelope = Ramp::new(if playing && !fades { 1.0 } else { 0.0 });
        if playing && fades {
            envelope.to(1.0, u64::from(current.buffer.rate) / 5);
        }
        Self {
            current,
            next: None,
            outgoing: None,
            transition_started: false,
            transition_finished: false,
            buffering: false,
            playing,
            fades,
            crossfade_ms,
            envelope,
            volume: Ramp::new(volume),
            fade_frame: 0,
            fade_frames: 0,
        }
    }

    pub fn intent(&mut self, playing: bool, volume: f32) {
        let rate = u64::from(self.current.buffer.rate);
        if self.playing != playing {
            self.playing = playing;
            self.envelope.to(
                if playing { 1.0 } else { 0.0 },
                if self.fades { rate / 5 } else { 0 },
            );
        }
        if self.volume.target != volume {
            self.volume.to(volume, rate / 100);
        }
    }

    pub fn configure(&mut self, fades: bool, crossfade_ms: u32) {
        self.fades = fades;
        self.crossfade_ms = crossfade_ms;
        if !fades {
            self.envelope.to(if self.playing { 1.0 } else { 0.0 }, 0);
        }
    }

    pub fn render<T: cpal::Sample + cpal::FromSample<f32>>(&mut self, data: &mut [T]) {
        let channels = self.current.buffer.channels;
        let mut incoming = [0.0; MAX_CHANNELS];
        let mut outgoing = [0.0; MAX_CHANNELS];
        self.buffering = false;
        for frame in data.chunks_exact_mut(channels) {
            if !self.playing && self.envelope.value == 0.0 {
                frame.fill(T::EQUILIBRIUM);
                continue;
            }
            let remaining = self.current.duration_ms().saturating_sub(
                self.current
                    .position_ms()
                    .saturating_sub(self.current.cue_start_ms),
            );
            let crossfade_ms = self.next.as_ref().map_or(0, |next| {
                let duration = next.duration_ms();
                if duration > 0 {
                    u64::from(self.crossfade_ms).min(duration)
                } else {
                    u64::from(self.crossfade_ms)
                }
            });
            if crossfade_ms > 0
                && self.playing
                && self.outgoing.is_none()
                && self.current.duration_ms() > 0
                && remaining <= crossfade_ms
                && self.next.as_ref().is_some_and(Voice::ready)
            {
                let next = self.next.take().unwrap();
                self.outgoing = Some(std::mem::replace(&mut self.current, next));
                self.fade_frames =
                    remaining.min(crossfade_ms) * u64::from(self.current.buffer.rate) / 1000;
                self.fade_frame = 0;
                self.transition_started = true;
                self.transition_finished = self.fade_frames == 0;
            }
            incoming[..channels].fill(0.0);
            let consumed = self.current.frame(&mut incoming[..channels]);
            if !consumed && !self.current.finished() {
                self.buffering = true;
            }
            let envelope = if consumed || !self.playing {
                self.envelope.next()
            } else {
                self.envelope.value
            };
            let gain = envelope * self.volume.next();
            let mut incoming_gain = 1.0;
            let mut outgoing_gain = 0.0;
            let mut outgoing_consumed = false;
            if self.outgoing.is_some() && !self.transition_finished {
                // Freeze both voices when incoming PCM stalls, preserving the transition clock.
                if consumed || self.current.finished() {
                    outgoing[..channels].fill(0.0);
                    outgoing_consumed = self
                        .outgoing
                        .as_mut()
                        .unwrap()
                        .frame(&mut outgoing[..channels]);
                    self.fade_frame += 1;
                }
                incoming_gain = self.fade_frame as f32 / self.fade_frames.max(1) as f32;
                outgoing_gain = 1.0 - incoming_gain;
                if self.fade_frame >= self.fade_frames {
                    self.transition_finished = true;
                }
            }
            for channel in 0..channels {
                let sample = if consumed || outgoing_consumed {
                    incoming[channel] * incoming_gain + outgoing[channel] * outgoing_gain
                } else {
                    0.0
                };
                frame[channel] = T::from_sample((sample * gain).clamp(-1.0, 1.0));
            }
        }
        let tail = data.len() / channels * channels;
        data[tail..].fill(T::EQUILIBRIUM);
    }
}
