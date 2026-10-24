//! Transport gain runs after the playback queue and uses running time across song boundaries.
use crate::model::AppResult;
use gstreamer::{self as gst, prelude::*};
use gstreamer_controller::{prelude::*, InterpolationControlSource, InterpolationMode};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, TryLockError,
};

pub const PLAYBACK_FADE_MS: u64 = 200;
// Let the device/mixer consume the final quiet buffers before suspending it.
pub const PAUSE_GRACE_MS: u64 = 80;

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
    enabled: Arc<AtomicBool>,
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
        let enabled = Arc::new(AtomicBool::new(false));
        let active = enabled.clone();
        element
            .static_pad("sink")
            .ok_or("播放渐变入口不可用")?
            .add_probe(gst::PadProbeType::BUFFER, move |pad, info| {
                // The default disabled path must not contend with transport controls
                // or query segment/controller state for every 5 ms of PCM.
                if !active.load(Ordering::Acquire) {
                    return gst::PadProbeReturn::Ok;
                }
                let Some(gst::PadProbeData::Buffer(buffer)) = &info.data else {
                    return gst::PadProbeReturn::Ok;
                };
                // A descheduled control thread must never hold up audio supply.
                // Keep its last committed amplification until the next buffer.
                let mut envelope = match shared.try_lock() {
                    Ok(envelope) => envelope,
                    Err(TryLockError::WouldBlock) => return gst::PadProbeReturn::Ok,
                    Err(TryLockError::Poisoned(error)) => error.into_inner(),
                };
                if !envelope.enabled {
                    return gst::PadProbeReturn::Ok;
                }
                let time = buffer.pts().and_then(|pts| {
                    pad.sticky_event::<gst::event::Segment>(0)
                        .and_then(|event| {
                            event
                                .segment()
                                .downcast_ref::<gst::ClockTime>()
                                .and_then(|segment| segment.to_running_time(pts))
                        })
                });
                // Let the audio sink fill and pace its bounded ring buffer. Waiting
                // here leaves too little queued PCM to survive scheduling stalls.
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
        Ok(Self {
            element,
            envelope,
            enabled,
        })
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
        if let Some(parent) = self.element.parent().and_then(|p| {
            p.downcast::<gst::Bin>()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("playback.fade.parent_bin", error);
                })
                .ok()
        }) {
            parent.remove(&self.element).map_err(|e| e.to_string())?;
        }
        // Keep the desktop timeline on a monotonic clock. AudioBaseSink slaves
        // its device clock to it and owns output buffering and pacing.
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
        #[cfg(all(feature = "performance-probe", not(test)))]
        if crate::performance::root().is_some() {
            output
                .static_pad("src")
                .ok_or("音频输出出口不可用")?
                .add_probe(gst::PadProbeType::BUFFER, |_, _| {
                    crate::performance::output_buffer();
                    gst::PadProbeReturn::Ok
                });
        }
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
        self.enabled.store(enabled, Ordering::Release);
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

pub fn gapless_allowed(enabled: &std::sync::atomic::AtomicBool) -> bool {
    enabled.load(std::sync::atomic::Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_supply_does_not_wait_for_the_control_envelope() {
        gst::init().unwrap();
        for enabled in [false, true] {
            let fade = PlaybackFade::new().unwrap();
            fade.set_enabled(enabled);
            let source = gst::ElementFactory::make("audiotestsrc")
                .property("num-buffers", 8i32)
                .build()
                .unwrap();
            let sink = gst::ElementFactory::make("fakesink")
                .property("sync", false)
                .property("signal-handoffs", true)
                .build()
                .unwrap();
            let (sender, receiver) = std::sync::mpsc::channel();
            sink.connect("handoff", false, move |_| {
                let _ = sender.send(());
                None
            });
            let pipeline = gst::Pipeline::new();
            pipeline.add_many([&source, &fade.element, &sink]).unwrap();
            gst::Element::link_many([&source, &fade.element, &sink]).unwrap();
            // Simulate a control operation descheduled while it owns the envelope.
            // Audio must keep the last committed gain and continue supplying PCM.
            let envelope = fade.envelope.lock().unwrap();
            pipeline.set_state(gst::State::Playing).unwrap();
            let supplied = receiver.recv_timeout(std::time::Duration::from_millis(200));
            drop(envelope);
            pipeline.set_state(gst::State::Null).unwrap();
            assert!(
                supplied.is_ok(),
                "control lock blocked audio; enabled={enabled}"
            );
        }
    }

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
