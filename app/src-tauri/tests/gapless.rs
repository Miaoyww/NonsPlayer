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
    capture_transition(true, false);
    capture_transition(false, false);
}

#[test]
fn gapless_transition_fades_both_tracks_without_adding_or_dropping_samples() {
    capture_transition(true, true);
}

fn capture_transition(enabled: bool, fades: bool) {
    gst::init().unwrap();
    let root = std::env::temp_dir().join(format!("nons-gapless-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let (samples, split): (Vec<i16>, usize) = if fades {
        (vec![10000; 96000 * 7], 96000 * 3)
    } else {
        (
            (0..96000 * 2)
                .map(|i| ((i * 37) % 20001) as i16 - 10000)
                .collect(),
            96000,
        )
    };
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
    fade.prepare(fades, true, true, if fades { 3000 } else { 1000 });
    let gapless = Arc::new(AtomicBool::new(enabled));
    let next = Arc::new(Mutex::new(Some(
        url::Url::from_file_path(&files[1]).unwrap().to_string(),
    )));
    playbin.connect("about-to-finish", false, move |values| {
        if !transitions::gapless_allowed(&gapless) {
            return None;
        }
        if let Some(uri) = next.lock().unwrap().take() {
            fade.prepare_next(if fades { 4000 } else { 1000 });
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
    if fades {
        let actual = actual.lock().unwrap();
        let pcm: Vec<i16> = actual
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(
            pcm.len(),
            samples.len(),
            "fades must not change the sample count"
        );
        for (start, duration) in [(0, 3000), (split, 4000)] {
            let at = |ms: usize| pcm[start + ms * 96];
            assert!(at(0).abs() < 500, "track start must fade in: {}", at(0));
            assert!(
                (at(400) - 5000).abs() < 700,
                "800 ms fade-in midpoint: {}",
                at(400)
            );
            assert!((at(1500) - 10000).abs() < 10, "middle must keep unity gain");
            assert!(
                (at(duration - 400) - 5000).abs() < 700,
                "800 ms fade-out midpoint: {}",
                at(duration - 400)
            );
            assert!(
                at(duration - 10).abs() < 700,
                "track end must fade out: {}",
                at(duration - 10)
            );
        }
        return;
    }
    assert_eq!(
        *actual.lock().unwrap(),
        expected,
        "decoded PCM must contain neither added silence nor repeated/dropped samples"
    );
}
