use super::*;
use crate::music::{account::SessionContext, service::MusicService};
use identity::{EntityKind, EntityRef, MusicTrack, OpaqueId, SourceId};
use serde_json::json;

struct Fixture {
    descriptor: SourceDescriptor,
    calls: Mutex<Vec<usize>>,
    total: AtomicU64,
    blocked: AtomicBool,
    empty: AtomicBool,
    duplicate: AtomicBool,
    changed: AtomicBool,
    fail: AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
fn source() -> SourceId {
    SourceId::try_from("fixture-cache".to_owned()).unwrap()
}
fn entity() -> EntityRef {
    EntityRef {
        source: source(),
        kind: EntityKind::Playlist,
        id: OpaqueId::try_from("歌单/opaque".to_owned()).unwrap(),
    }
}
fn track(index: usize, changed: bool) -> MusicTrack {
    serde_json::from_value(json!({"reference":{"source":"fixture-cache","kind":"track","id":format!("{}:{index}",if changed {"new"}else{"original"})},"title":format!("Song {index}"),"aliases":[],"artist":"Artist","artists":[],"album":"Album","albumReference":null,"durationMs":1000,"cover":nons_music_adapter_sdk::cover::CoverImage::default(),"associations":[]})).unwrap()
}
impl MusicAdapter for Fixture {
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
        _: &'a super::super::resource::ResolveRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, super::super::resource::PlaybackResource> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn business<'a>(
        &'a self,
        request: &'a business::BusinessRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, business::BusinessResponse> {
        Box::pin(async move {
            if request.is_write() {
                if self.fail.load(Ordering::Acquire) {
                    return Err(ErrorCode::Network.into());
                }
                return Ok(business::BusinessResponse::Write(WriteImpact {
                    operations: ["tracks".to_owned()].into(),
                    entities: vec![entity()],
                }));
            }
            let business::BusinessRequest::Tracks { page, .. } = request else {
                return Err(ErrorCode::Unsupported.into());
            };
            let offset = page
                .cursor
                .as_ref()
                .map_or(0, |cursor| cursor.as_str().parse::<usize>().unwrap());
            self.calls.lock().unwrap().push(offset);
            self.entered.notify_one();
            if self.blocked.load(Ordering::Acquire) {
                self.release.notified().await;
            }
            if self.fail.load(Ordering::Acquire) {
                return Err(ErrorCode::Network.into());
            }
            let total = self.total.load(Ordering::Acquire) as usize;
            let end = (offset + usize::from(page.limit)).min(total);
            let items = if offset == 100 && self.empty.load(Ordering::Acquire) {
                vec![]
            } else {
                (offset..end)
                    .map(|index| {
                        track(
                            if index == 100 && self.duplicate.load(Ordering::Acquire) {
                                99
                            } else {
                                index
                            },
                            self.changed.load(Ordering::Acquire),
                        )
                    })
                    .collect()
            };
            Ok(business::BusinessResponse::Tracks(business::TrackPage {
                page: Page {
                    items,
                    next_cursor: (end < total).then(|| Cursor::try_from(end.to_string()).unwrap()),
                },
                total: Some(total as u64),
                description: Some("Description".into()),
            }))
        })
    }
}
struct Harness {
    _directory: tempfile::TempDir,
    store: Arc<Store>,
    fixture: Arc<Fixture>,
    session: Arc<Mutex<SessionContext>>,
    music: Arc<MusicService>,
}
fn harness(total: u64) -> Harness {
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&directory.path().join("library.sqlite3")).unwrap());
    let mut descriptor = nons_adapter_netease::NeteaseAdapter::new(Arc::new(
        nons_adapter_netease::platform::Netease::with_cookie(None),
    ))
    .descriptor()
    .clone();
    descriptor.source = source();
    descriptor.platforms = [source()].into();
    let fixture = Arc::new(Fixture {
        descriptor,
        calls: Default::default(),
        total: AtomicU64::new(total),
        blocked: Default::default(),
        empty: Default::default(),
        duplicate: Default::default(),
        changed: Default::default(),
        fail: Default::default(),
        entered: Default::default(),
        release: Default::default(),
    });
    let session = Arc::new(Mutex::new(SessionContext {
        source: source(),
        account: Some(OpaqueId::try_from("listener".to_owned()).unwrap()),
        generation: 1,
    }));
    let adapters = Arc::new(AdapterManager::default());
    let current = session.clone();
    adapters
        .register(
            fixture.clone(),
            Arc::new(move || current.lock().unwrap().clone()),
        )
        .unwrap();
    let mut music = MusicService::new(adapters.clone());
    music.collections = Some(Collections::new(store.clone(), adapters));
    Harness {
        _directory: directory,
        store,
        fixture,
        session,
        music: Arc::new(music),
    }
}
async fn browse(h: &Harness, pages: usize) -> Option<Cursor> {
    let mut cursor = None;
    for _ in 0..pages {
        let request = business::BusinessRequest::Tracks {
            entity: entity(),
            page: PageRequest { cursor, limit: 100 },
        };
        let business::BusinessResponse::Tracks(page) =
            h.music.query(&source(), &request, false).await.unwrap()
        else {
            panic!()
        };
        cursor = page.page.next_cursor;
    }
    cursor
}

#[tokio::test]
async fn clearing_playlist_cache_preserves_tracks_and_rejects_old_cursors_and_loads() {
    let h = harness(200);
    let cursor = browse(&h, 1).await.unwrap();
    let collections = h.music.collections.as_ref().unwrap();
    assert_eq!(collections.cache_status().await.unwrap().entries, 1);
    assert!(collections.cache_status().await.unwrap().used_bytes > 0);
    h.fixture.blocked.store(true, Ordering::Release);
    let music = h.music.clone();
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest {
            cursor: Some(cursor.clone()),
            limit: 100,
        },
    };
    // Consume the notification left by the first page before observing continuation.
    h.fixture.entered.notified().await;
    let continuation = tokio::spawn(async move { music.query(&source(), &request, false).await });
    h.fixture.entered.notified().await;
    collections.clear_cache().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), continuation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.unwrap_err().code, ErrorCode::StaleContext);
    assert_eq!(collections.cache_status().await.unwrap().entries, 0);
    assert_eq!(collections.cache_status().await.unwrap().used_bytes, 0);
    assert_eq!(
        h.store
            .0
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        100
    );
    h.fixture.blocked.store(false, Ordering::Release);
    let error = h
        .music
        .query(
            &source(),
            &business::BusinessRequest::Tracks {
                entity: entity(),
                page: PageRequest {
                    cursor: Some(cursor),
                    limit: 100,
                },
            },
            false,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleContext);
    browse(&h, 1).await;
    assert_eq!(collections.cache_status().await.unwrap().entries, 1);
}

#[tokio::test]
async fn cached_thousand_track_queue_uses_no_adapter_calls() {
    let h = harness(1500);
    browse(&h, 10).await;
    h.fixture.calls.lock().unwrap().clear();
    let cached = h.music.cached_collection(&entity()).await.unwrap().unwrap();
    assert!(cached.ready);
    assert!(cached.more);
    assert_eq!(cached.tracks.len(), 1000);
    let queue = h.music.collection_queue(&entity()).await.unwrap();
    assert_eq!(queue.tracks.len(), 1000);
    assert!(queue.more);
    assert!(h.fixture.calls.lock().unwrap().is_empty());
    assert_eq!(queue.tracks[999].title, "Song 999");
}

#[tokio::test]
async fn query_diagnostics_identify_cursor_and_snapshot_failures() {
    use crate::infrastructure::diagnostics::test_log;
    let h = harness(10);
    browse(&h, 1).await;
    let cache = h.music.collections.as_ref().unwrap();
    let request = |cursor| PageRequest { cursor, limit: 100 };
    for (cursor, refresh, stage, reason) in [
        (
            Some(Cursor::try_from("not-json token=SECRET".to_owned()).unwrap()),
            false,
            "cursor_decode",
            "snapshot_cursor_malformed",
        ),
        (
            Some(Cursor::try_from("{}".to_owned()).unwrap()),
            false,
            "cursor_decode",
            "snapshot_cursor_malformed",
        ),
    ] {
        test_log::start();
        let error = cache
            .query(&entity(), &request(cursor), refresh)
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidData);
        let logs = test_log::take().join("\n");
        assert!(logs.contains("operation=tracks"), "{logs}");
        assert!(logs.contains("route=collection_query"), "{logs}");
        assert!(logs.contains("request="), "{logs}");
        assert!(logs.contains(&format!("stage={stage}")), "{logs}");
        assert!(logs.contains(&format!("reason={reason}")), "{logs}");
        assert!(logs.contains("source=fixture-cache"), "{logs}");
        assert!(
            !logs.contains("SECRET") && !logs.contains("not-json"),
            "{logs}"
        );
    }
    cache.query(&entity(), &request(None), false).await.unwrap();
    test_log::start();
    let error = h
        .music
        .query(
            &SourceId::try_from("different-source".to_owned()).unwrap(),
            &business::BusinessRequest::Tracks {
                entity: entity(),
                page: request(None),
            },
            false,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidData);
    let logs = test_log::take().join("\n");
    assert!(logs.contains("stage=request_source"), "{logs}");
    assert!(logs.contains("reason=entity_source_mismatch"), "{logs}");
    h.store
        .0
        .lock()
        .unwrap()
        .execute("DELETE FROM tracks", [])
        .unwrap();
    test_log::start();
    let error = cache
        .query(&entity(), &request(None), false)
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidData);
    let logs = test_log::take().join("\n");
    assert!(logs.contains("stage=snapshot_tracks"), "{logs}");
    assert!(
        logs.contains("reason=snapshot_track_count_mismatch"),
        "{logs}"
    );
    assert!(logs.contains("expected=10 actual=0"), "{logs}");
}

#[tokio::test]
async fn cached_three_hundred_are_available_before_only_missing_pages_are_fetched() {
    let h = harness(1500);
    browse(&h, 3).await;
    h.fixture.calls.lock().unwrap().clear();
    let cached = h.music.cached_collection(&entity()).await.unwrap().unwrap();
    assert_eq!(cached.tracks.len(), 300);
    assert!(!cached.ready);
    assert!(h.fixture.calls.lock().unwrap().is_empty());
    assert_eq!(
        h.music
            .collection_queue(&entity())
            .await
            .unwrap()
            .tracks
            .len(),
        1000
    );
    assert_eq!(
        *h.fixture.calls.lock().unwrap(),
        (300..1000).step_by(100).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn invalid_cached_metadata_refetches_the_playlist_without_converting_old_keys() {
    use crate::infrastructure::diagnostics::test_log;
    let h = harness(10);
    browse(&h, 1).await;
    h.fixture.calls.lock().unwrap().clear();
    h.store.0.lock().unwrap().execute_batch(
        "UPDATE tracks SET key='fixture-cache:old-key',value=json_set(value,'$.key','fixture-cache:old-key') WHERE key=(SELECT key FROM music_collection_tracks WHERE position=0 LIMIT 1);
         UPDATE music_collection_tracks SET key='fixture-cache:old-key' WHERE position=0;"
    ).unwrap();
    test_log::start();
    let response = h
        .music
        .query(
            &source(),
            &business::BusinessRequest::Tracks {
                entity: entity(),
                page: PageRequest {
                    cursor: None,
                    limit: 100,
                },
            },
            false,
        )
        .await
        .unwrap();
    let logs = test_log::take().join("\n");
    assert!(logs.contains("stage=track_metadata"), "{logs}");
    assert!(logs.contains("rule=queue.key"), "{logs}");
    let business::BusinessResponse::Tracks(page) = response else {
        panic!()
    };
    assert_eq!(page.page.items.len(), 10);
    assert_eq!(*h.fixture.calls.lock().unwrap(), vec![0]);
    let cached = h.music.cached_collection(&entity()).await.unwrap().unwrap();
    nons_music_adapter_sdk::metadata::validate_online_queue_track(&cached.tracks[0]).unwrap();
}

#[tokio::test]
async fn invalid_cached_metadata_refresh_failure_is_bounded_and_retryable() {
    let h = harness(10);
    browse(&h, 1).await;
    h.fixture.calls.lock().unwrap().clear();
    h.store
        .0
        .lock()
        .unwrap()
        .execute(
            "UPDATE tracks SET value=json_set(value,'$.key','fixture-cache:old-key')",
            [],
        )
        .unwrap();
    h.fixture.fail.store(true, Ordering::Release);
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest {
            cursor: None,
            limit: 100,
        },
    };
    let error = h.music.query(&source(), &request, false).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Network);
    assert_eq!(*h.fixture.calls.lock().unwrap(), vec![0]);
    h.fixture.fail.store(false, Ordering::Release);
    let business::BusinessResponse::Tracks(page) =
        h.music.query(&source(), &request, false).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(page.page.items.len(), 10);
    assert_eq!(*h.fixture.calls.lock().unwrap(), vec![0, 0]);
}

#[tokio::test]
async fn invalid_cached_metadata_on_continuation_requires_a_fresh_prefix() {
    let h = harness(150);
    let cursor = browse(&h, 1).await;
    browse(&h, 2).await;
    h.fixture.calls.lock().unwrap().clear();
    h.store.0.lock().unwrap().execute(
        "UPDATE tracks SET value=json_set(value,'$.key','fixture-cache:old-key') WHERE key=(SELECT key FROM music_collection_tracks WHERE position=100 LIMIT 1)", []
    ).unwrap();
    let error = h
        .music
        .query(
            &source(),
            &business::BusinessRequest::Tracks {
                entity: entity(),
                page: PageRequest {
                    cursor: cursor.clone(),
                    limit: 100,
                },
            },
            false,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleContext);
    assert!(h.fixture.calls.lock().unwrap().is_empty());
    let cache = h.music.collections.as_ref().unwrap();
    assert!(cache.cached(&entity()).await.unwrap().is_none());
    browse(&h, 2).await;
    assert_eq!(*h.fixture.calls.lock().unwrap(), vec![0, 100]);
    let old = h
        .music
        .query(
            &source(),
            &business::BusinessRequest::Tracks {
                entity: entity(),
                page: PageRequest { cursor, limit: 100 },
            },
            false,
        )
        .await
        .unwrap_err();
    assert_eq!(old.code, ErrorCode::StaleContext);
}

#[tokio::test]
async fn browsing_and_playback_share_inflight_page_and_persist_empty_progress() {
    let h = harness(350);
    h.fixture.empty.store(true, Ordering::Release);
    let reference = entity();
    let (cursor, queue) = tokio::join!(browse(&h, 1), h.music.collection_queue(&reference));
    assert!(cursor.is_some());
    assert_eq!(queue.unwrap().tracks.len(), 250);
    assert_eq!(*h.fixture.calls.lock().unwrap(), vec![0, 100, 200, 300]);
    h.fixture.calls.lock().unwrap().clear();
    assert_eq!(
        h.music
            .collection_queue(&entity())
            .await
            .unwrap()
            .tracks
            .len(),
        250
    );
    assert!(h.fixture.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_membership_write_rolls_back_tracks_and_does_not_advance_page() {
    let h = harness(100);
    h.store.0.lock().unwrap().execute_batch("CREATE TRIGGER fail_members BEFORE INSERT ON music_collection_tracks BEGIN SELECT RAISE(ABORT,'test write failure'); END;").unwrap();
    assert!(h.music.collection_queue(&entity()).await.is_err());
    assert_eq!(
        h.store
            .0
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        h.store
            .0
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM music_collection_snapshots", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    h.store
        .0
        .lock()
        .unwrap()
        .execute_batch("DROP TRIGGER fail_members")
        .unwrap();
    assert_eq!(
        h.music
            .collection_queue(&entity())
            .await
            .unwrap()
            .tracks
            .len(),
        100
    );
    assert_eq!(*h.fixture.calls.lock().unwrap(), vec![0, 0]);
}

#[tokio::test]
async fn account_changes_disable_and_old_cursor_cannot_reuse_another_scope() {
    let h = harness(300);
    let cursor = browse(&h, 1).await;
    h.session.lock().unwrap().account =
        Some(OpaqueId::try_from("second-listener".to_owned()).unwrap());
    assert!(h
        .music
        .cached_collection(&entity())
        .await
        .unwrap()
        .is_none());
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest { cursor, limit: 100 },
    };
    assert_eq!(
        h.music
            .query(&source(), &request, false)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleContext
    );
    h.music.adapters.set_enabled(&source(), false).unwrap();
    assert!(h.music.cached_collection(&entity()).await.is_err());
}

#[tokio::test]
async fn cross_restart_returns_old_snapshot_without_waiting_for_refresh() {
    let h = harness(100);
    browse(&h, 1).await;
    let collections = Collections::new(h.store.clone(), h.music.adapters.clone());
    h.fixture.blocked.store(true, Ordering::Release);
    let cached = tokio::time::timeout(Duration::from_secs(1), collections.cached(&entity()))
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(cached.tracks.len(), 100);
    assert!(cached.ready);
    h.fixture.entered.notified().await;
    h.fixture.blocked.store(false, Ordering::Release);
    h.fixture.release.notify_one();
}

#[tokio::test]
async fn explicit_refresh_rejects_old_cursors_and_preserves_old_snapshot_on_error() {
    let h = harness(300);
    let cursor = browse(&h, 1).await;
    h.fixture.fail.store(true, Ordering::Release);
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest {
            cursor: None,
            limit: 100,
        },
    };
    assert!(h.music.query(&source(), &request, true).await.is_err());
    assert_eq!(
        h.music
            .cached_collection(&entity())
            .await
            .unwrap()
            .unwrap()
            .tracks
            .len(),
        100
    );
    let old = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest { cursor, limit: 100 },
    };
    assert_eq!(
        h.music
            .query(&source(), &old, false)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleContext
    );
}

#[tokio::test]
async fn successful_writes_invalidate_and_failed_writes_keep_snapshot() {
    let h = harness(100);
    browse(&h, 1).await;
    let request = business::BusinessRequest::UpdatePlaylist {
        playlist: entity(),
        name: "Updated".into(),
        description: "Description".into(),
    };
    h.fixture.fail.store(true, Ordering::Release);
    assert!(h.music.query(&source(), &request, false).await.is_err());
    assert!(h
        .music
        .cached_collection(&entity())
        .await
        .unwrap()
        .is_some());
    h.fixture.fail.store(false, Ordering::Release);
    h.music.query(&source(), &request, false).await.unwrap();
    assert!(h
        .music
        .cached_collection(&entity())
        .await
        .unwrap()
        .is_none());
}

#[test]
fn current_schema_initialization_is_repeatable_and_retains_current_data() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("library.sqlite3");
    {
        let store = Store::open(&path).unwrap();
        store.set_setting("current", "preserved").unwrap();
        store
            .save_tracks(&[Track::from_music(track(0, false)).unwrap()])
            .unwrap();
    }
    for _ in 0..2 {
        let store = Store::open(&path).unwrap();
        assert_eq!(
            store.setting("current").unwrap().as_deref(),
            Some("preserved")
        );
        let db = store.0.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM music_collection_snapshots", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[tokio::test]
async fn manual_refresh_cancels_older_inflight_delivery_and_storage() {
    let h = harness(100);
    h.fixture.blocked.store(true, Ordering::Release);
    let music = h.music.clone();
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest {
            cursor: None,
            limit: 100,
        },
    };
    let first_request = request.clone();
    let first = tokio::spawn(async move { music.query(&source(), &first_request, false).await });
    h.fixture.entered.notified().await;
    h.fixture.changed.store(true, Ordering::Release);
    let music = h.music.clone();
    let second = tokio::spawn(async move { music.query(&source(), &request, true).await });
    let error = tokio::time::timeout(Duration::from_secs(1), first)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleContext);
    h.fixture.blocked.store(false, Ordering::Release);
    h.fixture.release.notify_one();
    second.await.unwrap().unwrap();
    let cached = h.music.cached_collection(&entity()).await.unwrap().unwrap();
    assert!(cached.tracks[0].title.starts_with("Song"));
    assert!(cached.tracks[0].key.contains("new:"));
    let db = h.store.0.lock().unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM tracks WHERE key LIKE '%original%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn account_change_cancels_inflight_fetch_without_persisting_old_results() {
    let h = harness(100);
    h.fixture.blocked.store(true, Ordering::Release);
    let music = h.music.clone();
    let task = tokio::spawn(async move { music.collection_queue(&entity()).await });
    h.fixture.entered.notified().await;
    h.session.lock().unwrap().generation += 1;
    assert!(tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert_eq!(
        h.store
            .0
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn expired_snapshot_and_cached_pages_are_readable_while_refresh_waits() {
    let h = harness(300);
    browse(&h, 3).await;
    h.store
        .0
        .lock()
        .unwrap()
        .execute("UPDATE music_collection_snapshots SET updated=0", [])
        .unwrap();
    h.fixture.blocked.store(true, Ordering::Release);
    let cache = h.music.collections.as_ref().unwrap();
    let done = Arc::new(tokio::sync::Notify::new());
    let event = done.clone();
    cache.on_change(Arc::new(move |_| event.notify_one()));
    assert_eq!(
        cache.cached(&entity()).await.unwrap().unwrap().tracks.len(),
        300
    );
    // Let the background worker acquire its gate and wait on the fixture network.
    tokio::time::timeout(Duration::from_secs(1), async {
        while h.fixture.calls.lock().unwrap().len() <= 3 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest {
            cursor: None,
            limit: 100,
        },
    };
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        h.music.query(&source(), &request, false),
    )
    .await
    .unwrap()
    .unwrap();
    let business::BusinessResponse::Tracks(page) = result else {
        panic!()
    };
    assert_eq!(page.page.items.len(), 100);
    h.fixture.blocked.store(false, Ordering::Release);
    h.fixture.release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), done.notified())
        .await
        .unwrap();
}

#[tokio::test]
async fn membership_eviction_is_lru_and_does_not_delete_shared_track_metadata() {
    let h = harness(1);
    for index in 0..129 {
        let mut reference = entity();
        reference.id = OpaqueId::try_from(format!("playlist:{index}")).unwrap();
        let request = business::BusinessRequest::Tracks {
            entity: reference,
            page: PageRequest {
                cursor: None,
                limit: 100,
            },
        };
        h.music.query(&source(), &request, false).await.unwrap();
        if index == 0 {
            h.store
                .0
                .lock()
                .unwrap()
                .execute("UPDATE music_collection_snapshots SET accessed=0", [])
                .unwrap();
        }
    }
    let db = h.store.0.lock().unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM music_collection_snapshots", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        128
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM music_collection_snapshots WHERE entity='playlist:0'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(db.query_row("SELECT COUNT(*) FROM music_collection_tracks WHERE snapshot NOT IN (SELECT id FROM music_collection_snapshots)",[],|r|r.get::<_,i64>(0)).unwrap(),0);
}

#[tokio::test]
async fn byte_budget_does_not_evict_active_snapshot_to_admit_an_unfinished_refresh() {
    let h = harness(100);
    browse(&h, 1).await;
    h.store
        .0
        .lock()
        .unwrap()
        .execute("UPDATE music_collection_snapshots SET bytes=33554431", [])
        .unwrap();
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest {
            cursor: None,
            limit: 100,
        },
    };
    assert!(h.music.query(&source(), &request, true).await.is_err());
    assert_eq!(
        h.music
            .cached_collection(&entity())
            .await
            .unwrap()
            .unwrap()
            .tracks
            .len(),
        100
    );
    assert_eq!(
        h.store
            .0
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM music_collection_snapshots", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn materializing_a_persisted_page_is_readonly_and_keeps_the_requested_order() {
    let h = harness(100);
    browse(&h, 1).await;
    h.store.0.lock().unwrap().execute_batch("CREATE TRIGGER forbid_track_update BEFORE UPDATE ON tracks BEGIN SELECT RAISE(ABORT,'unexpected duplicate write'); END;").unwrap();
    let scope = h.music.adapters.cache_identity(&source()).unwrap();
    let tracks = h
        .music
        .collections
        .as_ref()
        .unwrap()
        .stored_tracks(&source(), &scope, vec![track(5, false), track(2, false)])
        .await
        .unwrap();
    assert_eq!(
        tracks
            .iter()
            .map(|track| track.title.as_str())
            .collect::<Vec<_>>(),
        ["Song 5", "Song 2"]
    );
}

#[tokio::test]
async fn removing_an_account_clears_only_its_membership_and_retains_tracks() {
    let h = harness(100);
    browse(&h, 1).await;
    let cache = h.music.collections.as_ref().unwrap();
    cache
        .remove_account(account::AccountRef {
            source: source(),
            id: OpaqueId::try_from("listener".to_owned()).unwrap(),
        })
        .await
        .unwrap();
    assert!(cache.cached(&entity()).await.unwrap().is_none());
    let db = h.store.0.lock().unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM music_collection_tracks", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        100
    );
}

#[tokio::test]
async fn duplicate_tracks_do_not_stall_continuation_or_change_first_occurrence_order() {
    let h = harness(350);
    h.fixture.duplicate.store(true, Ordering::Release);
    let queue = h.music.collection_queue(&entity()).await.unwrap();
    assert_eq!(queue.tracks.len(), 349);
    assert_eq!(queue.tracks[99].title, "Song 99");
    assert_eq!(queue.tracks[100].title, "Song 101");
    assert_eq!(*h.fixture.calls.lock().unwrap(), vec![0, 100, 200, 300]);
}

#[tokio::test]
async fn coordinator_eviction_cannot_resurrect_an_older_refresh_cursor() {
    let h = harness(300);
    let cursor = browse(&h, 1).await;
    let cache = h.music.collections.as_ref().unwrap();
    let request = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest {
            cursor: None,
            limit: 100,
        },
    };
    h.music.query(&source(), &request, true).await.unwrap();
    cache.states.lock().unwrap().clear();
    let old = business::BusinessRequest::Tracks {
        entity: entity(),
        page: PageRequest { cursor, limit: 100 },
    };
    assert_eq!(
        h.music
            .query(&source(), &old, false)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleContext
    );
}

#[tokio::test]
async fn restarted_partial_snapshot_continues_only_after_comparing_the_cached_prefix() {
    for changed in [false, true] {
        let h = harness(500);
        browse(&h, 3).await;
        h.fixture.changed.store(changed, Ordering::Release);
        let cache = Collections::new(h.store.clone(), h.music.adapters.clone());
        let published = Arc::new(tokio::sync::Notify::new());
        let event = published.clone();
        cache.on_change(Arc::new(move |_| event.notify_one()));
        let business::BusinessResponse::Tracks(first) = cache
            .query(
                &entity(),
                &PageRequest {
                    cursor: None,
                    limit: 100,
                },
                false,
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        let mut cursor = first.page.next_cursor;
        tokio::time::timeout(Duration::from_secs(2), published.notified())
            .await
            .unwrap();
        for _ in 0..2 {
            let business::BusinessResponse::Tracks(page) = cache
                .query(&entity(), &PageRequest { cursor, limit: 100 }, false)
                .await
                .unwrap()
            else {
                panic!()
            };
            cursor = page.page.next_cursor;
        }
        h.fixture.calls.lock().unwrap().clear();
        let result = cache
            .query(&entity(), &PageRequest { cursor, limit: 100 }, false)
            .await;
        if changed {
            assert_eq!(result.unwrap_err().code, ErrorCode::StaleContext);
        } else {
            let business::BusinessResponse::Tracks(page) = result.unwrap() else {
                panic!()
            };
            assert_eq!(page.page.items[0].title, "Song 300");
        }
        assert_eq!(
            *h.fixture.calls.lock().unwrap(),
            vec![300],
            "a revalidated prefix must not be downloaded twice"
        );
    }
}
