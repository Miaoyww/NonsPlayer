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
#[ignore = "requires XIBERIA S21 and Realtek output devices; renders silence"]
fn exclusive_switch_between_usb_surround_and_realtek_stereo() {
    gst::init().unwrap();
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Sink"), None);
    monitor.start().unwrap();
    let devices = monitor.devices();
    let usb = devices
        .iter()
        .find(|d| d.display_name().contains("XIBERIA S21"))
        .unwrap();
    let realtek = devices
        .iter()
        .find(|d| d.display_name().contains("Realtek"))
        .unwrap();
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
    let fade = transitions::PlaybackFade::new().unwrap();
    let file = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    // 44.1 kHz stereo exercises conversion to the USB device's 48 kHz / 8 channels.
    let bytes = 44100u32 * 4 * 2;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((bytes + 36).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(44100u32.to_le_bytes());
    wav.extend(176400u32.to_le_bytes());
    wav.extend(4u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    wav.resize(44 + bytes as usize, 0);
    std::fs::write(file.path(), wav).unwrap();
    let uri = url::Url::from_file_path(file.path()).unwrap().to_string();
    for device in [usb, realtek].into_iter().cycle().take(20) {
        eprintln!("switch to {}", device.display_name());
        let sink = output::create_sink(Some(device), model::AudioOutputMode::Exclusive).unwrap();
        fade.set_output(&playbin, sink).unwrap();
        audio::start_stream(&playbin, &uri, 0.0, gst::State::Paused).unwrap();
        let result = playbin.state(gst::ClockTime::from_seconds(5)).0;
        if result.is_ok() {
            audio::seek_stream(&playbin, 250).unwrap();
            playbin.state(gst::ClockTime::from_seconds(5)).0.unwrap();
            playbin.set_state(gst::State::Playing).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        playbin.set_state(gst::State::Null).unwrap();
        assert!(
            result.is_ok(),
            "exclusive preroll failed for {}",
            device.display_name()
        );
    }
    monitor.stop();
}

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
            let competing_exclusive = gst::parse::launch("audiotestsrc wave=silence ! audioconvert ! audioresample ! wasapi2sink exclusive=true").unwrap();
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
    let actual_endpoint = device
        .properties()
        .and_then(|p| p.get::<String>("device.actual-id").ok())
        .or_else(|| shared.property::<Option<String>>("device"));
    assert_eq!(
        actual_endpoint,
        exclusive.property::<Option<String>>("device")
    );
    assert!(exclusive.property::<bool>("exclusive"));
    monitor.stop();
}

#[test]
#[ignore = "requires Windows audio output; simulates WASAPI2 shared fallback with silence"]
fn shared_fallback_is_rejected_before_playing() {
    gst::init().unwrap();
    let pipeline = gst::Pipeline::new();
    let source = gst::ElementFactory::make("audiotestsrc")
        .property_from_str("wave", "silence")
        .build()
        .unwrap();
    let convert = gst::ElementFactory::make("audioconvert").build().unwrap();
    let resample = gst::ElementFactory::make("audioresample").build().unwrap();
    let sink = output::create_sink(None, model::AudioOutputMode::Exclusive).unwrap();
    // Reproduce the result of WASAPI2's fallback, independently of whether this
    // driver preempts existing shared streams or denies exclusive initialization.
    sink.downcast_ref::<gst::Bin>()
        .unwrap()
        .by_name("nons-exclusive-device")
        .unwrap()
        .set_property("exclusive", false);
    pipeline
        .add_many([&source, &convert, &resample, &sink])
        .unwrap();
    gst::Element::link_many([&source, &convert, &resample, &sink]).unwrap();
    let opened = pipeline.set_state(gst::State::Playing).is_ok()
        && pipeline.state(gst::ClockTime::from_seconds(5)).0.is_ok();
    pipeline.set_state(gst::State::Null).unwrap();
    assert!(
        !opened,
        "exclusive request must not render into a shared session"
    );
}

#[test]
#[ignore = "requires XIBERIA S21 and Realtek; captures and silences test-tone PCM"]
fn exclusive_output_preserves_tone_frequency_across_rates_and_channel_counts() {
    use std::sync::{Arc, Mutex};
    gst::init().unwrap();
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Sink"), None);
    monitor.start().unwrap();
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
    let fade = transitions::PlaybackFade::new().unwrap();
    for device in monitor.devices().iter().filter(|d| {
        d.display_name().contains("XIBERIA S21") || d.display_name().contains("Realtek")
    }) {
        for rate in [44100u32, 48000, 96000] {
            let file = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
            let bytes = rate * 4;
            let mut wav = Vec::new();
            wav.extend(b"RIFF");
            wav.extend((bytes + 36).to_le_bytes());
            wav.extend(b"WAVEfmt ");
            wav.extend(16u32.to_le_bytes());
            wav.extend(1u16.to_le_bytes());
            wav.extend(2u16.to_le_bytes());
            wav.extend(rate.to_le_bytes());
            wav.extend((rate * 4).to_le_bytes());
            wav.extend(4u16.to_le_bytes());
            wav.extend(16u16.to_le_bytes());
            wav.extend(b"data");
            wav.extend(bytes.to_le_bytes());
            for frame in 0..rate {
                let sample = (3276.0
                    * (frame as f64 * 440.0 * std::f64::consts::TAU / rate as f64).sin())
                    as i16;
                wav.extend(sample.to_le_bytes());
                wav.extend(sample.to_le_bytes());
            }
            std::fs::write(file.path(), wav).unwrap();
            let sink =
                output::create_sink(Some(device), model::AudioOutputMode::Exclusive).unwrap();
            if device.display_name().contains("Realtek") {
                // Exercise 44.1/96 -> 48 kHz and the 24-in-32 representation that
                // the legacy sink incorrectly copied straight into Windows.
                let caps: gst::Caps = "audio/x-raw,format=S24_32LE,layout=interleaved,rate=48000,channels=2,channel-mask=(bitmask)0x3".parse().unwrap();
                let filter = sink
                    .downcast_ref::<gst::Bin>()
                    .unwrap()
                    .by_name("nons-exclusive-format")
                    .unwrap();
                assert!(caps.is_subset(&filter.property::<gst::Caps>("caps")));
                filter.set_property("caps", caps);
            }
            let hardware = sink
                .downcast_ref::<gst::Bin>()
                .unwrap()
                .by_name("nons-exclusive-device")
                .unwrap();
            let actual = Arc::new(Mutex::new((0i32, Vec::<f64>::new())));
            let capture = actual.clone();
            hardware.static_pad("sink").unwrap().add_probe(
                gst::PadProbeType::BUFFER,
                move |pad, info| {
                    let caps = pad.current_caps().unwrap();
                    let structure = caps.structure(0).unwrap();
                    let format = structure.get::<String>("format").unwrap();
                    let rate = structure.get::<i32>("rate").unwrap();
                    let channels = structure.get::<i32>("channels").unwrap() as usize;
                    let sample_bytes = match format.as_str() {
                        "S16LE" => 2,
                        "S24LE" => 3,
                        "S24_32LE" | "S32LE" | "F32LE" => 4,
                        _ => panic!("unexpected device PCM: {caps}"),
                    };
                    if let Some(gst::PadProbeData::Buffer(buffer)) = &mut info.data {
                        let mut map = buffer.make_mut().map_writable().unwrap();
                        let mut actual = capture.lock().unwrap();
                        actual.0 = rate;
                        for frame in map.as_slice().chunks_exact(sample_bytes * channels) {
                            let value = match format.as_str() {
                                "S16LE" => {
                                    i16::from_le_bytes(frame[..2].try_into().unwrap()) as f64
                                        / 32768.0
                                }
                                "S24LE" => {
                                    ((i32::from_le_bytes([0, frame[0], frame[1], frame[2]])) >> 8)
                                        as f64
                                        / 8388608.0
                                }
                                "S24_32LE" => {
                                    i32::from_le_bytes(frame[..4].try_into().unwrap()) as f64
                                        / 8388608.0
                                }
                                "S32LE" => {
                                    i32::from_le_bytes(frame[..4].try_into().unwrap()) as f64
                                        / 2147483648.0
                                }
                                "F32LE" => {
                                    f32::from_le_bytes(frame[..4].try_into().unwrap()) as f64
                                }
                                _ => unreachable!(),
                            };
                            actual.1.push(value);
                        }
                        // Measure at the actual device input, then silence the tone.
                        map.as_mut_slice().fill(0);
                    }
                    gst::PadProbeReturn::Ok
                },
            );
            fade.set_output(&playbin, sink).unwrap();
            let uri = url::Url::from_file_path(file.path()).unwrap().to_string();
            audio::start_stream(&playbin, &uri, 0.45, gst::State::Playing).unwrap();
            let message = playbin.bus().unwrap().timed_pop_filtered(
                gst::ClockTime::from_seconds(5),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            );
            playbin.set_state(gst::State::Null).unwrap();
            assert!(
                message.is_some_and(|m| m.type_() == gst::MessageType::Eos),
                "{} / {rate}: playback did not complete",
                device.display_name()
            );
            let actual = actual.lock().unwrap();
            let crossings: Vec<usize> = actual
                .1
                .windows(2)
                .enumerate()
                .filter_map(|(i, pair)| (pair[0] <= 0.0 && pair[1] > 0.0).then_some(i))
                .collect();
            assert!(crossings.len() > 400, "tone lost or distorted");
            let frequency = (crossings.len() - 1) as f64 * actual.0 as f64
                / (crossings.last().unwrap() - crossings[0]) as f64;
            let peak = actual.1.iter().map(|s| s.abs()).fold(0.0f64, f64::max);
            eprintln!(
                "{}: {rate} -> {} Hz, tone={frequency:.3} Hz, peak={peak:.4}",
                device.display_name(),
                actual.0
            );
            assert!((frequency - 440.0).abs() < 1.0, "resampling changed pitch");
            assert!(
                (0.01..0.06).contains(&peak),
                "software volume lost or PCM clipped: {peak}"
            );
        }
    }
    monitor.stop();
}
