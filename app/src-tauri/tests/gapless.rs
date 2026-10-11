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
    capture_transition(true, false, false);
    capture_transition(false, false, false);
}

#[test]
fn playback_fade_only_changes_start_and_keeps_gapless_seam_and_tail_intact() {
    capture_transition(true, true, false);
}

#[test]
fn transport_pause_envelope_continues_across_a_gapless_segment_boundary() {
    capture_transition(true, true, true);
}

fn capture_transition(enabled: bool, fades: bool, pause_across_seam: bool) {
    gst::init().unwrap();
    let root = std::env::temp_dir().join(format!("nons-gapless-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let (samples, split): (Vec<i16>, usize) = if fades {
        (vec![10000; 96000 * 7], 96000 * 3)
    } else {
        (
            (0..96000 * 2 + 74)
                .map(|i| ((i * 37) % 20001) as i16 - 10000)
                .collect(),
            96000 + 22,
        )
    };
    let files = [root.join("one.wav"), root.join("two.wav")];
    std::fs::write(&files[0], wav(&samples[..split])).unwrap();
    std::fs::write(&files[1], wav(&samples[split..])).unwrap();
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
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
    let fade = transitions::PlaybackFade::new().unwrap();
    fade.attach(&playbin).unwrap();
    fade.set_enabled(fades);
    fade.prepare_start();
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
    if pause_across_seam {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while playbin
            .current_running_time()
            .is_none_or(|time| time.mseconds() < 2850)
        {
            assert!(
                std::time::Instant::now() < deadline,
                "playback clock did not advance"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // Deliberately spans A's end and B's first segment. Media time resets to zero;
        // the transport envelope must finish on the continuous running-time axis.
        fade.fade_out(gst::ClockTime::from_mseconds(2900));
    }
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
        assert!(pcm[0].abs() < 500, "playback starts muted");
        assert!((pcm[100 * 96] - 5000).abs() < 900, "200 ms start midpoint");
        if pause_across_seam {
            assert!(
                (pcm[3000 * 96] - 5000).abs() < 900,
                "fade midpoint must cross the seam"
            );
            assert!(
                pcm[3150 * 96..].iter().all(|sample| sample.abs() < 10),
                "new segment must not reset the paused gain"
            );
        } else {
            assert!(
                pcm[250 * 96..]
                    .iter()
                    .all(|sample| (*sample - 10000).abs() < 10),
                "seam and both tails must remain at the user's original gain"
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

#[test]
fn live_pause_and_resume_change_pcm_gain_during_the_transition() {
    check_live_transport(false);
}

#[test]
#[ignore = "requires a Windows audio output device"]
fn wasapi_pause_and_resume_change_pcm_before_hardware_buffering() {
    check_live_transport(true);
}

fn check_live_transport(hardware: bool) {
    use std::time::{Duration, Instant};
    gst::init().unwrap();
    let file = std::env::temp_dir().join(format!("nons-live-fade-{}.wav", std::process::id()));
    std::fs::write(&file, wav(&vec![10000; 96000 * 8])).unwrap();
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
    let sink = if hardware {
        gst::parse::bin_from_description(
            "capsfilter name=pcm caps=audio/x-raw,format=S16LE,rate=48000,channels=2 ! audioconvert ! autoaudiosink",
            true,
        ).unwrap().upcast::<gst::Element>()
    } else {
        gst::ElementFactory::make("fakesink")
            .property("sync", true)
            .build()
            .unwrap()
    };
    let actual = Arc::new(Mutex::new(Vec::<(u64, f64)>::new()));
    let capture = actual.clone();
    sink.static_pad("sink")
        .unwrap()
        .add_probe(gst::PadProbeType::BUFFER, move |_, info| {
            let Some(gst::PadProbeData::Buffer(buffer)) = &info.data else {
                return gst::PadProbeReturn::Ok;
            };
            let map = buffer.map_readable().unwrap();
            let mean = map
                .as_slice()
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes(b.try_into().unwrap()).unsigned_abs() as f64)
                .sum::<f64>()
                / (map.size() / 2) as f64;
            capture.lock().unwrap().push((
                buffer.pts().unwrap().mseconds(),
                mean / if hardware { 10000.0 } else { 6000.0 },
            ));
            gst::PadProbeReturn::Ok
        });
    playbin.set_property("audio-sink", &sink);
    let fade = transitions::PlaybackFade::new().unwrap();
    fade.attach(&playbin).unwrap();
    fade.set_enabled(true);
    fade.prepare_start();
    playbin.set_property("volume", 0.6f64);
    playbin.set_property("uri", url::Url::from_file_path(&file).unwrap().to_string());
    playbin.set_state(gst::State::Playing).unwrap();
    let wait_until = |target: u64| {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let pos = playbin
                .query_position::<gst::ClockTime>()
                .map_or(0, |p| p.mseconds());
            if pos >= target {
                return pos;
            }
            assert!(
                Instant::now() < deadline,
                "playback position did not advance: state={:?}, bus error={:?}",
                playbin.current_state(),
                playbin
                    .bus()
                    .unwrap()
                    .pop_filtered(&[gst::MessageType::Error])
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    // Capture only this test process, after the Windows mixer, to catch stale
    // hardware PCM on resume rather than only inspecting pre-sink samples.
    let heard = Arc::new(Mutex::new(Vec::<(Instant, f64)>::new()));
    let loopback = hardware.then(|| {
        let pipeline = gst::parse::launch(&format!(
            "wasapi2src loopback=true loopback-mode=include-process-tree loopback-target-pid={} low-latency=true ! audioconvert ! audio/x-raw,format=F32LE,rate=48000,channels=2 ! fakesink name=capture sync=false",
            std::process::id()
        )).unwrap();
        let capture = pipeline.downcast_ref::<gst::Bin>().unwrap().by_name("capture").unwrap();
        pipeline.downcast_ref::<gst::Pipeline>().unwrap().use_clock(Some(&gst::SystemClock::obtain()));
        let weak_capture = capture.downgrade();
        let heard = heard.clone();
        capture.static_pad("sink").unwrap().add_probe(gst::PadProbeType::BUFFER, move |_, info| {
            if let Some(gst::PadProbeData::Buffer(buffer)) = &info.data {
                let map = buffer.map_readable().unwrap();
                let mean = map.as_slice().chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap()) as f64).sum::<f64>() / (map.size() / 4) as f64;
                let center = buffer.pts().unwrap() + buffer.duration().unwrap() / 2;
                let now = weak_capture.upgrade().unwrap().current_running_time().unwrap();
                // Loopback hands over a block after it was captured. Compare the
                // capture timestamp, not callback arrival (which adds source latency).
                let at = Instant::now() - std::time::Duration::from_nanos(now.saturating_sub(center).nseconds());
                heard.lock().unwrap().push((at, mean.abs()));
            }
            gst::PadProbeReturn::Ok
        });
        pipeline.set_state(gst::State::Playing).unwrap();
        pipeline
    });
    let start = wait_until(1500);
    let pause_at = Instant::now();
    fade.fade_out(playbin.current_running_time().unwrap());
    wait_until(start + transitions::PLAYBACK_FADE_MS + transitions::PAUSE_GRACE_MS);
    playbin.set_state(gst::State::Paused).unwrap();
    let (_, state, _) = playbin.state(gst::ClockTime::from_seconds(2));
    assert_eq!(state, gst::State::Paused);
    let paused = playbin
        .query_position::<gst::ClockTime>()
        .unwrap()
        .mseconds();
    std::thread::sleep(Duration::from_millis(100));
    let resume_at = Instant::now();
    fade.prepare_start();
    playbin.set_state(gst::State::Playing).unwrap();
    wait_until(paused + 450);
    playbin.set_state(gst::State::Null).unwrap();
    if let Some(loopback) = loopback {
        loopback.set_state(gst::State::Null).unwrap();
        let heard = heard.lock().unwrap();
        let mean_between = |from: Instant, to: Instant| {
            let window: Vec<_> = heard
                .iter()
                .filter(|(at, _)| *at >= from && *at < to)
                .collect();
            assert!(!window.is_empty(), "no process loopback samples in window");
            window.iter().map(|(_, mean)| mean).sum::<f64>() / window.len() as f64
        };
        let level = mean_between(
            pause_at - Duration::from_millis(300),
            pause_at - Duration::from_millis(100),
        );
        assert!(
            level > 0.01,
            "loopback did not capture the test stream: {level}"
        );
        let out_onset = heard
            .iter()
            .find(|(at, mean)| *at >= pause_at && *mean / level < 0.95 && *mean / level > 0.05)
            .expect("pause must produce intermediate audible gains")
            .0;
        assert!(
            out_onset.duration_since(pause_at) < Duration::from_millis(150),
            "fade-out started too late"
        );
        let out = mean_between(
            out_onset + Duration::from_millis(70),
            out_onset + Duration::from_millis(130),
        ) / level;
        let first = mean_between(resume_at, resume_at + Duration::from_millis(30)) / level;
        let onset = heard
            .iter()
            .find(|(at, mean)| *at >= resume_at && *mean / level > 0.05)
            .unwrap()
            .0;
        assert!(
            onset.duration_since(resume_at) < Duration::from_millis(150),
            "audio device took too long to resume"
        );
        let mid = mean_between(
            onset + Duration::from_millis(70),
            onset + Duration::from_millis(130),
        ) / level;
        let quiet = mean_between(
            pause_at
                + Duration::from_millis(
                    transitions::PLAYBACK_FADE_MS + transitions::PAUSE_GRACE_MS - 20,
                ),
            pause_at
                + Duration::from_millis(
                    transitions::PLAYBACK_FADE_MS + transitions::PAUSE_GRACE_MS,
                ),
        ) / level;
        let end = mean_between(
            resume_at + Duration::from_millis(300),
            resume_at + Duration::from_millis(400),
        ) / level;
        assert!(
            quiet < 0.1,
            "output must finish fading before pipeline pause: {quiet}"
        );
        assert!((0.25..0.8).contains(&out), "audible pause midpoint={out}");
        assert!(
            first < 0.25,
            "resume must not first play stale full-volume PCM: {first}"
        );
        assert!((0.2..0.8).contains(&mid), "audible resume midpoint={mid}");
        assert!(end > 0.9, "audible resume final gain={end}");
    }
    std::fs::remove_file(file).unwrap();
    let records = actual.lock().unwrap();
    let average = |from: u64, to: u64| {
        let window: Vec<_> = records
            .iter()
            .filter(|(ms, _)| *ms >= from && *ms < to)
            .collect();
        assert!(!window.is_empty(), "no PCM in window {from}..{to}");
        window.iter().map(|(_, gain)| gain).sum::<f64>() / window.len() as f64
    };
    let out_mid = average(start + 60, start + 140);
    let out_end = average(start + 160, start + transitions::PLAYBACK_FADE_MS);
    let in_mid = average(paused + 60, paused + 140);
    let in_end = average(paused + 250, paused + 350);
    assert!(
        (0.3..0.7).contains(&out_mid),
        "pause must gradually lower actual PCM, midpoint gain={out_mid}, end={out_end}"
    );
    assert!(
        out_end < 0.25,
        "PCM must become quiet before pausing: {out_end}"
    );
    assert!(
        (0.3..0.7).contains(&in_mid),
        "resume must gradually raise actual PCM: {in_mid}"
    );
    assert!(
        in_end > 0.9,
        "resume must recover the original gain: {in_end}"
    );
}

#[test]
fn paced_output_preserves_preroll_and_restored_position() {
    gst::init().unwrap();
    let file = std::env::temp_dir().join(format!("nons-fade-preroll-{}.wav", std::process::id()));
    std::fs::write(&file, wav(&vec![10000; 96000 * 5])).unwrap();
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
    let sink = gst::ElementFactory::make("fakesink")
        .property("sync", true)
        .property("signal-handoffs", true)
        .build()
        .unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    sink.connect("handoff", false, move |values| {
        let buffer = values[1].get::<gst::Buffer>().unwrap();
        let _ = sender.send(buffer.pts().unwrap().mseconds());
        None
    });
    playbin.set_property("audio-sink", &sink);
    let fade = transitions::PlaybackFade::new().unwrap();
    fade.attach(&playbin).unwrap();
    fade.set_enabled(true);
    fade.prepare_start();
    playbin.set_property("uri", url::Url::from_file_path(&file).unwrap().to_string());
    playbin.set_state(gst::State::Paused).unwrap();
    assert_eq!(
        playbin.state(gst::ClockTime::from_seconds(2)).1,
        gst::State::Paused
    );
    assert!(
        receiver.try_recv().is_err(),
        "preroll must not render audio"
    );
    playbin
        .seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::ClockTime::from_seconds(2),
        )
        .unwrap();
    playbin.set_state(gst::State::Playing).unwrap();
    let first = receiver.recv_timeout(std::time::Duration::from_secs(2));
    playbin.set_state(gst::State::Null).unwrap();
    std::fs::remove_file(file).unwrap();
    assert_eq!(first.unwrap(), 2000);
}
