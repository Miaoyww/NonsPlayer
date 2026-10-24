use crate::model::{AppResult, Lyrics, PlayerSnapshot, Track};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::Path,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub use crate::model::MAX_LYRIC_BYTES;
const LYRIC_CACHE_BYTES: i64 = 64 * 1024 * 1024;

pub struct Store(pub(crate) Mutex<Connection>);

pub struct CachedLyrics {
    pub value: Option<Lyrics>,
    pub fresh: bool,
}

pub fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl Store {
    pub fn open(path: &Path) -> AppResult<Self> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        Self::initialize(connection)
    }

    fn initialize(connection: Connection) -> AppResult<Self> {
        connection
            .busy_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA auto_vacuum=INCREMENTAL;
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            CREATE TABLE IF NOT EXISTS tracks (key TEXT PRIMARY KEY, source TEXT NOT NULL, title TEXT NOT NULL, artist TEXT NOT NULL, value TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS tracks_source ON tracks(source);
            CREATE TABLE IF NOT EXISTS local_origins (key TEXT NOT NULL, origin TEXT NOT NULL, PRIMARY KEY(key,origin));
            CREATE TABLE IF NOT EXISTS local_file_stats (key TEXT PRIMARY KEY, modified TEXT NOT NULL, bytes INTEGER NOT NULL);
            PRAGMA temp_store=MEMORY;
            CREATE TEMP TABLE runtime_lyrics (key TEXT PRIMARY KEY, value TEXT, expires INTEGER NOT NULL, accessed INTEGER NOT NULL, bytes INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS playback_history (
                instance_id TEXT PRIMARY KEY,
                track_key TEXT NOT NULL,
                source TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album TEXT NOT NULL,
                started_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS playback_totals (id INTEGER PRIMARY KEY CHECK(id=1), play_count INTEGER NOT NULL);
            INSERT OR IGNORE INTO playback_totals(id, play_count) VALUES(1, 0);
            CREATE TABLE IF NOT EXISTS local_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS local_entities (kind TEXT NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, cover TEXT NOT NULL, subtitle TEXT NOT NULL, PRIMARY KEY(kind,id));
            CREATE TABLE IF NOT EXISTS local_members (key TEXT NOT NULL, kind TEXT NOT NULL, id TEXT NOT NULL, PRIMARY KEY(key,kind,id));
            CREATE INDEX IF NOT EXISTS local_members_entity ON local_members(kind,id);
            CREATE TABLE IF NOT EXISTS local_playlists (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS local_playlist_tracks (playlist_id INTEGER NOT NULL, key TEXT NOT NULL, position INTEGER NOT NULL, manual INTEGER NOT NULL DEFAULT 1, remote INTEGER NOT NULL DEFAULT 0, excluded INTEGER NOT NULL DEFAULT 0, value TEXT NOT NULL, PRIMARY KEY(playlist_id,key));
            CREATE INDEX IF NOT EXISTS local_audio_path ON tracks(json_extract(value,'$.source.path')) WHERE source='local';
            CREATE VIEW IF NOT EXISTS visible_local_tracks AS
                SELECT t.* FROM tracks t WHERE t.source='local' AND (
                    json_extract(t.value,'$.source.cue') IS NOT NULL OR NOT EXISTS (
                        SELECT 1 FROM tracks c WHERE c.source='local'
                        AND json_extract(c.value,'$.source.path')=json_extract(t.value,'$.source.path')
                        AND json_extract(c.value,'$.source.cue') IS NOT NULL));
            ").map_err(|e| e.to_string())?;
        crate::aggregate::initialize(&connection).map_err(|e| e.to_string())?;
        crate::music::collections::store::initialize(&connection).map_err(|e| e.to_string())?;
        Ok(Self(Mutex::new(connection)))
    }

    pub fn record_playback(
        &self,
        instance_id: &str,
        track: &Track,
        started_at: i64,
    ) -> AppResult<bool> {
        let mut db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO playback_history(instance_id,track_key,source,title,artist,album,started_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![instance_id, track.key, track.source_id(), track.title, track.artist, track.album, started_at],
        ).map_err(|e| e.to_string())?;
        if inserted > 0 {
            tx.execute(
                "UPDATE playback_totals SET play_count=play_count+1 WHERE id=1",
                [],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(inserted > 0)
    }

    pub fn playback_count(&self) -> AppResult<i64> {
        self.0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT play_count FROM playback_totals WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn save_tracks(&self, tracks: &[Track]) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.save_tracks.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        Self::save_tracks_in(&tx, tracks)?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub(crate) fn save_tracks_in(
        tx: &rusqlite::Transaction<'_>,
        tracks: &[Track],
    ) -> AppResult<()> {
        let options = crate::local_library::options_from_db(&tx)?;
        {
            let mut insert = tx.prepare_cached("INSERT INTO tracks(key,source,title,artist,value) VALUES(?1,?2,?3,?4,?5)
                ON CONFLICT(key) DO UPDATE SET title=excluded.title,artist=excluded.artist,value=excluded.value").map_err(|e| e.to_string())?;
            for (index, track) in tracks.iter().enumerate() {
                let source = track.source_id();
                if matches!(track.source, crate::model::TrackSource::Online { .. }) {
                    nons_music_adapter_sdk::metadata::validate_online_queue_track(track)
                        .map_err(|reason| format!("online_track_validation index={index} source={source} rule={reason}"))?;
                }
                insert
                    .execute(params![
                        track.key,
                        source,
                        track.title,
                        track.artist,
                        serde_json::to_string(track).map_err(|e| e.to_string())?
                    ])
                    .map_err(|e| e.to_string())?;
                if source == "local" {
                    crate::local_library::index_track(&tx, track, &options)?;
                }
            }
        }
        Ok(())
    }

    pub fn mark_origin(&self, tracks: &[Track], origin: &str) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.mark_origin.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        for track in tracks {
            tx.execute(
                "INSERT OR IGNORE INTO local_origins(key,origin) VALUES(?1,?2)",
                params![track.key, origin],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn origin_count(&self, origin: &str) -> AppResult<i64> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.origin_count.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT COUNT(*) FROM local_origins o JOIN visible_local_tracks t ON t.key=o.key WHERE o.origin=?1",
                [origin],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }
    pub fn file_unchanged(&self, key: &str, modified: &str, bytes: u64) -> AppResult<bool> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.file_unchanged.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT COUNT(*) FROM local_file_stats WHERE key=?1 AND modified=?2 AND bytes=?3",
                params![key, modified, bytes as i64],
                |r| r.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(|e| e.to_string())
    }
    pub fn save_file_stat(&self, key: &str, modified: &str, bytes: u64) -> AppResult<()> {
        self.0.lock().inspect_err(|error| { crate::infrastructure::diagnostics::warn("storage.save_file_stat.lock", error); }).map_err(|_| "曲库锁不可用")?.execute("INSERT INTO local_file_stats(key,modified,bytes) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET modified=excluded.modified,bytes=excluded.bytes", params![key, modified, bytes as i64]).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn reconcile_origin(
        &self,
        origin: &str,
        seen: &std::collections::HashSet<String>,
    ) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.reconcile_origin.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let keys = tx
            .prepare("SELECT key FROM local_origins WHERE origin=?1")
            .map_err(|e| e.to_string())?
            .query_map([origin], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for key in keys {
            if !seen.contains(&key) {
                tx.execute(
                    "DELETE FROM local_origins WHERE key=?1 AND origin=?2",
                    params![key, origin],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        tx.execute("UPDATE local_playlist_tracks SET value=(SELECT value FROM tracks WHERE tracks.key=local_playlist_tracks.key) WHERE key IN (SELECT key FROM tracks WHERE source='local')", []).map_err(|e|e.to_string())?;
        tx.execute("DELETE FROM tracks WHERE source='local' AND key NOT IN (SELECT key FROM local_origins)", []).map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM local_file_stats WHERE key NOT IN (SELECT key FROM tracks)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn track(&self, key: &str) -> AppResult<Track> {
        let db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.track.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?;
        let value: String = db
            .query_row("SELECT value FROM tracks WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.track.query", error);
            })
            .map_err(|_| "歌曲不存在，请刷新曲库")?;
        serde_json::from_str(&value).map_err(|e| e.to_string())
    }

    pub fn local_tracks(&self, search: &str, offset: u32) -> AppResult<Vec<Track>> {
        let db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.local_tracks.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?;
        let mut statement = db.prepare_cached("SELECT value FROM visible_local_tracks WHERE source='local' AND (instr(lower(title),lower(?1))>0 OR instr(lower(artist),lower(?1))>0) ORDER BY title,key LIMIT 100 OFFSET ?2").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![search, offset], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.map(|row| {
            serde_json::from_str(&row.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
        })
        .collect()
    }

    pub fn setting(&self, key: &str) -> AppResult<Option<String>> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.setting.lock", error);
            })
            .map_err(|_| "配置锁不可用")?
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.0.lock().inspect_err(|error| { crate::infrastructure::diagnostics::warn("storage.set_setting.lock", error); }).map_err(|_| "配置锁不可用")?.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_player(&self, state: &PlayerSnapshot) -> AppResult<()> {
        self.set_setting(
            "player",
            &serde_json::to_string(state).map_err(|e| e.to_string())?,
        )
    }

    pub fn cached_lyrics(&self, key: &str) -> AppResult<Option<CachedLyrics>> {
        let db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.cached_lyrics.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?;
        let row: Option<(Option<String>, i64)> = db
            .query_row(
                "SELECT value,expires FROM runtime_lyrics WHERE key=?1",
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some((value, expires)) = row {
            db.execute(
                "UPDATE runtime_lyrics SET accessed=?2 WHERE key=?1",
                params![key, now_seconds()],
            )
            .map_err(|e| e.to_string())?;
            Ok(Some(CachedLyrics {
                value: value
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e| e.to_string())?,
                fresh: expires > now_seconds(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn cache_lyrics(&self, key: &str, value: Option<&Lyrics>, ttl: i64) -> AppResult<()> {
        let value = value
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?;
        let size = value.as_ref().map_or(0, |s| s.len());
        if size > MAX_LYRIC_BYTES {
            return Err("歌词文件过大".into());
        }
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.cache_lyrics.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO runtime_lyrics(key,value,expires,accessed,bytes) VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value,expires=excluded.expires,accessed=excluded.accessed,bytes=excluded.bytes",
            params![key, value, now_seconds() + ttl, now_seconds(), size as i64]).map_err(|e| e.to_string())?;
        loop {
            let (bytes, count): (i64, i64) = tx
                .query_row(
                    "SELECT COALESCE(SUM(bytes),0),COUNT(*) FROM runtime_lyrics",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(|e| e.to_string())?;
            if bytes <= LYRIC_CACHE_BYTES && count <= 2000 {
                break;
            }
            tx.execute("DELETE FROM runtime_lyrics WHERE key=(SELECT key FROM runtime_lyrics ORDER BY accessed,key LIMIT 1)", []).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA incremental_vacuum(32)")
            .map_err(|e| e.to_string())
    }

    pub fn invalidate_lyrics(&self, key: &str) -> AppResult<()> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.invalidate_lyrics.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?
            .execute("DELETE FROM runtime_lyrics WHERE key=?1", [key])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_runtime_lyrics(&self) -> AppResult<()> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "storage.clear_runtime_lyrics.lock",
                    error,
                );
            })
            .map_err(|_| "歌词缓存锁不可用")?
            .execute("DELETE FROM runtime_lyrics", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl crate::music::accounts::AccountRecordStore for Store {
    fn load(&self) -> crate::music::MusicResult<Option<String>> {
        self.setting("musicAccountsV2")
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.accounts.load", error);
            })
            .map_err(|_| crate::music::ErrorCode::Internal.into())
    }
    fn save(&self, json: &str) -> crate::music::MusicResult<()> {
        self.set_setting("musicAccountsV2", json)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.accounts.save", error);
            })
            .map_err(|_| crate::music::ErrorCode::Internal.into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn account_index_uses_current_namespace_without_reading_old_records() {
        use crate::music::accounts::AccountRecordStore;
        let store =
            super::Store::initialize(rusqlite::Connection::open_in_memory().unwrap()).unwrap();
        store.set_setting("musicAccountsV1", "obsolete").unwrap();
        assert!(store.load().unwrap().is_none());
        let current = r#"{"records":[],"selected":[],"pendingDeletions":[]}"#;
        store.save(current).unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some(current));
        assert_eq!(
            store.setting("musicAccountsV1").unwrap().as_deref(),
            Some("obsolete")
        );
    }
    use super::*;
    #[test]
    fn absence_and_expired_lyrics_are_distinct() {
        let store = Store::initialize(Connection::open_in_memory().unwrap()).unwrap();
        assert!(store.cached_lyrics("amll:1").unwrap().is_none());
        store.cache_lyrics("amll:1", None, 86400).unwrap();
        let miss = store.cached_lyrics("amll:1").unwrap().unwrap();
        assert!(miss.fresh && miss.value.is_none());
        let lyrics = Lyrics {
            match_score: None,
            source: "amll".into(),
            format: "ttml".into(),
            content: "example".into(),
            translation: None,
            romanization: None,
        };
        store.cache_lyrics("amll:2", Some(&lyrics), -1).unwrap();
        let stale = store.cached_lyrics("amll:2").unwrap().unwrap();
        assert!(!stale.fresh && stale.value.is_some());
    }
    #[test]
    fn large_lyrics_are_rejected_before_writing() {
        let store = Store::initialize(Connection::open_in_memory().unwrap()).unwrap();
        let lyrics = Lyrics {
            match_score: None,
            source: "amll".into(),
            format: "ttml".into(),
            content: "a".repeat(MAX_LYRIC_BYTES),
            translation: None,
            romanization: None,
        };
        assert!(store.cache_lyrics("amll:1", Some(&lyrics), 1).is_err());
        assert!(store.cached_lyrics("amll:1").unwrap().is_none());
    }
}
