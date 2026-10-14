use super::capability::MAX_JSON;
use crate::model::{AppResult, Track};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

type CacheStamp = String;
type CacheEntry = (CacheStamp, u64, Instant, crate::music::identity::MusicTrack);

pub struct SongService {
    generation: AtomicU64,
    pub(super) music: Arc<crate::music::service::MusicService>,
    cache: tokio::sync::Mutex<HashMap<crate::music::identity::EntityRef, CacheEntry>>,
}
impl SongService {
    pub(super) async fn account_credentials(&self) -> AppResult<Value> {
        let stamp = self.music.legacy_stamp()?;
        let accounts = self.music.accounts.as_ref().ok_or("账号管理不可用")?;
        let source = stamp.0.source.clone();
        let credential = accounts
            .blocking(move |accounts| accounts.provider_credential(&source))
            .await
            .map_err(|e| e.to_string())?;
        if self.music.legacy_stamp()? != stamp {
            return Err("账号或来源状态已变化".into());
        }
        let cookie = credential
            .map(|value| {
                String::from_utf8(value.expose().to_vec())
                    .map_err(|_| "兼容凭据格式无效".to_owned())
            })
            .transpose()?;
        Ok(json!({"cookie":cookie,"generation":stamp.0.generation}))
    }
    pub fn new(music: Arc<crate::music::service::MusicService>) -> Self {
        Self {
            music,
            generation: AtomicU64::new(0),
            cache: Default::default(),
        }
    }
    pub fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
    pub async fn song(&self, id: u64) -> AppResult<Value> {
        Ok(song_value(id, &self.track(id).await?))
    }
    pub(super) async fn track(&self, id: u64) -> AppResult<Track> {
        let reference = nons_adapter_netease::reference(id).map_err(|e| e.to_string())?;
        nons_adapter_netease::legacy(self.read_track(&reference).await?).map_err(|e| e.to_string())
    }
    pub async fn read_track(
        &self,
        reference: &crate::music::identity::EntityRef,
    ) -> AppResult<crate::music::identity::MusicTrack> {
        // A bounded serial fetch also coalesces duplicate concurrent requests.
        let stamp = self
            .music
            .adapters
            .cache_identity(&reference.source)
            .map_err(|e| e.to_string())?;
        let generation = self.generation.load(Ordering::SeqCst);
        let mut cache = self.cache.lock().await;
        cache.retain(|_, (entry_stamp, entry_generation, time, _)| {
            *entry_stamp == stamp
                && *entry_generation == generation
                && time.elapsed() < Duration::from_secs(600)
        });
        // Cache entries belong to the account generation in which they were fetched.
        if generation != self.generation.load(Ordering::SeqCst)
            || stamp
                != self
                    .music
                    .adapters
                    .cache_identity(&reference.source)
                    .map_err(|e| e.to_string())?
        {
            return Err("账号状态已变化".into());
        }
        if let Some((_, _, _, track)) = cache.get(reference) {
            return Ok(track.clone());
        }
        let track = tokio::time::timeout(Duration::from_secs(3), async {
            self.music
                .adapters
                .read_track(reference)
                .await
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|_| "歌曲信息查询超时")??;
        if generation != self.generation.load(Ordering::SeqCst)
            || stamp
                != self
                    .music
                    .adapters
                    .cache_identity(&reference.source)
                    .map_err(|e| e.to_string())?
        {
            cache.clear();
            return Err("账号状态已变化".into());
        }
        while cache.len() >= 128 {
            if let Some(key) = cache
                .iter()
                .min_by_key(|(_, (_, _, time, _))| *time)
                .map(|(key, _)| key.clone())
            {
                cache.remove(&key);
            }
        }
        let value = serde_json::to_value(&track).map_err(|e| e.to_string())?;
        if serde_json::to_vec(&value).map_err(|e| e.to_string())?.len() > MAX_JSON {
            return Err("歌曲信息过大".into());
        }
        cache.insert(
            reference.clone(),
            (stamp, generation, Instant::now(), track.clone()),
        );
        Ok(track)
    }
    pub async fn clear(&self) {
        self.invalidate();
        self.cache.lock().await.clear();
    }
}
fn song_value(id: u64, track: &Track) -> Value {
    json!({"id":id,"key":track.key,"title":track.title,"artist":track.artist,"artists":track.artists,"album":track.album,"durationMs":track.duration_ms,"cover":track.cover.to_string()})
}
