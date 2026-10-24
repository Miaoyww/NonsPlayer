use super::*;
use crate::{local::library, storage::Store, test_support::TestDir};
use gstreamer::{self as gst, prelude::*};
use sha2::Digest;
use std::sync::{Arc, Mutex};

fn wav(path: &Path) {
    let bytes = 8_000u32 * 2 * 4;
    let mut data = Vec::new();
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(bytes + 36).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&8_000u32.to_le_bytes());
    data.extend_from_slice(&16_000u32.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&16u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&bytes.to_le_bytes());
    data.resize(44 + bytes as usize, 0);
    std::fs::write(path, data).unwrap();
}

const CUE: &str = "PERFORMER \"艺术家\"\nTITLE \"专辑\"\nFILE \"album.wav\" WAVE\n TRACK 01 AUDIO\n TITLE \"第一首\"\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n TITLE \"第二首\"\n INDEX 00 00:01:60\n INDEX 01 00:02:00\n";

#[test]
fn cue_cover_falls_back_in_order_and_refreshes_without_audio_changes() {
    let dir = TestDir::new();
    let music = dir.0.join("music");
    let covers = dir.0.join("covers");
    std::fs::create_dir(&music).unwrap();
    std::fs::create_dir(&covers).unwrap();
    wav(&music.join("album.wav"));
    std::fs::write(music.join("album.cue"), CUE).unwrap();
    let image = |name: &str, color: u8| {
        image::RgbImage::from_pixel(2, 2, image::Rgb([color, 0, 0]))
            .save(music.join(name))
            .unwrap();
    };
    image("COVER.PNG", 80);
    let store = Arc::new(Store::open(&dir.0.join("library.sqlite")).unwrap());
    let scan = || {
        let report = library::import(
            vec![music.to_string_lossy().into_owned()],
            store.clone(),
            covers.clone(),
            String::new(),
            Some("managed".into()),
        )
        .unwrap();
        assert!(report.errors.is_empty());
        store.local_tracks("", 0).unwrap()
    };
    let tracks = scan();
    let cover = tracks[0].cover.to_string();
    assert!(tracks.iter().all(|track| track.cover.to_string() == cover));
    assert_eq!(
        std::fs::read(&cover).unwrap(),
        std::fs::read(music.join("COVER.PNG")).unwrap()
    );
    image("COVER.PNG", 120);
    let updated = scan()[0].cover.to_string();
    assert_ne!(cover, updated);
    image("folder.png", 20);
    std::fs::write(music.join("COVER.PNG"), "corrupt").unwrap();
    let fallback = scan()[0].cover.to_string();
    assert_eq!(
        std::fs::read(&fallback).unwrap(),
        std::fs::read(music.join("folder.png")).unwrap()
    );
    std::fs::remove_file(music.join("COVER.PNG")).unwrap();
    std::fs::remove_file(music.join("folder.png")).unwrap();
    assert!(scan()
        .iter()
        .all(|track| track.cover.to_string().is_empty()));
    // A dangling cached path must not be treated as a valid embedded cover.
    let mut base = store
        .track(&format!(
            "local:{:x}",
            sha2::Sha256::digest(
                music
                    .join("album.wav")
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .trim_start_matches(r"\\?\")
                    .as_bytes()
            )
        ))
        .unwrap();
    base.cover = "embedded.img".into();
    store.save_tracks(&[base]).unwrap();
    assert!(scan()
        .iter()
        .all(|track| track.cover.to_string().is_empty()));
}

#[test]
fn cue_directory_rescan_hides_whole_file_and_restores_it_after_removal() {
    let dir = TestDir::new();
    let music = dir.0.join("music");
    let covers = dir.0.join("covers");
    std::fs::create_dir(&music).unwrap();
    std::fs::create_dir(&covers).unwrap();
    wav(&music.join("album.wav"));
    let store = Arc::new(Store::open(&dir.0.join("library.sqlite")).unwrap());
    let scan = || {
        library::import(
            vec![music.to_string_lossy().into_owned()],
            store.clone(),
            covers.clone(),
            String::new(),
            Some("managed".into()),
        )
        .unwrap()
    };
    assert_eq!(scan().imported, 1);
    let base = store.local_tracks("", 0).unwrap()[0].clone();
    std::fs::write(music.join("album.cue"), CUE).unwrap();
    let report = scan();
    assert_eq!(report.errors, Vec::<String>::new());
    assert_eq!(report.imported, 2);
    assert_eq!(store.origin_count("managed").unwrap(), 2);
    let tracks = store.local_tracks("", 0).unwrap();
    assert_eq!(tracks.len(), 2);
    assert!(tracks
        .iter()
        .all(|track| track.cue().is_some() && track.duration_ms == 2_000));
    assert!(tracks.iter().all(|track| track.album == "专辑"));
    assert!(
        store.track(&base.key).is_ok(),
        "hidden originals retain their identity and origins"
    );
    let mut bound = tracks[0].clone();
    if let TrackSource::Local { netease_id, .. } = &mut bound.source {
        *netease_id = Some(42);
    }
    store.save_tracks(&[bound.clone()]).unwrap();
    assert_eq!(scan().imported, 2);
    assert_eq!(store.track(&bound.key).unwrap().netease_id(), Some(42));
    // A corrupt/unsupported sheet must not hide the usable audio or retain stale tracks.
    std::fs::write(
        music.join("album.cue"),
        format!("{CUE}FILE \"other.wav\" WAVE\n"),
    )
    .unwrap();
    let report = scan();
    assert_eq!(report.skipped, 1);
    assert!(report.errors[0].contains("多 FILE"));
    assert_eq!(store.local_tracks("", 0).unwrap().len(), 1);
    std::fs::write(music.join("album.cue"), CUE).unwrap();
    scan();
    let keys = store
        .local_tracks("", 0)
        .unwrap()
        .into_iter()
        .map(|track| track.key)
        .collect::<Vec<_>>();
    assert!(keys.contains(&bound.key));
    std::fs::remove_file(music.join("album.cue")).unwrap();
    scan();
    assert_eq!(store.local_tracks("", 0).unwrap()[0].key, base.key);
    assert!(keys.iter().all(|key| store.track(key).is_err()));
    assert_eq!(store.origin_count("managed").unwrap(), 1);
}

#[test]
fn cue_decodes_supported_encodings_and_rejects_invalid_timestamps() {
    assert_eq!(timestamp_ms("03:40:63".parse().unwrap()).unwrap(), 220_840);
    assert_eq!(timestamp_ms("00:00:74".parse().unwrap()).unwrap(), 986);
    for encoding in [encoding_rs::UTF_8, encoding_rs::GB18030] {
        let (bytes, _, errors) = encoding.encode(CUE);
        assert!(!errors);
        assert_eq!(
            parse(&decode(&bytes).unwrap())
                .unwrap()
                .performer
                .as_deref(),
            Some("艺术家")
        );
    }
    for little in [true, false] {
        let mut bytes = if little {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for unit in CUE.encode_utf16() {
            bytes.extend_from_slice(&if little {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        assert_eq!(parse(&decode(&bytes).unwrap()).unwrap().tracks.len(), 2);
    }
    for invalid in [
        CUE.replace("00:02:00", "00:00:00"),
        CUE.replace("INDEX 01 00:02:00", ""),
        CUE.replace("TRACK 02", "TRACK 01"),
    ] {
        assert!(parse(&invalid).is_err());
    }
}

fn verify_audio_range(track: &Track, seek_ms: u64) {
    gst::init().unwrap();
    let pipeline = gst::ElementFactory::make("playbin").build().unwrap();
    let sink = gst::ElementFactory::make("fakesink")
        .property("sync", false)
        .property("signal-handoffs", true)
        .build()
        .unwrap();
    let times = Arc::new(Mutex::new(Vec::new()));
    let samples = times.clone();
    sink.connect("handoff", false, move |values| {
        let buffer = values[1].get::<gst::Buffer>().unwrap();
        if let Some(pts) = buffer.pts() {
            samples.lock().unwrap().push((
                pts.nseconds(),
                (pts + buffer.duration().unwrap_or(gst::ClockTime::ZERO)).nseconds(),
            ));
        }
        None
    });
    pipeline.set_property("audio-sink", &sink);
    let TrackSource::Local { path, .. } = &track.source else {
        unreachable!()
    };
    let uri = url::Url::from_file_path(path).unwrap();
    crate::audio::start_stream(&pipeline, uri.as_str(), 1.0, gst::State::Paused).unwrap();
    assert_eq!(
        pipeline.state(gst::ClockTime::from_seconds(5)).1,
        gst::State::Paused
    );
    crate::audio::seek_track(&pipeline, track, seek_ms).unwrap();
    pipeline.state(gst::ClockTime::from_seconds(5)).0.unwrap();
    assert!(
        times.lock().unwrap().is_empty(),
        "preroll cannot leak pre-track audio"
    );
    pipeline.set_state(gst::State::Playing).unwrap();
    let bus = pipeline.bus().unwrap();
    let end = bus.timed_pop_filtered(
        gst::ClockTime::from_seconds(15),
        &[gst::MessageType::Eos, gst::MessageType::Error],
    );
    pipeline.set_state(gst::State::Null).unwrap();
    assert!(
        end.is_some_and(|message| matches!(message.view(), gst::MessageView::Eos(_))),
        "bounded playback must finish with EOS"
    );
    let times = times.lock().unwrap();
    let cue = track.cue().unwrap();
    let first = times.first().unwrap().0;
    let last = times.last().unwrap().1;
    // Seek starts allow one sample of rounding. EOF duration metadata is rounded to ms.
    assert!(
        first.abs_diff((cue.start_ms + seek_ms) * 1_000_000) < 150_000,
        "first={first} cue={cue:?}"
    );
    assert!(
        last <= cue.end_ms * 1_000_000 + 150_000
            && last.abs_diff(cue.end_ms * 1_000_000) < 1_000_000,
        "last={last} cue={cue:?}"
    );
    assert_eq!(
        crate::audio::track_position(Some(track), first / 1_000_000),
        seek_ms
    );
    assert_eq!(
        crate::audio::track_duration(Some(track), 999_999),
        track.duration_ms
    );
}

#[test]
fn cue_playback_seek_starts_and_stops_at_virtual_track_boundaries() {
    let dir = TestDir::new();
    let audio = dir.0.join("album.wav");
    wav(&audio);
    let cue = dir.0.join("album.cue");
    std::fs::write(&cue, CUE).unwrap();
    let covers = dir.0.join("covers");
    std::fs::create_dir(&covers).unwrap();
    let store = Arc::new(Store::open(&dir.0.join("library.sqlite")).unwrap());
    library::import(
        vec![cue.to_string_lossy().into_owned()],
        store.clone(),
        covers,
        String::new(),
        None,
    )
    .unwrap();
    for track in store.local_tracks("", 0).unwrap() {
        verify_audio_range(&track, 0);
        verify_audio_range(&track, 1_000);
    }
}

#[test]
#[ignore = "set NONS_CUE_TEST_FILE to a read-only More One Night sample"]
fn cue_real_album_has_four_tracks_and_correct_pcm_boundaries() {
    let path = std::path::PathBuf::from(std::env::var("NONS_CUE_TEST_FILE").unwrap());
    let before = std::fs::read(&path).unwrap();
    let dir = TestDir::new();
    let covers = dir.0.join("covers");
    std::fs::create_dir(&covers).unwrap();
    let store = Arc::new(Store::open(&dir.0.join("library.sqlite")).unwrap());
    let report = library::import(
        vec![path.to_string_lossy().into_owned()],
        store.clone(),
        covers,
        String::new(),
        None,
    )
    .unwrap();
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.imported, 4);
    let mut tracks = store.local_tracks("", 0).unwrap();
    tracks.sort_by_key(|track| track.cue().unwrap().track_number);
    let expected = [
        "More One Night",
        "雨だれの歌",
        "More One Night(instrumental)",
        "雨だれの歌(instrumental)",
    ];
    assert_eq!(tracks.len(), 4);
    let cover = tracks[0].cover.to_string();
    assert!(!cover.is_empty());
    assert_eq!(
        std::fs::read(&cover).unwrap(),
        std::fs::read(path.parent().unwrap().join("cover.jpg")).unwrap()
    );
    assert!(tracks.iter().all(|track| track.cover.to_string() == cover));
    for (index, track) in tracks.iter().enumerate() {
        assert_eq!(track.title, expected[index]);
        assert_eq!(
            track.duration_ms,
            [220_840, 265_053, 220_853, 263_921][index]
        );
        println!("{}: {} ({} ms)", index + 1, track.title, track.duration_ms);
        verify_audio_range(track, 0);
        verify_audio_range(track, track.duration_ms - 1_000);
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
}
