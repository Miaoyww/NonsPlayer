use super::*;
use crate::local::library;
use lofty::probe::Probe;
use nons_music_adapter_sdk::queue::CueTrack;
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
        wav(&music.join("album.wav"));
        let store = Arc::new(Store::open(&root.path().join("library.sqlite")).unwrap());
        let fallback = covers.join("default.png").to_string_lossy().into_owned();
        library::prepare_covers(&covers, &fallback).unwrap();
        Self {
            _root: root,
            music,
            covers,
            fallback,
            store,
        }
    }

    fn track(&self) -> Track {
        let report = library::import(
            vec![self.music.to_string_lossy().into_owned()],
            self.store.clone(),
            self.covers.clone(),
            self.fallback.clone(),
            Some("managed".into()),
        )
        .unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        self.store.local_tracks("", 0).unwrap().remove(0)
    }

    fn image(&self) -> String {
        let path = self.music.join("selected.png");
        image::RgbImage::from_pixel(3, 3, image::Rgb([12, 34, 56]))
            .save(&path)
            .unwrap();
        path.to_string_lossy().into_owned()
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

fn update(cover: CoverChange) -> LocalTrackTagUpdate {
    LocalTrackTagUpdate {
        title: "新标题".into(),
        artist: "新艺术家".into(),
        album_artist: "新专辑艺术家".into(),
        album: "新专辑".into(),
        year: "2026".into(),
        track_number: Some(3),
        disc_number: Some(2),
        genre: "电子".into(),
        comment: "测试备注".into(),
        cover,
    }
}

#[test]
fn writing_tags_updates_file_indexes_cover_and_queue_metadata() {
    let fixture = Fixture::new();
    let track = fixture.track();
    let cover_path = fixture.image();
    let cover_bytes = std::fs::read(&cover_path).unwrap();
    let updated = write_tags(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        &track.key,
        update(CoverChange::Replace { path: cover_path }),
    )
    .unwrap();

    assert_eq!(updated.title, "新标题");
    assert_eq!(updated.artist, "新艺术家");
    assert_eq!(updated.album, "新专辑");
    assert_eq!(std::fs::read(&*updated.cover).unwrap(), cover_bytes);
    let information = fixture.store.local_information(&track.key).unwrap();
    assert_eq!(information.track_number, Some(3));
    assert_eq!(information.disc_number, Some(2));
    assert_eq!(information.album_artist.as_deref(), Some("新专辑艺术家"));
    let album = fixture
        .store
        .0
        .lock()
        .unwrap()
        .query_row(
            "SELECT subtitle FROM local_entities WHERE kind='album' AND id='新专辑'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap();
    assert_eq!(album, "新专辑艺术家");

    let tags = read_tags(&updated.source_path()).unwrap();
    assert_eq!(tags.title, "新标题");
    assert_eq!(tags.artist, "新艺术家");
    assert_eq!(tags.album_artist, "新专辑艺术家");
    assert_eq!(tags.album, "新专辑");
    assert_eq!(tags.year, "2026");
    assert_eq!(tags.track_number, Some(3));
    assert_eq!(tags.disc_number, Some(2));
    assert_eq!(tags.genre, "电子");
    assert_eq!(tags.comment, "测试备注");
    let tagged = Probe::open(updated.source_path()).unwrap().read().unwrap();
    assert_eq!(
        tagged
            .primary_tag()
            .unwrap()
            .get_picture_type(PictureType::CoverFront)
            .unwrap()
            .data(),
        cover_bytes
    );
}

#[test]
fn empty_fields_and_removed_cover_clear_tags_and_restore_fallback() {
    let fixture = Fixture::new();
    let track = fixture.track();
    let cover_path = fixture.image();
    let first = write_tags(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        &track.key,
        update(CoverChange::Replace { path: cover_path }),
    )
    .unwrap();
    assert_ne!(first.cover.to_string(), fixture.fallback);
    std::fs::remove_file(fixture.music.join("selected.png")).unwrap();

    let cleared = write_tags(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        &track.key,
        LocalTrackTagUpdate {
            title: String::new(),
            artist: String::new(),
            album_artist: String::new(),
            album: String::new(),
            year: String::new(),
            track_number: None,
            disc_number: None,
            genre: String::new(),
            comment: String::new(),
            cover: CoverChange::Remove,
        },
    )
    .unwrap();
    assert_eq!(cleared.title, "album");
    assert_eq!(cleared.artist, "未知艺术家");
    assert_eq!(cleared.album, "未知专辑");
    assert_eq!(cleared.cover.to_string(), fixture.fallback);
    let tags = read_tags(&cleared.source_path()).unwrap();
    assert!(tags.title.is_empty());
    assert!(tags.artist.is_empty());
    assert!(tags.album_artist.is_empty());
    assert!(tags.album.is_empty());
    assert!(tags.year.is_empty());
    assert_eq!(tags.track_number, None);
    assert_eq!(tags.disc_number, None);
    assert!(tags.genre.is_empty());
    assert!(tags.comment.is_empty());
    let tagged = Probe::open(cleared.source_path()).unwrap().read().unwrap();
    assert!(tagged
        .primary_tag()
        .is_none_or(|tag| tag.pictures().is_empty()));
}

#[test]
fn cue_and_ncm_tracks_are_not_editable() {
    for (cue, path, expected) in [
        (
            Some(CueTrack {
                sheet_path: "album.cue".into(),
                track_number: 1,
                start_ms: 0,
                end_ms: 1_000,
            }),
            "album.wav",
            "CUE",
        ),
        (None, "album.ncm", "NCM"),
        (None, "nons-saf/token/music/album.flac", "Android"),
    ] {
        let track = Track {
            key: "local:test".into(),
            title: String::new(),
            aliases: Vec::new(),
            artist: String::new(),
            album: String::new(),
            artists: Vec::new(),
            duration_ms: 0,
            cover: Default::default(),
            source: TrackSource::Local {
                path: path.into(),
                netease_id: None,
                cue,
            },
        };
        assert!(editable_path(&track).unwrap_err().contains(expected));
    }
}

#[test]
fn invalid_years_and_zero_numbers_are_rejected_without_touching_the_file() {
    let fixture = Fixture::new();
    let track = fixture.track();
    let audio = fixture.music.join("album.wav");
    let original = std::fs::read(&audio).unwrap();
    let mut tags = update(CoverChange::Keep);
    tags.year = "2024-13".into();
    let error = write_tags(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        &track.key,
        tags,
    )
    .unwrap_err();
    assert!(error.contains("年份格式无效"), "{error}");
    assert_eq!(std::fs::read(&audio).unwrap(), original);

    let mut tags = update(CoverChange::Keep);
    tags.track_number = Some(0);
    assert!(write_tags(
        &fixture.store,
        &fixture.covers,
        &fixture.fallback,
        &track.key,
        tags,
    )
    .unwrap_err()
    .contains("必须大于 0"));
    assert_eq!(std::fs::read(audio).unwrap(), original);
}

trait SourcePath {
    fn source_path(&self) -> &str;
}

impl SourcePath for Track {
    fn source_path(&self) -> &str {
        match &self.source {
            TrackSource::Local { path, .. } => path,
            TrackSource::Online { .. } => "",
        }
    }
}
