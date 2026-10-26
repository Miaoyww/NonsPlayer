//! Persistent membership snapshots; song metadata remains in the shared tracks table.
use super::*;
use rusqlite::{params, Connection, OptionalExtension};

pub(crate) fn failure(error: impl std::fmt::Display) -> MusicError {
    crate::infrastructure::diagnostics::warn("collection.store", error);
    ErrorCode::Internal.into()
}
pub(crate) fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("BEGIN;
        CREATE TABLE IF NOT EXISTS music_collection_snapshots (
            id TEXT PRIMARY KEY, identity TEXT NOT NULL, source TEXT NOT NULL, account TEXT NOT NULL,
            entity TEXT NOT NULL, scope TEXT NOT NULL, run TEXT NOT NULL,
            total INTEGER CHECK(total>=0), description TEXT, loaded INTEGER NOT NULL CHECK(loaded BETWEEN 0 AND 100000), pages INTEGER NOT NULL CHECK(pages BETWEEN 0 AND 1000),
            complete INTEGER NOT NULL, cursor TEXT, updated INTEGER NOT NULL, accessed INTEGER NOT NULL,
            active INTEGER NOT NULL, bytes INTEGER NOT NULL);
        CREATE UNIQUE INDEX IF NOT EXISTS music_collection_active ON music_collection_snapshots(identity) WHERE active=1;
        CREATE TABLE IF NOT EXISTS music_collection_tracks (
            snapshot TEXT NOT NULL, position INTEGER NOT NULL, key TEXT NOT NULL,
            PRIMARY KEY(snapshot,position), UNIQUE(snapshot,key));
        ")?;
    db.execute_batch("COMMIT;")
}

impl Collections {
    pub(crate) async fn cache_status(
        &self,
    ) -> MusicResult<crate::infrastructure::other_cache::Usage> {
        self.database(|db| {
            db.query_row(
                "SELECT COUNT(*),COALESCE(SUM(bytes),0) FROM music_collection_snapshots",
                [],
                |row| {
                    Ok(crate::infrastructure::other_cache::Usage {
                        entries: row.get::<_, i64>(0)?.max(0) as u64,
                        used_bytes: row.get::<_, i64>(1)?.max(0) as u64,
                    })
                },
            )
            .map_err(failure)
        })
        .await
    }

    pub(crate) async fn clear_cache(self: &Arc<Self>) -> MusicResult<()> {
        let this = self.clone();
        self.database(move |db| {
            // Keep coordinators alive so existing cursors cannot acquire a fresh identity.
            let states = this.states.lock().map_err(failure)?;
            for state in states.values() {
                state.epoch.store(
                    this.epochs.fetch_add(1, Ordering::AcqRel),
                    Ordering::Release,
                );
            }
            let tx = db.transaction().map_err(failure)?;
            tx.execute("DELETE FROM music_collection_tracks", [])
                .map_err(failure)?;
            tx.execute("DELETE FROM music_collection_snapshots", [])
                .map_err(failure)?;
            tx.commit().map_err(failure)?;
            Ok(())
        })
        .await
    }

    pub(super) async fn invalidate_snapshot(&self, snapshot: String) -> MusicResult<()> {
        self.database(move |db| {
            db.execute(
                "UPDATE music_collection_snapshots SET active=0 WHERE id=?1",
                [snapshot],
            )
            .map_err(failure)?;
            Ok(())
        })
        .await
    }

    pub(super) async fn database<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection) -> MusicResult<T> + Send + 'static,
    ) -> MusicResult<T> {
        let store = self.store.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut db = store.0.lock().map_err(failure)?;
            work(&mut db)
        })
        .await
        .map_err(failure)?
    }

    pub(super) async fn snapshot(
        &self,
        identity: String,
        id: Option<String>,
    ) -> MusicResult<Option<Snapshot>> {
        self.database(move |db| {
            let sql = "SELECT id,scope,run,total,description,loaded,pages,complete,cursor,updated FROM music_collection_snapshots WHERE identity=?1 AND ((?2 IS NULL AND active=1) OR id=?2)";
            let snapshot = db.query_row(sql, params![identity, id], |r| Ok(Snapshot {
                id:r.get(0)?, scope:r.get(1)?, run:r.get(2)?, total:r.get::<_,Option<i64>>(3)?.map(|value|value as u64), description:r.get(4)?,
                loaded:r.get::<_,u32>(5)? as usize, pages:r.get::<_,u32>(6)? as usize, complete:r.get(7)?, cursor:r.get(8)?, updated:r.get(9)?,
            })).optional().map_err(failure)?;
            if let Some(snapshot) = &snapshot {
                db.execute("UPDATE music_collection_snapshots SET accessed=?1 WHERE id=?2",params![crate::storage::now_seconds(),snapshot.id]).map_err(failure)?;
            }
            Ok(snapshot)
        }).await
    }

    pub(super) async fn tracks(
        &self,
        id: String,
        offset: usize,
        limit: usize,
    ) -> MusicResult<Vec<Track>> {
        self.database(move |db| {
            let mut query = db.prepare_cached("SELECT t.value FROM music_collection_tracks c JOIN tracks t ON t.key=c.key WHERE c.snapshot=?1 AND c.position>=?2 ORDER BY c.position LIMIT ?3").map_err(failure)?;
            let rows = query.query_map(params![id,offset as i64,limit as i64], |r| r.get::<_,String>(0)).map_err(failure)?;
            rows.map(|row| serde_json::from_str(&row.map_err(failure)?).map_err(failure)).collect()
        }).await
    }

    pub(super) async fn append(
        &self,
        key: Key,
        scope: String,
        state: Arc<KeyState>,
        epoch: u64,
        previous: Option<Snapshot>,
        page: business::TrackPage,
        publish: bool,
    ) -> MusicResult<Snapshot> {
        let adapters = self.adapters.clone();
        let run = self.run.clone();
        self.database(move |db| {
            let check = || -> MusicResult<()> {
                if state.epoch.load(Ordering::Acquire)!=epoch || adapters.cache_identity(&key.entity.source)?!=scope {
                    return Err(ErrorCode::StaleContext.into());
                }
                Ok(())
            };
            check()?;
            let tx = db.transaction().map_err(failure)?;
            let mut snapshot = previous.unwrap_or_else(|| Snapshot {
                id:uuid::Uuid::new_v4().to_string(), scope:scope.clone(), run:run.clone(), total:None,
                description:None, loaded:0, pages:0, complete:false, cursor:None, updated:0,
            });
            let old_bytes:i64=tx.query_row("SELECT bytes FROM music_collection_snapshots WHERE id=?1",[&snapshot.id],|row|row.get(0)).optional().map_err(failure)?.unwrap_or(0);
            let old_overhead=serde_json::to_vec(&(&key.identity,&scope,&snapshot.description,&snapshot.cursor)).map_err(failure)?.len() as i64;
            let mut relation_bytes=old_bytes.saturating_sub(old_overhead).max(0);
            let tracks = page.page.items.into_iter().map(Track::from_music).collect::<MusicResult<Vec<_>>>()?;
            crate::storage::Store::save_tracks_in(&tx,&tracks).map_err(failure)?;
            for track in tracks {
                let inserted = tx.execute("INSERT OR IGNORE INTO music_collection_tracks(snapshot,position,key) VALUES(?1,?2,?3)",params![snapshot.id,snapshot.loaded as i64,track.key]).map_err(failure)?;
                if inserted!=0 {relation_bytes+=serde_json::to_vec(&(&snapshot.id,snapshot.loaded,&track.key)).map_err(failure)?.len() as i64;}
                snapshot.loaded += inserted;
            }
            snapshot.pages += 1;
            snapshot.complete = page.page.next_cursor.is_none();
            snapshot.cursor = page.page.next_cursor.map(|cursor|cursor.as_str().to_owned());
            snapshot.total = page.total.or(snapshot.total);
            snapshot.description = page.description.or(snapshot.description);
            snapshot.updated = crate::storage::now_seconds();
            if publish {
                tx.execute("UPDATE music_collection_snapshots SET active=0 WHERE identity=?1",[&key.identity]).map_err(failure)?;
            }
            let overhead=serde_json::to_vec(&(&key.identity,&scope,&snapshot.description,&snapshot.cursor)).map_err(failure)?.len() as i64;
            let total=snapshot.total.map(i64::try_from).transpose().map_err(|_|MusicError::from(ErrorCode::InvalidData))?;
            tx.execute("INSERT INTO music_collection_snapshots(id,identity,source,account,entity,scope,run,total,description,loaded,pages,complete,cursor,updated,accessed,active,bytes)
                VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?14,?15,?16)
                ON CONFLICT(id) DO UPDATE SET total=excluded.total,description=excluded.description,loaded=excluded.loaded,pages=excluded.pages,complete=excluded.complete,cursor=excluded.cursor,updated=excluded.updated,accessed=excluded.accessed,active=MAX(music_collection_snapshots.active,excluded.active),bytes=excluded.bytes",
                params![snapshot.id,key.identity,key.entity.source.as_str(),key.account,key.entity.id.as_str(),scope,run,total,snapshot.description,snapshot.loaded as i64,snapshot.pages as i64,snapshot.complete,snapshot.cursor,snapshot.updated,publish,relation_bytes+overhead]).map_err(failure)?;
            prune(&tx,&snapshot.id,&key.identity)?;
            check()?;
            tx.commit().map_err(failure)?;
            check()?;
            Ok(snapshot)
        }).await
    }

    pub async fn invalidate_source(&self, source: identity::SourceId) -> MusicResult<()> {
        // Admission has already advanced the host write revision. Do not wait on a network gate.
        self.database(move |db| {
            db.execute(
                "UPDATE music_collection_snapshots SET active=0 WHERE source=?1",
                [source.as_str()],
            )
            .map_err(failure)?;
            Ok(())
        })
        .await
    }

    pub(super) async fn promote(
        &self,
        key: Key,
        scope: String,
        state: Arc<KeyState>,
        epoch: u64,
        id: String,
    ) -> MusicResult<()> {
        let adapters = self.adapters.clone();
        self.database(move |db| {
            let check = || -> MusicResult<()> {
                if state.epoch.load(Ordering::Acquire) != epoch
                    || adapters.cache_identity(&key.entity.source)? != scope
                {
                    return Err(ErrorCode::StaleContext.into());
                }
                Ok(())
            };
            check()?;
            let tx = db.transaction().map_err(failure)?;
            tx.execute(
                "UPDATE music_collection_snapshots SET active=0 WHERE identity=?1",
                [&key.identity],
            )
            .map_err(failure)?;
            let changed = tx
                .execute(
                    "UPDATE music_collection_snapshots SET active=1 WHERE id=?1 AND identity=?2",
                    params![id, key.identity],
                )
                .map_err(failure)?;
            if changed != 1 {
                return Err(ErrorCode::StaleContext.into());
            }
            check()?;
            tx.commit().map_err(failure)?;
            check()
        })
        .await
    }

    pub async fn remove_account(&self, reference: account::AccountRef) -> MusicResult<()> {
        self.database(move |db| {
            let tx=db.transaction().map_err(failure)?;
            tx.execute("DELETE FROM music_collection_tracks WHERE snapshot IN (SELECT id FROM music_collection_snapshots WHERE source=?1 AND account=?2)",params![reference.source.as_str(),reference.id.as_str()]).map_err(failure)?;
            tx.execute("DELETE FROM music_collection_snapshots WHERE source=?1 AND account=?2",params![reference.source.as_str(),reference.id.as_str()]).map_err(failure)?;
            tx.commit().map_err(failure)?;
            Ok(())
        }).await
    }
}

fn prune(db: &Connection, keep: &str, identity: &str) -> MusicResult<()> {
    loop {
        let (count, bytes): (i64, i64) = db
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(bytes),0) FROM music_collection_snapshots",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(failure)?;
        if count <= 128 && bytes <= 32 * 1024 * 1024 {
            return Ok(());
        }
        let oldest:Option<String>=db.query_row("SELECT id FROM music_collection_snapshots WHERE id<>?1 AND NOT(identity=?2 AND active=1) ORDER BY accessed,id LIMIT 1",params![keep,identity],|r|r.get(0)).optional().map_err(failure)?;
        let Some(oldest) = oldest else {
            return Err(ErrorCode::InvalidData.into());
        };
        db.execute(
            "DELETE FROM music_collection_tracks WHERE snapshot=?1",
            [&oldest],
        )
        .map_err(failure)?;
        db.execute(
            "DELETE FROM music_collection_snapshots WHERE id=?1",
            [oldest],
        )
        .map_err(failure)?;
    }
}
