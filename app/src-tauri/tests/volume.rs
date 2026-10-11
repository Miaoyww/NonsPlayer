//! Measure decoded output amplitude across the same NULL/reload used by track changes.
use gstreamer::{self as gst, prelude::*};
use std::sync::{Arc, Mutex};
#[path = "../src/playback/audio.rs"]
#[allow(dead_code)]
mod audio;
#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;
#[path = "../src/playback/transitions.rs"]
#[allow(dead_code)]
mod transitions;

#[test]
#[cfg(windows)]
#[ignore = "requires a Windows audio output device; plays silence"]
fn windows_output_reload_keeps_selected_volume() {
    gst::init().unwrap();
    let path = wav_fixture("windows", 0);
    let bin = gst::ElementFactory::make("playbin3")
        .build()
        .unwrap()
        .downcast::<gst::Bin>()
        .unwrap();
    bin.set_property_from_str("flags", "audio+soft-volume+buffering");
    let fade = transitions::PlaybackFade::new().unwrap();
    fade.attach(bin.upcast_ref()).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().to_string();
    let mut actual = Vec::new();
    for track in 0..2 {
        audio::start_stream(bin.upcast_ref(), &uri, 0.45, gst::State::Paused).unwrap();
        let (result, _, _) = bin.state(gst::ClockTime::from_seconds(5));
        result.unwrap();
        if track == 0 {
            bin.set_property("volume", 0.45f64);
        }
        let mut elements = bin.iterate_recurse();
        let mut output_volume = None;
        while let Ok(Some(element)) = elements.next() {
            if element.factory().is_some_and(|f| f.name() == "wasapi2sink") {
                output_volume = Some(element.property::<f64>("volume"));
            }
        }
        actual.push(output_volume.expect("wasapi2 output required"));
        bin.set_state(gst::State::Null).unwrap();
    }
    std::fs::remove_file(path).unwrap();
    assert!(
        (actual[1] - 0.45).abs() < 0.001,
        "Windows output volume after reload: {actual:?}"
    );
}

#[test]
fn track_and_output_reload_keep_selected_volume() {
    gst::init().unwrap();
    let path = wav_fixture("software", 10000);

    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
    let peaks = Arc::new(Mutex::new(Vec::<i16>::new()));
    let sink = recording_sink(peaks.clone());
    playbin.set_property("audio-sink", &sink);
    let fade = transitions::PlaybackFade::new().unwrap();
    fade.attach(&playbin).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().to_string();
    let bus = playbin.bus().unwrap();
    let mut actual = Vec::new();
    for track in 0..3 {
        if track == 2 {
            fade.set_output(&playbin, recording_sink(peaks.clone()))
                .unwrap();
        }
        audio::start_stream(&playbin, &uri, 0.45, gst::State::Playing).unwrap();
        loop {
            let message = bus
                .timed_pop(gst::ClockTime::from_seconds(5))
                .expect("decode timeout");
            match message.view() {
                gst::MessageView::Eos(_) => break,
                gst::MessageView::Error(e) => panic!("{} {:?}", e.error(), e.debug()),
                _ => {}
            }
        }
        actual.push(*peaks.lock().unwrap().iter().max().unwrap());
        peaks.lock().unwrap().clear();
        playbin.set_state(gst::State::Null).unwrap();
    }
    std::fs::remove_file(path).unwrap();
    assert!(
        (actual[0] - 4500).abs() <= 5,
        "first track at 45%: {actual:?}"
    );
    assert!(
        (actual[1] - 4500).abs() <= 5,
        "next track must stay at 45%: {actual:?}"
    );
    assert!(
        (actual[2] - 4500).abs() <= 5,
        "new output must stay at 45%: {actual:?}"
    );
}

fn recording_sink(peaks: Arc<Mutex<Vec<i16>>>) -> gst::Element {
    let sink = gst::ElementFactory::make("fakesink")
        .property("signal-handoffs", true)
        .property("sync", false)
        .build()
        .unwrap();
    let captured = peaks;
    sink.connect("handoff", false, move |values| {
        let buffer = values[1].get::<gst::Buffer>().unwrap();
        let data = buffer.map_readable().unwrap();
        let peak = data
            .as_slice()
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]).abs())
            .max()
            .unwrap_or(0);
        captured.lock().unwrap().push(peak);
        None
    });
    sink
}

fn wav_fixture(name: &str, sample: i16) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("nons-volume-{name}-{}.wav", std::process::id()));
    let size = 4800u32 * 2;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((size + 36).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(48000u32.to_le_bytes());
    wav.extend(96000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(size.to_le_bytes());
    for _ in 0..4800 {
        wav.extend(sample.to_le_bytes());
    }
    std::fs::write(&path, wav).unwrap();

    path
}
