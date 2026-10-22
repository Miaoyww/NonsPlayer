use crate::music::{account::AccountRef, identity::EntityRef};
use crate::{
    model::{AppResult, Track},
    storage::{now_seconds, Store},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub(crate) const MAX_TRACKS: usize = 10000;
pub(crate) fn queue(store: &Store, id: &str) -> AppResult<(Vec<Track>, u32)> {
    let db = store.0.lock().map_err(|_| "曲库锁不可用")?;
    exists(&db, id)?;
    let mut statement = db.prepare("SELECT COALESCE(t.value,p.value), CASE WHEN json_extract(COALESCE(t.value,p.value),'$.source.kind')='local' AND NOT EXISTS(SELECT 1 FROM local_origins o WHERE o.key=p.key) THEN 1 ELSE 0 END FROM local_playlist_tracks p LEFT JOIN tracks t ON t.key=p.key WHERE p.playlist_id=?1 AND p.excluded=0 ORDER BY p.position,p.key LIMIT 10001").map_err(|e|e.to_string())?;
    let rows = statement
        .query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut tracks = Vec::new();
    let mut skipped = 0;
    for row in rows {
        let (value, unavailable) = row.map_err(|e| e.to_string())?;
        if unavailable {
            skipped += 1
        } else {
            tracks.push(serde_json::from_str(&value).map_err(|e| e.to_string())?);
        }
    }
    if tracks.len() + skipped as usize > MAX_TRACKS {
        return Err("歌单超过 10000 首".into());
    }
    Ok((tracks, skipped))
}
pub(crate) fn migrate(db: &Connection) -> rusqlite::Result<()> {
    let columns = db
        .prepare("PRAGMA table_info(local_playlist_tracks)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    for (name, definition) in [
        ("manual", "INTEGER NOT NULL DEFAULT 1"),
        ("remote", "INTEGER NOT NULL DEFAULT 0"),
        ("excluded", "INTEGER NOT NULL DEFAULT 0"),
        ("value", "TEXT"),
    ] {
        if !columns.iter().any(|v| v == name) {
            db.execute_batch(&format!(
                "ALTER TABLE local_playlist_tracks ADD COLUMN {name} {definition};"
            ))?;
        }
    }
    db.execute_batch("UPDATE local_playlist_tracks SET value=COALESCE((SELECT value FROM tracks WHERE tracks.key=local_playlist_tracks.key),json_object('key',key,'title','已退出曲库的歌曲','artist','未知艺术家','album','','durationMs',0,'cover','','source',json_object('kind','local','path',''))) WHERE value IS NULL;
        CREATE INDEX IF NOT EXISTS aggregate_member_order ON local_playlist_tracks(playlist_id,position);
        CREATE TABLE IF NOT EXISTS aggregate_links(playlist_id INTEGER PRIMARY KEY, source TEXT NOT NULL, account TEXT NOT NULL, entity TEXT NOT NULL, reference TEXT NOT NULL, last_success INTEGER NOT NULL DEFAULT 0, last_attempt INTEGER NOT NULL DEFAULT 0, error TEXT, UNIQUE(source,account,entity));
        DELETE FROM local_playlist_tracks WHERE excluded=1 AND remote=0;
        UPDATE local_playlist_tracks SET manual=0,remote=1,excluded=0 WHERE playlist_id IN (SELECT playlist_id FROM aggregate_links) AND (remote=1 OR COALESCE(json_extract(value,'$.source.track.reference.source'),json_extract(value,'$.source.kind'))=(SELECT source FROM aggregate_links WHERE playlist_id=local_playlist_tracks.playlist_id));")
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub reference: EntityRef,
    pub account: AccountRef,
    pub last_success: i64,
    pub last_attempt: i64,
    pub error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub covers: Vec<String>,
    pub track_count: u32,
    pub link: Option<Link>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub track: Track,
    pub remote: bool,
    pub unavailable: Option<String>,
}
#[derive(Serialize)]
pub struct ReadPage {
    pub items: Vec<Member>,
    pub more: bool,
}
#[derive(Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Write {
    Create {
        name: String,
        #[serde(default)]
        keys: Vec<String>,
    },
    Rename {
        id: String,
        name: String,
    },
    Delete {
        id: String,
    },
    Unlink {
        id: String,
    },
    Add {
        id: String,
        keys: Vec<String>,
    },
    Remove {
        id: String,
        keys: Vec<String>,
    },
    Reorder {
        id: String,
        keys: Vec<String>,
    },
}
fn name(value: &str) -> AppResult<&str> {
    let v = value.trim();
    if v.is_empty() || v.chars().count() > 100 {
        Err("歌单名称需要 1–100 个字符".into())
    } else {
        Ok(v)
    }
}
pub(crate) fn linked(db: &Connection, id: &str) -> AppResult<Option<Link>> {
    db.query_row("SELECT reference,source,account,last_success,last_attempt,error FROM aggregate_links WHERE playlist_id=?1", [id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(|e|e.to_string())?.map(|(reference,source,account,last_success,last_attempt,error)| Ok(Link {reference:serde_json::from_str(&reference).map_err(|e|e.to_string())?,account:AccountRef{source:source.try_into().map_err(|e:crate::music::MusicError|e.to_string())?,id:account.try_into().map_err(|e:crate::music::MusicError|e.to_string())?},last_success,last_attempt,error})).transpose()
}
fn summary(db: &Connection, id: String, name: String) -> AppResult<Playlist> {
    let track_count = db
        .query_row(
            "SELECT COUNT(*) FROM local_playlist_tracks WHERE playlist_id=?1 AND excluded=0",
            [&id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let covers=db.prepare("SELECT DISTINCT COALESCE(json_extract(COALESCE(t.value,p.value),'$.cover.original'),json_extract(COALESCE(t.value,p.value),'$.cover')) FROM local_playlist_tracks p LEFT JOIN tracks t ON t.key=p.key WHERE p.playlist_id=?1 AND p.excluded=0 AND COALESCE(json_extract(COALESCE(t.value,p.value),'$.cover.original'),json_extract(COALESCE(t.value,p.value),'$.cover'),'')<>'' ORDER BY p.position LIMIT 4").map_err(|e|e.to_string())?.query_map([&id],|r|r.get(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(Playlist {
        link: linked(db, &id)?,
        id,
        name,
        covers,
        track_count,
    })
}
pub(crate) fn playlists(store: &Store) -> AppResult<Vec<Playlist>> {
    let db = store.0.lock().map_err(|_| "曲库锁不可用")?;
    let rows = db
        .prepare("SELECT CAST(id AS TEXT),name FROM local_playlists ORDER BY id DESC")
        .map_err(|e| e.to_string())?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<(String, String)>, _>>()
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|(id, name)| summary(&db, id, name))
        .collect()
}
pub(crate) fn playlist(store: &Store, id: &str) -> AppResult<Playlist> {
    let db = store.0.lock().map_err(|_| "曲库锁不可用")?;
    let name = db
        .query_row("SELECT name FROM local_playlists WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    summary(&db, id.into(), name)
}
pub(crate) fn tracks(store: &Store, id: &str, offset: u32, excluded: bool) -> AppResult<ReadPage> {
    let db = store.0.lock().map_err(|_| "曲库锁不可用")?;
    let mut statement=db.prepare("SELECT COALESCE(t.value,p.value),p.manual,p.remote,CASE WHEN json_extract(COALESCE(t.value,p.value),'$.source.kind')='local' AND NOT EXISTS(SELECT 1 FROM local_origins o WHERE o.key=p.key) THEN '本地歌曲已退出曲库' END FROM local_playlist_tracks p LEFT JOIN tracks t ON t.key=p.key WHERE p.playlist_id=?1 AND p.excluded=?2 ORDER BY p.position,p.key LIMIT 101 OFFSET ?3").map_err(|e|e.to_string())?;
    let rows = statement
        .query_map(params![id, excluded, offset], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, bool>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        let (value, _manual, remote, unavailable) = row.map_err(|e| e.to_string())?;
        if let Some(value) = value {
            items.push(Member {
                track: serde_json::from_str(&value).map_err(|e| e.to_string())?,
                remote,
                unavailable,
            });
        }
    }
    let more = items.len() > 100;
    items.truncate(100);
    Ok(ReadPage { items, more })
}
fn exists(db: &Connection, id: &str) -> AppResult<()> {
    let found: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM local_playlists WHERE id=?1)",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if found {
        Ok(())
    } else {
        Err("歌单不存在".into())
    }
}
fn capacity(db: &Connection, id: &str) -> AppResult<()> {
    let count: u32 = db
        .query_row(
            "SELECT COUNT(*) FROM local_playlist_tracks WHERE playlist_id=?1 AND excluded=0",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if count as usize > MAX_TRACKS {
        Err("一个歌单最多保存 10000 首歌曲".into())
    } else {
        Ok(())
    }
}
pub(crate) fn create(db: &Connection, value: &str) -> AppResult<String> {
    let count: u32 = db
        .query_row("SELECT COUNT(*) FROM local_playlists", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count >= 200 {
        return Err("最多创建 200 个聚合歌单".into());
    }
    db.execute(
        "INSERT INTO local_playlists(name) VALUES(?1)",
        [name(value)?],
    )
    .map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid().to_string())
}
pub(crate) fn write(store: &Store, request: &Write) -> AppResult<String> {
    let mut db = store.0.lock().map_err(|_| "曲库锁不可用")?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let keys = match request {
        Write::Create { keys, .. }
        | Write::Add { keys, .. }
        | Write::Remove { keys, .. }
        | Write::Reorder { keys, .. } => Some(keys),
        _ => None,
    };
    if keys.is_some_and(|keys| keys.len() > MAX_TRACKS) {
        return Err("一次最多处理 10000 首歌曲".into());
    }
    let id = match request {
        Write::Create { name, .. } => create(&tx, name)?,
        Write::Rename { id, .. }
        | Write::Delete { id }
        | Write::Unlink { id }
        | Write::Add { id, .. }
        | Write::Remove { id, .. }
        | Write::Reorder { id, .. } => {
            exists(&tx, id)?;
            id.clone()
        }
    };
    match request {
        Write::Create { keys, .. } | Write::Add { keys, .. } => {
            let link = linked(&tx, &id)?;
            // Prepend new members as one ordered batch; conflicts retain their existing position.
            let first: i64 = tx.query_row("SELECT COALESCE(MIN(position),0) FROM local_playlist_tracks WHERE playlist_id=?1",[&id],|row| row.get(0)).map_err(|e|e.to_string())?;
            let first = first
                .checked_sub(keys.len() as i64)
                .ok_or("歌单位置超出范围")?;
            for (index, key) in keys.iter().enumerate() {
                let value:String=tx.query_row("SELECT value FROM tracks WHERE key=?1",[key],|r|r.get(0)).optional().map_err(|e|e.to_string())?.or(tx.query_row("SELECT value FROM local_playlist_tracks WHERE playlist_id=?1 AND key=?2",params![id,key],|r|r.get(0)).optional().map_err(|e|e.to_string())?.flatten()).ok_or("歌曲不存在")?;
                let track: Track = serde_json::from_str(&value).map_err(|e| e.to_string())?;
                let remote = link
                    .as_ref()
                    .is_some_and(|link| link.reference.source.as_str() == track.source_id());
                tx.execute("INSERT INTO local_playlist_tracks(playlist_id,key,position,manual,remote,excluded,value) VALUES(?1,?2,?6,?4,?5,0,?3) ON CONFLICT(playlist_id,key) DO UPDATE SET manual=?4,remote=MAX(remote,?5),excluded=0,value=excluded.value",params![id,key,value,!remote,remote,first + index as i64]).map_err(|e|e.to_string())?;
            }
        }
        Write::Remove { keys, .. } => {
            for key in keys {
                let remote: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM local_playlist_tracks WHERE playlist_id=?1 AND key=?2 AND remote=1)",params![id,key],|row| row.get(0)).map_err(|e|e.to_string())?;
                if remote {
                    return Err("原歌单歌曲始终跟随平台，请在原歌单中删除".into());
                }
                tx.execute(
                    "DELETE FROM local_playlist_tracks WHERE playlist_id=?1 AND key=?2",
                    params![id, key],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Write::Rename { name, .. } => {
            if linked(&tx, &id)?.is_some() {
                return Err("派生歌单名称跟随原歌单，解除关联后可改名".into());
            }
            tx.execute(
                "UPDATE local_playlists SET name=?1 WHERE id=?2",
                params![name.trim(), id],
            )
            .map_err(|e| e.to_string())?;
            self::name(name)?;
        }
        Write::Delete { .. } => {
            tx.execute(
                "DELETE FROM local_playlist_tracks WHERE playlist_id=?1",
                [&id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM aggregate_links WHERE playlist_id=?1", [&id])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM local_playlists WHERE id=?1", [&id])
                .map_err(|e| e.to_string())?;
        }
        Write::Unlink { .. } => {
            tx.execute("DELETE FROM aggregate_links WHERE playlist_id=?1", [&id])
                .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM local_playlist_tracks WHERE playlist_id=?1 AND excluded=1",
                [&id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE local_playlist_tracks SET manual=1,remote=0 WHERE playlist_id=?1",
                [&id],
            )
            .map_err(|e| e.to_string())?;
        }
        Write::Reorder { keys, .. } => {
            let current = tx
                .prepare(
                    "SELECT key FROM local_playlist_tracks WHERE playlist_id=?1 AND excluded=0",
                )
                .map_err(|e| e.to_string())?
                .query_map([&id], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<HashSet<_>, _>>()
                .map_err(|e| e.to_string())?;
            let wanted = keys.iter().cloned().collect::<HashSet<_>>();
            if wanted.len() != keys.len() || wanted != current {
                return Err("歌单已变化，请重新读取后排序".into());
            }
            for (position, key) in keys.iter().enumerate() {
                tx.execute(
                    "UPDATE local_playlist_tracks SET position=?1 WHERE playlist_id=?2 AND key=?3",
                    params![position as u32, id, key],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    capacity(&tx, &id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}
#[cfg(test)]
pub(crate) fn apply(
    store: &Store,
    id: Option<&str>,
    link: &Link,
    title: &str,
    tracks: &[Track],
) -> AppResult<String> {
    apply_checked(store, id, link, title, tracks, || Ok(()))
}
pub(crate) fn apply_checked(
    store: &Store,
    id: Option<&str>,
    link: &Link,
    title: &str,
    tracks: &[Track],
    validate: impl Fn() -> AppResult<()>,
) -> AppResult<String> {
    if tracks.len() > MAX_TRACKS {
        return Err("原歌单超过 10000 首，未应用同步".into());
    }
    let mut db = store.0.lock().map_err(|_| "曲库锁不可用")?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    validate()?;
    let id = if let Some(id) = id {
        exists(&tx, id)?;
        let old = linked(&tx, id)?.ok_or("关联已解除")?;
        if old.reference != link.reference || old.account != link.account {
            return Err("同步关联已变化".into());
        }
        id.into()
    } else {
        create(&tx, title)?
    };
    Store::save_tracks_in(&tx, tracks)?;
    tx.execute(
        "UPDATE local_playlist_tracks SET manual=0 WHERE playlist_id=?1 AND (remote=1 OR COALESCE(json_extract(value,'$.source.track.reference.source'),json_extract(value,'$.source.kind'))=?2)",
        params![id,link.reference.source.as_str()],
    ).map_err(|e|e.to_string())?;
    tx.execute(
        "UPDATE local_playlist_tracks SET remote=0,excluded=0 WHERE playlist_id=?1",
        [&id],
    )
    .map_err(|e| e.to_string())?;
    for (index, track) in tracks.iter().enumerate() {
        if index % 100 == 0 {
            validate()?;
        }
        tx.execute("INSERT INTO local_playlist_tracks(playlist_id,key,position,manual,remote,excluded,value) VALUES(?1,?2,COALESCE((SELECT MAX(position)+1 FROM local_playlist_tracks WHERE playlist_id=?1),0),0,1,0,?3) ON CONFLICT(playlist_id,key) DO UPDATE SET manual=0,remote=1,excluded=0,value=excluded.value",params![id,track.key,serde_json::to_string(track).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
    }
    tx.execute("DELETE FROM local_playlist_tracks WHERE playlist_id=?1 AND remote=0 AND manual=0 AND excluded=0",[&id]).map_err(|e|e.to_string())?;
    capacity(&tx, &id)?;
    tx.execute(
        "UPDATE local_playlists SET name=?1 WHERE id=?2",
        params![name(title)?, id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO aggregate_links(playlist_id,source,account,entity,reference,last_success,last_attempt) VALUES(?1,?2,?3,?4,?5,?6,?6) ON CONFLICT(playlist_id) DO UPDATE SET last_success=excluded.last_success,last_attempt=excluded.last_attempt,error=NULL",params![id,link.reference.source.as_str(),link.account.id.as_str(),link.reference.id.as_str(),serde_json::to_string(&link.reference).map_err(|e|e.to_string())?,now_seconds()]).map_err(|e|e.to_string())?;
    validate()?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}
