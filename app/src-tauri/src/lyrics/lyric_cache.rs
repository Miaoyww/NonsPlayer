use crate::{
    model::{AppResult, Lyrics},
    storage::{now_seconds, CachedLyrics, LyricCacheScope, Store},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

const DEFAULT_MAX_MB: u64 = 128;
const MAX_MB: u64 = 4096;
const SETTING: &str = "localCache";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheOptions {
    pub enabled: bool,
    pub max_mb: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    pub options: CacheOptions,
    pub used_bytes: i64,
    pub entries: i64,
}

struct State {
    options: CacheOptions,
    generation: u64,
}

/// Persists every lyric format in the library database while enabled. Disabling
/// keeps existing entries but limits new results to the session-only table.
pub struct LyricCache {
    state: Mutex<State>,
    store: Arc<Store>,
}

impl LyricCache {
    pub fn new(store: Arc<Store>) -> AppResult<Self> {
        let options = store
            .setting(SETTING)?
            .and_then(|s| {
                serde_json::from_str::<CacheOptions>(&s)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn_error(
                            "lyrics.cache.load_options.decode",
                            error,
                        );
                    })
                    .ok()
            })
            .filter(|o| (1..=MAX_MB).contains(&o.max_mb))
            .unwrap_or(CacheOptions {
                enabled: true,
                max_mb: DEFAULT_MAX_MB,
            });
        Ok(Self {
            state: Mutex::new(State {
                options,
                generation: 0,
            }),
            store,
        })
    }

    pub fn status(&self) -> AppResult<CacheStatus> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("lyrics.cache.status.lock", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        let (used_bytes, entries) = self.store.lyric_cache_usage()?;
        Ok(CacheStatus {
            options: state.options.clone(),
            used_bytes,
            entries,
        })
    }

    pub fn get(&self, key: &str) -> AppResult<Option<CachedLyrics>> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("lyrics.cache.get.lock", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        self.store.cached_lyrics(scope(&state.options), key)
    }

    pub fn generation(&self) -> u64 {
        self.state.lock().map(|s| s.generation).unwrap_or_default()
    }

    pub fn set_lyric_sources(&self, value: &str) -> AppResult<()> {
        self.set_lyric_setting("lyricSources", value)
    }

    pub fn set_lyric_setting(&self, name: &str, value: &str) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("lyrics.cache.set_setting.lock", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        self.store.set_setting(name, value)?;
        state.generation += 1;
        self.store.clear_lyric_cache(LyricCacheScope::Session)
    }

    /// `ttl: None` keeps a found lyric until capacity eviction or an explicit
    /// refresh. A miss still expires so it can be retried.
    pub fn cache_result(
        &self,
        key: &str,
        lyrics: Option<&Lyrics>,
        ttl: Option<i64>,
        generation: u64,
    ) -> AppResult<()> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("lyrics.cache.store.lock", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        if state.generation != generation {
            return Ok(());
        }
        self.store.cache_lyrics(
            scope(&state.options),
            key,
            lyrics,
            ttl.map(|ttl| now_seconds() + ttl),
            (state.options.max_mb * 1024 * 1024) as i64,
        )
    }

    pub fn refresh_lyrics(&self, keys: &[&str]) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("lyrics.cache.refresh.lock", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        state.generation += 1;
        for key in keys {
            self.store
                .invalidate_lyrics(LyricCacheScope::Persistent, key)?;
            self.store
                .invalidate_lyrics(LyricCacheScope::Session, key)?;
        }
        Ok(())
    }

    pub fn clear(&self) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("lyrics.cache.clear.lock", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        state.generation += 1;
        self.store.clear_lyric_cache(LyricCacheScope::Persistent)?;
        self.store.clear_lyric_cache(LyricCacheScope::Session)
    }

    pub fn configure(&self, options: CacheOptions) -> AppResult<()> {
        if !(1..=MAX_MB).contains(&options.max_mb) {
            return Err("歌词缓存上限须为 1–4096 MB".into());
        }
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("lyrics.cache.configure.lock", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        self.store.set_setting(
            SETTING,
            &serde_json::to_string(&options).map_err(|e| e.to_string())?,
        )?;
        state.options = options;
        state.generation += 1;
        self.store.clear_lyric_cache(LyricCacheScope::Session)?;
        self.store
            .prune_lyric_cache((state.options.max_mb * 1024 * 1024) as i64)
    }
}

fn scope(options: &CacheOptions) -> LyricCacheScope {
    if options.enabled {
        LyricCacheScope::Persistent
    } else {
        LyricCacheScope::Session
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;

    fn lyric(format: &str, size: usize) -> Lyrics {
        Lyrics {
            match_score: None,
            source: "test".into(),
            format: format.into(),
            content: format!("{}{}", "<tt><body>", "a".repeat(size)),
            translation: None,
            romanization: None,
        }
    }

    fn open_cache(root: &std::path::Path) -> LyricCache {
        LyricCache::new(Arc::new(
            Store::open(&root.join("library.sqlite3")).unwrap(),
        ))
        .unwrap()
    }

    #[test]
    fn all_formats_survive_restart() {
        let root = TestDir::new();
        {
            let cache = open_cache(&root.0);
            let generation = cache.generation();
            for (key, format) in [
                ("qq:qrc:1", "qrc"),
                ("qq:lrc:1", "lrc"),
                ("netease:search:1", "lrc"),
                ("amll:1", "ttml"),
            ] {
                cache
                    .cache_result(key, Some(&lyric(format, 10)), None, generation)
                    .unwrap();
            }
        }
        let cache = open_cache(&root.0);
        for key in ["qq:qrc:1", "qq:lrc:1", "netease:search:1", "amll:1"] {
            assert!(cache.get(key).unwrap().is_some_and(|v| v.fresh));
        }
    }

    #[test]
    fn disabled_cache_only_keeps_new_results_for_the_session() {
        let root = TestDir::new();
        let cache = open_cache(&root.0);
        let generation = cache.generation();
        cache
            .cache_result("persisted", Some(&lyric("qrc", 10)), None, generation)
            .unwrap();
        cache
            .configure(CacheOptions {
                enabled: false,
                max_mb: 128,
            })
            .unwrap();
        let generation = cache.generation();
        cache
            .cache_result("session", Some(&lyric("lrc", 10)), None, generation)
            .unwrap();
        assert!(cache.get("session").unwrap().is_some());
        drop(cache);

        let cache = open_cache(&root.0);
        assert!(cache.get("session").unwrap().is_none());
        cache
            .configure(CacheOptions {
                enabled: true,
                max_mb: 128,
            })
            .unwrap();
        assert!(cache.get("persisted").unwrap().is_some());
    }

    #[test]
    fn source_changes_clear_session_but_keep_persisted_results() {
        let root = TestDir::new();
        let cache = open_cache(&root.0);
        let generation = cache.generation();
        cache
            .cache_result("qq:1", Some(&lyric("qrc", 10)), None, generation)
            .unwrap();
        cache
            .configure(CacheOptions {
                enabled: false,
                max_mb: 128,
            })
            .unwrap();
        let generation = cache.generation();
        cache
            .cache_result("session", Some(&lyric("lrc", 10)), None, generation)
            .unwrap();
        cache.set_lyric_sources("{}").unwrap();
        assert!(cache.get("session").unwrap().is_none());
        cache
            .configure(CacheOptions {
                enabled: true,
                max_mb: 128,
            })
            .unwrap();
        assert!(cache.get("qq:1").unwrap().is_some());
    }

    #[test]
    fn refresh_and_clear_reject_inflight_results() {
        let root = TestDir::new();
        let cache = open_cache(&root.0);
        let generation = cache.generation();
        cache
            .cache_result("qq:1", Some(&lyric("qrc", 10)), None, generation)
            .unwrap();
        cache.refresh_lyrics(&["qq:1"]).unwrap();
        assert!(cache.get("qq:1").unwrap().is_none());
        cache
            .cache_result("qq:1", Some(&lyric("qrc", 10)), None, generation)
            .unwrap();
        assert!(cache.get("qq:1").unwrap().is_none());

        let generation = cache.generation();
        cache
            .cache_result("qq:2", Some(&lyric("qrc", 10)), None, generation)
            .unwrap();
        cache.clear().unwrap();
        assert!(cache.get("qq:2").unwrap().is_none());
        cache
            .cache_result("qq:2", Some(&lyric("qrc", 10)), None, generation)
            .unwrap();
        assert!(cache.get("qq:2").unwrap().is_none());
    }

    #[test]
    fn capacity_evicts_the_least_recently_used_only() {
        let root = TestDir::new();
        let cache = open_cache(&root.0);
        let generation = cache.generation();
        let content = lyric("lrc", 600 * 1024);
        cache
            .cache_result("first", Some(&content), None, generation)
            .unwrap();
        cache
            .cache_result("second", Some(&content), None, generation)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        cache.get("first").unwrap();
        cache
            .configure(CacheOptions {
                enabled: true,
                max_mb: 1,
            })
            .unwrap();
        assert!(cache.get("second").unwrap().is_none());
        assert!(cache.get("first").unwrap().is_some());
    }

    #[test]
    fn misses_expire_but_found_lyrics_do_not() {
        let root = TestDir::new();
        let cache = open_cache(&root.0);
        let generation = cache.generation();
        cache
            .cache_result("found", Some(&lyric("lrc", 10)), None, generation)
            .unwrap();
        cache
            .cache_result("miss", None, Some(-1), generation)
            .unwrap();
        let found = cache.get("found").unwrap().unwrap();
        assert!(found.fresh && found.value.is_some());
        let missed = cache.get("miss").unwrap().unwrap();
        assert!(!missed.fresh && missed.value.is_none());
    }

    #[test]
    fn oversized_lyrics_are_rejected_before_writing() {
        let root = TestDir::new();
        let cache = open_cache(&root.0);
        let generation = cache.generation();
        let oversized = lyric("lrc", crate::storage::MAX_LYRIC_BYTES);
        assert!(cache
            .cache_result("too-large", Some(&oversized), None, generation)
            .is_err());
        assert!(cache.get("too-large").unwrap().is_none());
    }

    #[test]
    fn invalid_capacity_is_rejected() {
        let root = TestDir::new();
        let cache = open_cache(&root.0);
        let error = cache
            .configure(CacheOptions {
                enabled: true,
                max_mb: 0,
            })
            .unwrap_err();
        assert!(error.contains("1–4096"));
    }
}
