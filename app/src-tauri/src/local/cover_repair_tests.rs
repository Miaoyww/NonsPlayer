use super::*;
use crate::local::library;
use std::{path::PathBuf, sync::Arc};

struct Fixture {
    _root: tempfile::TempDir,
    music: PathBuf,
    covers: PathBuf,
    fallback: String,
    store: Arc<Store>,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let music = root.path().join("music");
        let covers = root.path().join("covers");
        std::fs::create_dir(&music).unwrap();
        std::fs::create_dir(&covers).unwrap();
        let fallback = covers.join("default.png").to_string_lossy().into_owned();
        std::fs::write(&fallback, include_bytes!("../../icons/128x128.png")).unwrap();
        wav(&music.join("album.wav"));
        let store = Arc::new(Store::open(&root.path().join("library.sqlite")).unwrap());
        Self {
            _root: root,
            music,
            covers,
            fallback,
            store,
        }
    }

    fn scan(&self) -> Vec<Track> {
        let report = library::import(
            vec![self.music.to_string_lossy().into_owned()],
            self.store.clone(),
            self.covers.clone(),
            self.fallback.clone(),
            Some("managed".into()),
        )
        .unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        self.store.local_tracks("", 0).unwrap()
    }

    fn image(&self, name: &str, color: u8) -> Vec<u8> {
        let path = self.music.join(name);
        image::RgbImage::from_pixel(2, 2, image::Rgb([color, 0, 0]))
            .save(&path)
            .unwrap();
        std::fs::read(path).unwrap()
    }

    fn repair(&self, cover: &str) -> (String, bool) {
        repair(cover, &self.store, &self.covers, &self.fallback).unwrap()
    }

    fn album_cover(&self) -> String {
        self.store
            .0
            .lock()
            .unwrap()
            .query_row(
                "SELECT cover FROM local_entities WHERE kind='album' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }
}

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

fn audio_stat(path: &Path) -> (u64, std::time::SystemTime) {
    let metadata = std::fs::metadata(path).unwrap();
    (metadata.len(), metadata.modified().unwrap())
}

#[test]
fn rebuild_recovers_unchanged_placeholder_beyond_former_budget_and_reuses_assets() {
    let fixture = Fixture::new();
    let bytes = fixture.image("cover.png", 33);
    let mut track = fixture.scan().remove(0);
    let expected = track.cover.clone();
    let stat = audio_stat(&fixture.music.join("album.wav"));
    track.cover = fixture.fallback.clone().into();
    fixture.store.save_tracks(&[track.clone()]).unwrap();
    assert_eq!(fixture.scan()[0].cover.to_string(), fixture.fallback);
    std::fs::File::create(fixture.covers.join("old-assets.img"))
        .unwrap()
        .set_len(32 * 1024 * 1024)
        .unwrap();
    std::fs::remove_file(&*expected).unwrap();
    let mut progress = Vec::new();
    super::super::cover_rebuild::rebuild(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        |value| progress.push(serde_json::to_value(value).unwrap()),
    )
    .unwrap();
    let restored = fixture.store.track(&track.key).unwrap();
    assert_eq!(std::fs::read(&*restored.cover).unwrap(), bytes);
    assert_eq!(fixture.album_cover(), restored.cover.to_string());
    assert_eq!(progress.last().unwrap()["processed"], 1);
    assert_eq!(progress.last().unwrap()["failed"], 0);
    assert_eq!(audio_stat(&fixture.music.join("album.wav")), stat);
    let cover_stat = audio_stat(Path::new(&*restored.cover));
    super::super::cover_rebuild::rebuild(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        |_| {},
    )
    .unwrap();
    assert_eq!(audio_stat(Path::new(&*restored.cover)), cover_stat);
}

#[test]
fn rebuild_keeps_missing_sources_and_continues_with_other_tracks() {
    let fixture = Fixture::new();
    fixture.image("cover.png", 34);
    wav(&fixture.music.join("second.wav"));
    let tracks = fixture.scan();
    let missing = tracks.iter().find(|track| matches!(&track.source, TrackSource::Local { path, .. } if path.ends_with("second.wav"))).unwrap();
    std::fs::remove_file(fixture.music.join("second.wav")).unwrap();
    let before = serde_json::to_value(missing).unwrap();
    let mut last = serde_json::Value::Null;
    super::super::cover_rebuild::rebuild(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        |value| last = serde_json::to_value(value).unwrap(),
    )
    .unwrap();
    assert_eq!(last["processed"], 2);
    assert_eq!(last["failed"], 1);
    assert_eq!(
        serde_json::to_value(fixture.store.track(&missing.key).unwrap()).unwrap(),
        before
    );
}

#[test]
fn rebuild_embedded_cover_preserves_cue_metadata_bindings_playlists_and_history() {
    use lofty::{
        config::WriteOptions,
        picture::Picture,
        prelude::*,
        tag::{Tag, TagType},
    };
    let fixture = Fixture::new();
    let embedded = fixture.image("embedded.png", 35);
    std::fs::remove_file(fixture.music.join("embedded.png")).unwrap();
    fixture.image("cover.png", 36);
    let mut tag = Tag::new(TagType::Id3v2);
    tag.push_picture(Picture::from_reader(&mut std::io::Cursor::new(&embedded)).unwrap());
    tag.save_to_path(fixture.music.join("album.wav"), WriteOptions::default())
        .unwrap();
    std::fs::write(fixture.music.join("album.cue"), "PERFORMER \"乐队\"\nTITLE \"多轨\"\nFILE \"album.wav\" WAVE\n TRACK 01 AUDIO\n TITLE \"第一首\"\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n TITLE \"第二首\"\n INDEX 01 00:02:00\n").unwrap();
    let mut tracks = fixture.scan();
    if let TrackSource::Local { netease_id, .. } = &mut tracks[0].source {
        *netease_id = Some(42);
    }
    for track in &mut tracks {
        track.cover = fixture.fallback.clone().into();
    }
    fixture.store.save_tracks(&tracks).unwrap();
    fixture
        .store
        .record_playback("rebuild-test", &tracks[0], 1)
        .unwrap();
    let playlist_value = serde_json::to_string(&tracks[0]).unwrap();
    {
        let db = fixture.store.0.lock().unwrap();
        db.execute(
            "INSERT INTO local_playlists(id,name) VALUES(1,'测试歌单')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO local_playlist_tracks(playlist_id,key,position,value) VALUES(1,?1,0,?2)",
            rusqlite::params![tracks[0].key, playlist_value],
        )
        .unwrap();
    }
    let metadata = fixture.store.local_information(&tracks[0].key).unwrap();
    super::super::cover_rebuild::rebuild(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        |_| {},
    )
    .unwrap();
    for track in &tracks {
        let mut expected = serde_json::to_value(track).unwrap();
        let restored = fixture.store.track(&track.key).unwrap();
        assert_eq!(std::fs::read(&*restored.cover).unwrap(), embedded);
        expected["cover"] = serde_json::to_value(&restored.cover).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), expected);
    }
    assert_eq!(
        serde_json::to_value(fixture.store.local_information(&tracks[0].key).unwrap()).unwrap(),
        serde_json::to_value(metadata).unwrap()
    );
    assert_eq!(fixture.store.playback_count().unwrap(), 1);
    assert_eq!(
        fixture
            .store
            .0
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM local_playlist_tracks WHERE playlist_id=1",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        playlist_value
    );
}

#[test]
fn unchanged_audio_refreshes_added_replaced_and_removed_directory_covers() {
    let fixture = Fixture::new();
    let audio = fixture.music.join("album.wav");
    let stat = audio_stat(&audio);
    let initial = fixture.scan().remove(0);
    assert_eq!(initial.cover.to_string(), fixture.fallback);
    let first_bytes = fixture.image("cover.png", 20);
    let first = fixture.scan().remove(0);
    assert_eq!(first.key, initial.key);
    assert_eq!(std::fs::read(&*first.cover).unwrap(), first_bytes);
    assert_eq!(fixture.album_cover(), first.cover.to_string());
    let second_bytes = fixture.image("cover.png", 180);
    let second = fixture.scan().remove(0);
    assert_ne!(second.cover, first.cover);
    assert_eq!(std::fs::read(&*second.cover).unwrap(), second_bytes);
    std::fs::remove_file(fixture.music.join("cover.png")).unwrap();
    let final_track = fixture.scan().remove(0);
    assert_eq!(final_track.cover.to_string(), fixture.fallback);
    assert!(fixture.album_cover().is_empty());
    assert_eq!(audio_stat(&audio), stat);
}

#[test]
fn missing_and_corrupt_generated_assets_are_rebuilt_without_source_changes() {
    let fixture = Fixture::new();
    let bytes = fixture.image("cover.png", 30);
    let track = fixture.scan().remove(0);
    let source_stat = audio_stat(&fixture.music.join("cover.png"));
    let audio_stat_before = audio_stat(&fixture.music.join("album.wav"));
    for corrupt in [false, true] {
        if corrupt {
            std::fs::write(&*track.cover, b"damaged").unwrap();
        } else {
            std::fs::remove_file(&*track.cover).unwrap();
        }
        let (recovered, changed) = fixture.repair(&track.cover);
        assert!(changed);
        assert_eq!(recovered, track.cover.to_string());
        assert_eq!(std::fs::read(&recovered).unwrap(), bytes);
        assert_eq!(fixture.album_cover(), recovered);
        assert_eq!(
            fixture.store.track(&track.key).unwrap().cover.to_string(),
            recovered
        );
        assert!(
            !fixture.repair(&recovered).1,
            "a repeated repair must not publish a change"
        );
    }
    assert_eq!(audio_stat(&fixture.music.join("cover.png")), source_stat);
    assert_eq!(
        audio_stat(&fixture.music.join("album.wav")),
        audio_stat_before
    );
}

#[test]
fn unchanged_embedded_cover_is_restored_and_remains_ahead_of_external_images() {
    use lofty::{
        config::WriteOptions,
        picture::{Picture, PictureType},
        prelude::*,
        tag::{Tag, TagType},
    };
    let fixture = Fixture::new();
    let embedded = fixture.image("embedded.png", 40);
    std::fs::remove_file(fixture.music.join("embedded.png")).unwrap();
    fixture.image("cover.png", 210);
    let audio = fixture.music.join("album.wav");
    let mut tag = Tag::new(TagType::Id3v2);
    let mut picture = Picture::from_reader(&mut std::io::Cursor::new(&embedded)).unwrap();
    picture.set_pic_type(PictureType::CoverFront);
    tag.push_picture(picture);
    tag.set_album("内嵌专辑".into());
    tag.save_to_path(&audio, WriteOptions::default()).unwrap();
    let stat = audio_stat(&audio);
    let track = fixture.scan().remove(0);
    assert_eq!(std::fs::read(&*track.cover).unwrap(), embedded);
    std::fs::remove_file(&*track.cover).unwrap();
    let (cover, changed) = fixture.repair(&track.cover);
    assert!(changed);
    assert_eq!(std::fs::read(&cover).unwrap(), embedded);
    assert_eq!(audio_stat(&audio), stat);
    assert_eq!(fixture.album_cover(), cover);
}

#[test]
fn ncm_embedded_cover_recovers_without_changing_the_container() {
    use lofty::{
        config::WriteOptions,
        picture::{Picture, PictureType},
        prelude::*,
        tag::{Tag, TagType},
    };
    let fixture = Fixture::new();
    let embedded = fixture.image("embedded.png", 55);
    std::fs::remove_file(fixture.music.join("embedded.png")).unwrap();
    std::fs::remove_file(fixture.music.join("album.wav")).unwrap();
    let flac = fixture.music.join("album.flac");
    std::fs::write(&flac, crate::local::encoded_audio::tests::flac()).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    let mut picture = Picture::from_reader(&mut std::io::Cursor::new(&embedded)).unwrap();
    picture.set_pic_type(PictureType::CoverFront);
    tag.push_picture(picture);
    tag.save_to_path(&flac, WriteOptions::default()).unwrap();
    let encoded = crate::local::encoded_audio::tests::fixture(&std::fs::read(&flac).unwrap(), 0);
    let ncm = fixture.music.join("album.ncm");
    std::fs::write(&ncm, encoded).unwrap();
    std::fs::remove_file(flac).unwrap();
    let stat = audio_stat(&ncm);
    let track = fixture.scan().remove(0);
    assert_eq!(std::fs::read(&*track.cover).unwrap(), embedded);
    std::fs::write(&*track.cover, b"broken").unwrap();
    let (cover, changed) = fixture.repair(&track.cover);
    assert!(changed);
    assert_eq!(std::fs::read(&cover).unwrap(), embedded);
    assert_eq!(audio_stat(&ncm), stat);
    assert_eq!(fixture.store.track(&track.key).unwrap().key, track.key);
}

#[test]
fn concurrent_repairs_publish_only_one_rebuild() {
    let fixture = Fixture::new();
    fixture.image("cover.png", 70);
    let track = fixture.scan().remove(0);
    std::fs::remove_file(&*track.cover).unwrap();
    let replies = std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..4 {
            workers.push(scope.spawn(|| fixture.repair(&track.cover)));
        }
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(replies.iter().filter(|(_, changed)| *changed).count(), 1);
    assert!(replies
        .iter()
        .all(|(cover, _)| *cover == track.cover.to_string()));
    assert!(super::super::cover_art::valid(Path::new(&*track.cover)));
}

#[test]
fn repairs_reject_outside_paths_and_clear_dangling_references_when_audio_is_unavailable() {
    let fixture = Fixture::new();
    fixture.image("cover.png", 90);
    assert!(repair(
        fixture.music.join("cover.png").to_str().unwrap(),
        &fixture.store,
        &fixture.covers,
        &fixture.fallback
    )
    .is_err());
    assert!(repair(
        fixture.covers.join("../music/cover.png").to_str().unwrap(),
        &fixture.store,
        &fixture.covers,
        &fixture.fallback
    )
    .is_err());
    let track = fixture.scan().remove(0);
    std::fs::remove_file(&*track.cover).unwrap();
    std::fs::remove_file(fixture.music.join("album.wav")).unwrap();
    std::fs::remove_file(fixture.music.join("cover.png")).unwrap();
    let (cover, changed) = fixture.repair(&track.cover);
    assert!(changed);
    assert_eq!(cover, fixture.fallback);
    assert_eq!(
        fixture.store.track(&track.key).unwrap().cover.to_string(),
        fixture.fallback
    );
    assert!(fixture.album_cover().is_empty());
    assert!(super::super::cover_art::valid(Path::new(&cover)));
}

#[test]
fn cue_repair_keeps_virtual_track_ids_and_lyric_bindings() {
    let fixture = Fixture::new();
    fixture.image("cover.png", 110);
    std::fs::write(fixture.music.join("album.cue"), "PERFORMER \"乐队\"\nTITLE \"多轨\"\nFILE \"album.wav\" WAVE\n TRACK 01 AUDIO\n TITLE \"第一首\"\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n TITLE \"第二首\"\n INDEX 01 00:02:00\n").unwrap();
    let mut tracks = fixture.scan();
    assert_eq!(tracks.len(), 2);
    assert!(tracks.iter().all(|track| track.cue().is_some()));
    if let TrackSource::Local { netease_id, .. } = &mut tracks[0].source {
        *netease_id = Some(42);
    }
    fixture.store.save_tracks(&[tracks[0].clone()]).unwrap();
    std::fs::remove_file(&*tracks[0].cover).unwrap();
    let (cover, changed) = fixture.repair(&tracks[0].cover);
    assert!(changed);
    for track in &tracks {
        let restored = fixture.store.track(&track.key).unwrap();
        assert_eq!(restored.cover.to_string(), cover);
        assert_eq!(
            restored.cue().unwrap().track_number,
            track.cue().unwrap().track_number
        );
        assert_eq!(restored.netease_id(), track.netease_id());
    }
    assert_eq!(fixture.album_cover(), cover);
}

#[test]
fn repair_restores_a_deleted_cover_directory_and_its_default_asset() {
    let fixture = Fixture::new();
    let bytes = fixture.image("cover.png", 130);
    let track = fixture.scan().remove(0);
    let audio_stat_before = audio_stat(&fixture.music.join("album.wav"));
    let covers = fixture.covers.canonicalize().unwrap();
    assert!(covers.starts_with(fixture._root.path().canonicalize().unwrap()));
    for entry in std::fs::read_dir(&covers).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file());
        std::fs::remove_file(entry.path()).unwrap();
    }
    std::fs::remove_dir(&covers).unwrap();
    assert!(!fixture.covers.exists());
    let (cover, changed) = fixture.repair(&track.cover);
    assert!(changed);
    assert_eq!(cover, track.cover.to_string());
    assert_eq!(std::fs::read(&cover).unwrap(), bytes);
    assert!(super::super::cover_art::valid(Path::new(&fixture.fallback)));
    assert_eq!(fixture.album_cover(), cover);
    assert_eq!(
        audio_stat(&fixture.music.join("album.wav")),
        audio_stat_before
    );
}

#[test]
fn cue_without_album_performer_uses_its_track_artists_without_inventing_album_artists() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.music.join("album.cue"),
        "TITLE \"合辑\"\nFILE \"album.wav\" WAVE\n TRACK 01 AUDIO\n TITLE \"第一首\"\n PERFORMER \"歌手甲\"\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n TITLE \"第二首\"\n PERFORMER \"歌手乙\"\n INDEX 01 00:02:00\n",
    ).unwrap();
    let tracks = fixture.scan();
    assert_eq!(tracks.len(), 2);
    for track in &tracks {
        assert!(track.cue().is_some());
        assert_eq!(
            fixture
                .store
                .local_information(&track.key)
                .unwrap()
                .album_artist,
            None
        );
    }
    let subtitle: String = fixture
        .store
        .0
        .lock()
        .unwrap()
        .query_row(
            "SELECT subtitle FROM local_entities WHERE kind='album' AND id='合辑'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(subtitle, "歌手甲 / 歌手乙");
}
