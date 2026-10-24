use super::*;
use crate::music::{
    account::{AccountRecord, OpaqueCredential},
    accounts::{AccountManager, AccountRecordStore, CredentialStore},
    adapter::{AdapterFuture, Cursor, MusicAdapter, Page, RequestContext, SourceDescriptor},
    business::{MusicEntity, TrackPage},
    identity::{MusicTrack, SourceId},
    manager::AdapterManager,
    resource::{PlaybackResource, ResolveRequest},
    ErrorCode, MusicResult,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[derive(Default)]
struct Records(Mutex<Option<String>>);
impl AccountRecordStore for Records {
    fn load(&self) -> MusicResult<Option<String>> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(&self, json: &str) -> MusicResult<()> {
        *self.0.lock().unwrap() = Some(json.into());
        Ok(())
    }
}
#[derive(Default)]
struct Secrets(Mutex<HashMap<String, Vec<u8>>>);
impl CredentialStore for Secrets {
    fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>> {
        Ok(self.0.lock().unwrap().get(key).cloned())
    }
    fn write(&self, key: &str, bytes: &[u8]) -> MusicResult<()> {
        self.0.lock().unwrap().insert(key.into(), bytes.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> MusicResult<()> {
        self.0.lock().unwrap().remove(key);
        Ok(())
    }
}
fn source() -> SourceId {
    "fixture-aggregate".to_owned().try_into().unwrap()
}
fn entity() -> EntityRef {
    serde_json::from_value(
        serde_json::json!({"source":"fixture-aggregate","kind":"playlist","id":"list"}),
    )
    .unwrap()
}
fn account() -> crate::music::account::AccountRef {
    serde_json::from_value(serde_json::json!({"source":"fixture-aggregate","id":"listener"}))
        .unwrap()
}
struct Fixture {
    descriptor: SourceDescriptor,
    total: AtomicU64,
    fail: AtomicBool,
    cycle: AtomicBool,
    owned: AtomicBool,
    block: AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
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
        _: &'a ResolveRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn business<'a>(
        &'a self,
        request: &'a BusinessRequest,
        _: &'a RequestContext,
    ) -> AdapterFuture<'a, BusinessResponse> {
        Box::pin(async move {
            match request {
                BusinessRequest::LibraryCollections { page, .. } => {
                    // The bundled provider accepts library pages of exactly 30.
                    assert_eq!(page.limit, 30);
                    let mut item:MusicEntity=serde_json::from_value(serde_json::json!({"reference":entity(),"name":"测试歌单","cover":nons_music_adapter_sdk::cover::CoverImage::default(),"subtitle":"","trackCount":0,"creator":account(),"liked":false,"playCount":null,"publishedAt":null,"artists":[]})).unwrap();
                    if !self.owned.load(Ordering::Acquire) {
                        item.creator = None;
                    }
                    if page.cursor.is_none() {
                        item.reference.id = "other".to_owned().try_into().unwrap();
                    }
                    Ok(BusinessResponse::Entities(Page {
                        items: vec![item],
                        next_cursor: page
                            .cursor
                            .is_none()
                            .then(|| Cursor::try_from("second".to_owned()).unwrap()),
                    }))
                }
                BusinessRequest::Tracks { page, .. } => {
                    let offset = page
                        .cursor
                        .as_ref()
                        .map_or(0, |c| c.as_str().parse::<usize>().unwrap());
                    if offset >= 100 && self.block.load(Ordering::Acquire) {
                        self.entered.notify_one();
                        self.release.notified().await;
                    }
                    if offset >= 100 && self.fail.load(Ordering::Acquire) {
                        return Err(ErrorCode::Network.into());
                    }
                    let total = self.total.load(Ordering::Acquire) as usize;
                    let end = (offset + 100).min(total);
                    let items=(offset..end).map(|i|serde_json::from_value(serde_json::json!({"reference":{"source":"fixture-aggregate","kind":"track","id":format!("曲目{}",if i==100{99}else{i})},"title":format!("歌曲{i}"),"aliases":[],"artist":"Artist","artists":[],"album":"Album","albumReference":null,"durationMs":1000,"cover":nons_music_adapter_sdk::cover::CoverImage::default(),"associations":[]})).unwrap()).collect();
                    Ok(BusinessResponse::Tracks(TrackPage {
                        page: Page {
                            items,
                            next_cursor: (end < total).then(|| {
                                Cursor::try_from(if self.cycle.load(Ordering::Acquire) {
                                    "100".into()
                                } else {
                                    end.to_string()
                                })
                                .unwrap()
                            }),
                        },
                        total: Some(total as u64),
                        description: None,
                    }))
                }
                _ => Err(ErrorCode::Unsupported.into()),
            }
        })
    }
}
fn harness() -> (Arc<MusicService>, Arc<Fixture>, Link, Arc<Job>) {
    let accounts = Arc::new(
        AccountManager::new(Arc::new(Records::default()), Arc::new(Secrets::default())).unwrap(),
    );
    accounts
        .accept(
            &accounts.session(&source()),
            AccountRecord {
                reference: account(),
                display_name: "Listener".into(),
                avatar: "".into(),
            },
            OpaqueCredential::new(b"credential".to_vec()).unwrap(),
        )
        .unwrap();
    let mut descriptor = nons_adapter_netease::NeteaseAdapter::new(Arc::new(
        nons_adapter_netease::platform::Netease::with_cookie(None),
    ))
    .descriptor()
    .clone();
    descriptor.source = source();
    descriptor.platforms = [source()].into();
    let fixture = Arc::new(Fixture {
        descriptor,
        total: AtomicU64::new(205),
        fail: AtomicBool::new(false),
        cycle: AtomicBool::new(false),
        owned: AtomicBool::new(true),
        block: AtomicBool::new(false),
        entered: Default::default(),
        release: Default::default(),
    });
    let adapters = Arc::new(AdapterManager::default());
    let sessions = accounts.clone();
    adapters
        .register(
            fixture.clone(),
            Arc::new(move || sessions.session(&source())),
        )
        .unwrap();
    accounts.bind(&adapters);
    let mut music = MusicService::new(adapters);
    music.accounts = Some(accounts);
    let link = Link {
        reference: entity(),
        account: account(),
        last_success: 0,
        last_attempt: 0,
        error: None,
    };
    let (cancel, _) = watch::channel(false);
    let job = Arc::new(Job {
        cancel,
        progress: Mutex::new(Progress {
            key: "fixture".into(),
            running: true,
            count: 0,
            playlist_id: None,
            error: None,
        }),
    });
    (Arc::new(music), fixture, link, job)
}
#[tokio::test]
async fn full_traversal_finds_second_page_owner_and_deduplicates_tracks() {
    let (m, _, l, j) = harness();
    let scope = m.adapters.cache_identity(&source()).unwrap();
    let (title, tracks) = read(&m, &l, &scope, &j).await.unwrap();
    assert_eq!(title, "测试歌单");
    assert_eq!(tracks.len(), 204);
}
#[tokio::test]
async fn partial_errors_cycles_and_unowned_never_produce_complete_candidates() {
    for mode in 0..3 {
        let (m, f, l, j) = harness();
        match mode {
            0 => f.fail.store(true, Ordering::Release),
            1 => f.cycle.store(true, Ordering::Release),
            _ => f.owned.store(false, Ordering::Release),
        }
        let scope = m.adapters.cache_identity(&source()).unwrap();
        assert!(read(&m, &l, &scope, &j).await.is_err());
    }
}
#[tokio::test]
async fn source_change_and_ownership_loss_reject_late_results() {
    for mode in 0..4 {
        let (m, f, l, j) = harness();
        f.block.store(true, Ordering::Release);
        let scope = m.adapters.cache_identity(&source()).unwrap();
        let music = m.clone();
        let task = tokio::spawn(async move { read(&music, &l, &scope, &j).await });
        f.entered.notified().await;
        if mode == 0 {
            m.adapters.set_enabled(&source(), false).unwrap()
        } else if mode == 1 {
            f.owned.store(false, Ordering::Release)
        } else if mode == 2 {
            m.accounts.as_ref().unwrap().remove(&account()).unwrap();
        } else {
            let mut other = account();
            other.id = "other".to_owned().try_into().unwrap();
            let accounts = m.accounts.as_ref().unwrap();
            accounts
                .accept(
                    &accounts.session(&source()),
                    AccountRecord {
                        reference: other,
                        display_name: "Other".into(),
                        avatar: "".into(),
                    },
                    OpaqueCredential::new(b"other".to_vec()).unwrap(),
                )
                .unwrap();
        }
        f.block.store(false, Ordering::Release);
        f.release.notify_one();
        assert!(task.await.unwrap().is_err());
    }
}

#[tokio::test]
async fn empty_oversized_cancelled_and_timed_out_reads_never_become_partial_success() {
    let (m, f, l, j) = harness();
    let scope = m.adapters.cache_identity(&source()).unwrap();
    f.total.store(0, Ordering::Release);
    assert!(read(&m, &l, &scope, &j).await.unwrap().1.is_empty());
    f.total.store(10002, Ordering::Release);
    assert!(read(&m, &l, &scope, &j).await.is_err());
    f.total.store(205, Ordering::Release);
    j.cancel.send_replace(true);
    assert!(read(&m, &l, &scope, &j).await.is_err());
    j.cancel.send_replace(false);
    f.block.store(true, Ordering::Release);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), read(&m, &l, &scope, &j))
            .await
            .is_err()
    );
}
