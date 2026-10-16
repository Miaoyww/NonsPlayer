mod amll;
mod identity;
pub(crate) mod local_lyrics;
pub(crate) mod lyric_matching;
pub(crate) mod media_metadata;
pub(crate) mod netease_lyrics;
pub(crate) mod qq_lyrics;
pub(crate) mod qrc_decrypt;
pub(crate) mod ttml_cache;

use crate::{
    model::{AppResult, Lyrics, Track, TrackSource},
    netease::Netease,
    storage::{now_seconds, Store, MAX_LYRIC_BYTES},
    ttml_cache::TtmlCache,
};
use quick_xml::{events::Event, Reader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::Emitter;

const FOUND_TTL: i64 = 600;
const MISS_TTL: i64 = 86400;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricSources {
    #[serde(default)]
    pub local_source: LocalLyricSource,
    pub amll: bool,
    pub qq: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LocalLyricSource {
    #[default]
    Online,
    Local,
}

impl Default for LyricSources {
    fn default() -> Self {
        Self {
            local_source: LocalLyricSource::Online,
            amll: true,
            qq: true,
        }
    }
}

pub struct LyricService {
    store: Arc<Store>,
    netease: Arc<Netease>,
    client: reqwest::Client,
    refreshing: Mutex<HashSet<String>>,
    retry_after: Mutex<HashMap<String, i64>>,
    disk: Arc<TtmlCache>,
}

enum Lookup {
    Found(Lyrics),
    Missing,
    Failed,
}

pub struct LyricSkips {
    pub amll: bool,
    pub qq: bool,
    pub qrc: bool,
    pub netease: bool,
}

impl LyricService {
    pub fn set_endpoints(&self, platform: Option<String>, endpoints: Vec<String>) -> AppResult<()> {
        let platform = platform.unwrap_or_else(|| "netease".into());
        if identity::directory(&platform).is_none() {
            return Err("该平台尚无 AMLL 歌词索引".into());
        }
        amll::validate_endpoints(&endpoints)?;
        let mut retries = self.retry_after.lock().map_err(|_| "歌词状态不可用")?;
        let mut configured: std::collections::BTreeMap<String, Vec<String>> = self
            .store
            .setting("lyricEndpointsV2")?
            .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
            .transpose()?
            .unwrap_or_default();
        configured.insert(platform, endpoints);
        self.disk.set_lyric_setting(
            "lyricEndpointsV2",
            &serde_json::to_string(&configured).map_err(|e| e.to_string())?,
        )?;
        retries.clear();
        Ok(())
    }

    pub fn sources(&self) -> AppResult<LyricSources> {
        self.store
            .setting("lyricSources")?
            .map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
            .unwrap_or_else(|| Ok(LyricSources::default()))
    }

    pub fn set_sources(&self, sources: LyricSources) -> AppResult<()> {
        self.disk
            .set_lyric_sources(&serde_json::to_string(&sources).map_err(|e| e.to_string())?)?;
        self.retry_after
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "歌词状态不可用")?
            .clear();
        Ok(())
    }
    pub fn new(store: Arc<Store>, netease: Arc<Netease>, disk: Arc<TtmlCache>) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("NonsPlayer/0.1")
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(1))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            store,
            netease,
            client,
            refreshing: Mutex::new(HashSet::new()),
            retry_after: Mutex::new(HashMap::new()),
            disk,
        })
    }

    pub async fn get(
        self: &Arc<Self>,
        track: Track,
        refresh: bool,
        skips: LyricSkips,
        app: tauri::AppHandle,
    ) -> AppResult<Option<Lyrics>> {
        let separators = if matches!(track.source, TrackSource::Local { .. }) {
            crate::local_library::preferences(&self.store)?.artist_separators
        } else {
            vec![" / ".into()]
        };
        let generation = self.disk.generation() + u64::from(refresh);
        let requested = serde_json::to_vec(&track).map_err(|e| e.to_string())?;
        let key = track.key.clone();
        let value = self
            .get_online(track, refresh, skips, separators, app)
            .await?;
        if self.disk.generation() != generation
            || self
                .store
                .track(&key)
                .ok()
                .and_then(|v| serde_json::to_vec(&v).ok())
                .as_ref()
                != Some(&requested)
        {
            return Ok(None);
        }
        Ok(value)
    }

    async fn get_online(
        self: &Arc<Self>,
        track: Track,
        refresh: bool,
        skips: LyricSkips,
        separators: Vec<String>,
        app: tauri::AppHandle,
    ) -> AppResult<Option<Lyrics>> {
        let started = std::time::Instant::now();
        let identity = identity::Identity::of(&track);
        let mut netease_ids: Vec<u64> = identity
            .as_ref()
            .filter(|v| v.platform == "netease")
            .map(|v| {
                v.identifiers
                    .iter()
                    .filter(|v| v.kind == "song-id")
                    .filter_map(|v| v.id.as_str().parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        if netease_ids.is_empty() {
            netease_ids.extend(track.netease_id());
        }
        netease_ids.sort_unstable();
        netease_ids.dedup();
        let id = netease_ids.first().copied();
        let amll_request = identity
            .map(|identity| amll::Request::new(identity, &self.store))
            .transpose()?
            .flatten();
        let sources = self.sources()?;
        let amll_key = amll_request
            .as_ref()
            .map(amll::Request::key)
            .transpose()?
            .unwrap_or_else(|| "amll:unavailable".into());
        let ncm_key = if matches!(track.source, TrackSource::Local { .. }) {
            format!("netease:{}", id.unwrap_or(0))
        } else {
            format!(
                "netease:exact-v2:{}",
                serde_json::to_string(&netease_ids).map_err(|e| e.to_string())?
            )
        };
        // Metadata belongs in the key: local bindings and corrected tags can change matching.
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(&track, &separators)).map_err(|e| e.to_string())?)
        );
        let mode = if qq_lyrics::has_exact_identity(&track) {
            "exact-v4"
        } else {
            "search-v4"
        };
        let qrc_key = format!("qq:qrc-{mode}:{digest}");
        let lrc_key = format!("qq:lrc-{mode}:{digest}");
        let search_key = format!("netease:search-v1:{digest}");
        let qq_key = if skips.qrc { &lrc_key } else { &qrc_key };
        if refresh {
            self.disk
                .refresh_lyrics(&[&amll_key, &ncm_key, &qrc_key, &lrc_key, &search_key])?;
            let mut retries = self
                .retry_after
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "歌词状态不可用")?;
            retries.remove(&amll_key);
            retries.remove(&qrc_key);
            retries.remove(&lrc_key);
            retries.remove(&search_key);
        }
        if let Some(request) = amll_request.filter(|_| sources.amll && !skips.amll) {
            // Disk failure is a cache miss; it must never prevent online fallback.
            if let Some(cached) = self.store.cached_lyrics(&amll_key)?.or_else(|| {
                self.disk
                    .get(&amll_key)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
                    .flatten()
            }) {
                if let Some(value) = cached.value {
                    if !cached.fresh {
                        let mut refreshing = self
                            .refreshing
                            .lock()
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("translate_error", error);
                            })
                            .map_err(|_| "歌词状态不可用")?;
                        if refreshing.insert(amll_key.clone()) {
                            let this = Arc::clone(self);
                            let refreshed_track = track.clone();
                            let generation = self.disk.generation();
                            let request = request.clone();
                            let cache_key = amll_key.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Ok(Some(lyrics)) = (this.update_amll(&request).await)
                                    .inspect_err(|error| {
                                        crate::infrastructure::diagnostics::warn(
                                            "optional_success",
                                            error,
                                        );
                                    })
                                {
                                    this.emit_update(&refreshed_track, generation, lyrics, &app);
                                }
                                if let Ok(mut pending) =
                                    (this.refreshing.lock()).inspect_err(|error| {
                                        crate::infrastructure::diagnostics::warn(
                                            "optional_success",
                                            error,
                                        );
                                    })
                                {
                                    pending.remove(&cache_key);
                                }
                            });
                        }
                    }
                    return Ok(Some(value));
                }
                if !cached.fresh {
                    if let Some(value) = self.update_amll(&request).await? {
                        return Ok(Some(value));
                    }
                }
            } else if let Some(value) = self.update_amll(&request).await? {
                return Ok(Some(value));
            }
        }
        let preferred = if sources.qq && !skips.qq {
            self.get_qq(&track, qq_key, skips.qrc, &separators, app.clone())
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_result", error);
                })
                .ok()
                .flatten()
        } else {
            None
        };
        if preferred.as_ref().is_some_and(|value| {
            value.match_score.unwrap_or(0) >= crate::lyric_matching::PREFERRED_MINIMUM_SCORE
        }) {
            return Ok(preferred);
        }
        if skips.netease {
            return Ok(preferred);
        }
        let fallback = if id.is_some() {
            if !refresh {
                if let Some(cached) = self.store.cached_lyrics(&ncm_key)? {
                    if cached.fresh {
                        return Ok(cached.value.or(preferred));
                    }
                }
            }
            let generation = self.disk.generation();
            let budget = Duration::from_secs(12)
                .saturating_sub(started.elapsed())
                .min(Duration::from_secs(6));
            let fetch = async {
                let mut failed = false;
                for id in &netease_ids {
                    match self.netease.lyrics(*id).await {
                        Ok(Some(value)) => return Ok(Some(value)),
                        Ok(None) => {}
                        Err(error) => {
                            crate::infrastructure::diagnostics::warn("lyrics.netease.fetch", error);
                            failed = true;
                        }
                    }
                }
                if failed {
                    Err("网易云精准歌词查询失败".to_string())
                } else {
                    Ok(None)
                }
            };
            match tokio::time::timeout(budget, fetch).await {
                Ok(Ok(value)) => {
                    let _ = (self.disk.cache_result(
                        &ncm_key,
                        value.as_ref(),
                        if value.is_some() { FOUND_TTL } else { MISS_TTL },
                        generation,
                    ))
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("self.disk.cache_result", error);
                    });
                    if generation != self.disk.generation() {
                        return Ok(None);
                    }
                    value
                }
                Ok(Err(error)) => {
                    crate::infrastructure::diagnostics::warn("lyrics.netease.fetch", error);
                    None
                }
                Err(error) => {
                    crate::infrastructure::diagnostics::warn("lyrics.netease.timeout", error);
                    None
                }
            }
        } else {
            let budget = Duration::from_secs(12)
                .saturating_sub(started.elapsed())
                .min(Duration::from_secs(6));
            tokio::time::timeout(
                budget,
                self.get_searched_netease(&track, &search_key, &separators, app),
            )
            .await
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_result", error);
            })
            .ok()
            .and_then(|result| {
                result
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("lyrics.netease.search", error)
                    })
                    .ok()
            })
            .flatten()
        };
        // Known playback/bound ID overrides scores; otherwise compare scores with QQ winning ties.
        Ok(if id.is_some() {
            fallback.or(preferred)
        } else {
            crate::lyric_matching::select(preferred, fallback)
        })
    }

    async fn get_searched_netease(
        self: &Arc<Self>,
        track: &Track,
        key: &str,
        separators: &[String],
        app: tauri::AppHandle,
    ) -> AppResult<Option<Lyrics>> {
        if let Some(cached) = self.store.cached_lyrics(key)? {
            if cached.fresh {
                return Ok(cached.value);
            }
            if let Some(value) = cached.value {
                let mut refreshing = self
                    .refreshing
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "歌词状态不可用")?;
                if refreshing.insert(key.into()) {
                    let this = Arc::clone(self);
                    let generation = self.disk.generation();
                    let track = track.clone();
                    let key = key.to_owned();
                    let separators = separators.to_vec();
                    tauri::async_runtime::spawn(async move {
                        if let Ok(Some(lyrics)) = (this
                            .update_searched_netease(&track, &key, &separators)
                            .await)
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("optional_success", error);
                            })
                        {
                            this.emit_update(&track, generation, lyrics, &app);
                        }
                        if let Ok(mut pending) = (this.refreshing.lock()).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("optional_success", error);
                        }) {
                            pending.remove(&key);
                        }
                    });
                }
                return Ok(Some(value));
            }
        }
        self.update_searched_netease(track, key, separators).await
    }

    async fn update_searched_netease(
        &self,
        track: &Track,
        key: &str,
        separators: &[String],
    ) -> AppResult<Option<Lyrics>> {
        let generation = self.disk.generation();
        if self
            .retry_after
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "歌词状态不可用")?
            .get(key)
            .is_some_and(|t| *t > now_seconds())
        {
            return Ok(None);
        }
        match crate::netease_lyrics::lookup(&self.netease, track, separators).await {
            Ok(value) => {
                let _ = (self.disk.cache_result(
                    key,
                    value.as_ref(),
                    if value.is_some() { FOUND_TTL } else { MISS_TTL },
                    generation,
                ))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("self.disk.cache_result", error);
                });
                if generation != self.disk.generation() {
                    return Ok(None);
                }
                Ok(value)
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("error_fallback", error);
                let mut retries = self
                    .retry_after
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "歌词状态不可用")?;
                retries.retain(|_, t| *t > now_seconds());
                if generation == self.disk.generation() && retries.len() < 2000 {
                    retries.insert(key.into(), now_seconds() + 30);
                }
                Ok(None)
            }
        }
    }

    async fn get_qq(
        self: &Arc<Self>,
        track: &Track,
        key: &str,
        skip_qrc: bool,
        separators: &[String],
        app: tauri::AppHandle,
    ) -> AppResult<Option<Lyrics>> {
        if let Some(cached) = self.store.cached_lyrics(key)? {
            if cached.fresh {
                return Ok(cached.value);
            }
            if let Some(value) = cached.value {
                let mut refreshing = self
                    .refreshing
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "歌词状态不可用")?;
                if refreshing.insert(key.into()) {
                    let this = Arc::clone(self);
                    let generation = self.disk.generation();
                    let track = track.clone();
                    let cache_key = key.to_owned();
                    let separators = separators.to_vec();
                    tauri::async_runtime::spawn(async move {
                        if let Ok(Some(lyrics)) = (this
                            .update_qq(&track, &cache_key, skip_qrc, &separators)
                            .await)
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("optional_success", error);
                            })
                        {
                            this.emit_update(&track, generation, lyrics, &app);
                        }
                        if let Ok(mut pending) = (this.refreshing.lock()).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("optional_success", error);
                        }) {
                            pending.remove(&cache_key);
                        }
                    });
                }
                return Ok(Some(value));
            }
        }
        self.update_qq(track, key, skip_qrc, separators).await
    }

    async fn update_qq(
        &self,
        track: &Track,
        key: &str,
        skip_qrc: bool,
        separators: &[String],
    ) -> AppResult<Option<Lyrics>> {
        let generation = self.disk.generation();
        if self
            .retry_after
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "歌词状态不可用")?
            .get(key)
            .is_some_and(|t| *t > now_seconds())
        {
            return Ok(None);
        }
        match crate::qq_lyrics::lookup(&self.client, track, skip_qrc, separators).await {
            Ok(value) => {
                let _ = (self.disk.cache_result(
                    key,
                    value.as_ref(),
                    if value.is_some() { FOUND_TTL } else { MISS_TTL },
                    generation,
                ))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("self.disk.cache_result", error);
                });
                if generation != self.disk.generation() {
                    return Ok(None);
                }
                Ok(value)
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("error_fallback", error);
                let mut retries = self
                    .retry_after
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "歌词状态不可用")?;
                retries.retain(|_, t| *t > now_seconds());
                if generation == self.disk.generation() && retries.len() < 2000 {
                    retries.insert(key.into(), now_seconds() + 30);
                }
                Ok(None)
            }
        }
    }

    async fn update_amll(&self, request: &amll::Request) -> AppResult<Option<Lyrics>> {
        let generation = self.disk.generation();
        let key = request.key()?;
        if self
            .retry_after
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "歌词状态不可用")?
            .get(&key)
            .is_some_and(|t| *t > now_seconds())
        {
            return Ok(None);
        }
        match request.lookup(&self.client).await {
            Lookup::Found(lyrics) => {
                let _ = (self
                    .disk
                    .cache_result(&key, Some(&lyrics), FOUND_TTL, generation))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("self .disk .cache_result", error);
                });
                if generation != self.disk.generation() {
                    return Ok(None);
                }
                Ok(Some(lyrics))
            }
            Lookup::Missing => {
                let _ = (self.disk.cache_result(&key, None, MISS_TTL, generation)).inspect_err(
                    |error| {
                        crate::infrastructure::diagnostics::warn("self.disk.cache_result", error);
                    },
                );
                Ok(None)
            }
            Lookup::Failed => {
                let mut retries = self
                    .retry_after
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "歌词状态不可用")?;
                retries.retain(|_, t| *t > now_seconds());
                if generation == self.disk.generation() && retries.len() < 2000 {
                    retries.insert(key, now_seconds() + 30);
                }
                Ok(None)
            }
        }
    }

    fn emit_update(&self, track: &Track, generation: u64, lyrics: Lyrics, app: &tauri::AppHandle) {
        if self.disk.generation() != generation {
            return;
        }
        let current = self
            .store
            .track(&track.key)
            .ok()
            .and_then(|v| serde_json::to_vec(&v).ok());
        if current != serde_json::to_vec(track).ok() {
            return;
        }
        if let Err(error) = app.emit(
            "lyrics-updated",
            serde_json::json!({"key":track.key,"track":track,"lyrics":lyrics}),
        ) {
            crate::infrastructure::diagnostics::warn("lyrics.emit", error);
        }
    }
}

// This only validates the XML envelope. The AMLL parser owns lyric semantics.
pub fn valid_ttml(content: &str) -> bool {
    if content.len() > MAX_LYRIC_BYTES {
        return false;
    }
    let mut reader = Reader::from_str(content);
    let mut root = false;
    let mut body = false;
    let mut depth = 0usize;
    let mut closed = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if closed {
                    return false;
                }
                if !root {
                    if e.local_name().as_ref() != b"tt" {
                        return false;
                    }
                    root = true;
                }
                if e.local_name().as_ref() == b"body" {
                    body = true;
                }
                depth += 1;
            }
            Ok(Event::Empty(e)) => {
                if closed || !root {
                    return false;
                }
                if e.local_name().as_ref() == b"body" {
                    body = true;
                }
            }
            Ok(Event::End(_)) => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
                if depth == 0 {
                    closed = true;
                }
            }
            Ok(Event::DocType(_)) => return false,
            Err(error) => {
                crate::infrastructure::diagnostics::warn("lyrics.validate_ttml", error);
                return false;
            }
            Ok(Event::Eof) => return root && body && closed && depth == 0,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_source_settings_keep_online_default_and_local_selection_roundtrips() {
        let old: LyricSources = serde_json::from_str(r#"{"amll":true,"qq":false}"#).unwrap();
        assert_eq!(old.local_source, LocalLyricSource::Online);
        let mut selected = old;
        selected.local_source = LocalLyricSource::Local;
        let restored: LyricSources =
            serde_json::from_str(&serde_json::to_string(&selected).unwrap()).unwrap();
        assert_eq!(restored.local_source, LocalLyricSource::Local);
        assert!(restored.amll && !restored.qq);
    }
    #[test]
    fn server_error_pages_and_truncated_xml_are_not_lyrics() {
        assert!(!valid_ttml("<html><body>Unavailable</body></html>"));
        assert!(!valid_ttml("<tt><body>"));
        assert!(!valid_ttml(
            "<!DOCTYPE tt SYSTEM 'example'><tt><body/></tt>"
        ));
        assert!(valid_ttml(
            "<tt xmlns='http://www.w3.org/ns/ttml'><body/></tt>"
        ));
    }
}
