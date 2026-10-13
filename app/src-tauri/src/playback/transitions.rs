//! Transport gain runs after the playback queue and uses running time across song boundaries.
use crate::model::AppResult;
use gstreamer::{self as gst, prelude::*};
use gstreamer_controller::{prelude::*, InterpolationControlSource, InterpolationMode};
use std::sync::{Arc, Mutex};

pub const PLAYBACK_FADE_MS: u64 = 200;
// Let the device/mixer consume the final quiet buffers before suspending it.
pub const PAUSE_GRACE_MS: u64 = 80;
#[cfg(not(target_os = "android"))]
const OUTPUT_LEAD_MS: u64 = 30;
#[cfg(not(target_os = "android"))]
const CLOCK_WAIT_MS: u64 = 5;

struct Envelope {
    source: InterpolationControlSource,
    enabled: bool,
    pending_in: bool,
}

impl Envelope {
    fn gain(&self, time: gst::ClockTime) -> f64 {
        gst::prelude::ControlSourceExt::value(&self.source, time).unwrap_or(1.0)
    }

    fn hold(&mut self, gain: f64) {
        self.pending_in = false;
        self.source.unset_all();
        self.source.set(gst::ClockTime::ZERO, gain);
    }

    fn ramp(&mut self, time: gst::ClockTime, target: f64) -> gst::ClockTime {
        let gain = self.gain(time);
        self.hold(gain);
        let end = time + gst::ClockTime::from_mseconds(PLAYBACK_FADE_MS);
        self.source.set(time, gain);
        self.source.set(end, target);
        end
    }
}

#[derive(Clone)]
pub struct PlaybackFade {
    element: gst::Element,
    envelope: Arc<Mutex<Envelope>>,
}

impl PlaybackFade {
    pub fn new() -> AppResult<Self> {
        let element = gst::ElementFactory::make("audioamplify")
            .name("nons-playback-fade")
            .build()
            .map_err(|e| e.to_string())?;
        let source = InterpolationControlSource::new();
        source.set_mode(InterpolationMode::Linear);
        source.set(gst::ClockTime::ZERO, 1.0);
        let envelope = Arc::new(Mutex::new(Envelope {
            source,
            enabled: false,
            pending_in: false,
        }));
        let weak_element = element.downgrade();
        let shared = envelope.clone();
        element
            .static_pad("sink")
            .ok_or("播放渐变入口不可用")?
            .add_probe(gst::PadProbeType::BUFFER, move |pad, info| {
                let Some(gst::PadProbeData::Buffer(buffer)) = &info.data else {
                    return gst::PadProbeReturn::Ok;
                };
                let time = buffer.pts().and_then(|pts| {
                    pad.sticky_event::<gst::event::Segment>(0)
                        .and_then(|event| {
                            event
                                .segment()
                                .downcast_ref::<gst::ClockTime>()
                                .and_then(|segment| segment.to_running_time(pts))
                        })
                });
                // Android's OpenSL ES ring buffer owns output pacing. The desktop
                // 30 ms feeding margin starves it and causes repeated skew correction.
                #[cfg(not(target_os = "android"))]
                if let (Some(element), Some(time)) = (weak_element.upgrade(), time) {
                    pace_output(&element, pad, time);
                }
                let mut envelope = shared.lock().unwrap_or_else(|e| e.into_inner());
                if !envelope.enabled {
                    return gst::PadProbeReturn::Ok;
                }
                let gain = if let Some(time) = time {
                    if envelope.pending_in {
                        envelope.ramp(time, 1.0);
                    }
                    envelope.gain(time)
                } else {
                    // Untimestamped audio must remain playable rather than held silent forever.
                    envelope.hold(1.0);
                    1.0
                };
                if let Some(element) = weak_element.upgrade() {
                    // BaseTransform syncs bound properties in stream time. Read the Controller
                    // explicitly in running time so a new track/seek cannot restart this ramp.
                    element.set_property("amplification", gain as f32);
                }
                gst::PadProbeReturn::Ok
            });
        Ok(Self { element, envelope })
    }

    pub fn attach(&self, playbin: &gst::Element) -> AppResult<()> {
        let sink = match playbin.property::<Option<gst::Element>>("audio-sink") {
            Some(sink) => sink,
            None => gst::ElementFactory::make("autoaudiosink")
                .build()
                .map_err(|e| e.to_string())?,
        };
        self.set_output(playbin, sink)
    }

    /// Call in NULL. This bin is after playsink's queue, so queued PCM can still fade.
    pub fn set_output(&self, playbin: &gst::Element, sink: gst::Element) -> AppResult<()> {
        let input = gst::ElementFactory::make("audioconvert")
            .build()
            .map_err(|e| e.to_string())?;
        let output = gst::ElementFactory::make("audioconvert")
            .build()
            .map_err(|e| e.to_string())?;
        // Decoder block sizes can exceed a third of the whole ramp. Bound control
        // granularity without inserting silence or discarding a short final block.
        let split = gst::ElementFactory::make("audiobuffersplit")
            .property("output-buffer-duration", gst::Fraction::new(1, 200))
            .property("gapless", false)
            .property("strict-buffer-size", false)
            .build()
            .map_err(|e| e.to_string())?;
        if let Some(parent) = self
            .element
            .parent()
            .and_then(|p| p.downcast::<gst::Bin>().ok())
        {
            parent.remove(&self.element).map_err(|e| e.to_string())?;
        }
        // Pacing must not depend on a device clock that waits for its ring buffer
        // to fill. AudioBaseSink slaves its device clock to this monotonic clock.
        let pipeline = playbin
            .downcast_ref::<gst::Pipeline>()
            .ok_or("播放管线不可用")?;
        if cfg!(target_os = "android") {
            pipeline.auto_clock();
        } else {
            pipeline.use_clock(Some(&gst::SystemClock::obtain()));
        }
        let bin = gst::Bin::new();
        bin.add_many([&input, &split, &self.element, &output, &sink])
            .map_err(|e| e.to_string())?;
        gst::Element::link_many([&input, &split, &self.element, &output, &sink])
            .map_err(|e| e.to_string())?;
        let pad = input.static_pad("sink").ok_or("音频输出入口不可用")?;
        let ghost = gst::GhostPad::builder_with_target(&pad)
            .map_err(|e| e.to_string())?
            .build();
        bin.add_pad(&ghost).map_err(|e| e.to_string())?;
        playbin.set_property("audio-sink", &bin);
        Ok(())
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut envelope = self.envelope.lock().unwrap_or_else(|e| e.into_inner());
        envelope.enabled = enabled;
        if !enabled {
            envelope.hold(1.0);
            self.element.set_property("amplification", 1.0f32);
        }
    }

    pub fn reset(&self) {
        let mut envelope = self.envelope.lock().unwrap_or_else(|e| e.into_inner());
        envelope.hold(1.0);
        self.element.set_property("amplification", 1.0f32);
    }

    /// Mute before starting, then ramp from the first buffer's running time (also after preroll).
    pub fn prepare_start(&self) {
        let mut envelope = self.envelope.lock().unwrap_or_else(|e| e.into_inner());
        if envelope.enabled {
            envelope.hold(0.0);
            envelope.pending_in = true;
            self.element.set_property("amplification", 0.0f32);
        }
    }

    pub fn fade_out(&self, time: gst::ClockTime) -> gst::ClockTime {
        self.envelope
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ramp(time, 0.0)
    }

    pub fn reverse_pause(&self, time: gst::ClockTime) {
        self.envelope
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ramp(time, 1.0);
    }
}

// WASAPI2 keeps a fixed 500 ms ring buffer even in low-latency mode. Pace
// before gain with a small feeding margin. clocksync marks the stream live and
// blocks preroll in this nested output bin; preserve non-live preroll here.
#[cfg(not(target_os = "android"))]
fn pace_output(element: &gst::Element, pad: &gst::Pad, time: gst::ClockTime) {
    while element.current_state() == gst::State::Playing
        && !pad.pad_flags().contains(gst::PadFlags::FLUSHING)
        && !matches!(
            element.pending_state(),
            gst::State::Paused | gst::State::Ready | gst::State::Null
        )
    {
        let Some(now) = element.current_running_time() else {
            break;
        };
        let lead = time.saturating_sub(now).mseconds();
        if lead <= OUTPUT_LEAD_MS {
            break;
        }
        let Some(clock) = element.clock() else {
            break;
        };
        // Bounded waits let state changes and flush interrupt a future timestamp.
        let deadline = clock.time()
            + gst::ClockTime::from_mseconds((lead - OUTPUT_LEAD_MS).min(CLOCK_WAIT_MS));
        let _ = clock.new_single_shot_id(deadline).wait();
    }
}

pub fn gapless_allowed(enabled: &std::sync::atomic::AtomicBool) -> bool {
    enabled.load(std::sync::atomic::Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_reverses_from_current_gain_without_a_step() {
        gst::init().unwrap();
        let fade = PlaybackFade::new().unwrap();
        fade.set_enabled(true);
        fade.fade_out(gst::ClockTime::from_seconds(1));
        let at = gst::ClockTime::from_mseconds(1100);
        assert!((fade.envelope.lock().unwrap().gain(at) - 0.5).abs() < 1e-6);
        fade.reverse_pause(at);
        let envelope = fade.envelope.lock().unwrap();
        assert!((envelope.gain(at) - 0.5).abs() < 1e-6);
        assert!((envelope.gain(gst::ClockTime::from_mseconds(1200)) - 0.75).abs() < 1e-6);
        assert_eq!(envelope.gain(gst::ClockTime::from_mseconds(1300)), 1.0);
    }

    #[test]
    fn disabling_or_resetting_cancels_silence_and_pending_start() {
        gst::init().unwrap();
        let fade = PlaybackFade::new().unwrap();
        fade.set_enabled(true);
        fade.prepare_start();
        fade.reset();
        assert!(!fade.envelope.lock().unwrap().pending_in);
        fade.fade_out(gst::ClockTime::ZERO);
        fade.set_enabled(false);
        let envelope = fade.envelope.lock().unwrap();
        assert!(!envelope.enabled);
        assert_eq!(envelope.gain(gst::ClockTime::from_seconds(1)), 1.0);
        assert_eq!(fade.element.property::<f32>("amplification"), 1.0);
    }
}
