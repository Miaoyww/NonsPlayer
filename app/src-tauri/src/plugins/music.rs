use super::capability::MAX_JSON;
use crate::model::AppResult;
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
    pub(super) async fn account_credentials(&self, args: &Value) -> AppResult<Value> {
        use crate::music::{account::AccountAccess, adapter::RequestContext, identity::SourceId};
        use base64::Engine;
        let source: SourceId =
            serde_json::from_value(args.get("source").ok_or("缺少账号来源")?.clone())
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "账号来源无效")?;
        let stamp = self
            .music
            .adapters
            .stamp(&source)
            .map_err(|e| e.to_string())?;
        let scope = self
            .music
            .adapters
            .cache_identity(&source)
            .map_err(|e| e.to_string())?;
        if args
            .get("scope")
            .and_then(Value::as_str)
            .is_some_and(|expected| expected != scope)
        {
            return Err("账号上下文已变化".into());
        }
        let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0);
        let length = args.get("length").and_then(Value::as_u64).unwrap_or(4096);
        if offset > 65536 || length == 0 || length > 4096 {
            return Err("凭据读取范围无效".into());
        }
        let accounts = self.music.accounts.as_ref().ok_or("账号管理不可用")?;
        let context = RequestContext {
            session: stamp.0.clone(),
            adapter_generation: stamp.1,
            deadline: Instant::now() + Duration::from_secs(3),
            cancellation: Default::default(),
        };
        let credential = accounts
            .blocking(move |accounts| {
                if context.session.account.is_some() {
                    accounts.scoped_access(&context)?.read(&context).map(Some)
                } else {
                    Ok(None)
                }
            })
            .await
            .map_err(|e| e.to_string())?;
        if self
            .music
            .adapters
            .stamp(&source)
            .map_err(|e| e.to_string())?
            != stamp
        {
            return Err("账号或来源状态已变化".into());
        }
        let bytes = credential
            .as_ref()
            .map(|value| value.expose())
            .unwrap_or_default();
        if offset as usize > bytes.len() {
            return Err("凭据读取范围无效".into());
        }
        let data = &bytes[offset as usize..bytes.len().min(offset as usize + length as usize)];
        Ok(
            json!({"data":base64::engine::general_purpose::STANDARD.encode(data),"offset":offset,"total":bytes.len(),"generation":stamp.0.generation,"scope":scope}),
        )
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
        cache.retain(|_, (_, entry_generation, time, _)| {
            *entry_generation == generation && time.elapsed() < Duration::from_secs(600)
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
        if let Some((entry_stamp, _, _, track)) = cache.get(reference) {
            if *entry_stamp == stamp {
                return Ok(track.clone());
            }
        }
        let track = tokio::time::timeout(Duration::from_secs(3), async {
            self.music
                .adapters
                .read_track(reference)
                .await
                .map_err(|e| e.to_string())
        })
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
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
