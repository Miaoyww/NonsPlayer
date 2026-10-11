//! Gain envelopes run in GStreamer's streaming thread, independent of UI/tick rate.
use crate::model::AppResult;
use gstreamer::{self as gst, prelude::*};
use gstreamer_controller::{
    prelude::*, DirectControlBinding, InterpolationControlSource, InterpolationMode,
};
use std::sync::{Arc, Mutex};

pub const FADE_MS: u64 = 800;

#[derive(Clone, Copy, Default)]
struct Plan {
    enabled: bool,
    fade_in: bool,
    fade_out: bool,
    duration_ms: u64,
}

#[derive(Default)]
struct Plans {
    current: Plan,
    next_duration: Option<u64>,
    first_segment: bool,
}

#[derive(Clone)]
pub struct FadeFilter {
    element: gst::Element,
    source: InterpolationControlSource,
    plan: Arc<Mutex<Plans>>,
}

impl FadeFilter {
    pub fn new() -> AppResult<Self> {
        let element = gst::ElementFactory::make("audioamplify")
            .name("nons-fade")
            .build()
            .map_err(|e| e.to_string())?;
        let source = InterpolationControlSource::new();
        source.set_mode(InterpolationMode::Linear);
        // Absolute binding keeps the gain in 0..1 regardless of the property range.
        // audioamplify does not implement StreamVolume, so playbin cannot mistake
        // the fade gain for the user volume when scanning our output bin.
        element
            .add_control_binding(&DirectControlBinding::new_absolute(
                &element,
                "amplification",
                &source,
            ))
            .map_err(|e| e.to_string())?;
        let this = Self {
            element,
            source,
            plan: Arc::new(Mutex::new(Plans::default())),
        };
        this.reset();
        let element = this.element.downgrade();
        let source = this.source.clone();
        let plan = this.plan.clone();
        this.element
            .static_pad("sink")
            .ok_or("渐变音量入口不可用")?
            .add_probe(
                gst::PadProbeType::EVENT_DOWNSTREAM | gst::PadProbeType::BUFFER,
                move |pad, info| {
                    if let Some(gst::PadProbeData::Buffer(buffer)) = &info.data {
                        // audioamplify bypasses its transform at unity gain. Sync here
                        // so it can leave passthrough for a newly scheduled fade and
                        // the natural tail, even after a long stretch at full gain.
                        let enabled = plan
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .current
                            .enabled;
                        if enabled {
                            if let Some(time) = buffer.pts().and_then(|pts| {
                                pad.sticky_event::<gst::event::Segment>(0)
                                    .and_then(|event| {
                                        event
                                            .segment()
                                            .downcast_ref::<gst::ClockTime>()
                                            .and_then(|segment| segment.to_stream_time(pts))
                                    })
                            }) {
                                if let Some(element) = element.upgrade() {
                                    let _ = element.sync_values(time);
                                }
                            }
                        }
                    }
                    if let Some(gst::PadProbeData::Event(event)) = &info.data {
                        if let gst::EventView::Segment(segment) = event.view() {
                            if let Some(segment) =
                                segment.segment().downcast_ref::<gst::ClockTime>()
                            {
                                let Some(element) = element.upgrade() else {
                                    return gst::PadProbeReturn::Ok;
                                };
                                let filter = Self {
                                    element,
                                    source: source.clone(),
                                    plan: plan.clone(),
                                };
                                let position =
                                    segment.time().unwrap_or(gst::ClockTime::ZERO).mseconds();
                                let plan = {
                                    let mut plans =
                                        filter.plan.lock().unwrap_or_else(|e| e.into_inner());
                                    if plans.first_segment {
                                        plans.first_segment = false;
                                    } else if let Some(duration) = plans.next_duration.take() {
                                        plans.current.duration_ms = duration;
                                    }
                                    plans.current
                                };
                                filter.envelope(
                                    position,
                                    if plan.enabled && plan.fade_in {
                                        0.0
                                    } else {
                                        1.0
                                    },
                                    plan,
                                );
                            }
                        }
                    }
                    gst::PadProbeReturn::Ok
                },
            );
        Ok(this)
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

    /// Call while playbin is NULL. The output bin sits after playsink's audio
    /// queue; audio-filter runs before that queue and cannot fade queued PCM.
    pub fn set_output(&self, playbin: &gst::Element, sink: gst::Element) -> AppResult<()> {
        let input = gst::ElementFactory::make("audioconvert")
            .build()
            .map_err(|e| e.to_string())?;
        let output = gst::ElementFactory::make("audioconvert")
            .build()
            .map_err(|e| e.to_string())?;
        let bin = gst::Bin::new();
        if let Some(parent) = self
            .element
            .parent()
            .and_then(|p| p.downcast::<gst::Bin>().ok())
        {
            parent.remove(&self.element).map_err(|e| e.to_string())?;
        }
        bin.add_many([&input, &self.element, &output, &sink])
            .map_err(|e| e.to_string())?;
        gst::Element::link_many([&input, &self.element, &output, &sink])
            .map_err(|e| e.to_string())?;
        let pad = input.static_pad("sink").ok_or("音频输出入口不可用")?;
        let ghost = gst::GhostPad::builder_with_target(&pad)
            .map_err(|e| e.to_string())?
            .name("sink")
            .build();
        bin.add_pad(&ghost).map_err(|e| e.to_string())?;
        playbin.set_property("audio-sink", &bin);
        Ok(())
    }

    pub fn prepare(&self, enabled: bool, fade_in: bool, fade_out: bool, duration_ms: u64) {
        *self.plan.lock().unwrap_or_else(|e| e.into_inner()) = Plans {
            current: Plan {
                enabled,
                fade_in,
                fade_out,
                duration_ms,
            },
            next_duration: None,
            first_segment: true,
        };
    }

    // Keep the outgoing envelope intact until the next audio segment arrives.
    pub fn prepare_next(&self, duration_ms: u64) {
        self.plan
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .next_duration = Some(duration_ms);
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.plan
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .current
            .enabled = enabled;
    }

    pub fn reset(&self) {
        self.source.unset_all();
        self.source.set(gst::ClockTime::ZERO, 1.0);
        self.element.set_property("amplification", 1.0f32);
    }

    pub fn update_duration(&self, position_ms: u64, duration_ms: u64) {
        let changed = {
            let mut plans = self.plan.lock().unwrap_or_else(|e| e.into_inner());
            // playbin can report the next URI's duration before the audio seam.
            if plans.next_duration.is_some() {
                return;
            }
            let plan = &mut plans.current;
            if plan.duration_ms == duration_ms {
                false
            } else {
                plan.duration_ms = duration_ms;
                plan.enabled && plan.fade_out
            }
        };
        if changed {
            self.resume(position_ms);
        }
    }

    pub fn resume(&self, position_ms: u64) {
        let plan = self.plan.lock().unwrap_or_else(|e| e.into_inner()).current;
        let gain = gst::prelude::ControlSourceExt::value(
            &self.source,
            gst::ClockTime::from_mseconds(position_ms),
        )
        .unwrap_or(1.0);
        self.envelope(position_ms, gain, plan);
    }

    pub fn fade_out(&self, position_ms: u64) -> u64 {
        let gain = gst::prelude::ControlSourceExt::value(
            &self.source,
            gst::ClockTime::from_mseconds(position_ms),
        )
        .unwrap_or(1.0);
        let end = position_ms.saturating_add(FADE_MS);
        self.source.unset_all();
        self.source.set(gst::ClockTime::ZERO, gain);
        self.source
            .set(gst::ClockTime::from_mseconds(position_ms), gain);
        self.source.set(gst::ClockTime::from_mseconds(end), 0.0);
        end
    }

    fn envelope(&self, position: u64, gain: f64, plan: Plan) {
        if !plan.enabled {
            self.reset();
            return;
        }
        let remaining = plan.duration_ms.saturating_sub(position);
        let natural_end = plan.fade_out && remaining > 0;
        let width = if natural_end {
            FADE_MS.min(remaining / 2)
        } else {
            FADE_MS
        };
        self.source.unset_all();
        self.source.set(gst::ClockTime::ZERO, gain);
        self.source
            .set(gst::ClockTime::from_mseconds(position), gain);
        self.source.set(
            gst::ClockTime::from_mseconds(position.saturating_add(width)),
            1.0,
        );
        if natural_end {
            self.source
                .set(gst::ClockTime::from_mseconds(plan.duration_ms - width), 1.0);
            self.source
                .set(gst::ClockTime::from_mseconds(plan.duration_ms), 0.0);
        }
        self.element.set_property("amplification", gain as f32);
    }
}

/// The same gate used by the streaming callback; disabling takes effect while
/// holding its prepared lock, so an old preload cannot bypass the setting.
pub fn gapless_allowed(enabled: &std::sync::atomic::AtomicBool) -> bool {
    enabled.load(std::sync::atomic::Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_pcm_has_faded_edges_and_disabled_filter_keeps_original_gain() {
        gst::init().unwrap();
        for enabled in [false, true] {
            let fade = FadeFilter::new().unwrap();
            fade.prepare(enabled, true, true, 3000);
            let pipeline = gst::Pipeline::new();
            let source = gst::ElementFactory::make("audiotestsrc")
                .property_from_str("wave", "square")
                .property("freq", 1000.0f64)
                .property("volume", 0.5f64)
                .property("samplesperbuffer", 480i32)
                .property("num-buffers", 300i32)
                .build()
                .unwrap();
            let caps = gst::ElementFactory::make("capsfilter")
                .property(
                    "caps",
                    gst::Caps::builder("audio/x-raw")
                        .field("format", "F32LE")
                        .field("rate", 48000i32)
                        .field("channels", 1i32)
                        .build(),
                )
                .build()
                .unwrap();
            let sink = gst::ElementFactory::make("fakesink")
                .property("sync", false)
                .property("signal-handoffs", true)
                .build()
                .unwrap();
            let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
            let capture = samples.clone();
            sink.connect("handoff", false, move |values| {
                let buffer = values[1].get::<gst::Buffer>().unwrap();
                let data = buffer.map_readable().unwrap();
                capture.lock().unwrap().extend(
                    data.as_slice()
                        .chunks_exact(4)
                        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap())),
                );
                None
            });
            pipeline
                .add_many([&source, &caps, &fade.element, &sink])
                .unwrap();
            gst::Element::link_many([&source, &caps, &fade.element, &sink]).unwrap();
            pipeline.set_state(gst::State::Playing).unwrap();
            let message = pipeline
                .bus()
                .unwrap()
                .timed_pop_filtered(
                    gst::ClockTime::from_seconds(5),
                    &[gst::MessageType::Eos, gst::MessageType::Error],
                )
                .expect("PCM capture timeout");
            pipeline.set_state(gst::State::Null).unwrap();
            assert!(
                matches!(message.view(), gst::MessageView::Eos(_)),
                "{message:?}"
            );
            let samples = samples.lock().unwrap();
            assert_eq!(samples.len(), 144000);
            if enabled {
                assert!(samples[0].abs() < 0.001);
                assert!((samples[19200].abs() - 0.25).abs() < 0.025);
                assert!((samples[72000].abs() - 0.5).abs() < 0.001);
                assert!((samples[124800].abs() - 0.25).abs() < 0.025);
                assert!(samples[143999].abs() < 0.025);
            } else {
                assert!(samples
                    .iter()
                    .all(|value| (value.abs() - 0.5).abs() < 0.001));
            }
        }
    }

    #[test]
    fn envelopes_are_bounded_interruptible_and_do_not_scale_user_volume() {
        gst::init().unwrap();
        let fade = FadeFilter::new().unwrap();
        let plan = Plan {
            enabled: true,
            fade_in: true,
            fade_out: true,
            duration_ms: 4000,
        };
        fade.envelope(0, 0.0, plan);
        let value = |ms| {
            gst::prelude::ControlSourceExt::value(&fade.source, gst::ClockTime::from_mseconds(ms))
                .unwrap()
        };
        assert_eq!(value(0), 0.0);
        assert!((value(400) - 0.5).abs() < 1e-6);
        assert_eq!(value(1000), 1.0);
        assert!((value(3600) - 0.5).abs() < 1e-6);
        assert_eq!(value(4000), 0.0);
        fade.prepare(true, true, true, 4000);
        fade.fade_out(1000);
        assert!((value(1400) - 0.5).abs() < 1e-6);
        fade.resume(1400);
        assert!((value(1400) - 0.5).abs() < 1e-6);
        assert_eq!(value(2200), 1.0);
        fade.reset();
        assert_eq!(value(2000), 1.0);
    }

    #[test]
    fn short_tracks_and_unknown_duration_do_not_stay_silent() {
        gst::init().unwrap();
        let fade = FadeFilter::new().unwrap();
        for duration_ms in [0, 100, 2000] {
            fade.envelope(
                0,
                0.0,
                Plan {
                    enabled: true,
                    fade_in: true,
                    fade_out: true,
                    duration_ms,
                },
            );
            let peak = if duration_ms == 0 {
                FADE_MS
            } else {
                FADE_MS.min(duration_ms / 2)
            };
            assert_eq!(
                gst::prelude::ControlSourceExt::value(
                    &fade.source,
                    gst::ClockTime::from_mseconds(peak)
                ),
                Some(1.0)
            );
        }
        fade.envelope(
            0,
            1.0,
            Plan {
                enabled: true,
                fade_in: false,
                fade_out: false,
                duration_ms: 2000,
            },
        );
        assert_eq!(
            gst::prelude::ControlSourceExt::value(
                &fade.source,
                gst::ClockTime::from_mseconds(2000)
            ),
            Some(1.0)
        );
    }
}
