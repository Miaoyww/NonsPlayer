use super::*;
use crate::model::{Track, TrackSource};
use rusqlite::params;

struct Fixture {
    _root: tempfile::TempDir,
    store: Arc<Store>,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&root.path().join("library.sqlite")).unwrap());
        Self { _root: root, store }
    }

    fn artist(&self, name: &str) {
        let key = format!("local:{name}");
        let track = Track {
            key: key.clone(),
            title: "Song".into(),
            aliases: vec![],
            artist: name.into(),
            artists: vec![],
            album: "Album".into(),
            duration_ms: 1000,
            cover: "album-cover.jpg".into(),
            source: TrackSource::Local {
                path: format!("C:/Music/{name}.flac"),
                netease_id: None,
                cue: None,
            },
        };
        let db = self.store.0.lock().unwrap();
        db.execute(
            "INSERT INTO tracks(key,source,title,artist,value) VALUES(?1,'local','Song',?2,?3)",
            params![key, name, serde_json::to_string(&track).unwrap()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO local_entities(kind,id,name,cover,subtitle) VALUES('artist',?1,?1,'album-cover.jpg','')",
            [name],
        )
        .unwrap();
        db.execute(
            "INSERT INTO local_members(key,kind,id) VALUES(?1,'artist',?2)",
            params![key, name],
        )
        .unwrap();
    }

    fn record(
        &self,
        artist_id: &str,
        status: ArtistAvatarStatus,
        avatar_path: Option<&str>,
    ) -> ArtistAvatarRecord {
        ArtistAvatarRecord {
            artist_id: artist_id.into(),
            status,
            source: Some("netease".into()),
            platform_id: Some("42".into()),
            platform_name: Some(artist_id.into()),
            cover_variants: Some("{}".into()),
            cover_size: Some("exlarge".into()),
            avatar_path: avatar_path.map(str::to_owned),
            width: Some(1024),
            height: Some(1024),
            error: None,
            updated_at: 0,
        }
    }
}

#[test]
fn page_prefers_the_saved_avatar_and_falls_back_to_the_local_entity_cover() {
    let fixture = Fixture::new();
    fixture.artist("A");
    let page = fixture.store.artist_avatar_page("", 0).unwrap();
    assert_eq!(page.0[0].cover, "album-cover.jpg");
    assert_eq!(page.0[0].status, None);

    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "A",
            ArtistAvatarStatus::Auto,
            Some("C:/cache/avatars/a.img"),
        ))
        .unwrap();
    let page = fixture.store.artist_avatar_page("", 0).unwrap();
    assert_eq!(page.0[0].cover, "C:/cache/avatars/a.img");
    assert_eq!(page.0[0].status, Some(ArtistAvatarStatus::Auto));
    assert_eq!(page.0[0].avatar_size.as_deref(), Some("exlarge"));
    assert_eq!(page.0[0].width, Some(1024));

    fixture.store.set_artist_avatar_default("A").unwrap();
    let page = fixture.store.artist_avatar_page("", 0).unwrap();
    assert_eq!(page.0[0].cover, "album-cover.jpg");
    assert_eq!(page.0[0].status, Some(ArtistAvatarStatus::Default));
    assert!(page.0[0].platform_id.is_none());
    assert!(page.0[0].avatar_size.is_none());
}

#[test]
fn automatic_results_cannot_replace_manual_or_restored_artists() {
    let fixture = Fixture::new();
    fixture.artist("A");
    fixture.artist("B");
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "A",
            ArtistAvatarStatus::Manual,
            Some("C:/cache/avatars/manual.img"),
        ))
        .unwrap();
    fixture.store.set_artist_avatar_default("B").unwrap();

    assert!(!fixture
        .store
        .save_artist_avatar_auto(&fixture.record(
            "A",
            ArtistAvatarStatus::Auto,
            Some("C:/cache/avatars/auto-a.img"),
        ))
        .unwrap());
    assert!(!fixture
        .store
        .save_artist_avatar_auto(&fixture.record(
            "B",
            ArtistAvatarStatus::Auto,
            Some("C:/cache/avatars/auto-b.img"),
        ))
        .unwrap());

    let a = fixture.store.artist_avatar_record("A").unwrap().unwrap();
    assert_eq!(a.status, ArtistAvatarStatus::Manual);
    assert_eq!(
        a.avatar_path.as_deref(),
        Some("C:/cache/avatars/manual.img")
    );
    let b = fixture.store.artist_avatar_record("B").unwrap().unwrap();
    assert_eq!(b.status, ArtistAvatarStatus::Default);
    assert!(b.avatar_path.is_none());
}

#[test]
fn explicit_force_rematch_can_replace_a_locked_manual_result() {
    let fixture = Fixture::new();
    fixture.artist("A");
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "A",
            ArtistAvatarStatus::Manual,
            Some("C:/cache/avatars/manual.img"),
        ))
        .unwrap();
    let started_at = crate::storage::now_millis();
    assert!(save_auto_result(
        &fixture.store,
        &fixture.record(
            "A",
            ArtistAvatarStatus::Auto,
            Some("C:/cache/avatars/rematched.img"),
        ),
        true,
        started_at,
    )
    .unwrap());
    let value = fixture.store.artist_avatar_record("A").unwrap().unwrap();
    assert_eq!(value.status, ArtistAvatarStatus::Auto);
    assert_eq!(
        value.avatar_path.as_deref(),
        Some("C:/cache/avatars/rematched.img")
    );
}

#[test]
fn explicit_rematch_keeps_a_manual_avatar_when_the_result_is_not_automatic() {
    let fixture = Fixture::new();
    fixture.artist("A");
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "A",
            ArtistAvatarStatus::Manual,
            Some("C:/cache/avatars/manual.img"),
        ))
        .unwrap();
    let started_at = crate::storage::now_millis();
    let mut pending = fixture.record("A", ArtistAvatarStatus::Pending, None);
    pending.error = Some("找到多个候选".into());
    assert!(!save_auto_result(&fixture.store, &pending, true, started_at).unwrap());
    let value = fixture.store.artist_avatar_record("A").unwrap().unwrap();
    assert_eq!(value.status, ArtistAvatarStatus::Manual);
    assert_eq!(
        value.avatar_path.as_deref(),
        Some("C:/cache/avatars/manual.img")
    );
    assert_eq!(value.error.as_deref(), Some("找到多个候选"));
}

#[test]
fn restore_or_manual_change_during_a_job_cannot_be_overwritten() {
    let fixture = Fixture::new();
    fixture.artist("A");
    fixture.store.set_artist_avatar_default("A").unwrap();
    let started_at = crate::storage::now_millis() - 1_000;
    assert!(!save_auto_result(
        &fixture.store,
        &fixture.record(
            "A",
            ArtistAvatarStatus::Auto,
            Some("C:/cache/avatars/rematched.img"),
        ),
        true,
        started_at,
    )
    .unwrap());
    assert_eq!(
        fixture
            .store
            .artist_avatar_record("A")
            .unwrap()
            .unwrap()
            .status,
        ArtistAvatarStatus::Default
    );
}

#[test]
fn batch_targets_skip_locked_and_already_filled_artists_unless_forced() {
    let fixture = Fixture::new();
    for name in ["Auto", "Manual", "Default", "Missing", "New"] {
        fixture.artist(name);
    }
    let auto_path = fixture._root.path().join("avatars/auto.img");
    std::fs::create_dir_all(auto_path.parent().unwrap()).unwrap();
    std::fs::write(&auto_path, b"avatar").unwrap();
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "Auto",
            ArtistAvatarStatus::Auto,
            auto_path.to_str(),
        ))
        .unwrap();
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "Manual",
            ArtistAvatarStatus::Manual,
            Some("C:/cache/avatars/manual.img"),
        ))
        .unwrap();
    fixture.store.set_artist_avatar_default("Default").unwrap();
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record("Missing", ArtistAvatarStatus::NotFound, None))
        .unwrap();

    let all = fixture
        .store
        .artist_avatar_targets(&[], true, false, "")
        .unwrap();
    assert_eq!(all, ["Missing", "New"]);
    let filtered = fixture
        .store
        .artist_avatar_targets(&[], true, false, "new")
        .unwrap();
    assert_eq!(filtered, ["New"]);
    let forced = fixture
        .store
        .artist_avatar_targets(&["Manual".into(), "Default".into()], false, true, "")
        .unwrap();
    assert_eq!(forced, ["Manual", "Default"]);
    assert!(fixture
        .store
        .artist_avatar_targets(&["Missing".into()], false, false, "")
        .unwrap()
        .contains(&"Missing".to_string()));
}

#[test]
fn duplicate_job_start_is_rejected_and_status_can_restart() {
    let job = ArtistAvatarJob::default();
    assert!(!job.status().unwrap().running);
    assert!(job.begin(3, 1500).unwrap().running);
    assert!(job.begin(1, 1500).is_err());
    job.0.lock().unwrap().progress.running = false;
    let progress = job.begin(1, 500).unwrap();
    assert!(progress.running);
    assert_eq!(progress.delay_ms, 500);
}

#[test]
fn request_delay_defaults_clamps_and_roundtrips() {
    let fixture = Fixture::new();
    assert_eq!(request_delay(&fixture.store).unwrap(), DEFAULT_DELAY_MS);
    fixture.store.set_setting(DELAY_KEY, "invalid").unwrap();
    assert_eq!(request_delay(&fixture.store).unwrap(), DEFAULT_DELAY_MS);
    fixture.store.set_setting(DELAY_KEY, "9000").unwrap();
    assert_eq!(request_delay(&fixture.store).unwrap(), MAX_DELAY_MS);
    fixture.store.set_setting(DELAY_KEY, "2000").unwrap();
    assert_eq!(request_delay(&fixture.store).unwrap(), 2_000);
    assert!(validate_delay(MAX_DELAY_MS).is_ok());
    assert!(validate_delay(MAX_DELAY_MS + 1).is_err());
}

#[tokio::test]
async fn request_pacer_leaves_the_configured_interval_between_requests() {
    let mut pacer = RequestPacer::new(Duration::from_millis(100));
    let first = Instant::now();
    pacer.wait().await;
    assert!(first.elapsed() < Duration::from_millis(80));
    let second = Instant::now();
    pacer.wait().await;
    assert!(second.elapsed() >= Duration::from_millis(80));
}

#[test]
fn unreferenced_cleanup_keeps_a_file_still_used_by_the_current_artist() {
    let fixture = Fixture::new();
    fixture.artist("A");
    let avatars = fixture._root.path().join("avatars");
    std::fs::create_dir_all(&avatars).unwrap();
    let path = avatars.join("avatar.img");
    std::fs::write(&path, b"avatar").unwrap();
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "A",
            ArtistAvatarStatus::Manual,
            Some(path.to_str().unwrap()),
        ))
        .unwrap();
    remove_unreferenced_avatar(&fixture.store, &avatars, path.to_str().unwrap()).unwrap();
    assert!(path.is_file());
    fixture
        .store
        .save_artist_avatar_replace(&fixture.record(
            "A",
            ArtistAvatarStatus::Manual,
            Some("C:/cache/avatars/other.img"),
        ))
        .unwrap();
    remove_unreferenced_avatar(&fixture.store, &avatars, path.to_str().unwrap()).unwrap();
    assert!(!path.exists());
}
