//! Checks decoded PCM, including the seam, rather than listening for a gap.
use gstreamer::{self as gst, prelude::*};
use std::sync::{atomic::AtomicBool, Arc, Mutex};
#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;
#[path = "../src/playback/transitions.rs"]
#[allow(dead_code)]
mod transitions;

fn wav(samples: &[i16]) -> Vec<u8> {
    let bytes = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend((bytes + 36).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(48000u32.to_le_bytes());
    out.extend(192000u32.to_le_bytes());
    out.extend(4u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(bytes.to_le_bytes());
    for sample in samples {
        out.extend(sample.to_le_bytes());
    }
    out
}

#[test]
fn prepared_wav_transition_preserves_every_pcm_sample() {
    capture_transition(true);
    capture_transition(false);
}

fn capture_transition(enabled: bool) {
    gst::init().unwrap();
    let root = std::env::temp_dir().join(format!("nons-gapless-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let samples: Vec<i16> = (0..96000 * 2)
        .map(|i| ((i * 37) % 20001) as i16 - 10000)
        .collect();
    let split = samples.len() / 2;
    let files = [root.join("one.wav"), root.join("two.wav")];
    std::fs::write(&files[0], wav(&samples[..split])).unwrap();
    std::fs::write(&files[1], wav(&samples[split..])).unwrap();
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio");
    let sink = gst::ElementFactory::make("fakesink")
        .property("signal-handoffs", true)
        .property("sync", true)
        .build()
        .unwrap();
    let actual = Arc::new(Mutex::new(Vec::<u8>::new()));
    let capture = actual.clone();
    sink.connect("handoff", false, move |values| {
        let buffer = values[1].get::<gst::Buffer>().unwrap();
        let map = buffer.map_readable().unwrap();
        capture.lock().unwrap().extend_from_slice(map.as_slice());
        None
    });
    playbin.set_property("audio-sink", &sink);
    let fade = transitions::FadeFilter::new().unwrap();
    fade.attach(&playbin);
    let gapless = Arc::new(AtomicBool::new(enabled));
    let next = Arc::new(Mutex::new(Some(
        url::Url::from_file_path(&files[1]).unwrap().to_string(),
    )));
    playbin.connect("about-to-finish", false, move |values| {
        if !transitions::gapless_allowed(&gapless) {
            return None;
        }
        if let Some(uri) = next.lock().unwrap().take() {
            values[0]
                .get::<gst::Element>()
                .unwrap()
                .set_property("uri", uri);
        }
        None
    });
    playbin.set_property(
        "uri",
        url::Url::from_file_path(&files[0]).unwrap().to_string(),
    );
    playbin.set_state(gst::State::Playing).unwrap();
    let bus = playbin.bus().unwrap();
    let result = loop {
        let Some(message) = bus.timed_pop(gst::ClockTime::from_seconds(10)) else {
            break Err("pipeline timeout".to_string());
        };
        match message.view() {
            gst::MessageView::Eos(_) => break Ok(()),
            gst::MessageView::Error(error) => {
                break Err(format!("{} {:?}", error.error(), error.debug()))
            }
            _ => {}
        }
    };
    playbin.set_state(gst::State::Null).unwrap();
    std::fs::remove_file(&files[0]).unwrap();
    std::fs::remove_file(&files[1]).unwrap();
    std::fs::remove_dir(&root).unwrap();
    result.unwrap();
    let expected_samples = if enabled {
        &samples[..]
    } else {
        &samples[..split]
    };
    let expected: Vec<u8> = expected_samples
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    assert_eq!(
        *actual.lock().unwrap(),
        expected,
        "decoded PCM must contain neither added silence nor repeated/dropped samples"
    );
}
