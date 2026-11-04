use crate::model::{AppResult, ListeningStatistics, Lyrics, PlayerSnapshot, Track};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::Path,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub use crate::model::MAX_LYRIC_BYTES;

pub struct Store(pub(crate) Mutex<Connection>);

pub struct CachedLyrics {
    pub value: Option<Lyrics>,
    pub fresh: bool,
}

pub struct DownloadedLocal {
    pub path: String,
    pub quality: Option<String>,
}

pub struct LocalMvLink {
    pub path: String,
    pub offset_ms: i64,
}

pub struct MvLink {
    pub source: String,
    pub mv_id: String,
    pub title: String,
    pub offset_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LyricCacheScope {
    Persistent,
    Session,
}

impl LyricCacheScope {
    fn table(self) -> &'static str {
        match self {
            Self::Persistent => "lyric_cache",
            Self::Session => "runtime_lyrics",
        }
    }
}

pub fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
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
            CREATE TABLE IF NOT EXISTS lyric_cache (key TEXT PRIMARY KEY, value TEXT, expires INTEGER NOT NULL, accessed INTEGER NOT NULL, bytes INTEGER NOT NULL);
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
            CREATE TABLE IF NOT EXISTS listening_days (day TEXT PRIMARY KEY, seconds INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS local_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS local_entities (kind TEXT NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, cover TEXT NOT NULL, subtitle TEXT NOT NULL, PRIMARY KEY(kind,id));
            CREATE TABLE IF NOT EXISTS local_members (key TEXT NOT NULL, kind TEXT NOT NULL, id TEXT NOT NULL, PRIMARY KEY(key,kind,id));
            CREATE INDEX IF NOT EXISTS local_members_entity ON local_members(kind,id);
            CREATE TABLE IF NOT EXISTS local_artist_avatars (
                artist_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                source TEXT,
                platform_id TEXT,
                platform_name TEXT,
                cover_variants TEXT,
                cover_size TEXT,
                avatar_path TEXT,
                width INTEGER,
                height INTEGER,
                error TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS local_playlists (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, playlist_type TEXT NOT NULL DEFAULT 'aggregate');
            CREATE TABLE IF NOT EXISTS local_playlist_tracks (playlist_id INTEGER NOT NULL, key TEXT NOT NULL, position INTEGER NOT NULL, manual INTEGER NOT NULL DEFAULT 1, remote INTEGER NOT NULL DEFAULT 0, excluded INTEGER NOT NULL DEFAULT 0, value TEXT NOT NULL, PRIMARY KEY(playlist_id,key));
            CREATE TABLE IF NOT EXISTS local_mv_links (track_key TEXT PRIMARY KEY, path TEXT NOT NULL, offset_ms INTEGER NOT NULL DEFAULT 0, updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS mv_links (track_key TEXT PRIMARY KEY, source TEXT NOT NULL, mv_id TEXT NOT NULL, title TEXT NOT NULL, offset_ms INTEGER NOT NULL DEFAULT 0, updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS download_local_tracks (track_key TEXT PRIMARY KEY, path TEXT NOT NULL, quality TEXT, updated_at INTEGER NOT NULL);
            CREATE INDEX IF NOT EXISTS local_audio_path ON tracks(json_extract(value,'$.source.path')) WHERE source='local';
            CREATE VIEW IF NOT EXISTS visible_local_tracks AS
                SELECT t.* FROM tracks t WHERE t.source='local' AND (
                    json_extract(t.value,'$.source.cue') IS NOT NULL OR NOT EXISTS (
                        SELECT 1 FROM tracks c WHERE c.source='local'
                        AND json_extract(c.value,'$.source.path')=json_extract(t.value,'$.source.path')
                        AND json_extract(c.value,'$.source.cue') IS NOT NULL));
            ").map_err(|e| e.to_string())?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS local_cover_state (key TEXT PRIMARY KEY, signature TEXT NOT NULL, embedded TEXT NOT NULL)").map_err(|e| e.to_string())?;
        if !has_column(&connection, "local_playlists", "playlist_type")
            .map_err(|e| e.to_string())?
        {
            connection
                .execute(
                    "ALTER TABLE local_playlists ADD COLUMN playlist_type TEXT NOT NULL DEFAULT 'aggregate'",
                    [],
                )
                .map_err(|e| e.to_string())?;
        }
        if !has_column(&connection, "local_mv_links", "offset_ms").map_err(|e| e.to_string())? {
            connection
                .execute(
                    "ALTER TABLE local_mv_links ADD COLUMN offset_ms INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| e.to_string())?;
        }
        let recorded_days: i64 = connection
            .query_row("SELECT COUNT(*) FROM listening_days", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if recorded_days == 0 {
            connection
                .execute_batch(
                    "INSERT OR IGNORE INTO listening_days(day, seconds)
                     SELECT date(started_at,'unixepoch','localtime'), 0 FROM playback_history GROUP BY 1",
                )
                .map_err(|e| e.to_string())?;
        }
        crate::local_library::initialize_index(&connection)?;
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
            tx.execute(
                "INSERT OR IGNORE INTO listening_days(day, seconds) VALUES(date(?1,'unixepoch','localtime'), 0)",
                params![started_at],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(inserted > 0)
    }

    pub fn add_listening_seconds(&self, at: i64, seconds: i64) -> AppResult<()> {
        if seconds <= 0 {
            return Ok(());
        }
        self.0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .execute(
                "INSERT INTO listening_days(day, seconds) VALUES(date(?1,'unixepoch','localtime'), ?2)
                 ON CONFLICT(day) DO UPDATE SET seconds = seconds + excluded.seconds",
                params![at, seconds],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn listening_statistics(&self) -> AppResult<ListeningStatistics> {
        let db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        let (seconds, days) = db
            .query_row(
                "SELECT COALESCE(SUM(seconds), 0), COUNT(*) FROM listening_days",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let plays = db
            .query_row(
                "SELECT play_count FROM playback_totals WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(ListeningStatistics {
            seconds,
            days,
            plays,
        })
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

    pub fn refresh_local_albums(&self) -> AppResult<()> {
        let mut db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        crate::local_library::refresh_albums(&tx)?;
        tx.commit().map_err(|e| e.to_string())
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
        let removed = tx.execute("DELETE FROM tracks WHERE source='local' AND key NOT IN (SELECT key FROM local_origins)", []).map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM local_members WHERE key NOT IN (SELECT key FROM tracks)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM local_cover_state WHERE key NOT IN (SELECT key FROM tracks)",
            [],
        )
        .map_err(|e| e.to_string())?;
        if removed > 0 {
            crate::local_library::refresh_albums(&tx)?;
        }
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

    pub fn link_download(
        &self,
        track_key: &str,
        path: &str,
        quality: Option<&str>,
    ) -> AppResult<()> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.link_download.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?
            .execute(
                "INSERT INTO download_local_tracks(track_key,path,quality,updated_at) VALUES(?1,?2,?3,?4)
                 ON CONFLICT(track_key) DO UPDATE SET path=excluded.path,quality=excluded.quality,updated_at=excluded.updated_at",
                params![track_key, path, quality, now_millis()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn download_local(&self, track_key: &str) -> AppResult<Option<DownloadedLocal>> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.download_local.lock", error);
            })
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT path,quality FROM download_local_tracks WHERE track_key=?1",
                [track_key],
                |row| {
                    Ok(DownloadedLocal {
                        path: row.get(0)?,
                        quality: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())
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

    pub fn remove_setting(&self, key: &str) -> AppResult<()> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.remove_setting.lock", error);
            })
            .map_err(|_| "配置锁不可用")?
            .execute("DELETE FROM settings WHERE key=?1", [key])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Temporary per-track MV path association; remove with the local-MV MVP.
    pub fn local_mv_link(&self, key: &str) -> AppResult<Option<LocalMvLink>> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.local_mv_link.lock", error);
            })
            .map_err(|_| "本地 MV 关联锁不可用")?
            .query_row(
                "SELECT path,offset_ms FROM local_mv_links WHERE track_key=?1",
                [key],
                |row| {
                    Ok(LocalMvLink {
                        path: row.get(0)?,
                        offset_ms: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn link_local_mv(&self, key: &str, path: &str) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.link_local_mv.lock", error);
            })
            .map_err(|_| "本地 MV 关联锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM mv_links WHERE track_key=?1", [key])
            .map_err(|e| e.to_string())?;
        tx.execute(
                "INSERT INTO local_mv_links(track_key,path,offset_ms,updated_at) VALUES(?1,?2,0,?3)
                 ON CONFLICT(track_key) DO UPDATE SET path=excluded.path,offset_ms=0,updated_at=excluded.updated_at",
                params![key, path, now_seconds()],
            )
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn unlink_local_mv(&self, key: &str) -> AppResult<()> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.unlink_local_mv.lock", error);
            })
            .map_err(|_| "本地 MV 关联锁不可用")?
            .execute("DELETE FROM local_mv_links WHERE track_key=?1", [key])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_local_mv_offset(&self, key: &str, offset_ms: i64) -> AppResult<()> {
        let updated = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.set_local_mv_offset.lock", error);
            })
            .map_err(|_| "本地 MV 关联锁不可用")?
            .execute(
                "UPDATE local_mv_links SET offset_ms=?2,updated_at=?3 WHERE track_key=?1",
                params![key, offset_ms, now_seconds()],
            )
            .map_err(|e| e.to_string())?;
        if updated == 0 {
            return Err("本地 MV 关联不存在".into());
        }
        Ok(())
    }

    pub fn mv_link(&self, key: &str) -> AppResult<Option<MvLink>> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.mv_link.lock", error);
            })
            .map_err(|_| "MV 关联锁不可用")?
            .query_row(
                "SELECT source,mv_id,title,offset_ms FROM mv_links WHERE track_key=?1",
                [key],
                |row| {
                    Ok(MvLink {
                        source: row.get(0)?,
                        mv_id: row.get(1)?,
                        title: row.get(2)?,
                        offset_ms: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn link_mv(&self, key: &str, source: &str, mv_id: &str, title: &str) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.link_mv.lock", error);
            })
            .map_err(|_| "MV 关联锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM local_mv_links WHERE track_key=?1", [key])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO mv_links(track_key,source,mv_id,title,offset_ms,updated_at) VALUES(?1,?2,?3,?4,0,?5)
             ON CONFLICT(track_key) DO UPDATE SET source=excluded.source,mv_id=excluded.mv_id,title=excluded.title,offset_ms=0,updated_at=excluded.updated_at",
            params![key, source, mv_id, title, now_seconds()],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn unlink_mv(&self, key: &str) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.unlink_mv.lock", error);
            })
            .map_err(|_| "MV 关联锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM local_mv_links WHERE track_key=?1", [key])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM mv_links WHERE track_key=?1", [key])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn set_mv_offset(&self, key: &str, offset_ms: i64) -> AppResult<()> {
        let updated = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.set_mv_offset.lock", error);
            })
            .map_err(|_| "MV 关联锁不可用")?
            .execute(
                "UPDATE mv_links SET offset_ms=?2,updated_at=?3 WHERE track_key=?1",
                params![key, offset_ms, now_seconds()],
            )
            .map_err(|e| e.to_string())?;
        if updated == 0 {
            return Err("MV 关联不存在".into());
        }
        Ok(())
    }

    pub fn save_player(&self, state: &PlayerSnapshot) -> AppResult<()> {
        self.set_setting(
            "player",
            &serde_json::to_string(state).map_err(|e| e.to_string())?,
        )
    }

    pub fn cached_lyrics(
        &self,
        scope: LyricCacheScope,
        key: &str,
    ) -> AppResult<Option<CachedLyrics>> {
        let table = scope.table();
        let db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.cached_lyrics.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?;
        let row: Option<(Option<String>, i64)> = db
            .query_row(
                &format!("SELECT value,expires FROM {table} WHERE key=?1"),
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some((value, expires)) = row {
            db.execute(
                &format!("UPDATE {table} SET accessed=?2 WHERE key=?1"),
                params![key, now_millis()],
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

    /// `expires: None` keeps a found lyric until capacity eviction or an
    /// explicit refresh. A miss passes its retry window explicitly.
    pub fn cache_lyrics(
        &self,
        scope: LyricCacheScope,
        key: &str,
        value: Option<&Lyrics>,
        expires: Option<i64>,
        max_bytes: i64,
    ) -> AppResult<()> {
        let value = value
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?;
        let size = value.as_ref().map_or(0, |s| s.len());
        if size > MAX_LYRIC_BYTES {
            return Err("歌词文件过大".into());
        }
        let table = scope.table();
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.cache_lyrics.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            &format!(
                "INSERT INTO {table}(key,value,expires,accessed,bytes) VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value,expires=excluded.expires,accessed=excluded.accessed,bytes=excluded.bytes"
            ),
            params![
                key,
                value,
                expires.unwrap_or(i64::MAX),
                now_millis(),
                size as i64
            ],
        )
        .map_err(|e| e.to_string())?;
        prune_lyrics(&tx, table, max_bytes)?;
        tx.commit().map_err(|e| e.to_string())?;
        if scope == LyricCacheScope::Persistent {
            db.execute_batch("PRAGMA incremental_vacuum(32)")
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn prune_lyric_cache(&self, max_bytes: i64) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.prune_lyric_cache.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        prune_lyrics(&tx, LyricCacheScope::Persistent.table(), max_bytes)?;
        tx.commit().map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA incremental_vacuum(32)")
            .map_err(|e| e.to_string())
    }

    pub fn lyric_cache_usage(&self) -> AppResult<(i64, i64)> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.lyric_cache_usage.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?
            .query_row(
                "SELECT COALESCE(SUM(bytes),0),COUNT(*) FROM lyric_cache",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())
    }

    pub fn invalidate_lyrics(&self, scope: LyricCacheScope, key: &str) -> AppResult<()> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.invalidate_lyrics.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?
            .execute(
                &format!("DELETE FROM {} WHERE key=?1", scope.table()),
                [key],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_lyric_cache(&self, scope: LyricCacheScope) -> AppResult<()> {
        let db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("storage.clear_lyric_cache.lock", error);
            })
            .map_err(|_| "歌词缓存锁不可用")?;
        db.execute(&format!("DELETE FROM {}", scope.table()), [])
            .map_err(|e| e.to_string())?;
        if scope == LyricCacheScope::Persistent {
            db.execute_batch("PRAGMA incremental_vacuum(32)")
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn has_column(connection: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, String>(1)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn prune_lyrics(tx: &rusqlite::Transaction<'_>, table: &str, max_bytes: i64) -> AppResult<()> {
    loop {
        let (bytes, count): (i64, i64) = tx
            .query_row(
                &format!("SELECT COALESCE(SUM(bytes),0),COUNT(*) FROM {table}"),
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        if bytes <= max_bytes || count == 0 {
            return Ok(());
        }
        tx.execute(
            &format!(
                "DELETE FROM {table} WHERE key=(SELECT key FROM {table} ORDER BY accessed,key LIMIT 1)"
            ),
            [],
        )
        .map_err(|e| e.to_string())?;
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
    fn download_links_replace_the_previous_file_and_survive_as_store_data() {
        let store = Store::initialize(Connection::open_in_memory().unwrap()).unwrap();
        assert!(store.download_local("netease/track/1").unwrap().is_none());
        store
            .link_download("netease/track/1", r"C:\music\first.flac", Some("lossless"))
            .unwrap();
        let first = store.download_local("netease/track/1").unwrap().unwrap();
        assert_eq!(first.path, r"C:\music\first.flac");
        assert_eq!(first.quality.as_deref(), Some("lossless"));
        store
            .link_download("netease/track/1", r"C:\music\second.mp3", Some("higher"))
            .unwrap();
        let second = store.download_local("netease/track/1").unwrap().unwrap();
        assert_eq!(second.path, r"C:\music\second.mp3");
        assert_eq!(second.quality.as_deref(), Some("higher"));
    }

    #[test]
    fn absence_expiry_and_permanent_results_are_distinct() {
        let store = Store::initialize(Connection::open_in_memory().unwrap()).unwrap();
        let scope = LyricCacheScope::Persistent;
        assert!(store.cached_lyrics(scope, "amll:1").unwrap().is_none());
        store
            .cache_lyrics(
                scope,
                "amll:1",
                None,
                Some(now_seconds() + 86400),
                1024 * 1024,
            )
            .unwrap();
        let miss = store.cached_lyrics(scope, "amll:1").unwrap().unwrap();
        assert!(miss.fresh && miss.value.is_none());
        let lyrics = Lyrics {
            match_score: None,
            source: "amll".into(),
            format: "ttml".into(),
            content: "example".into(),
            translation: None,
            romanization: None,
        };
        store
            .cache_lyrics(
                scope,
                "amll:2",
                Some(&lyrics),
                Some(now_seconds() - 1),
                1024 * 1024,
            )
            .unwrap();
        let stale = store.cached_lyrics(scope, "amll:2").unwrap().unwrap();
        assert!(!stale.fresh && stale.value.is_some());
        store
            .cache_lyrics(scope, "amll:3", Some(&lyrics), None, 1024 * 1024)
            .unwrap();
        let permanent = store.cached_lyrics(scope, "amll:3").unwrap().unwrap();
        assert!(permanent.fresh && permanent.value.is_some());
        let session = LyricCacheScope::Session;
        store
            .cache_lyrics(session, "amll:4", Some(&lyrics), None, 1024 * 1024)
            .unwrap();
        assert!(store.cached_lyrics(session, "amll:4").unwrap().is_some());
    }
    #[test]
    fn listening_statistics_accumulate_seconds_days_and_plays() {
        use crate::model::TrackSource;
        let store = Store::initialize(Connection::open_in_memory().unwrap()).unwrap();
        let track = Track {
            key: "local:a".into(),
            title: "A".into(),
            aliases: vec![],
            artist: "".into(),
            artists: vec![],
            album: "".into(),
            duration_ms: 1000,
            cover: "".into(),
            source: TrackSource::Local {
                path: "a.mp3".into(),
                netease_id: None,
                cue: None,
            },
        };
        let at = 1_700_000_000;
        assert!(store.record_playback("first", &track, at).unwrap());
        assert!(!store.record_playback("first", &track, at).unwrap());
        store.add_listening_seconds(at, 90).unwrap();
        store.add_listening_seconds(at, 30).unwrap();
        let statistics = store.listening_statistics().unwrap();
        assert_eq!(statistics.seconds, 120);
        assert_eq!(statistics.days, 1);
        assert_eq!(statistics.plays, 1);
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
        assert!(store
            .cache_lyrics(
                LyricCacheScope::Persistent,
                "amll:1",
                Some(&lyrics),
                None,
                1024 * 1024
            )
            .is_err());
        assert!(store
            .cached_lyrics(LyricCacheScope::Persistent, "amll:1")
            .unwrap()
            .is_none());
    }
}
