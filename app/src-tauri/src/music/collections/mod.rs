//! Shared playlist browsing and queue preparation. The host owns persistence and scopes.
use super::{
    account, adapter::*, business, identity, manager::AdapterManager, ErrorCode, MusicError,
    MusicResult,
};
use crate::{model::Track, storage::Store};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

mod diagnostics;
#[cfg(test)]
mod probe;
pub(crate) mod store;
#[cfg(test)]
mod tests;

#[derive(Clone)]
struct Key {
    identity: String,
    entity: identity::EntityRef,
    account: String,
}
#[derive(Clone, Debug)]
struct Snapshot {
    id: String,
    scope: String,
    run: String,
    total: Option<u64>,
    description: Option<String>,
    loaded: usize,
    pages: usize,
    complete: bool,
    cursor: Option<String>,
    updated: i64,
}
#[derive(Default)]
struct KeyState {
    gate: tokio::sync::Mutex<()>,
    epoch: AtomicU64,
    background: AtomicBool,
    attempted: AtomicU64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotCursor {
    snapshot: String,
    offset: usize,
    progress: usize,
    identity: String,
    scope: String,
    run: String,
    epoch: u64,
}
type Changed = Arc<dyn Fn(&identity::EntityRef) + Send + Sync>;
pub struct Collections {
    store: Arc<Store>,
    adapters: Arc<AdapterManager>,
    run: String,
    states: Mutex<HashMap<String, Arc<KeyState>>>,
    epochs: AtomicU64,
    changed: Mutex<Option<Changed>>,
}
pub struct CachedCollection {
    pub tracks: Vec<Track>,
    pub more: bool,
    pub ready: bool,
}

impl Collections {
    pub fn new(store: Arc<Store>, adapters: Arc<AdapterManager>) -> Arc<Self> {
        Arc::new(Self {
            store,
            adapters,
            run: uuid::Uuid::new_v4().to_string(),
            states: Default::default(),
            epochs: AtomicU64::new(1),
            changed: Default::default(),
        })
    }
    pub(crate) fn on_change(&self, changed: Changed) {
        *self.changed.lock().expect("collection notification") = Some(changed);
    }
    pub async fn stored_tracks(
        &self,
        source: &identity::SourceId,
        scope: &str,
        tracks: Vec<identity::MusicTrack>,
    ) -> MusicResult<Vec<Track>> {
        if tracks.len() > 100
            || tracks.iter().any(|track| track.reference.source != *source)
            || serde_json::to_vec(&tracks).map_err(store::failure)?.len() > 2 * 1024 * 1024
        {
            return Err(ErrorCode::InvalidData.into());
        }
        if self.adapters.cache_identity(source)? != scope {
            return Err(ErrorCode::StaleContext.into());
        }
        let keys = tracks
            .into_iter()
            .map(|track| Track::from_music(track).map(|track| track.key))
            .collect::<MusicResult<Vec<_>>>()?;
        let result = self
            .database(move |db| {
                if keys.is_empty() {
                    return Ok(vec![]);
                }
                let sql = format!(
                    "SELECT key,value FROM tracks WHERE key IN ({})",
                    vec!["?"; keys.len()].join(",")
                );
                let mut query = db.prepare(&sql).map_err(store::failure)?;
                let rows = query
                    .query_map(rusqlite::params_from_iter(keys.iter()), |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(store::failure)?;
                let values = rows
                    .collect::<Result<HashMap<_, _>, _>>()
                    .map_err(store::failure)?;
                keys.iter()
                    .map(|key| {
                        serde_json::from_str(values.get(key).ok_or(ErrorCode::NotFound)?)
                            .map_err(store::failure)
                    })
                    .collect()
            })
            .await?;
        if self.adapters.cache_identity(source)? != scope {
            return Err(ErrorCode::StaleContext.into());
        }
        Ok(result)
    }
    fn context(&self, entity: &identity::EntityRef) -> MusicResult<(Key, String)> {
        if entity.kind != identity::EntityKind::Playlist {
            return Err(ErrorCode::InvalidData.into());
        }
        let request = business::BusinessRequest::Tracks {
            entity: entity.clone(),
            page: PageRequest {
                cursor: None,
                limit: 100,
            },
        };
        let scope = self.adapters.authorize_business(&entity.source, &request)?;
        let account = self
            .adapters
            .stamp(&entity.source)?
            .0
            .account
            .map(|id| id.as_str().to_owned())
            .unwrap_or_default();
        let identity = serde_json::to_string(&(entity, &account)).map_err(store::failure)?;
        Ok((
            Key {
                identity,
                entity: entity.clone(),
                account,
            },
            scope,
        ))
    }
    fn state(&self, key: &Key) -> MusicResult<Arc<KeyState>> {
        let mut states = self.states.lock().map_err(store::failure)?;
        if !states.contains_key(&key.identity) && states.len() >= 256 {
            states.retain(|_, state| Arc::strong_count(state) > 1);
            if states.len() >= 256 {
                return Err(ErrorCode::RateLimited.into());
            }
        }
        Ok(states
            .entry(key.identity.clone())
            .or_insert_with(|| {
                Arc::new(KeyState {
                    epoch: AtomicU64::new(self.epochs.fetch_add(1, Ordering::AcqRel)),
                    ..Default::default()
                })
            })
            .clone())
    }
    fn check(&self, key: &Key, scope: &str, state: &KeyState, epoch: u64) -> MusicResult<()> {
        if state.epoch.load(Ordering::Acquire) != epoch
            || self.adapters.cache_identity(&key.entity.source)? != scope
        {
            return Err(ErrorCode::StaleContext.into());
        }
        Ok(())
    }
    async fn active(&self, key: &Key, scope: &str) -> MusicResult<Option<Snapshot>> {
        Ok(self
            .snapshot(key.identity.clone(), None)
            .await?
            .filter(|snapshot| snapshot.run != self.run || snapshot.scope == scope))
    }
    fn stale(&self, snapshot: &Snapshot) -> bool {
        snapshot.run != self.run
            || crate::storage::now_seconds().saturating_sub(snapshot.updated) >= 600
    }

    /// Returns cached prefix only; never blocks queue admission on a network request.
    pub async fn cached(
        self: &Arc<Self>,
        entity: &identity::EntityRef,
    ) -> MusicResult<Option<CachedCollection>> {
        let (key, scope) = self.context(entity)?;
        let state = self.state(&key)?;
        let epoch = state.epoch.load(Ordering::Acquire);
        let Some(snapshot) = self.active(&key, &scope).await? else {
            return Ok(None);
        };
        let tracks = self.tracks(snapshot.id.clone(), 0, 1000).await?;
        if tracks.len() != snapshot.loaded.min(1000) {
            return Err(ErrorCode::InvalidData.into());
        }
        self.check(&key, &scope, &state, epoch)?;
        if self.stale(&snapshot) {
            self.background(key, scope, state, snapshot.loaded.max(100));
        }
        Ok(Some(CachedCollection {
            more: !snapshot.complete || snapshot.loaded > 1000,
            ready: snapshot.complete || tracks.len() == 1000,
            tracks,
        }))
    }

    pub async fn query(
        self: &Arc<Self>,
        entity: &identity::EntityRef,
        page: &PageRequest,
        refresh: bool,
    ) -> MusicResult<business::BusinessResponse> {
        let diagnostic = diagnostics::Query::new(entity, page, refresh, "collection_query");
        let result = self.query_inner(entity, page, refresh, &diagnostic).await;
        diagnostic.finish(&result);
        result
    }

    async fn query_inner(
        self: &Arc<Self>,
        entity: &identity::EntityRef,
        page: &PageRequest,
        refresh: bool,
        diagnostic: &diagnostics::Query<'_>,
    ) -> MusicResult<business::BusinessResponse> {
        page.validate()?;
        diagnostic.stage("context", "collection_authorization_or_scope_failed");
        let (key, scope) = self.context(entity)?;
        diagnostic.stage("coordinator", "collection_coordinator_unavailable");
        let state = self.state(&key)?;
        let epoch = if refresh {
            let next = self.epochs.fetch_add(1, Ordering::AcqRel);
            state.epoch.fetch_max(next, Ordering::AcqRel);
            next
        } else {
            state.epoch.load(Ordering::Acquire)
        };
        let mut work = Box::pin(self.read(
            key.clone(),
            scope.clone(),
            state.clone(),
            epoch,
            page.clone(),
            refresh,
            diagnostic,
        ));
        let deadline = tokio::time::sleep(Duration::from_secs(12));
        tokio::pin!(deadline);
        let mut checks = tokio::time::interval(Duration::from_millis(10));
        loop {
            tokio::select! {
                biased;
                _=&mut deadline => {
                    diagnostic.details(format_args!("timeout_ms=12000"));
                    return Err(ErrorCode::DeadlineExceeded.into());
                },
                _=checks.tick()=>self.check(&key,&scope,&state,epoch).inspect_err(|_| diagnostic.stage("context_check", "collection_context_changed"))?,
                result=&mut work=> { self.check(&key,&scope,&state,epoch).inspect_err(|_| diagnostic.stage("context_check", "collection_context_changed"))?; return result; }
            }
        }
    }

    async fn read(
        self: &Arc<Self>,
        key: Key,
        scope: String,
        state: Arc<KeyState>,
        epoch: u64,
        page: PageRequest,
        refresh: bool,
        diagnostic: &diagnostics::Query<'_>,
    ) -> MusicResult<business::BusinessResponse> {
        diagnostic.stage("cursor_decode", "snapshot_cursor_malformed");
        let cursor = page
            .cursor
            .as_ref()
            .map(|cursor| {
                serde_json::from_str::<SnapshotCursor>(cursor.as_str())
                    .map_err(|_| MusicError::from(ErrorCode::InvalidData))
            })
            .transpose()?;
        if cursor.as_ref().is_some_and(|cursor| {
            cursor.identity != key.identity
                || cursor.scope != scope
                || cursor.run != self.run
                || cursor.epoch != epoch
                || cursor.offset > 100_000
        }) {
            diagnostic.stage("cursor_scope", "snapshot_cursor_context_mismatch");
            let cursor = cursor.as_ref().unwrap();
            diagnostic.details(format_args!(
                "identity_matches={} scope_matches={} run_matches={} epoch_matches={} offset={}",
                cursor.identity == key.identity,
                cursor.scope == scope,
                cursor.run == self.run,
                cursor.epoch == epoch,
                cursor.offset
            ));
            return Err(ErrorCode::StaleContext.into());
        }
        if refresh && cursor.is_some() {
            diagnostic.stage("cursor_refresh", "refresh_with_cursor");
            return Err(ErrorCode::InvalidData.into());
        }
        let offset = cursor.as_ref().map_or(0, |cursor| cursor.offset);
        diagnostic.stage("snapshot_read", "snapshot_lookup_failed");
        if !refresh {
            let cached = if let Some(cursor) = &cursor {
                self.snapshot(key.identity.clone(), Some(cursor.snapshot.clone()))
                    .await?
            } else {
                self.active(&key, &scope).await?
            };
            if let Some(cached) = cached {
                if cursor
                    .as_ref()
                    .is_some_and(|cursor| cursor.progress > cached.pages || offset > cached.loaded)
                {
                    diagnostic.stage("cursor_bounds", "cursor_exceeds_snapshot_progress");
                    diagnostic.details(format_args!(
                        "offset={offset} loaded={} progress={} pages={}",
                        cached.loaded,
                        cursor.as_ref().unwrap().progress,
                        cached.pages
                    ));
                    return Err(ErrorCode::InvalidData.into());
                }
                if offset < cached.loaded || cached.complete {
                    return self
                        .response(key, scope, state, epoch, page, offset, cached, diagnostic)
                        .await;
                }
            }
        }
        diagnostic.stage("snapshot_gate", "waiting_for_snapshot_writer");
        let _gate = state.gate.lock().await;
        diagnostic.stage("context_check", "collection_context_changed");
        self.check(&key, &scope, &state, epoch)?;
        let mut snapshot = if refresh {
            Some(
                self.refresh_locked(&key, &scope, &state, epoch, 100, diagnostic)
                    .await?,
            )
        } else if let Some(cursor) = &cursor {
            diagnostic.stage("snapshot_read", "cursor_snapshot_missing_or_unreadable");
            let found = self
                .snapshot(key.identity.clone(), Some(cursor.snapshot.clone()))
                .await?
                .ok_or(ErrorCode::StaleContext)?;
            if cursor.progress > found.pages || offset > found.loaded {
                diagnostic.stage("cursor_bounds", "cursor_exceeds_snapshot_progress");
                diagnostic.details(format_args!(
                    "offset={offset} loaded={} progress={} pages={}",
                    found.loaded, cursor.progress, found.pages
                ));
                return Err(ErrorCode::InvalidData.into());
            }
            Some(found)
        } else {
            diagnostic.stage("snapshot_read", "active_snapshot_lookup_failed");
            self.active(&key, &scope).await?
        };
        if snapshot.is_none() {
            snapshot = Some(
                self.fetch(&key, &scope, &state, epoch, None, true, diagnostic)
                    .await?,
            );
        }
        let mut snapshot = snapshot.unwrap();
        if offset >= snapshot.loaded && !snapshot.complete {
            diagnostic.stage("snapshot_read", "active_snapshot_lookup_failed");
            let active = self.active(&key, &scope).await?;
            if self.stale(&snapshot)
                || snapshot.run != self.run
                || snapshot.scope != scope
                || active
                    .as_ref()
                    .is_none_or(|active| active.id != snapshot.id)
            {
                diagnostic.stage("snapshot_tracks", "previous_snapshot_tracks_read_failed");
                let previous = self.tracks(snapshot.id.clone(), 0, snapshot.loaded).await?;
                snapshot = if let Some(mut active) = active.filter(|active| {
                    !self.stale(active) && active.run == self.run && active.scope == scope
                }) {
                    while active.loaded < offset + usize::from(page.limit) && !active.complete {
                        active = self
                            .fetch(&key, &scope, &state, epoch, Some(active), true, diagnostic)
                            .await?;
                    }
                    active
                } else {
                    self.refresh_locked(
                        &key,
                        &scope,
                        &state,
                        epoch,
                        offset + usize::from(page.limit),
                        diagnostic,
                    )
                    .await?
                };
                diagnostic.stage("snapshot_prefix", "snapshot_prefix_read_failed");
                let current = self.tracks(snapshot.id.clone(), 0, previous.len()).await?;
                if previous
                    .iter()
                    .map(|t| &t.key)
                    .ne(current.iter().map(|t| &t.key))
                {
                    diagnostic.stage("snapshot_prefix", "snapshot_prefix_changed");
                    return Err(ErrorCode::StaleContext.into());
                }
            } else {
                snapshot = self
                    .fetch(
                        &key,
                        &scope,
                        &state,
                        epoch,
                        Some(snapshot),
                        true,
                        diagnostic,
                    )
                    .await?;
            }
        }
        self.response(
            key,
            scope,
            state.clone(),
            epoch,
            page,
            offset,
            snapshot,
            diagnostic,
        )
        .await
    }

    async fn response(
        self: &Arc<Self>,
        key: Key,
        scope: String,
        state: Arc<KeyState>,
        epoch: u64,
        page: PageRequest,
        offset: usize,
        snapshot: Snapshot,
        diagnostic: &diagnostics::Query<'_>,
    ) -> MusicResult<business::BusinessResponse> {
        diagnostic.stage("snapshot_tracks", "snapshot_tracks_read_failed");
        let tracks = self
            .tracks(snapshot.id.clone(), offset, usize::from(page.limit))
            .await?;
        if tracks.len()
            != snapshot
                .loaded
                .saturating_sub(offset)
                .min(usize::from(page.limit))
        {
            diagnostic.stage("snapshot_tracks", "snapshot_track_count_mismatch");
            diagnostic.details(format_args!(
                "offset={offset} loaded={} expected={} actual={}",
                snapshot.loaded,
                snapshot
                    .loaded
                    .saturating_sub(offset)
                    .min(usize::from(page.limit)),
                tracks.len()
            ));
            return Err(ErrorCode::InvalidData.into());
        }
        let end = offset + tracks.len();
        diagnostic.stage("cursor_encode", "snapshot_cursor_encode_failed");
        let next_cursor = if end < snapshot.loaded || !snapshot.complete {
            Some(Cursor::try_from(
                serde_json::to_string(&SnapshotCursor {
                    snapshot: snapshot.id.clone(),
                    offset: end,
                    progress: snapshot.pages,
                    identity: key.identity.clone(),
                    scope: scope.clone(),
                    run: self.run.clone(),
                    epoch,
                })
                .map_err(store::failure)?,
            )?)
        } else {
            None
        };
        diagnostic.stage("track_metadata", "cached_track_metadata_invalid");
        let items = tracks
            .into_iter()
            .enumerate()
            .map(|(index, track)| {
                nons_music_adapter_sdk::metadata::queue_metadata(track)
                    .inspect_err(|_| {
                        diagnostic.details(format_args!("row_index={}", offset + index))
                    })
                    .map(|metadata| metadata.track)
            })
            .collect::<MusicResult<_>>()?;
        let result = business::BusinessResponse::Tracks(business::TrackPage {
            page: Page { items, next_cursor },
            total: snapshot.total,
            description: snapshot.description.clone(),
        });
        let request = business::BusinessRequest::Tracks {
            entity: key.entity.clone(),
            page,
        };
        diagnostic.stage("response_validate", "collection_response_contract_invalid");
        result.validate(&request, &key.entity.source)?;
        diagnostic.stage("descriptor", "source_descriptor_unavailable");
        let descriptor = self
            .adapters
            .descriptors()?
            .into_iter()
            .find(|descriptor| descriptor.source == key.entity.source)
            .ok_or(ErrorCode::SourceUnavailable)?;
        diagnostic.stage("cover_validate", "collection_cover_host_invalid");
        result.validate_cover_hosts(&descriptor.cover_hosts)?;
        diagnostic.stage("context_check", "collection_context_changed");
        self.check(&key, &scope, &state, epoch)?;
        if self.stale(&snapshot) {
            self.background(key, scope, state.clone(), snapshot.loaded.max(100));
        }
        Ok(result)
    }

    async fn fetch(
        &self,
        key: &Key,
        scope: &str,
        state: &Arc<KeyState>,
        epoch: u64,
        previous: Option<Snapshot>,
        publish: bool,
        diagnostic: &diagnostics::Query<'_>,
    ) -> MusicResult<Snapshot> {
        diagnostic.stage("context_check", "collection_context_changed");
        self.check(key, scope, state, epoch)?;
        if previous
            .as_ref()
            .is_some_and(|snapshot| snapshot.pages >= 1000)
        {
            diagnostic.stage("page_budget", "snapshot_page_limit_exceeded");
            diagnostic.details(format_args!(
                "pages={} maximum=1000",
                previous.as_ref().unwrap().pages
            ));
            return Err(ErrorCode::InvalidData.into());
        }
        diagnostic.stage("provider_cursor", "stored_provider_cursor_invalid");
        let cursor = previous
            .as_ref()
            .and_then(|snapshot| snapshot.cursor.clone())
            .map(Cursor::try_from)
            .transpose()?;
        let request = business::BusinessRequest::Tracks {
            entity: key.entity.clone(),
            page: PageRequest {
                cursor: cursor.clone(),
                limit: 100,
            },
        };
        diagnostic.stage("adapter_query", "adapter_tracks_request_failed");
        diagnostic.details(format_args!(
            "pages={} loaded={} provider_cursor_present={}",
            previous.as_ref().map_or(0, |snapshot| snapshot.pages),
            previous.as_ref().map_or(0, |snapshot| snapshot.loaded),
            cursor.is_some()
        ));
        let business::BusinessResponse::Tracks(page) =
            self.adapters.business(&key.entity.source, &request).await?
        else {
            diagnostic.stage("adapter_response", "unexpected_response_type");
            return Err(ErrorCode::InvalidData.into());
        };
        if page.page.next_cursor.is_some() && page.page.next_cursor == cursor {
            diagnostic.stage("adapter_cursor", "provider_cursor_did_not_advance");
            return Err(ErrorCode::InvalidData.into());
        }
        diagnostic.stage("context_check", "collection_context_changed");
        self.check(key, scope, state, epoch)?;
        diagnostic.stage("snapshot_append", "snapshot_append_failed");
        self.append(
            key.clone(),
            scope.to_owned(),
            state.clone(),
            epoch,
            previous,
            page,
            publish,
        )
        .await
    }

    async fn refresh_locked(
        &self,
        key: &Key,
        scope: &str,
        state: &Arc<KeyState>,
        epoch: u64,
        target: usize,
        diagnostic: &diagnostics::Query<'_>,
    ) -> MusicResult<Snapshot> {
        let mut snapshot = self
            .fetch(key, scope, state, epoch, None, false, diagnostic)
            .await?;
        while snapshot.loaded < target && !snapshot.complete {
            snapshot = self
                .fetch(key, scope, state, epoch, Some(snapshot), false, diagnostic)
                .await?;
        }
        diagnostic.stage("snapshot_promote", "snapshot_publish_failed");
        self.promote(
            key.clone(),
            scope.to_owned(),
            state.clone(),
            epoch,
            snapshot.id.clone(),
        )
        .await?;
        if let Some(changed) = self.changed.lock().map_err(store::failure)?.as_ref() {
            changed(&key.entity);
        }
        Ok(snapshot)
    }
    fn background(self: &Arc<Self>, key: Key, scope: String, state: Arc<KeyState>, target: usize) {
        let now = crate::storage::now_seconds() as u64;
        if now.saturating_sub(state.attempted.load(Ordering::Acquire)) < 30
            || state.background.swap(true, Ordering::AcqRel)
        {
            return;
        }
        state.attempted.store(now, Ordering::Release);
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let reference = key.entity.clone();
            let diagnostic = diagnostics::Query::new(
                &reference,
                &PageRequest {
                    cursor: None,
                    limit: 100,
                },
                true,
                "collection_refresh",
            );
            let epoch = state.epoch.load(Ordering::Acquire);
            let result = tokio::time::timeout(Duration::from_secs(12), async {
                diagnostic.stage("snapshot_gate", "waiting_for_snapshot_writer");
                let _gate = state.gate.lock().await;
                diagnostic.stage("context_check", "collection_context_changed");
                this.check(&key, &scope, &state, epoch)?;
                // Another reader may already have replaced the stale snapshot.
                diagnostic.stage("snapshot_read", "active_snapshot_lookup_failed");
                if let Some(active) = this.active(&key, &scope).await? {
                    if !this.stale(&active) {
                        return Ok(());
                    }
                }
                this.refresh_locked(&key, &scope, &state, epoch, target, &diagnostic)
                    .await
                    .map(|_| ())
            })
            .await;
            let result = result.unwrap_or_else(|_| Err(ErrorCode::DeadlineExceeded.into()));
            diagnostic.finish(&result);
            if result.is_err() {
                crate::infrastructure::diagnostics::warn(
                    "collection.refresh",
                    "background refresh failed; retained previous snapshot",
                );
            }
            state.background.store(false, Ordering::Release);
        });
    }
}
