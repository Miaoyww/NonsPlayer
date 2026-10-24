use super::{account::*, adapter::*, identity::*, resource::*, *};
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

fn online_track(provider: &str, value: &str) -> crate::model::Track {
    crate::model::Track::from_music(MusicTrack {
        unavailable_reason: None,
        reference: track_ref(provider, value),
        title: "Song".into(),
        aliases: vec![],
        artist: "Singer".into(),
        artists: vec![],
        album: "Album".into(),
        album_reference: None,
        duration_ms: 1000,
        cover: Default::default(),
        associations: vec![],
        lyric_identifiers: vec![],
    })
    .unwrap()
}

#[test]
fn uniform_tracks_survive_storage_and_mixed_queue_restore() {
    use crate::model::{PlayerSnapshot, TrackSource};
    let a = online_track("qqmusic", "same/雪:1");
    let b = online_track("netease", "123");
    assert_ne!(a.key, b.key);
    assert_eq!(b.key, track_ref("netease", "123").key());
    let mut local = online_track("other", "local-template");
    local.key = "local:stable".into();
    local.source = TrackSource::Local {
        path: "music.wav".into(),
        netease_id: None,
        cue: None,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("library.sqlite3");
    let store = crate::storage::Store::open(&path).unwrap();
    let queue = vec![a.clone(), local, b.clone()];
    store.save_tracks(&queue).unwrap();
    store
        .save_player(&PlayerSnapshot {
            queue: queue.into(),
            index: Some(1),
            position_ms: 400,
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let restored = crate::storage::Store::open(&path).unwrap();
    assert_eq!(restored.track(&a.key).unwrap().source_id(), "qqmusic");
    let snapshot: PlayerSnapshot =
        serde_json::from_str(&restored.setting("player").unwrap().unwrap()).unwrap();
    assert_eq!(snapshot.queue.len(), 3);
    assert_eq!(snapshot.index, Some(1));
    assert_eq!(snapshot.position_ms, 400);
    assert!(matches!(
        snapshot.queue[1].source,
        TrackSource::Local { .. }
    ));
    assert_eq!(snapshot.queue[2].key, b.key);
    let mut invalid = a;
    invalid.key = "netease:123".into();
    assert!(restored.save_tracks(&[invalid]).is_err());
}

#[test]
fn online_queue_requires_canonical_key_and_rejects_old_serialized_shapes() {
    let folder = tempfile::tempdir().unwrap();
    let store = crate::storage::Store::open(&folder.path().join("library.sqlite3")).unwrap();
    let track = online_track("netease", "1");
    store.save_tracks(&[track.clone()]).unwrap();
    let mut wrong = track;
    wrong.key = "netease:1".into();
    assert!(store
        .save_tracks(&[wrong])
        .unwrap_err()
        .contains("rule=queue.key"));
    assert!(serde_json::from_value::<crate::model::Track>(json!({"key":"netease:1","title":"Song","artist":"Singer","album":"Album","durationMs":1000,"cover":{},"source":{"kind":"netease","id":1}})).is_err());
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
