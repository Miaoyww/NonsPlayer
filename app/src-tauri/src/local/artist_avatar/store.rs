use super::{ArtistAvatarItem, ArtistAvatarRecord, ArtistAvatarStatus};
use crate::{
    model::AppResult,
    storage::{now_millis, Store},
};
use rusqlite::{params, OptionalExtension};
use std::{collections::HashMap, path::Path};

impl Store {
    pub(super) fn artist_avatar_record(
        &self,
        artist_id: &str,
    ) -> AppResult<Option<ArtistAvatarRecord>> {
        let db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        db.query_row(
            "SELECT artist_id,status,source,platform_id,platform_name,cover_variants,cover_size,avatar_path,width,height,error,updated_at FROM local_artist_avatars WHERE artist_id=?1",
            [artist_id],
            row_record,
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub(super) fn artist_avatar_name(&self, artist_id: &str) -> AppResult<Option<String>> {
        self.0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT name FROM local_entities WHERE kind='artist' AND id=?1",
                [artist_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub(super) fn artist_avatar_page(
        &self,
        keyword: &str,
        offset: u32,
    ) -> AppResult<(Vec<ArtistAvatarItem>, bool, u64)> {
        let db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        let total = db
            .query_row(
                "SELECT COUNT(DISTINCT e.id) FROM local_entities e JOIN local_members m ON m.kind=e.kind AND m.id=e.id JOIN visible_local_tracks t ON t.key=m.key AND t.source='local' WHERE e.kind='artist' AND instr(lower(e.name),lower(?1))>0",
                [keyword],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| e.to_string())?;
        let mut statement = db
            .prepare(
                "SELECT e.id,e.name,COALESCE(NULLIF(a.avatar_path,''),e.cover),COUNT(t.key),a.status,a.source,a.platform_id,a.platform_name,a.cover_size,a.width,a.height,a.error
                 FROM local_entities e
                 JOIN local_members m ON m.kind=e.kind AND m.id=e.id
                 JOIN visible_local_tracks t ON t.key=m.key AND t.source='local'
                 LEFT JOIN local_artist_avatars a ON a.artist_id=e.id
                 WHERE e.kind='artist' AND instr(lower(e.name),lower(?1))>0
                 GROUP BY e.kind,e.id
                 ORDER BY e.name,e.id LIMIT 31 OFFSET ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![keyword, offset], |row| {
                let status: Option<String> = row.get(4)?;
                Ok(ArtistAvatarItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    cover: row.get(2)?,
                    track_count: row.get(3)?,
                    status: status.as_deref().map(ArtistAvatarStatus::from_storage),
                    source: row.get(5)?,
                    platform_id: row.get(6)?,
                    platform_name: row.get(7)?,
                    avatar_size: row.get(8)?,
                    width: row.get(9)?,
                    height: row.get(10)?,
                    error: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let more = items.len() > 30;
        items.truncate(30);
        Ok((items, more, total.max(0) as u64))
    }

    pub(super) fn artist_avatar_targets(
        &self,
        artist_ids: &[String],
        all: bool,
        force: bool,
        keyword: &str,
    ) -> AppResult<Vec<String>> {
        if artist_ids.len() > 50_000 {
            return Err("一次最多补齐 50000 位艺术家".into());
        }
        let db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        let mut statuses = HashMap::new();
        {
            let mut statement = db
                .prepare("SELECT artist_id,status,avatar_path FROM local_artist_avatars")
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                let (artist_id, status, avatar_path) = row.map_err(|e| e.to_string())?;
                statuses.insert(
                    artist_id,
                    (ArtistAvatarStatus::from_storage(&status), avatar_path),
                );
            }
        }
        let eligible = |artist_id: &str| -> bool {
            let Some((status, avatar_path)) = statuses.get(artist_id) else {
                return true;
            };
            match status {
                ArtistAvatarStatus::Manual | ArtistAvatarStatus::Default => force,
                ArtistAvatarStatus::Auto if all => avatar_path
                    .as_deref()
                    .is_none_or(|path| path.is_empty() || !Path::new(path).is_file()),
                _ => true,
            }
        };
        if all {
            let mut statement = db
                .prepare(
                    "SELECT e.id FROM local_entities e
                     WHERE e.kind='artist' AND instr(lower(e.name),lower(?1))>0 AND EXISTS (
                       SELECT 1 FROM local_members m JOIN visible_local_tracks t ON t.key=m.key
                       WHERE m.kind=e.kind AND m.id=e.id AND t.source='local'
                     )
                     ORDER BY e.name,e.id",
                )
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map([keyword], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            return Ok(rows
                .filter_map(Result::ok)
                .filter(|id| eligible(id))
                .collect());
        }
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for artist_id in artist_ids {
            if !seen.insert(artist_id.clone()) {
                continue;
            }
            let exists: bool = db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM local_entities WHERE kind='artist' AND id=?1)",
                    [artist_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if exists && eligible(artist_id) {
                result.push(artist_id.clone());
            }
        }
        Ok(result)
    }

    pub(super) fn save_artist_avatar_auto(&self, value: &ArtistAvatarRecord) -> AppResult<bool> {
        let db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        Ok(db
            .execute(
                "INSERT INTO local_artist_avatars(artist_id,status,source,platform_id,platform_name,cover_variants,cover_size,avatar_path,width,height,error,updated_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
                 ON CONFLICT(artist_id) DO UPDATE SET
                   status=excluded.status,source=excluded.source,platform_id=excluded.platform_id,
                   platform_name=excluded.platform_name,cover_variants=excluded.cover_variants,
                   cover_size=excluded.cover_size,avatar_path=excluded.avatar_path,width=excluded.width,
                   height=excluded.height,error=excluded.error,updated_at=excluded.updated_at
                 WHERE local_artist_avatars.status NOT IN ('manual','default')",
                params![
                    value.artist_id,
                    value.status.storage(),
                    value.source,
                    value.platform_id,
                    value.platform_name,
                    value.cover_variants,
                    value.cover_size,
                    value.avatar_path,
                    value.width,
                    value.height,
                    value.error,
                    now_millis()
                ],
            )
            .map_err(|e| e.to_string())?
            > 0)
    }

    pub(super) fn save_artist_avatar_replace(&self, value: &ArtistAvatarRecord) -> AppResult<()> {
        self.0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .execute(
                "INSERT INTO local_artist_avatars(artist_id,status,source,platform_id,platform_name,cover_variants,cover_size,avatar_path,width,height,error,updated_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
                 ON CONFLICT(artist_id) DO UPDATE SET
                   status=excluded.status,source=excluded.source,platform_id=excluded.platform_id,
                   platform_name=excluded.platform_name,cover_variants=excluded.cover_variants,
                   cover_size=excluded.cover_size,avatar_path=excluded.avatar_path,width=excluded.width,
                   height=excluded.height,error=excluded.error,updated_at=excluded.updated_at",
                params![
                    value.artist_id,
                    value.status.storage(),
                    value.source,
                    value.platform_id,
                    value.platform_name,
                    value.cover_variants,
                    value.cover_size,
                    value.avatar_path,
                    value.width,
                    value.height,
                    value.error,
                    now_millis()
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(super) fn set_artist_avatar_default(&self, artist_id: &str) -> AppResult<()> {
        self.save_artist_avatar_replace(&ArtistAvatarRecord {
            artist_id: artist_id.to_owned(),
            status: ArtistAvatarStatus::Default,
            ..Default::default()
        })
    }

    pub(super) fn set_artist_avatar_error(&self, artist_id: &str, error: &str) -> AppResult<()> {
        self.0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .execute(
                "UPDATE local_artist_avatars SET error=?2,updated_at=?3 WHERE artist_id=?1",
                params![artist_id, error, now_millis()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(super) fn artist_avatar_path_references(
        &self,
        artist_id: &str,
        path: &str,
    ) -> AppResult<u64> {
        self.0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT COUNT(*) FROM local_artist_avatars WHERE avatar_path=?1 AND artist_id<>?2",
                params![path, artist_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value.max(0) as u64)
            .map_err(|e| e.to_string())
    }

    pub(super) fn artist_avatar_path_referenced(&self, path: &str) -> AppResult<u64> {
        self.0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT COUNT(*) FROM local_artist_avatars WHERE avatar_path=?1",
                [path],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value.max(0) as u64)
            .map_err(|e| e.to_string())
    }
}

fn row_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtistAvatarRecord> {
    let status: String = row.get(1)?;
    Ok(ArtistAvatarRecord {
        artist_id: row.get(0)?,
        status: ArtistAvatarStatus::from_storage(&status),
        source: row.get(2)?,
        platform_id: row.get(3)?,
        platform_name: row.get(4)?,
        cover_variants: row.get(5)?,
        cover_size: row.get(6)?,
        avatar_path: row.get(7)?,
        width: row.get(8)?,
        height: row.get(9)?,
        error: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
