use super::*;

fn id(value: &str) -> SourceId {
    SourceId::try_from(value.to_owned()).unwrap()
}
fn request() -> ResourceRequest {
    ResourceRequest {
        track: MusicTrack {
            reference: EntityRef {
                source: id("catalog"),
                kind: EntityKind::Track,
                id: OpaqueId::try_from("opaque/song".to_owned()).unwrap(),
            },
            title: "Song".into(),
            aliases: vec![],
            artist: "Artist".into(),
            artists: vec![],
            album: "Album".into(),
            album_reference: None,
            duration_ms: 180000,
            cover: Default::default(),
            associations: vec![],
            lyric_identifiers: vec![],
            unavailable_reason: Some(ErrorCode::PermissionDenied),
        },
        preferred_quality: "320k".into(),
        allow_downgrade: false,
    }
}
struct Audio {
    descriptor: ResourceProviderDescriptor,
    calls: AtomicU64,
    blocked: bool,
    wrong_platform: bool,
    preview: bool,
}
impl ResourceProvider for Audio {
    fn descriptor(&self) -> &ResourceProviderDescriptor {
        &self.descriptor
    }
    fn resolve<'a>(
        &'a self,
        _: &'a ResourceRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::AcqRel);
            if self.blocked {
                std::future::pending::<()>().await;
            }
            Ok(PlaybackResource {
                metadata: ResourceMetadata {
                    actual_quality: "320k".into(),
                    expires_at_ms: None,
                    extent: if self.preview {
                        PlaybackExtent::Preview {
                            start_ms: 0,
                            end_ms: 30000,
                        }
                    } else {
                        PlaybackExtent::Full
                    },
                },
                access: ResourceAccess::Http(HttpAccess {
                    url: "https://audio.example.com/song.mp3".into(),
                    headers: vec![],
                }),
            })
        })
    }
    fn search<'a>(
        &'a self,
        _: &'a ResourceSearchRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, Page<MusicTrack>> {
        Box::pin(async move {
            let mut track = request().track;
            if self.wrong_platform {
                track.reference.source = id("other");
            }
            Ok(Page {
                items: vec![track],
                next_cursor: Some(Cursor::try_from("opaque/next".to_owned())?),
            })
        })
    }
}
fn audio(blocked: bool, wrong_platform: bool, preview: bool) -> Arc<Audio> {
    Arc::new(Audio {
        descriptor: ResourceProviderDescriptor {
            id: id("audio-test"),
            display_name: "Audio".into(),
            description: None,
            contract_version: 1,
            platforms: [id("catalog")].into(),
            search_platforms: [id("catalog")].into(),
            cover_hosts: Default::default(),
        },
        calls: AtomicU64::new(0),
        blocked,
        wrong_platform,
        preview,
    })
}
struct Catalog {
    descriptor: SourceDescriptor,
    error: ErrorCode,
}
impl MusicAdapter for Catalog {
    fn descriptor(&self) -> &SourceDescriptor {
        &self.descriptor
    }
    fn read_track<'a>(
        &'a self,
        _: &'a EntityRef,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, MusicTrack> {
        Box::pin(async { Ok(request().track) })
    }
    fn resolve_playback<'a>(
        &'a self,
        _: &'a ResolveRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async { Err(self.error.into()) })
    }
}
fn make_manager(error: ErrorCode, audio: Arc<Audio>) -> Arc<AdapterManager> {
    let manager = Arc::new(AdapterManager::default());
    manager
        .register(
            Arc::new(Catalog {
                descriptor: SourceDescriptor {
                    source: id("catalog"),
                    display_name: "Catalog".into(),
                    description: None,
                    platforms: Default::default(),
                    contract_version: 1,
                    login_methods: vec![],
                    capabilities: Default::default(),
                    cover_hosts: Default::default(),
                    share_link_templates: Default::default(),
                },
                error,
            }),
            Arc::new(|| SessionContext {
                source: id("catalog"),
                account: None,
                generation: 0,
            }),
        )
        .unwrap();
    manager.register_resource_provider(audio).unwrap();
    manager
}

struct CatalogReplacement {
    descriptor: SourceDescriptor,
    audio: Arc<Audio>,
}
impl MusicAdapter for CatalogReplacement {
    fn descriptor(&self) -> &SourceDescriptor {
        &self.descriptor
    }
    fn read_track<'a>(
        &'a self,
        _: &'a EntityRef,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, MusicTrack> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn resolve_playback<'a>(
        &'a self,
        _: &'a ResolveRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn resolve_resource<'a>(
        &'a self,
        request: &'a ResourceRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        assert_eq!(request.track.reference.source, id("catalog"));
        assert_eq!(context.session.source, id("replacement"));
        self.audio.resolve(request, context)
    }
}
fn register_replacement(
    manager: &AdapterManager,
    audio: Arc<Audio>,
    declared: bool,
    generation: Arc<AtomicU64>,
) {
    let descriptor: SourceDescriptor = serde_json::from_value(serde_json::json!({
        "source": "replacement", "displayName": "Replacement", "contractVersion": 1,
        "capabilities": ["search", "account"], "platforms": if declared { vec!["replacement", "catalog"] } else { vec!["replacement"] }
    })).unwrap();
    manager
        .register(
            Arc::new(CatalogReplacement { descriptor, audio }),
            Arc::new(move || SessionContext {
                source: id("replacement"),
                account: None,
                generation: generation.load(Ordering::Acquire),
            }),
        )
        .unwrap();
}

#[tokio::test]
async fn full_adapter_audio_uses_the_shared_dispatcher_and_its_own_lifecycle() {
    let manager = make_manager(ErrorCode::Network, audio(false, false, false));
    manager
        .set_resource_provider_enabled(&id("audio-test"), false)
        .unwrap();
    let replacement = audio(false, false, false);
    let session = Arc::new(AtomicU64::new(1));
    register_replacement(&manager, replacement.clone(), true, session.clone());
    assert!(manager
        .supports_audio_provider("replacement", &id("catalog"))
        .unwrap());
    assert!(!manager
        .supports_audio_provider("replacement", &id("unknown"))
        .unwrap());
    let (ticket, source) = manager
        .resolve_with_providers(&request(), None, 7)
        .await
        .unwrap();
    assert_eq!(source.provider, "replacement");
    assert_eq!(source.platform, id("catalog"));
    assert_eq!(source.track, request().track.reference);
    assert_eq!(replacement.calls.load(Ordering::Acquire), 1);
    assert!(ticket.check(7).is_ok());
    session.fetch_add(1, Ordering::AcqRel);
    assert!(ticket.check(7).is_err());
    let (ticket, _) = manager
        .resolve_with_providers(&request(), Some("replacement"), 7)
        .await
        .unwrap();
    manager.reload(&id("replacement")).unwrap();
    assert!(ticket.check(7).is_err());
    manager.set_enabled(&id("replacement"), false).unwrap();
    assert!(!manager
        .supports_audio_provider("replacement", &id("catalog"))
        .unwrap());
    assert!(manager
        .resolve_with_providers(&request(), Some("replacement"), 7)
        .await
        .is_err());
}

#[tokio::test]
async fn selected_full_adapter_preview_falls_back_to_another_provider_once() {
    let fallback = audio(false, false, false);
    let manager = make_manager(ErrorCode::Network, fallback.clone());
    let replacement = audio(false, false, true);
    register_replacement(
        &manager,
        replacement.clone(),
        true,
        Arc::new(AtomicU64::new(0)),
    );
    let attempts = Mutex::new(Vec::new());
    let (_, source) = manager
        .resolve_with_providers_notifying(&request(), Some("replacement"), 7, &|source| {
            attempts.lock().unwrap().push(source.provider)
        })
        .await
        .unwrap();
    assert_eq!(source.provider, "audio-test");
    assert_eq!(
        *attempts.lock().unwrap(),
        vec!["replacement", "official", "audio-test"]
    );
    assert_eq!(replacement.calls.load(Ordering::Acquire), 1);
    assert_eq!(fallback.calls.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn undeclared_full_adapters_are_never_used_and_revocation_cancels_cross_platform_audio() {
    let manager = make_manager(ErrorCode::Network, audio(false, false, false));
    let replacement = audio(false, false, false);
    register_replacement(
        &manager,
        replacement.clone(),
        false,
        Arc::new(AtomicU64::new(0)),
    );
    let (_, source) = manager
        .resolve_with_providers(&request(), None, 7)
        .await
        .unwrap();
    assert_eq!(source.provider, "audio-test");
    assert_eq!(replacement.calls.load(Ordering::Acquire), 0);
    let manager = make_manager(ErrorCode::Network, audio(false, false, false));
    manager
        .set_resource_provider_enabled(&id("audio-test"), false)
        .unwrap();
    let replacement = audio(true, false, false);
    register_replacement(
        &manager,
        replacement.clone(),
        true,
        Arc::new(AtomicU64::new(0)),
    );
    let running = manager.clone();
    let task =
        tokio::spawn(async move { running.resolve_with_providers(&request(), None, 7).await });
    while replacement.calls.load(Ordering::Acquire) == 0 {
        tokio::task::yield_now().await;
    }
    manager.set_enabled(&id("catalog"), false).unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
}
#[test]
fn provider_loading_failure_and_disabled_registration_do_not_create_a_catalog_source() {
    let manager = AdapterManager::default();
    let audio = audio(false, false, false);
    manager
        .begin_resource_loading(audio.descriptor.clone())
        .unwrap();
    assert!(manager.resource_provider_statuses().unwrap()[0].loading);
    manager.resource_loading_failed(&id("audio-test"), "compile failed");
    assert!(!manager.resource_provider_statuses().unwrap()[0].loading);
    assert!(manager.resource_provider_statuses().unwrap()[0]
        .load_error
        .is_some());
    manager
        .register_resource_provider_with_enabled(audio, false)
        .unwrap();
    let status = &manager.resource_provider_statuses().unwrap()[0];
    assert!(!status.enabled && !status.loading && status.load_error.is_none());
    assert!(manager.descriptors().unwrap().is_empty());
}
#[tokio::test]
async fn explicit_provider_precedes_catalog_and_provider_only_platform_is_playable() {
    let manager = make_manager(ErrorCode::Cancelled, audio(false, false, false));
    assert_eq!(
        manager
            .resolve_with_providers(&request(), Some("audio-test"), 1)
            .await
            .unwrap()
            .1
            .provider,
        "audio-test"
    );
    let manager = AdapterManager::default();
    manager
        .register_resource_provider(audio(false, false, false))
        .unwrap();
    assert_eq!(
        manager
            .resolve_with_providers(&request(), None, 1)
            .await
            .unwrap()
            .1
            .track,
        request().track.reference
    );
}
#[tokio::test]
async fn every_operational_failure_can_fallback_without_changing_track_identity() {
    for error in [
        ErrorCode::PermissionDenied,
        ErrorCode::Network,
        ErrorCode::NotFound,
        ErrorCode::DeadlineExceeded,
        ErrorCode::InvalidData,
        ErrorCode::RateLimited,
    ] {
        let audio = audio(false, false, false);
        let manager = make_manager(error, audio.clone());
        let attempts = Mutex::new(Vec::new());
        let (ticket, source) = manager
            .resolve_with_providers_notifying(&request(), None, 7, &|source| {
                attempts.lock().unwrap().push(source.provider)
            })
            .await
            .unwrap();
        assert_eq!(source.track, request().track.reference);
        assert_eq!(source.provider, "audio-test");
        assert!(source.changed);
        assert_eq!(*attempts.lock().unwrap(), vec!["official", "audio-test"]);
        assert_eq!(audio.calls.load(Ordering::Acquire), 1);
        assert_eq!(manager.descriptors().unwrap().len(), 1);
        assert_eq!(manager.resource_provider_statuses().unwrap().len(), 1);
        assert!(ticket.check(7).is_ok());
        assert_eq!(ticket.check(8).unwrap_err().code, ErrorCode::StaleContext);
        manager
            .set_resource_provider_enabled(&id("audio-test"), false)
            .unwrap();
        assert!(ticket.check(7).is_err());
    }
}
#[tokio::test]
async fn cancellation_stale_scope_and_preview_never_become_playable_fallbacks() {
    for error in [ErrorCode::Cancelled, ErrorCode::StaleContext] {
        let audio = audio(false, false, false);
        let manager = make_manager(error, audio.clone());
        assert_eq!(
            manager
                .resolve_with_providers(&request(), None, 7)
                .await
                .unwrap_err()
                .code,
            error
        );
        assert_eq!(audio.calls.load(Ordering::Acquire), 0);
    }
    let manager = make_manager(ErrorCode::Network, audio(false, false, true));
    assert_eq!(
        manager
            .resolve_with_providers(&request(), None, 7)
            .await
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
}
#[tokio::test]
async fn platform_revocation_cancels_inflight_provider_and_invalidates_its_lease() {
    let audio = audio(true, false, false);
    let manager = make_manager(ErrorCode::PermissionDenied, audio.clone());
    let running = manager.clone();
    let task =
        tokio::spawn(async move { running.resolve_with_providers(&request(), None, 7).await });
    while audio.calls.load(Ordering::Acquire) == 0 {
        tokio::task::yield_now().await;
    }
    manager.set_enabled(&id("catalog"), false).unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
}
#[tokio::test]
async fn search_preserves_platform_and_binds_cursor_to_query_and_instance() {
    let manager = make_manager(ErrorCode::Network, audio(false, false, false));
    let mut search = ResourceSearchRequest {
        platform: id("catalog"),
        keyword: "Song".into(),
        page: PageRequest {
            cursor: None,
            limit: 20,
        },
    };
    let page = manager
        .search_resource_provider(&id("audio-test"), &search)
        .await
        .unwrap();
    assert_eq!(page.items[0].reference.source, id("catalog"));
    search.page.cursor = page.next_cursor;
    search.keyword = "Different".into();
    assert_eq!(
        manager
            .search_resource_provider(&id("audio-test"), &search)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleContext
    );
    let manager = make_manager(ErrorCode::Network, audio(false, true, false));
    search.page.cursor = None;
    assert_eq!(
        manager
            .search_resource_provider(&id("audio-test"), &search)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidData
    );
}
