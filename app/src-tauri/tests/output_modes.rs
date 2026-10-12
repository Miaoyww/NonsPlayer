//! Explicit Windows modes through the same playbin output wrapper; only renders silence.
#![cfg(windows)]
use gstreamer::{self as gst, prelude::*};
#[path = "../src/playback/audio.rs"]
#[allow(dead_code)]
mod audio;
#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;
#[path = "../src/playback/output.rs"]
#[allow(dead_code)]
mod output;
#[path = "../src/playback/transitions.rs"]
#[allow(dead_code)]
mod transitions;

#[test]
#[cfg(windows)]
#[ignore = "requires a Windows output device with exclusive access; renders silence"]
fn shared_exclusive_shared_rebuild_keeps_mode_volume_and_seek() {
    gst::init().unwrap();
    let file = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    let bytes = 48000u32 * 2 * 2 * 4;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((bytes + 36).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(48000u32.to_le_bytes());
    wav.extend(192000u32.to_le_bytes());
    wav.extend(4u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    wav.resize(44 + bytes as usize, 0);
    std::fs::write(file.path(), wav).unwrap();
    let uri = url::Url::from_file_path(file.path()).unwrap().to_string();
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
    let fade = transitions::PlaybackFade::new().unwrap();
    for mode in [
        model::AudioOutputMode::Shared,
        model::AudioOutputMode::Exclusive,
        model::AudioOutputMode::Shared,
    ] {
        let sink = output::create_sink(None, mode).unwrap();
        fade.set_output(&playbin, sink.clone()).unwrap();
        audio::start_stream(&playbin, &uri, 0.45, gst::State::Paused).unwrap();
        let (result, state, _) = playbin.state(gst::ClockTime::from_seconds(5));
        if result.is_err() {
            let error = playbin
                .bus()
                .unwrap()
                .timed_pop_filtered(gst::ClockTime::ZERO, &[gst::MessageType::Error]);
            let _ = playbin.set_state(gst::State::Null);
            panic!("{mode:?} failed: {error:?}");
        }
        assert_eq!(state, gst::State::Paused);
        assert_eq!(
            sink.property::<bool>("exclusive"),
            mode == model::AudioOutputMode::Exclusive
        );
        assert!((playbin.property::<f64>("volume") - 0.45).abs() < 0.001);
        audio::seek_stream(&playbin, 1000).unwrap();
        playbin.state(gst::ClockTime::from_seconds(5)).0.unwrap();
        assert_eq!(
            playbin
                .query_position::<gst::ClockTime>()
                .unwrap()
                .mseconds(),
            1000
        );
        playbin.set_state(gst::State::Playing).unwrap();
        playbin.state(gst::ClockTime::from_seconds(5)).0.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(120));
        assert!(
            playbin
                .query_position::<gst::ClockTime>()
                .unwrap()
                .mseconds()
                > 1000
        );
        if mode == model::AudioOutputMode::Exclusive {
            // The endpoint must really be exclusive, rather than merely reporting a preference.
            let competitor = gst::parse::launch(
                "audiotestsrc wave=silence ! audioconvert ! audioresample ! wasapi2sink",
            )
            .unwrap();
            let opened = competitor.set_state(gst::State::Paused).is_ok()
                && competitor.state(gst::ClockTime::from_seconds(5)).0.is_ok();
            competitor.set_state(gst::State::Null).unwrap();
            assert!(
                !opened,
                "another shared stream must be rejected while exclusive output is open"
            );
            let competing_exclusive = gst::parse::launch("audiotestsrc wave=silence ! audioconvert ! audioresample ! wasapisink exclusive=true").unwrap();
            let opened = competing_exclusive.set_state(gst::State::Paused).is_ok()
                && competing_exclusive
                    .state(gst::ClockTime::from_seconds(5))
                    .0
                    .is_ok();
            competing_exclusive.set_state(gst::State::Null).unwrap();
            assert!(
                !opened,
                "an occupied exclusive request must fail instead of falling back to shared"
            );
            let bin = playbin.downcast_ref::<gst::Bin>().unwrap();
            let mut elements = bin.iterate_recurse();
            let mut software_volume = false;
            while let Ok(Some(element)) = elements.next() {
                if element.factory().is_some_and(|f| f.name() == "volume") {
                    software_volume = true;
                    assert!((element.property::<f64>("volume") - 0.45).abs() < 0.001);
                }
            }
            assert!(
                software_volume,
                "exclusive output must retain application volume control"
            );
        }
        playbin.set_state(gst::State::Null).unwrap();
    }
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Sink"), None);
    monitor.start().unwrap();
    let device = monitor
        .devices()
        .into_iter()
        .next()
        .expect("output device required");
    let shared = output::create_sink(Some(&device), model::AudioOutputMode::Shared).unwrap();
    let exclusive = output::create_sink(Some(&device), model::AudioOutputMode::Exclusive).unwrap();
    assert_eq!(
        shared.property::<Option<String>>("device"),
        exclusive.property::<Option<String>>("device")
    );
    assert!(exclusive.property::<bool>("exclusive"));
    monitor.stop();
}
