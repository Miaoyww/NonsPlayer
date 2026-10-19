//! Opt-in live measurement against a public playlist and an isolated temporary database.
use super::*;
use crate::music::{resource::*, service::MusicService};
use std::time::Instant;

struct Counted {
    inner: nons_adapter_netease::NeteaseAdapter,
    calls: Arc<AtomicU64>,
}
impl MusicAdapter for Counted {
    fn descriptor(&self) -> &SourceDescriptor {
        self.inner.descriptor()
    }
    fn business<'a>(
        &'a self,
        request: &'a business::BusinessRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, business::BusinessResponse> {
        if matches!(request, business::BusinessRequest::Tracks { .. }) {
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.business(request, context)
    }
    fn read_track<'a>(
        &'a self,
        reference: &'a identity::EntityRef,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, identity::MusicTrack> {
        self.inner.read_track(reference, context)
    }
    fn resolve_playback<'a>(
        &'a self,
        request: &'a ResolveRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        self.inner.resolve_playback(request, context)
    }
}
fn elapsed(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[tokio::test]
#[ignore = "Live public Netease playlist measurement; no credentials, user DB or audio playback"]
async fn live_playlist_cache_probe() {
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&directory.path().join("playlist-probe.sqlite3")).unwrap());
    let client = Arc::new(nons_adapter_netease::platform::Netease::with_cookie(None));
    let calls = Arc::new(AtomicU64::new(0));
    let adapter = Arc::new(Counted {
        inner: nons_adapter_netease::NeteaseAdapter::new(client.clone()),
        calls: calls.clone(),
    });
    let adapters = Arc::new(AdapterManager::default());
    adapters
        .register(
            adapter,
            Arc::new(move || nons_adapter_netease::session(&client)),
        )
        .unwrap();
    let mut music = MusicService::new(adapters.clone());
    music.collections = Some(Collections::new(store.clone(), adapters.clone()));
    let entity = identity::EntityRef {
        source: identity::SourceId::try_from("netease".to_owned()).unwrap(),
        kind: identity::EntityKind::Playlist,
        id: identity::OpaqueId::try_from("950080093".to_owned()).unwrap(),
    };
    let started = Instant::now();
    let mut cursor = None;
    let mut first = Vec::new();
    for _ in 0..3 {
        let request = business::BusinessRequest::Tracks {
            entity: entity.clone(),
            page: PageRequest { cursor, limit: 100 },
        };
        let business::BusinessResponse::Tracks(page) =
            music.query(&entity.source, &request, false).await.unwrap()
        else {
            panic!()
        };
        first.extend(page.page.items.iter().map(|track| track.reference.clone()));
        cursor = page.page.next_cursor;
    }
    let read_started = Instant::now();
    let partial = music.cached_collection(&entity).await.unwrap().unwrap();
    assert_eq!(partial.tracks.len(), 300);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    println!(
        "COLLECTION_PROBE {}",
        serde_json::json!({"stage":"cached_prefix","tracks":partial.tracks.len(),"read_ms":elapsed(read_started),"catalogue_calls":calls.load(Ordering::SeqCst)})
    );
    let queue = music.collection_queue(&entity).await.unwrap();
    assert_eq!(queue.tracks.len(), 1000);
    assert_eq!(calls.load(Ordering::SeqCst), 10);
    println!(
        "COLLECTION_PROBE {}",
        serde_json::json!({"stage":"first_thousand","tracks":queue.tracks.len(),"wall_ms":elapsed(started),"catalogue_calls":calls.load(Ordering::SeqCst),"missing_page_calls":7,"authenticated":false,"profile":"debug"})
    );
    let expected = queue
        .tracks
        .iter()
        .map(|track| track.key.clone())
        .collect::<Vec<_>>();
    for iteration in 0..5 {
        let before = calls.load(Ordering::SeqCst);
        let started = Instant::now();
        let cached = music.collection_queue(&entity).await.unwrap();
        let read_ms = elapsed(started);
        let build = Instant::now();
        let snapshot = crate::model::PlayerSnapshot {
            queue: cached.tracks.into(),
            index: Some(0),
            ..Default::default()
        };
        let build_ms = elapsed(build);
        assert_eq!(
            snapshot
                .queue
                .iter()
                .map(|track| track.key.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(calls.load(Ordering::SeqCst), before);
        println!(
            "COLLECTION_PROBE {}",
            serde_json::json!({"stage":"cached_queue","iteration":iteration,"read_ms":read_ms,"queue_model_build_ms":build_ms,"catalogue_calls":0})
        );
    }
    let reloaded = Collections::new(store, adapters);
    let changed = Arc::new(tokio::sync::Notify::new());
    let event = changed.clone();
    reloaded.on_change(Arc::new(move |_| event.notify_one()));
    let restart = Instant::now();
    let cached = reloaded.cached(&entity).await.unwrap().unwrap();
    assert_eq!(
        cached
            .tracks
            .iter()
            .map(|track| track.key.clone())
            .collect::<Vec<_>>(),
        expected
    );
    println!(
        "COLLECTION_PROBE {}",
        serde_json::json!({"stage":"restart_cached_prefix","tracks":cached.tracks.len(),"read_ms":elapsed(restart),"background_refresh":true})
    );
    tokio::time::timeout(Duration::from_secs(15), changed.notified())
        .await
        .expect("background refresh should complete");
    println!(
        "COLLECTION_PROBE {}",
        serde_json::json!({"stage":"restart_refresh_complete","catalogue_calls_total":calls.load(Ordering::SeqCst),"elapsed_ms":elapsed(restart)})
    );
}
