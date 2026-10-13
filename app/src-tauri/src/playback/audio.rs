use crate::model::PlaybackStatus;
use gstreamer::{self as gst, prelude::*};

/// OpenSL ES must receive preroll before playback starts. Buffering completion
/// and AsyncDone can arrive in either order, and neither alone is sufficient.
#[derive(Default)]
pub struct StreamStartGate {
    pending: bool,
    buffering: bool,
}

impl StreamStartGate {
    pub fn prepare(&mut self, preroll: bool) {
        self.pending = preroll;
        self.buffering = false;
    }

    pub fn complete(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }

    pub fn buffering(&mut self, percent: i32) {
        self.buffering = percent < 100;
    }

    pub fn ready(&self) -> bool {
        !self.pending && !self.buffering
    }
}

pub fn update_buffering(
    playbin: &gst::Element,
    percent: i32,
) -> Result<PlaybackStatus, gst::StateChangeError> {
    let buffering = percent < 100;
    playbin.set_state(if buffering {
        gst::State::Paused
    } else {
        gst::State::Playing
    })?;
    Ok(if buffering {
        PlaybackStatus::Buffering
    } else if playbin.current_state() == gst::State::Playing {
        // A late/repeated 100% message may arrive after StateChanged(Playing).
        // Reapplying the same target does not emit another state transition.
        PlaybackStatus::Playing
    } else {
        PlaybackStatus::Loading
    })
}

pub fn start_stream(
    playbin: &gst::Element,
    uri: &str,
    volume: f64,
    target: gst::State,
) -> Result<(), gst::StateChangeError> {
    playbin.set_property("uri", uri);
    // autoaudiosink can create a new hardware volume control after NULL.
    // Reapply the global volume before any buffers reach the new output.
    playbin.set_property("volume", volume.clamp(0.0, 1.0));
    playbin.set_state(target)?;
    Ok(())
}

pub fn seek_stream(playbin: &gst::Element, position_ms: u64) -> Result<(), gst::glib::BoolError> {
    playbin.seek_simple(
        gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
        gst::ClockTime::from_mseconds(position_ms),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_start_waits_for_preroll_and_buffering_in_either_order() {
        for preroll_first in [true, false] {
            let mut gate = StreamStartGate::default();
            gate.prepare(true);
            gate.buffering(0);
            assert!(!gate.ready());
            if preroll_first {
                assert!(gate.complete());
            } else {
                gate.buffering(100);
            }
            assert!(!gate.ready(), "one completed stage cannot start the sink");
            if preroll_first {
                gate.buffering(100);
            } else {
                assert!(gate.complete());
            }
            assert!(gate.ready());
            gate.buffering(10);
            assert!(!gate.ready(), "resume must respect renewed buffering");
            gate.prepare(true);
            gate.buffering(100);
            assert!(!gate.ready(), "switching tracks requires fresh preroll");
        }
    }

    #[test]
    fn restored_position_is_applied_before_the_first_playing_buffer() {
        gst::init().unwrap();
        let pipeline = gst::parse::launch(
            "audiotestsrc num-buffers=240 ! fakesink name=output sync=true signal-handoffs=true",
        )
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();
        let sink = pipeline.by_name("output").unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        sink.connect("handoff", false, move |values| {
            let buffer = values[1].get::<gst::Buffer>().unwrap();
            let _ = sender.send(buffer.pts().unwrap().mseconds());
            None
        });
        pipeline.set_state(gst::State::Paused).unwrap();
        assert_eq!(
            pipeline.state(gst::ClockTime::from_seconds(2)).1,
            gst::State::Paused
        );
        assert!(
            receiver.try_recv().is_err(),
            "preroll must not output audio"
        );
        seek_stream(pipeline.upcast_ref(), 2_000).unwrap();
        pipeline.set_state(gst::State::Playing).unwrap();
        let first = receiver.recv_timeout(std::time::Duration::from_secs(2));
        pipeline.set_state(gst::State::Null).unwrap();
        assert_eq!(first.unwrap(), 2_000);
    }

    #[test]
    fn late_completed_buffering_keeps_actual_playing_status() {
        gst::init().unwrap();
        let pipeline =
            gst::parse::launch("audiotestsrc is-live=true ! fakesink sync=true").unwrap();
        pipeline.set_state(gst::State::Playing).unwrap();
        assert_eq!(
            pipeline.state(gst::ClockTime::from_seconds(2)).1,
            gst::State::Playing
        );
        // Once already Playing, setting Playing again produces no new transition.
        let first = update_buffering(&pipeline, 100).unwrap();
        let repeated = update_buffering(&pipeline, 100).unwrap();
        pipeline.set_state(gst::State::Null).unwrap();
        assert_eq!(first, PlaybackStatus::Playing);
        assert_eq!(repeated, PlaybackStatus::Playing);
    }

    #[test]
    fn incomplete_buffering_pauses_without_reporting_playing() {
        gst::init().unwrap();
        let pipeline =
            gst::parse::launch("audiotestsrc is-live=true ! fakesink sync=true").unwrap();
        pipeline.set_state(gst::State::Playing).unwrap();
        assert_eq!(
            pipeline.state(gst::ClockTime::from_seconds(2)).1,
            gst::State::Playing
        );
        let status = update_buffering(&pipeline, 25).unwrap();
        let actual = pipeline.state(gst::ClockTime::from_seconds(2)).1;
        pipeline.set_state(gst::State::Null).unwrap();
        assert_eq!(status, PlaybackStatus::Buffering);
        assert_eq!(actual, gst::State::Paused);
    }
}
