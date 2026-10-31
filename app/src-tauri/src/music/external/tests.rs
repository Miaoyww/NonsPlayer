use super::*;
use crate::music::{account::SessionContext, manager::AdapterManager};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

fn source() -> SourceId {
    SourceId::try_from("fixture-radio".to_owned()).unwrap()
}
fn descriptor() -> SourceDescriptor {
    SourceDescriptor {
        source: source(),
        display_name: "外部测试来源".into(),
        description: Some("验证通用音源能力的测试平台".into()),
        platforms: [source(), SourceId::try_from("netease".to_owned()).unwrap()].into(),
        contract_version: 1,
        login_methods: Vec::new(),
        cover_hosts: Default::default(),
        share_link_templates: Default::default(),
        capabilities: [Capability::Search, Capability::Browse]
            .into_iter()
            .collect(),
    }
}
fn path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/music-fixture/backend/target/wasm32-unknown-unknown/release/music_fixture.wasm")
}
// Tests share the process-wide compilation budget; serialize fixture loading only.
static LOADS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
async fn load(expected: SourceDescriptor) -> MusicResult<ExternalAdapter> {
    let _guard = LOADS.lock().await;
    ExternalAdapter::load(&path(), expected).await
}

#[tokio::test]
async fn external_loading_rejects_overload_oversized_files_and_invalid_components() {
    let _guard = LOADS.lock().await;
    let one = COMPILATIONS.try_acquire().unwrap();
    let two = COMPILATIONS.try_acquire().unwrap();
    assert_eq!(
        ExternalAdapter::load(&path(), descriptor())
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::RateLimited
    );
    drop((one, two));
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file()
        .set_len(runtime::Budget::ADAPTER.component_bytes as u64 + 1)
        .unwrap();
    assert_eq!(
        ExternalAdapter::load(file.path(), descriptor())
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::InvalidData
    );
    file.as_file().set_len(0).unwrap();
    assert_eq!(
        ExternalAdapter::load(file.path(), descriptor())
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::InvalidData
    );
}
fn reference(id: &str) -> EntityRef {
    EntityRef {
        source: source(),
        kind: EntityKind::Track,
        id: OpaqueId::try_from(id.to_owned()).unwrap(),
    }
}
async fn setup() -> (Arc<AdapterManager>, Arc<AtomicU64>, Arc<ExternalAdapter>) {
    let adapter = Arc::new(
        load(descriptor())
            .await
            .expect("Build music fixture before running tests"),
    );
    let manager = Arc::new(AdapterManager::default());
    let session = Arc::new(AtomicU64::new(1));
    let generation = session.clone();
    manager
        .register(
            adapter.clone(),
            Arc::new(move || SessionContext {
                source: source(),
                account: Some(
                    OpaqueId::try_from(format!("account:{}", generation.load(Ordering::Acquire)))
                        .unwrap(),
                ),
                generation: generation.load(Ordering::Acquire),
            }),
        )
        .unwrap();
    (manager, session, adapter)
}
fn search(cursor: Option<Cursor>) -> BusinessRequest {
    BusinessRequest::Search {
        keyword: "fixture".into(),
        kind: EntityKind::Track,
        page: PageRequest { cursor, limit: 1 },
    }
}
fn resolve(id: &str) -> ResolveRequest {
    ResolveRequest {
        track: reference(id),
        preferred_quality: "fixture".into(),
        allow_downgrade: false,
    }
}

#[tokio::test]
async fn real_component_declared_platform_audio_uses_unified_dispatch_without_changing_identity() {
    let (manager, session, _) = setup().await;
    let mut track = manager
        .read_track(&reference("foreign-song"))
        .await
        .unwrap();
    track.reference.source = SourceId::try_from("netease".to_owned()).unwrap();
    let request = crate::music::provider::ResourceRequest {
        track,
        preferred_quality: "fixture-cross".into(),
        allow_downgrade: false,
    };
    let (ticket, attribution) = manager
        .resolve_with_providers(&request, None, 7)
        .await
        .unwrap();
    assert_eq!(attribution.provider, "fixture-radio");
    assert_eq!(attribution.track, request.track.reference);
    assert_eq!(attribution.platform.as_str(), "netease");
    assert_eq!(ticket.quality(), "fixture-cross");
    assert!(ticket.check(7).is_ok());
    session.fetch_add(1, Ordering::AcqRel);
    assert_eq!(ticket.check(7).unwrap_err().code, ErrorCode::StaleContext);
    let mut unsupported = request;
    unsupported.track.reference.source = SourceId::try_from("unknown-platform".to_owned()).unwrap();
    assert!(manager
        .resolve_with_providers(&unsupported, None, 7)
        .await
        .is_err());
}

#[tokio::test]
async fn real_component_routes_uniform_queue_track_through_playback_service() {
    let (manager, session, _) = setup().await;
    let reference = reference("音乐:α/001");
    let music = manager.read_track(&reference).await.unwrap();
    let track = crate::model::Track::from_music(music).unwrap();
    let key = track.key.clone();
    let service = crate::music::service::MusicService::new(manager.clone());
    let (resolved, ticket, _) = service.resolve(track, "fixture", false, 11).await.unwrap();
    assert_eq!(resolved.track.key, key);
    assert_eq!(resolved.track.source_id(), "fixture-radio");
    let ticket = ticket.unwrap();
    ticket.check(11).unwrap();
    session.fetch_add(1, Ordering::AcqRel);
    assert_eq!(ticket.check(11).unwrap_err().code, ErrorCode::StaleContext);
    // Revocation does not delete the stable queue identity.
    assert_eq!(resolved.track.key, key);
}

#[tokio::test]
async fn real_component_opaque_ids_two_pages_missing_capability_and_account_switch() {
    let (manager, session, _) = setup().await;
    let track = manager.read_track(&reference("音乐:α/001")).await.unwrap();
    assert_eq!(track.reference.id.as_str(), "音乐:α/001");
    let BusinessResponse::Tracks(first) = manager.business(&source(), &search(None)).await.unwrap()
    else {
        panic!()
    };
    let cursor = first.page.next_cursor.unwrap();
    let BusinessResponse::Tracks(last) = manager
        .business(&source(), &search(Some(cursor.clone())))
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(last.page.items[0].reference.id.as_str(), "广播:β/last");
    assert!(last.page.next_cursor.is_none());
    assert_eq!(
        manager
            .business(&source(), &BusinessRequest::Favorites)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unsupported
    );
    session.fetch_add(1, Ordering::AcqRel);
    assert_eq!(
        manager
            .business(&source(), &search(Some(cursor)))
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleContext
    );
    assert_eq!(
        manager
            .read_track(&reference("wrong-source"))
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidData
    );
    assert_eq!(
        manager
            .read_track(&reference("host-denied"))
            .await
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
}

#[tokio::test]
async fn real_component_resources_expiry_preview_headers_and_disable_recovery() {
    let (manager, session, _) = setup().await;
    for (id, code) in [
        ("expired", ErrorCode::DeadlineExceeded),
        ("preview", ErrorCode::PermissionDenied),
    ] {
        assert_eq!(
            manager
                .resolve_playback(&resolve(id), 7)
                .await
                .unwrap_err()
                .code,
            code
        );
    }
    let with_headers = manager
        .resolve_playback(&resolve("headers"), 7)
        .await
        .unwrap();
    assert_eq!(with_headers.headers().len(), 1);
    with_headers.check(7).unwrap();
    assert!(!format!("{with_headers:?}").contains("private"));
    let resource = manager
        .resolve_playback(&resolve("音乐:α/001"), 7)
        .await
        .unwrap();
    assert!(!format!("{resource:?}").contains("private"));
    session.fetch_add(1, Ordering::AcqRel);
    assert_eq!(resource.check(7).unwrap_err().code, ErrorCode::StaleContext);
    let resource = manager
        .resolve_playback(&resolve("音乐:α/001"), 7)
        .await
        .unwrap();
    manager.set_enabled(&source(), false).unwrap();
    assert_eq!(
        resource.check(7).unwrap_err().code,
        ErrorCode::SourceUnavailable
    );
    assert_eq!(
        manager
            .read_track(&reference("音乐:α/001"))
            .await
            .unwrap_err()
            .code,
        ErrorCode::SourceUnavailable
    );
    manager.set_enabled(&source(), true).unwrap();
    manager.read_track(&reference("音乐:α/001")).await.unwrap();
    assert_eq!(resource.check(7).unwrap_err().code, ErrorCode::StaleContext);
    let resource = manager
        .resolve_playback(&resolve("音乐:α/001"), 7)
        .await
        .unwrap();
    manager.reload(&source()).unwrap();
    assert_eq!(resource.check(7).unwrap_err().code, ErrorCode::StaleContext);
}

#[tokio::test]
async fn real_component_faults_are_bounded_and_do_not_poison_next_request() {
    let (manager, _, _) = setup().await;
    for id in ["loop", "memory", "trap", "invalid-json", "oversized"] {
        assert_eq!(
            manager.read_track(&reference(id)).await.unwrap_err().code,
            if id == "loop" {
                ErrorCode::DeadlineExceeded
            } else {
                ErrorCode::Internal
            },
            "{id}"
        );
        manager.read_track(&reference("healthy")).await.unwrap();
    }
}

#[tokio::test]
async fn external_reserved_playback_budget_and_drop_release_stores() {
    let (manager, _, adapter) = setup().await;
    let permits = (0..4)
        .map(|_| adapter.reads.try_acquire().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        manager
            .read_track(&reference("busy"))
            .await
            .unwrap_err()
            .code,
        ErrorCode::RateLimited
    );
    manager
        .resolve_playback(&resolve("healthy"), 1)
        .await
        .unwrap();
    drop(permits);
    let context = RequestContext {
        session: manager.stamp(&source()).unwrap().0,
        adapter_generation: 1,
        deadline: Instant::now() + Duration::from_secs(3),
        cancellation: Cancellation::default(),
    };
    let reference = reference("loop");
    let mut future = adapter.read_track(&reference, &context);
    // Poll once, then cancel during guest execution or initialization; Store and permit drop.
    std::future::poll_fn(|cx| {
        let _ = future.as_mut().poll(cx);
        std::task::Poll::Ready(())
    })
    .await;
    drop(future);
    assert_eq!(adapter.reads.available_permits(), 4);
    manager
        .read_track(&self::reference("healthy"))
        .await
        .unwrap();
}

#[tokio::test]
async fn external_descriptor_version_and_identity_are_verified_before_registration() {
    let mut wrong = descriptor();
    wrong.source = SourceId::try_from("other".to_owned()).unwrap();
    assert_eq!(
        load(wrong).await.err().unwrap().code,
        ErrorCode::InvalidData
    );
    let mut wrong = descriptor();
    wrong.contract_version = 2;
    assert_eq!(
        load(wrong).await.err().unwrap().code,
        ErrorCode::Unsupported
    );
    let mut wrong = descriptor();
    wrong.capabilities.insert(Capability::Account);
    assert_eq!(
        load(wrong).await.err().unwrap().code,
        ErrorCode::Unsupported
    );
}
