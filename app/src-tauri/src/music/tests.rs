use super::{account::*, adapter::*, identity::*, migration::*, resource::*, *};
use serde_json::json;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

fn source(value: &str) -> SourceId {
    SourceId::try_from(value.to_owned()).unwrap()
}
fn id(value: &str) -> OpaqueId {
    OpaqueId::try_from(value.to_owned()).unwrap()
}
fn track_ref(provider: &str, value: &str) -> EntityRef {
    EntityRef {
        source: source(provider),
        kind: EntityKind::Track,
        id: id(value),
    }
}

#[test]
fn uniform_tracks_survive_real_storage_and_mixed_queue_restore() {
    use crate::model::{PlayerSnapshot, Track, TrackSource};
    let music = |provider: &str, value: &str| MusicTrack {
        unavailable_reason: (provider == "qqmusic").then_some(ErrorCode::NotFound),
        reference: track_ref(provider, value),
        title: "Same song".into(),
        aliases: vec![],
        artist: "Singer".into(),
        artists: vec![MusicCredit {
            name: "Singer".into(),
            reference: Some(EntityRef {
                source: source(provider),
                kind: EntityKind::Artist,
                id: id("artist/雪"),
            }),
        }],
        album: "Album".into(),
        album_reference: Some(EntityRef {
            source: source(provider),
            kind: EntityKind::Album,
            id: id("album:雪"),
        }),
        duration_ms: 1000,
        cover: Default::default(),
        associations: vec![track_ref("netease", "123")],
    };
    let a = Track::from_music(music("qqmusic", "same/雪:1")).unwrap();
    let b = Track::from_music(music("other", "same/雪:1")).unwrap();
    assert_ne!(a.key, b.key);
    assert_eq!(a.source_id(), "qqmusic");
    let old: Track =
        serde_json::from_value(legacy(json!({"kind":"netease","id":1}), "netease:1")).unwrap();
    let local: Track = serde_json::from_value(legacy(
        json!({"kind":"local","path":"music.wav","neteaseId":null}),
        "local:old",
    ))
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("library.sqlite3");
    let store = crate::storage::Store::open(&path).unwrap();
    let queue = vec![old.clone(), a.clone(), local, b.clone()];
    store.save_tracks(&queue).unwrap();
    let saved = PlayerSnapshot {
        queue: queue.into(),
        index: Some(1),
        position_ms: 400,
        ..Default::default()
    };
    store.save_player(&saved).unwrap();
    drop(store);
    let restored = crate::storage::Store::open(&path).unwrap();
    let track = restored.track(&a.key).unwrap();
    assert_eq!(track.source_id(), "qqmusic");
    let uniform = legacy_track(&serde_json::to_string(&track).unwrap())
        .unwrap()
        .track;
    assert_eq!(uniform, music("qqmusic", "same/雪:1"));
    let snapshot: PlayerSnapshot =
        serde_json::from_str(&restored.setting("player").unwrap().unwrap()).unwrap();
    assert_eq!(snapshot.queue.len(), 4);
    assert_eq!(snapshot.index, Some(1));
    assert_eq!(snapshot.position_ms, 400);
    assert!(matches!(
        snapshot.queue[0].source,
        TrackSource::Netease { id: 1 }
    ));
    assert!(matches!(
        snapshot.queue[2].source,
        TrackSource::Local { .. }
    ));
    let db = restored.0.lock().unwrap();
    let stored_source: String = db
        .query_row("SELECT source FROM tracks WHERE key=?1", [&a.key], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(stored_source, "qqmusic");
    drop(db);
    let mut invalid = a;
    invalid.key = b.key;
    assert!(restored.save_tracks(&[invalid]).is_err());
}

#[test]
fn queue_bridge_preserves_netease_legacy_keys_and_rejects_local_uniform_entries() {
    use crate::model::{Track, TrackSource};
    let mut music =
        legacy_track(&legacy(json!({"kind":"netease","id":1}), "netease:1").to_string())
            .unwrap()
            .track;
    let track = Track::from_music(music.clone()).unwrap();
    assert_eq!(track.key, "netease:1");
    let TrackSource::Online { track: canonical } = &track.source else {
        panic!("new queue entries must use the uniform DTO")
    };
    assert_eq!(canonical.reference.id.as_str(), "1");
    assert_eq!(
        canonical.artists[0].reference.as_ref().unwrap().id.as_str(),
        "12"
    );
    assert_eq!(
        canonical.album_reference.as_ref().unwrap().id.as_str(),
        "20"
    );
    assert_eq!(track.artists[0].id, None);
    assert_eq!(track.album_id, None);
    let folder = tempfile::tempdir().unwrap();
    let store = crate::storage::Store::open(&folder.path().join("library.sqlite3")).unwrap();
    let mut canonical_key = track.clone();
    canonical_key.key = music.reference.key();
    store
        .save_tracks(&[canonical_key])
        .expect("相同元数据改用标准引用编码后应能保存");
    store
        .save_tracks(&[track.clone()])
        .expect("网易云统一曲目必须能经音乐库／搜索的队列转换路径保存");
    assert_eq!(store.track(&track.key).unwrap().key, "netease:1");
    assert_eq!(
        nons_music_adapter_sdk::migration::convert_track(track.clone())
            .unwrap()
            .track,
        music
    );
    let mut wrong = track;
    wrong.key = "netease:2".into();
    assert!(store
        .save_tracks(&[wrong])
        .unwrap_err()
        .contains("rule=queue.key"));
    music.reference.source = source("local");
    music.artists.clear();
    music.album_reference = None;
    assert!(Track::from_music(music).is_err());
}
fn context() -> RequestContext {
    RequestContext {
        session: SessionContext {
            source: source("netease"),
            account: Some(id("user-A")),
            generation: 3,
        },
        adapter_generation: 7,
        deadline: Instant::now() + Duration::from_secs(10),
        cancellation: Cancellation::default(),
    }
}
fn legacy(source: serde_json::Value, key: &str) -> serde_json::Value {
    json!({"key":key,"source":source,"title":"Song","artist":"Singer","album":"Album","durationMs":1000,"cover":"", "artists":[{"name":"Singer","id":12}],"albumId":20})
}

#[test]
fn identities_preserve_opaque_ids_and_do_not_collide() {
    let a = track_ref("netease", "a:b/雪 #01");
    let b = track_ref("other", "a:b/雪 #01");
    assert_ne!(a.key(), b.key());
    assert_eq!(
        serde_json::from_str::<EntityRef>(&serde_json::to_string(&a).unwrap()).unwrap(),
        a
    );
    assert_ne!(id("01"), id("1"));
    assert_ne!(id("A"), id("a"));
    assert_eq!(id(" x ").as_str(), " x ");
    for invalid in ["", " ", "x\n", &"x".repeat(1025)] {
        assert!(serde_json::from_value::<OpaqueId>(json!(invalid)).is_err());
    }
    for invalid in ["", "Netease", "foo:bar", "../provider", "1provider"] {
        assert!(serde_json::from_value::<SourceId>(json!(invalid)).is_err());
    }
    assert!(serde_json::from_value::<OpaqueId>(json!(42)).is_err());
}

#[test]
fn capability_and_account_restrictions_are_separate() {
    let mut descriptor = SourceDescriptor {
        source: source("netease"),
        display_name: "网易云".into(),
        description: None,
        platforms: Default::default(),
        contract_version: CONTRACT_VERSION,
        login_methods: Vec::new(),
        cover_hosts: Default::default(),
        share_link_templates: Default::default(),
        capabilities: BTreeSet::from([Capability::Favorites]),
    };
    descriptor.validate().unwrap();
    assert_eq!(
        descriptor.availability(Capability::Search, None),
        OperationAvailability::Unavailable {
            reason: ErrorCode::Unsupported
        }
    );
    assert_eq!(
        descriptor.availability(Capability::Favorites, Some(ErrorCode::Unauthenticated)),
        OperationAvailability::Unavailable {
            reason: ErrorCode::Unauthenticated
        }
    );
    assert_eq!(
        descriptor.availability(Capability::Favorites, None),
        OperationAvailability::Available
    );
    descriptor.contract_version += 1;
    assert_eq!(
        descriptor.validate().unwrap_err().code,
        ErrorCode::Unsupported
    );
    descriptor.contract_version = CONTRACT_VERSION;
    descriptor.source = source("local");
    assert!(descriptor.validate().is_err());
}

#[test]
fn stale_scope_cancellation_and_deadlines_reject_results() {
    let mut request = context();
    request.check(&request.session, 7).unwrap();
    let mut current = request.session.clone();
    current.generation += 1;
    assert_eq!(
        request.check(&current, 7).unwrap_err().code,
        ErrorCode::StaleContext
    );
    current = request.session.clone();
    current.account = Some(id("user-B"));
    assert!(request.check(&current, 7).is_err());
    current = request.session.clone();
    current.source = source("other");
    assert!(request.check(&current, 7).is_err());
    assert!(request.check(&request.session, 8).is_err());
    assert!(request.check_reference(&track_ref("other", "1")).is_err());
    let clone = request.clone();
    clone.cancellation.cancel();
    assert_eq!(
        request.check(&request.session, 7).unwrap_err().code,
        ErrorCode::Cancelled
    );
    request.cancellation = Cancellation::default();
    request.deadline = Instant::now();
    assert_eq!(
        request.check(&request.session, 7).unwrap_err().code,
        ErrorCode::DeadlineExceeded
    );
}

#[test]
fn cursor_pagination_has_count_and_payload_bounds() {
    let request = PageRequest {
        cursor: Some(Cursor::try_from("offset=30&token=abc".to_owned()).unwrap()),
        limit: 2,
    };
    let mut page = Page {
        items: vec!["a".to_owned(), "b".into()],
        next_cursor: None,
    };
    page.validate(&request).unwrap();
    page.items.push("c".into());
    assert!(page.validate(&request).is_err());
    page.items = vec!["a".repeat(2 * 1024 * 1024)];
    assert!(page.validate(&request).is_err());
    for limit in [0, 101] {
        assert!(PageRequest {
            cursor: None,
            limit
        }
        .validate()
        .is_err());
    }
    assert!(Cursor::try_from("x".repeat(4097)).is_err());
}

#[test]
fn resource_lease_rechecks_all_generations_and_expiry() {
    let request = context();
    let mut lease = ResourceLease {
        handle: ResourceHandle::try_from("resource-123".to_owned()).unwrap(),
        context: request.clone(),
        playback_generation: 4,
        valid_until: Instant::now() + Duration::from_secs(5),
    };
    lease.check(&request.session, 7, 4).unwrap();
    assert!(lease.check(&request.session, 7, 5).is_err());
    assert!(lease.check(&request.session, 8, 4).is_err());
    let mut next_account = request.session.clone();
    next_account.generation += 1;
    assert!(lease.check(&next_account, 7, 4).is_err());
    lease.valid_until = Instant::now();
    assert_eq!(
        lease.check(&request.session, 7, 4).unwrap_err().code,
        ErrorCode::DeadlineExceeded
    );
}

#[test]
fn credentials_and_transport_debug_are_redacted_and_preview_is_explicit() {
    let credential = OpaqueCredential::new(b"secret-cookie".to_vec()).unwrap();
    assert_eq!(credential.expose(), b"secret-cookie");
    assert!(!format!("{credential:?}").contains("secret-cookie"));
    assert!(OpaqueCredential::new(vec![]).is_err());
    assert!(OpaqueCredential::new(vec![0; 65537]).is_err());
    let mut resource = PlaybackResource {
        metadata: ResourceMetadata {
            actual_quality: "lossless".into(),
            extent: PlaybackExtent::Full,
            expires_at_ms: Some(100),
        },
        access: ResourceAccess::Http(HttpAccess {
            url: "https://example.com/secret-url".into(),
            headers: vec![("Cookie".into(), "secret-cookie".into())],
        }),
    };
    assert!(!format!("{resource:?}").contains("secret-url"));
    assert!(!format!("{resource:?}").contains("secret-cookie"));
    resource.metadata.require_full(99).unwrap();
    assert_eq!(
        resource.metadata.require_full(100).unwrap_err().code,
        ErrorCode::DeadlineExceeded
    );
    resource.metadata.extent = PlaybackExtent::Preview {
        start_ms: 0,
        end_ms: 30000,
    };
    assert_eq!(
        resource.metadata.require_full(0).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
}

#[test]
fn legacy_track_migration_preserves_large_ids_and_local_identity() {
    let remote = legacy(
        json!({"kind":"netease","id":u64::MAX}),
        "netease:18446744073709551615",
    );
    let migrated = legacy_track(&remote.to_string()).unwrap();
    assert_eq!(migrated.track.reference.id.as_str(), u64::MAX.to_string());
    assert_eq!(
        migrated.track.artists[0].reference.as_ref().unwrap().kind,
        EntityKind::Artist
    );
    migrated.track.validate(&migrated.track.reference).unwrap();
    assert!(migrated.local_binding.is_none());
    let mut local = legacy(
        json!({"kind":"local","path":"C:\\Music\\Song.ncm","neteaseId":123}),
        "local:stable-hash",
    );
    local["cover"] = json!("C:\\Music\\cover.img");
    let migrated = legacy_track(&local.to_string()).unwrap();
    assert_eq!(migrated.track.reference, track_ref("local", "stable-hash"));
    assert_eq!(
        migrated.track.associations,
        vec![track_ref("netease", "123")]
    );
    assert!(migrated.track.artists[0].reference.is_none());
    assert!(migrated.track.album_reference.is_none());
    let binding = migrated.local_binding.unwrap();
    assert_eq!(binding.path, "C:\\Music\\Song.ncm");
    assert_eq!(binding.cover, "C:\\Music\\cover.img");
    let public = serde_json::to_string(&migrated.track).unwrap();
    assert!(!public.contains("Music\\\\"));
    let mut wrong = migrated.track.clone();
    wrong.reference = track_ref("other", "stable-hash");
    assert!(wrong.validate(&migrated.track.reference).is_err());
}

#[test]
fn legacy_queue_conversion_preserves_order_duplicates_and_settings() {
    let a = legacy(json!({"kind":"netease","id":1}), "netease:1");
    let b = legacy(
        json!({"kind":"local","path":"/music/b.flac","neteaseId":1}),
        "local:b",
    );
    let legacy = json!({"queue":[a.clone(),b,a], "index":1,"shuffle":true,"shuffleOrder":[1,2,0], "repeatMode":"one", "revision":88,"status":"playing","positionMs":300,"durationMs":1000,"volume":0.4,"deviceId":"device-1","actualQuality":"lossless","error":null,"mediaError":null,"privateFmSession":99});
    let first = legacy_queue(&legacy.to_string()).unwrap();
    let second = legacy_queue(&legacy.to_string()).unwrap();
    let serialized = serde_json::to_value(&first.queue).unwrap();
    assert_eq!(serialized, serde_json::to_value(&second.queue).unwrap());
    assert_eq!(first.queue.index, Some(1));
    assert_eq!(first.queue.shuffle_order, vec![1, 2, 0]);
    assert_eq!(first.queue.repeat_mode, QueueRepeatMode::Off);
    assert_eq!(first.queue.tracks[0], first.queue.tracks[2]);
    assert_eq!(first.queue.volume, 0.4);
    assert_eq!(
        first.tracks[1].local_binding.as_ref().unwrap().path,
        "/music/b.flac"
    );
    for forbidden in [
        "path",
        "privateFmSession",
        "positionMs",
        "actualQuality",
        "revision",
    ] {
        assert!(serialized.get(forbidden).is_none());
    }
    let mut invalid = legacy;
    invalid["index"] = json!(99);
    assert!(legacy_queue(&invalid.to_string()).is_err());
}

#[test]
fn failed_legacy_conversion_does_not_invent_account_or_track_ids() {
    let profile =
        r#"{"userId":123,"nickname":"Listener","avatarUrl":"https://example.com/avatar"}"#;
    assert_eq!(
        legacy_account(profile).unwrap().reference.id.as_str(),
        "123"
    );
    assert!(legacy_account(r#"{"userId":0,"nickname":"x","avatarUrl":""}"#).is_err());
    assert!(legacy_account("{}").is_err());
    assert!(
        legacy_track(&legacy(json!({"kind":"netease","id":0}), "netease:0").to_string()).is_err()
    );
    assert!(legacy_track(
        &legacy(
            json!({"kind":"local","path":"x","neteaseId":null}),
            "invalid"
        )
        .to_string()
    )
    .is_err());
}

#[test]
fn cache_identity_isolated_by_source_account_operation_and_generations() {
    let scope = CacheScope {
        source: source("netease"),
        account: Some(id("A")),
        session_generation: 1,
        adapter_generation: 1,
        operation: "track.read".into(),
        parameters: track_ref("netease", "1").key(),
    };
    let key = serde_json::to_string(&scope).unwrap();
    let mut variants = vec![scope.clone(); 6];
    variants[0].source = source("other");
    variants[1].account = Some(id("B"));
    variants[2].session_generation += 1;
    variants[3].adapter_generation += 1;
    variants[4].operation = "track.search".into();
    variants[5].parameters = track_ref("netease", "2").key();
    for variant in variants {
        assert_ne!(serde_json::to_string(&variant).unwrap(), key);
    }
}

#[test]
fn prepared_resource_lease_uses_its_own_lifetime_after_request_completion() {
    let mut request = context();
    request.deadline = Instant::now();
    let lease = ResourceLease {
        handle: ResourceHandle::try_from("prepared-resource".to_owned()).unwrap(),
        context: request.clone(),
        playback_generation: 4,
        valid_until: Instant::now() + Duration::from_secs(60),
    };
    lease
        .check(&request.session, request.adapter_generation, 4)
        .unwrap();
    request.cancellation.cancel();
    assert_eq!(
        lease
            .check(&request.session, request.adapter_generation, 4)
            .unwrap_err()
            .code,
        ErrorCode::Cancelled
    );
}
