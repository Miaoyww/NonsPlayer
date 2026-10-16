use crate::{
    lyrics::valid_ttml,
    model::{AppResult, Lyrics},
    storage::{now_seconds, CachedLyrics, Store, MAX_LYRIC_BYTES},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const NAMESPACE: &str = "nons-cache-v1";
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheOptions {
    pub enabled: bool,
    pub max_mb: u64,
    pub directory: String,
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
    root: PathBuf,
    db: Connection,
    generation: u64,
}
pub struct TtmlCache {
    state: Mutex<State>,
    store: Arc<Store>,
}

fn open_state(mut options: CacheOptions) -> AppResult<State> {
    std::fs::create_dir_all(&options.directory).map_err(|e| format!("缓存目录不可用：{e}"))?;
    let base = Path::new(&options.directory)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    options.directory = base
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();
    let root = Path::new(&options.directory).join(NAMESPACE);
    std::fs::create_dir_all(root.join("lyrics/ttml"))
        .map_err(|e| format!("缓存目录不可用：{e}"))?;
    // Files and index live entirely inside our namespace, never in the chosen
    // directory itself. Clear and relocation touch indexed TTML files only.
    let db = Connection::open(root.join("index.sqlite3")).map_err(|e| e.to_string())?;
    db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS lyrics (key TEXT PRIMARY KEY, expires INTEGER NOT NULL, accessed INTEGER NOT NULL, bytes INTEGER NOT NULL, value TEXT NOT NULL);").map_err(|e| e.to_string())?;
    Ok(State {
        options,
        root,
        db,
        generation: 0,
    })
}
fn lyric_path(state: &State, key: &str) -> PathBuf {
    let digest = format!("{:x}", Sha256::digest(key.as_bytes()));
    state
        .root
        .join("lyrics/ttml")
        .join(format!("{digest}.ttml"))
}
fn remove(state: &State, key: &str) -> AppResult<()> {
    match std::fs::remove_file(lyric_path(state, key)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    state
        .db
        .execute("DELETE FROM lyrics WHERE key=?1", [key])
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn prune(state: &State) -> AppResult<()> {
    let limit = (state.options.max_mb * 1024 * 1024) as i64;
    let mut bytes: i64 = state
        .db
        .query_row("SELECT COALESCE(SUM(bytes),0) FROM lyrics", [], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    let mut count: i64 = state
        .db
        .query_row("SELECT COUNT(*) FROM lyrics", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    while bytes > limit || count > 2000 {
        let (key, size): (String, i64) = state
            .db
            .query_row(
                "SELECT key,bytes FROM lyrics ORDER BY accessed,key LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        remove(state, &key)?;
        bytes = bytes.saturating_sub(size);
        count -= 1;
    }
    Ok(())
}
fn write(state: &State, key: &str, lyrics: &Lyrics, expires: i64) -> AppResult<()> {
    if lyrics.format != "ttml"
        || !valid_ttml(&lyrics.content)
        || lyrics.content.len() > MAX_LYRIC_BYTES
    {
        return Err("仅缓存有效的 TTML 歌词".into());
    }
    // Index is committed only after the complete content is written. Invalid
    // or interrupted files are rejected on read and can be downloaded again.
    let destination = lyric_path(state, key);
    let pending = destination.with_extension("part");
    std::fs::write(&pending, &lyrics.content).map_err(|e| e.to_string())?;
    if destination.exists() {
        std::fs::remove_file(&destination).map_err(|e| e.to_string())?;
    }
    std::fs::rename(pending, destination).map_err(|e| e.to_string())?;
    let metadata = Lyrics {
        content: String::new(),
        ..lyrics.clone()
    };
    state.db.execute("INSERT INTO lyrics(key,expires,accessed,bytes,value) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(key) DO UPDATE SET expires=excluded.expires,accessed=excluded.accessed,bytes=excluded.bytes,value=excluded.value", params![key, expires, now_seconds(), lyrics.content.len() as i64, serde_json::to_string(&metadata).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
    prune(state)
}
impl TtmlCache {
    pub fn new(store: Arc<Store>, default_directory: PathBuf) -> AppResult<Self> {
        let options = store
            .setting("localCache")?
            .and_then(|s| {
                serde_json::from_str::<CacheOptions>(&s)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
            })
            .filter(|o| (1..=4096).contains(&o.max_mb) && Path::new(&o.directory).is_absolute())
            .unwrap_or(CacheOptions {
                enabled: true,
                max_mb: 64,
                directory: default_directory.to_string_lossy().into_owned(),
            });
        let state = open_state(options)?;
        // Migrate valid TTML only; legacy LRC/YRC and missing markers cease to
        // be persisted. Their runtime lifetime is controlled by the service.
        for (key, value, expires) in store.legacy_ttml()? {
            if valid_ttml(&value.content) {
                write(&state, &key, &value, expires.min(now_seconds() + 600))?;
            }
        }
        store.clear_legacy_lyrics()?;
        prune(&state)?;
        Ok(Self {
            state: Mutex::new(state),
            store,
        })
    }
    pub fn status(&self) -> AppResult<CacheStatus> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        let (used_bytes, entries) = state
            .db
            .query_row(
                "SELECT COALESCE(SUM(bytes),0),COUNT(*) FROM lyrics",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
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
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        if !state.options.enabled {
            return Ok(None);
        }
        let row: Option<(String, i64)> = state
            .db
            .query_row(
                "SELECT value,expires FROM lyrics WHERE key=?1",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some((value, expires)) = row else {
            return Ok(None);
        };
        let content = read_content(&lyric_path(&state, key));
        let Some(content) = content else {
            remove(&state, key)?;
            return Ok(None);
        };
        let mut lyrics: Lyrics = serde_json::from_str(&value).map_err(|e| e.to_string())?;
        lyrics.content = content;
        state
            .db
            .execute(
                "UPDATE lyrics SET accessed=?2 WHERE key=?1",
                params![key, now_seconds()],
            )
            .map_err(|e| e.to_string())?;
        Ok(Some(CachedLyrics {
            value: Some(lyrics),
            fresh: expires > now_seconds(),
        }))
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
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        self.store.set_setting(name, value)?;
        state.generation += 1;
        self.store.clear_runtime_lyrics()
    }
    pub fn cache_result(
        &self,
        key: &str,
        lyrics: Option<&Lyrics>,
        ttl: i64,
        generation: u64,
    ) -> AppResult<()> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        if state.generation != generation {
            return Ok(());
        }
        self.store.cache_lyrics(key, lyrics, ttl)?;
        if state.options.enabled {
            if let Some(lyrics) = lyrics.filter(|l| l.format == "ttml") {
                write(&state, key, lyrics, now_seconds() + ttl)?;
            }
        }
        Ok(())
    }
    pub fn refresh_lyrics(&self, keys: &[&str]) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        state.generation += 1;
        for key in keys {
            self.store.invalidate_lyrics(key)?;
            // A broken disk index must not prevent refreshing online sources.
            let _ = (remove(&state, key)).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("remove", error);
            });
        }
        Ok(())
    }
    pub fn clear(&self) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        state.generation += 1;
        self.store.clear_runtime_lyrics()?;
        let keys = state
            .db
            .prepare("SELECT key FROM lyrics")
            .map_err(|e| e.to_string())?
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for key in keys {
            remove(&state, &key)?;
        }
        state
            .db
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn configure(&self, options: CacheOptions) -> AppResult<()> {
        if !(1..=4096).contains(&options.max_mb) || !Path::new(&options.directory).is_absolute() {
            return Err("缓存上限须为 1–4096 MB，缓存目录须为绝对路径".into());
        }
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "缓存锁不可用")?;
        let mut next = open_state(options)?;
        next.generation = state.generation + 1;
        if state.root.canonicalize().map_err(|e| e.to_string())?
            != next.root.canonicalize().map_err(|e| e.to_string())?
        {
            let rows = state
                .db
                .prepare("SELECT key,value,expires FROM lyrics")
                .map_err(|e| e.to_string())?
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            for (key, value, expires) in &rows {
                if let Some(content) = read_content(&lyric_path(&state, key)) {
                    let mut lyrics: Lyrics =
                        serde_json::from_str(value).map_err(|e| e.to_string())?;
                    lyrics.content = content;
                    write(&next, key, &lyrics, *expires)?;
                }
            }
            self.store.set_setting(
                "localCache",
                &serde_json::to_string(&next.options).map_err(|e| e.to_string())?,
            )?;
            // Remove only files that were successfully migrated. The namespace's
            // database is tiny and retained; unrelated user files are untouched.
            let previous = std::mem::replace(&mut *state, next);
            for (key, _, _) in &rows {
                let _ = (remove(&previous, key)).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("remove", error);
                });
            }
        } else {
            self.store.set_setting(
                "localCache",
                &serde_json::to_string(&next.options).map_err(|e| e.to_string())?,
            )?;
            state.options = next.options;
            state.generation = next.generation;
        }
        self.store.clear_runtime_lyrics()?;
        prune(&state)
    }
}

fn read_content(path: &Path) -> Option<String> {
    let mut content = String::new();
    std::fs::File::open(path)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_result", error);
        })
        .ok()?
        .take((MAX_LYRIC_BYTES + 1) as u64)
        .read_to_string(&mut content)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_result", error);
        })
        .ok()?;
    valid_ttml(&content).then_some(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;
    fn lyric(size: usize) -> Lyrics {
        Lyrics {
            match_score: None,
            source: "amll".into(),
            format: "ttml".into(),
            content: format!("<tt><body>{}</body></tt>", "a".repeat(size)),
            translation: None,
            romanization: None,
        }
    }
    fn cache(root: &Path) -> TtmlCache {
        TtmlCache::new(
            Arc::new(Store::open(&root.join("library.sqlite3")).unwrap()),
            root.join("cache"),
        )
        .unwrap()
    }
    #[test]
    fn source_changes_persist_and_reject_inflight_lyrics_without_deleting_ttml() {
        let root = TestDir::new();
        let cache = cache(&root.0);
        let generation = cache.generation();
        cache
            .cache_result("amll:1", Some(&lyric(10)), 600, generation)
            .unwrap();
        let mut lrc = lyric(10);
        lrc.source = "qq".into();
        lrc.format = "lrc".into();
        cache
            .cache_result("qq:2", Some(&lrc), 600, generation)
            .unwrap();
        cache
            .set_lyric_sources(r#"{"amll":false,"qq":false}"#)
            .unwrap();
        assert!(cache.store.cached_lyrics("qq:2").unwrap().is_none());
        cache
            .cache_result("qq:2", Some(&lrc), 600, generation)
            .unwrap();
        assert!(cache.store.cached_lyrics("qq:2").unwrap().is_none());
        assert!(cache.get("amll:1").unwrap().is_some());
        let sources: crate::lyrics::LyricSources =
            serde_json::from_str(&cache.store.setting("lyricSources").unwrap().unwrap()).unwrap();
        assert!(!sources.amll && !sources.qq);
        drop(cache);
        let store = Store::open(&root.0.join("library.sqlite3")).unwrap();
        assert_eq!(
            store.setting("lyricSources").unwrap().as_deref(),
            Some(r#"{"amll":false,"qq":false}"#)
        );
    }
    #[test]
    fn manual_refresh_invalidates_all_source_keys_and_rejects_old_requests() {
        let root = TestDir::new();
        let cache = cache(&root.0);
        let generation = cache.generation();
        for key in ["amll:1", "qq:1", "netease:1"] {
            cache
                .cache_result(key, Some(&lyric(10)), 600, generation)
                .unwrap();
        }
        cache
            .refresh_lyrics(&["amll:1", "qq:1", "netease:1"])
            .unwrap();
        for key in ["amll:1", "qq:1", "netease:1"] {
            assert!(cache.store.cached_lyrics(key).unwrap().is_none());
            assert!(cache.get(key).unwrap().is_none());
            cache
                .cache_result(key, Some(&lyric(10)), 600, generation)
                .unwrap();
            assert!(cache.store.cached_lyrics(key).unwrap().is_none());
        }
    }
    #[test]
    fn ttml_survives_restart_but_netease_lrc_stays_in_memory() {
        let root = TestDir::new();
        {
            let cache = cache(&root.0);
            cache
                .cache_result("amll:1", Some(&lyric(10)), 600, cache.generation())
                .unwrap();
            let mut lrc = lyric(10);
            lrc.format = "lrc".into();
            cache
                .cache_result("netease:2", Some(&lrc), 600, cache.generation())
                .unwrap();
            assert_eq!(cache.status().unwrap().entries, 1);
            assert!(cache.store.cached_lyrics("netease:2").unwrap().is_some());
        }
        let cache = cache(&root.0);
        assert!(cache.get("amll:1").unwrap().unwrap().fresh);
        assert!(cache.store.cached_lyrics("netease:2").unwrap().is_none());
    }
    #[test]
    fn clear_rejects_late_responses_and_preserves_unrelated_files() {
        let root = TestDir::new();
        let cache = cache(&root.0);
        std::fs::write(root.0.join("music.flac"), "original").unwrap();
        let generation = cache.generation();
        cache
            .cache_result("amll:1", Some(&lyric(10)), 600, generation)
            .unwrap();
        cache.clear().unwrap();
        cache
            .cache_result("amll:2", Some(&lyric(10)), 600, generation)
            .unwrap();
        assert_eq!(cache.status().unwrap().entries, 0);
        assert!(cache.store.cached_lyrics("amll:2").unwrap().is_none());
        assert_eq!(
            std::fs::read_to_string(root.0.join("music.flac")).unwrap(),
            "original"
        );
    }
    #[test]
    fn disabled_cache_keeps_runtime_data_and_enforces_disk_capacity_when_enabled() {
        let root = TestDir::new();
        let cache = cache(&root.0);
        let mut options = cache.status().unwrap().options;
        options.enabled = false;
        options.max_mb = 1;
        cache.configure(options.clone()).unwrap();
        cache
            .cache_result("amll:1", Some(&lyric(700_000)), 600, cache.generation())
            .unwrap();
        assert_eq!(cache.status().unwrap().entries, 0);
        assert!(cache.store.cached_lyrics("amll:1").unwrap().is_some());
        options.enabled = true;
        cache.configure(options).unwrap();
        for id in 1..=3 {
            cache
                .cache_result(
                    &format!("amll:{id}"),
                    Some(&lyric(700_000)),
                    600,
                    cache.generation(),
                )
                .unwrap();
        }
        let status = cache.status().unwrap();
        assert_eq!(status.entries, 1);
        assert!(status.used_bytes <= 1024 * 1024);
        assert!(cache.get("amll:3").unwrap().is_some());
    }
    #[test]
    fn moving_directory_and_reselecting_same_directory_preserve_ttml() {
        let root = TestDir::new();
        let cache = cache(&root.0);
        cache
            .cache_result("amll:1", Some(&lyric(10)), 600, cache.generation())
            .unwrap();
        let mut options = cache.status().unwrap().options;
        options.directory = Path::new(&options.directory)
            .join(".")
            .to_string_lossy()
            .into_owned();
        cache.configure(options.clone()).unwrap();
        assert_eq!(cache.status().unwrap().entries, 1);
        let old = cache.state.lock().unwrap().root.clone();
        options.directory = root.0.join("new-cache").to_string_lossy().into_owned();
        cache.configure(options).unwrap();
        assert!(cache.get("amll:1").unwrap().is_some());
        assert!(std::fs::read_dir(old.join("lyrics/ttml"))
            .unwrap()
            .next()
            .is_none());
    }
    #[test]
    fn corrupt_and_oversized_ttml_are_misses_and_stale_ttml_remains_usable() {
        let root = TestDir::new();
        let cache = cache(&root.0);
        cache
            .cache_result("amll:1", Some(&lyric(10)), -1, cache.generation())
            .unwrap();
        assert!(!cache.get("amll:1").unwrap().unwrap().fresh);
        let path = lyric_path(&cache.state.lock().unwrap(), "amll:1");
        std::fs::write(path, "<html>error</html>").unwrap();
        assert!(cache.get("amll:1").unwrap().is_none());
        assert_eq!(cache.status().unwrap().entries, 0);
        assert!(cache
            .cache_result(
                "amll:2",
                Some(&lyric(MAX_LYRIC_BYTES)),
                600,
                cache.generation()
            )
            .is_err());
    }
}
