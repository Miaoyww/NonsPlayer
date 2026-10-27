use crate::{
    model::{AppResult, MusicCredit, Track, TrackSource},
    storage::Store,
    Backend,
};
use anyhow::Context;
use lofty::prelude::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalPreferences {
    pub artist_separators: Vec<String>,
}
impl Default for LocalPreferences {
    fn default() -> Self {
        Self {
            artist_separators: vec!["/".into(), "、".into(), ";".into()],
        }
    }
}
pub fn options_from_db(db: &Connection) -> AppResult<LocalPreferences> {
    let value: Option<String> = db
        .query_row(
            "SELECT value FROM settings WHERE key='localPreferences'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(value
        .map(|v| {
            serde_json::from_str(&v)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("local.preferences.decode", error);
                })
                .unwrap_or_default()
        })
        .unwrap_or_default())
}
pub fn preferences(store: &Store) -> AppResult<LocalPreferences> {
    {
        let db = store
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("local.preferences.read.db_lock", error);
            })
            .map_err(|_| "曲库锁不可用")?;
        options_from_db(&db)
    }
}
pub fn split_artists(value: &str, separators: &[String]) -> Vec<String> {
    let mut parts = vec![value.to_string()];
    for separator in separators.iter().filter(|s| !s.is_empty()) {
        parts = parts
            .into_iter()
            .flat_map(|s| s.split(separator).map(str::to_string).collect::<Vec<_>>())
            .collect();
    }
    let mut names = Vec::new();
    for name in parts {
        let name = name.trim();
        if !name.is_empty() && !names.iter().any(|v| v == name) {
            names.push(name.to_string());
        }
    }
    if names.is_empty() {
        names.push("未知艺术家".into());
    }
    names
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalInformation {
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub album_artist: Option<String>,
    pub file_size: u64,
    pub format: String,
}
pub fn read_information(path: &str) -> AppResult<LocalInformation> {
    let (file, _, _) = super::encoded_audio::read_tags(path)?;
    Ok(information(&file, path))
}
pub fn information(file: &lofty::file::TaggedFile, path: &str) -> LocalInformation {
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let properties = file.properties();
    LocalInformation {
        track_number: tag.and_then(|t| t.track()),
        disc_number: tag.and_then(|t| t.disk()),
        bitrate: properties.audio_bitrate(),
        sample_rate: properties.sample_rate(),
        bit_depth: properties.bit_depth(),
        channels: properties.channels(),
        album_artist: tag
            .and_then(|t| t.get_string(ItemKey::AlbumArtist))
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string),
        file_size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        format: std::path::Path::new(path)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_uppercase(),
    }
}
impl Store {
    pub fn local_information(&self, key: &str) -> AppResult<LocalInformation> {
        let value: String = self
            .0
            .lock()
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT value FROM local_metadata WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&value).map_err(|e| e.to_string())
    }

    pub fn has_local_information(&self, key: &str) -> AppResult<bool> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("local.metadata.exists.db_lock", error);
            })
            .map_err(|_| "曲库锁不可用")?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM local_metadata WHERE key=?1)",
                [key],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }
    pub fn save_local_information(&self, key: &str, value: &LocalInformation) -> AppResult<()> {
        self.0.lock().inspect_err(|error| { crate::infrastructure::diagnostics::warn("local.metadata.save.db_lock", error); }).map_err(|_| "曲库锁不可用")?.execute("INSERT INTO local_metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, serde_json::to_string(value).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn configure_local_preferences(&self, options: &LocalPreferences) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "local.preferences.configure.db_lock",
                    error,
                );
            })
            .map_err(|_| "曲库锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let previous = options_from_db(&tx)?;
        tx.execute("INSERT INTO settings(key,value) VALUES('localPreferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(options).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        if previous.artist_separators != options.artist_separators {
            rebuild_index(&tx)?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
}
const LOCAL_INDEX_VERSION: &str = "2";

pub fn initialize_index(db: &Connection) -> AppResult<()> {
    let version: Option<String> = db
        .query_row(
            "SELECT value FROM settings WHERE key='localIndexVersion'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if version.as_deref() == Some(LOCAL_INDEX_VERSION) {
        return Ok(());
    }
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM local_entities WHERE kind IN ('artist','album')",
        [],
    )
    .map_err(|e| e.to_string())?;
    rebuild_index(&tx)?;
    tx.execute("INSERT INTO settings(key,value) VALUES('localIndexVersion',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [LOCAL_INDEX_VERSION]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

fn rebuild_index(tx: &Connection) -> AppResult<()> {
    let options = options_from_db(tx)?;
    let mut invalid_keys = Vec::new();
    tx.execute(
        "DELETE FROM local_members WHERE kind IN ('artist','album')",
        [],
    )
    .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare("SELECT key,value FROM tracks WHERE source='local' ORDER BY rowid")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let key = row.get::<_, String>(0).map_err(|e| e.to_string())?;
            let value = row.get::<_, String>(1).map_err(|e| e.to_string())?;
            match serde_json::from_str::<Track>(&value)
                .with_context(|| format!("本地曲目索引解码失败 key={key}"))
            {
                Ok(track) => index_track(tx, &track, &options)?,
                Err(error) => {
                    crate::infrastructure::diagnostics::report_error(
                        "local.index.drop_invalid_track",
                        &error,
                    );
                    invalid_keys.push(key);
                }
            }
        }
    }
    for key in invalid_keys {
        discard_invalid_local_track(tx, &key)?;
    }
    refresh_albums(tx)
}

fn discard_invalid_local_track(tx: &Connection, key: &str) -> AppResult<()> {
    for table in [
        "local_members",
        "local_origins",
        "local_file_stats",
        "local_metadata",
        "local_cover_state",
        "local_playlist_tracks",
    ] {
        tx.execute(&format!("DELETE FROM {table} WHERE key=?1"), [key])
            .map_err(|e| e.to_string())?;
    }
    tx.execute("DELETE FROM tracks WHERE key=?1 AND source='local'", [key])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn index_track(db: &Connection, track: &Track, options: &LocalPreferences) -> AppResult<()> {
    db.execute(
        "DELETE FROM local_members WHERE key=?1 AND kind IN ('artist','album')",
        [&track.key],
    )
    .map_err(|e| e.to_string())?;
    for name in split_artists(&track.artist, &options.artist_separators) {
        // The first known cover remains the artist's avatar across scans and restarts.
        db.execute("INSERT OR IGNORE INTO local_entities(kind,id,name,cover,subtitle) VALUES('artist',?1,?1,?2,'')", params![name,track.cover.to_string()]).map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO local_members(key,kind,id) VALUES(?1,'artist',?2)",
            params![track.key, name],
        )
        .map_err(|e| e.to_string())?;
    }
    db.execute(
        "INSERT INTO local_members(key,kind,id) VALUES(?1,'album',?2)",
        params![track.key, track.album],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn normalized_path(track: &Track) -> String {
    match &track.source {
        TrackSource::Local { path, .. } => path.trim_start_matches(r"\\?\").replace('\\', "/"),
        _ => track.key.clone(),
    }
}

pub fn refresh_albums(db: &Connection) -> AppResult<()> {
    let options = options_from_db(db)?;
    let mut albums = std::collections::BTreeMap::<String, Vec<(Track, LocalInformation)>>::new();
    {
        let mut stmt = db.prepare("SELECT t.value,m.value FROM visible_local_tracks t LEFT JOIN local_metadata m ON m.key=t.key WHERE t.source='local'").map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let track: Track =
                serde_json::from_str(&row.get::<_, String>(0).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let metadata = row
                .get::<_, Option<String>>(1)
                .map_err(|e| e.to_string())?
                .and_then(|value| {
                    serde_json::from_str::<LocalInformation>(&value)
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn_error(
                                "local.index.decode_metadata",
                                error,
                            )
                        })
                        .ok()
                })
                .unwrap_or_default();
            albums
                .entry(track.album.clone())
                .or_default()
                .push((track, metadata));
        }
    }
    db.execute("DELETE FROM local_entities WHERE kind='album'", [])
        .map_err(|e| e.to_string())?;
    for (id, mut members) in albums {
        members.sort_by_cached_key(|(track, metadata)| {
            (
                metadata.disc_number.unwrap_or(0),
                metadata.track_number.unwrap_or(0),
                normalized_path(track),
                track.key.clone(),
            )
        });
        let has_album_artists = members.iter().any(|(_, metadata)| {
            metadata
                .album_artist
                .as_ref()
                .is_some_and(|name| !name.trim().is_empty())
        });
        let mut artists = Vec::new();
        for (track, metadata) in &members {
            let value = if has_album_artists {
                let Some(value) = metadata
                    .album_artist
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                else {
                    continue;
                };
                value
            } else {
                &track.artist
            };
            for artist in split_artists(value, &options.artist_separators) {
                if !artists.contains(&artist) {
                    artists.push(artist);
                }
            }
        }
        let tracks = members
            .into_iter()
            .map(|(track, _)| track)
            .collect::<Vec<_>>();
        let cover = super::cover_art::album_cover(db, &tracks)?.unwrap_or_default();
        db.execute(
            "INSERT INTO local_entities(kind,id,name,cover,subtitle) VALUES('album',?1,?1,?2,?3)",
            params![id, cover, artists.join(" / ")],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn aggregate_entity(p: crate::aggregate::Playlist) -> LocalEntity {
    LocalEntity {
        id: p.id,
        kind: "playlist".into(),
        name: p.name,
        cover: p.covers.first().cloned().unwrap_or_default(),
        subtitle: "聚合歌单".into(),
        track_count: p.track_count,
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEntity {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub cover: String,
    pub subtitle: String,
    pub track_count: u32,
}
#[derive(Serialize)]
pub struct EntityPage {
    pub items: Vec<LocalEntity>,
    pub more: bool,
}
#[derive(Serialize)]
pub struct TrackPage {
    pub items: Vec<Track>,
    pub more: bool,
}
#[tauri::command]
pub async fn local_entities(
    kind: String,
    keyword: String,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<EntityPage> {
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || entity_page(&store, kind, keyword, offset))
        .await
        .map_err(|e| e.to_string())?
}
fn entity_page(store: &Store, kind: String, keyword: String, offset: u32) -> AppResult<EntityPage> {
    if kind == "playlist" {
        let mut items = crate::aggregate::playlists(store)?
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&keyword.to_lowercase()))
            .skip(offset as usize)
            .take(31)
            .map(aggregate_entity)
            .collect::<Vec<_>>();
        let more = items.len() > 30;
        items.truncate(30);
        return Ok(EntityPage { items, more });
    }

    let db = store
        .0
        .lock()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("local.entities.list.db_lock", error);
        })
        .map_err(|_| "曲库锁不可用")?;
    let sql = "SELECT e.id,e.kind,e.name,e.cover,e.subtitle,COUNT(t.key) FROM local_entities e JOIN local_members m ON m.kind=e.kind AND m.id=e.id JOIN visible_local_tracks t ON t.key=m.key AND t.source='local' WHERE e.kind=?1 AND instr(lower(e.name),lower(?2))>0 GROUP BY e.kind,e.id ORDER BY e.name,e.id LIMIT 31 OFFSET ?3";
    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let mut items = stmt
        .query_map(params![kind, keyword, offset], |r| {
            Ok(LocalEntity {
                id: r.get(0)?,
                kind: r.get(1)?,
                name: r.get(2)?,
                cover: r.get(3)?,
                subtitle: r.get(4)?,
                track_count: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let more = items.len() > 30;
    items.truncate(30);
    Ok(EntityPage { items, more })
}

fn resolve_entity_id(db: &Connection, kind: &str, id: String) -> AppResult<String> {
    if kind == "album"
        && id.starts_with("local:")
        && !db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM local_entities WHERE kind='album' AND id=?1)",
                [&id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|e| e.to_string())?
    {
        db.query_row(
            "SELECT id FROM local_members WHERE key=?1 AND kind='album'",
            [&id],
            |r| r.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())
    } else {
        Ok(id)
    }
}
#[tauri::command]
pub async fn local_entity_detail(
    kind: String,
    id: String,
    backend: State<'_, Backend>,
) -> AppResult<LocalEntity> {
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || entity_detail(&store, kind, id))
        .await
        .map_err(|e| e.to_string())?
}
fn entity_detail(store: &Store, kind: String, id: String) -> AppResult<LocalEntity> {
    if kind == "playlist" {
        return crate::aggregate::playlist(store, &id).map(aggregate_entity);
    }

    let db = store
        .0
        .lock()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("local.entities.detail.db_lock", error);
        })
        .map_err(|_| "曲库锁不可用")?;
    let id = resolve_entity_id(&db, &kind, id)?;
    let row = |r: &rusqlite::Row<'_>| {
        Ok(LocalEntity {
            id: r.get(0)?,
            kind: r.get(1)?,
            name: r.get(2)?,
            cover: r.get(3)?,
            subtitle: r.get(4)?,
            track_count: r.get(5)?,
        })
    };
    db.query_row("SELECT e.id,e.kind,e.name,e.cover,e.subtitle,(SELECT COUNT(*) FROM local_members m JOIN visible_local_tracks t ON t.key=m.key WHERE m.kind=e.kind AND m.id=e.id AND t.source='local') FROM local_entities e WHERE e.kind=?1 AND e.id=?2", params![kind,id], row).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn local_entity_tracks(
    kind: String,
    id: String,
    keyword: String,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<TrackPage> {
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || entity_tracks(&store, kind, id, keyword, offset))
        .await
        .map_err(|e| e.to_string())?
}
fn entity_tracks(
    store: &Store,
    kind: String,
    id: String,
    keyword: String,
    offset: u32,
) -> AppResult<TrackPage> {
    if kind == "playlist" {
        let page = crate::aggregate::tracks(store, &id, offset, false)?;
        return Ok(TrackPage {
            items: page.items.into_iter().map(|m| m.track).collect(),
            more: page.more,
        });
    }

    let db = store
        .0
        .lock()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("local.entities.tracks.db_lock", error);
        })
        .map_err(|_| "曲库锁不可用")?;
    let id = resolve_entity_id(&db, &kind, id)?;
    let options = options_from_db(&db)?;
    let sql = match kind.as_str() {
            "artist" | "album" => "SELECT t.value FROM visible_local_tracks t JOIN local_members m ON m.key=t.key WHERE t.source='local' AND m.kind=?1 AND m.id=?2 ORDER BY COALESCE(json_extract((SELECT value FROM local_metadata WHERE key=t.key),'$.discNumber'),0),COALESCE(json_extract((SELECT value FROM local_metadata WHERE key=t.key),'$.trackNumber'),0),t.title,t.key LIMIT 101 OFFSET ?4",
            _ => "SELECT value FROM visible_local_tracks WHERE source='local' AND (instr(lower(title),lower(?3))>0 OR instr(lower(artist),lower(?3))>0 OR instr(lower(json_extract(value,'$.album')),lower(?3))>0) ORDER BY title,key LIMIT 101 OFFSET ?4"
        };
    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let values = stmt
        .query_map(params![kind, id, keyword, offset], |r| {
            r.get::<_, String>(0)
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for value in values {
        let mut track: Track =
            serde_json::from_str(&value.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        track.artists = split_artists(&track.artist, &options.artist_separators)
            .into_iter()
            .map(|name| MusicCredit { name })
            .collect();
        items.push(track);
    }
    let more = items.len() > 100;
    items.truncate(100);
    Ok(TrackPage { items, more })
}

#[tauri::command]
pub async fn local_track_information(
    key: String,
    backend: State<'_, Backend>,
) -> AppResult<LocalInformation> {
    let track = backend.store.track(&key)?;
    let TrackSource::Local { path, cue, .. } = track.source else {
        return Err("不是本地歌曲".into());
    };
    let album_artist = if cue.is_some() {
        backend.store.local_information(&key)?.album_artist
    } else {
        None
    };
    tauri::async_runtime::spawn_blocking(move || {
        let mut information = read_information(&path)?;
        if let Some(cue) = cue {
            information.track_number = Some(cue.track_number);
            information.album_artist = album_artist;
        }
        Ok(information)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn local_preferences(backend: State<'_, Backend>) -> AppResult<LocalPreferences> {
    preferences(&backend.store)
}
#[tauri::command]
pub async fn set_local_preferences(
    options: LocalPreferences,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if options.artist_separators.len() > 16
        || options
            .artist_separators
            .iter()
            .any(|s| s.is_empty() || s.len() > 32)
    {
        return Err("本地歌曲设置无效".into());
    }
    let store = backend.store.clone();
    let lyrics = backend.lyrics.clone();
    tauri::async_runtime::spawn_blocking(move || {
        store.configure_local_preferences(&options)?;
        lyrics.set_sources(lyrics.sources()?)
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = (app.emit(
        "local-library-updated",
        serde_json::json!({"scanning":false}),
    ))
    .inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("local.preferences.changed.emit", error);
    });
    Ok(())
}
#[tauri::command]
pub fn create_local_playlist(
    name: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<String> {
    let result = create_playlist(&backend.store, name)?;
    emit_library_change(&app);
    Ok(result)
}
fn create_playlist(store: &Store, name: String) -> AppResult<String> {
    crate::aggregate::write(
        store,
        &crate::aggregate::Write::Create { name, keys: vec![] },
    )
}

#[tauri::command]
pub fn rename_local_playlist(
    id: String,
    name: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    rename_playlist(&backend.store, id, name)?;
    emit_library_change(&app);
    Ok(())
}
fn rename_playlist(store: &Store, id: String, name: String) -> AppResult<()> {
    crate::aggregate::write(store, &crate::aggregate::Write::Rename { id, name }).map(|_| ())
}

#[tauri::command]
pub fn delete_local_playlist(
    id: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    delete_playlist(&backend.store, id)?;
    emit_library_change(&app);
    Ok(())
}
fn delete_playlist(store: &Store, id: String) -> AppResult<()> {
    crate::aggregate::write(store, &crate::aggregate::Write::Delete { id }).map(|_| ())
}

#[tauri::command]
pub fn add_local_playlist_track(
    id: String,
    key: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    add_playlist_track(&backend.store, id, key)?;
    emit_library_change(&app);
    Ok(())
}
fn add_playlist_track(store: &Store, id: String, key: String) -> AppResult<()> {
    crate::aggregate::write(
        store,
        &crate::aggregate::Write::Add {
            id,
            keys: vec![key],
        },
    )
    .map(|_| ())
}

#[tauri::command]
pub fn remove_local_playlist_track(
    id: String,
    key: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    remove_playlist_track(&backend.store, id, key)?;
    emit_library_change(&app);
    Ok(())
}
fn remove_playlist_track(store: &Store, id: String, key: String) -> AppResult<()> {
    crate::aggregate::write(
        store,
        &crate::aggregate::Write::Remove {
            id,
            keys: vec![key],
        },
    )
    .map(|_| ())
}

fn emit_library_change(app: &tauri::AppHandle) {
    let _ = (app.emit(
        "local-library-updated",
        serde_json::json!({"scanning":false}),
    ))
    .inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("local.library.changed.emit", error);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(key: &str, artist: &str, cover: &str) -> Track {
        Track {
            key: key.into(),
            title: key.into(),
            aliases: vec![],
            artist: artist.into(),
            artists: vec![],
            album: "同名专辑".into(),
            duration_ms: 1000,
            cover: cover.into(),
            source: TrackSource::Local {
                path: key.into(),
                netease_id: None,
                cue: None,
            },
        }
    }
    #[test]
    fn first_artist_cover_survives_reimport_reindex_and_restart() {
        let dir = crate::test_support::TestDir::new();
        let path = dir.0.join("library.sqlite");
        let store = Store::open(&path).unwrap();
        store
            .save_tracks(&[
                track("local:first", "A / B", "first.jpg"),
                track("local:second", "A", "second.jpg"),
            ])
            .unwrap();
        store
            .save_tracks(&[track("local:first", "A / B", "changed.jpg")])
            .unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        let artist = entity_detail(&store, "artist".into(), "A".into()).unwrap();
        assert_eq!(artist.cover, "first.jpg");
        assert_eq!(artist.track_count, 2);
        store
            .configure_local_preferences(&LocalPreferences {
                artist_separators: vec![],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            entity_detail(&store, "artist".into(), "A".into())
                .unwrap()
                .track_count,
            1
        );
        assert_eq!(
            entity_detail(&store, "artist".into(), "A / B".into())
                .unwrap()
                .track_count,
            1
        );
    }
    #[test]
    fn album_title_merges_artists_and_track_links_resolve_the_album() {
        let dir = crate::test_support::TestDir::new();
        let store = Store::open(&dir.0.join("library.sqlite")).unwrap();
        for (key, artist, album_artist, number) in [
            ("local:b", "Guest", "Band", 2),
            ("local:a", "Singer", "Band", 1),
            ("local:c", "Singer", "Other", 1),
        ] {
            store
                .save_local_information(
                    key,
                    &LocalInformation {
                        album_artist: Some(album_artist.into()),
                        track_number: Some(number),
                        ..Default::default()
                    },
                )
                .unwrap();
            store.save_tracks(&[track(key, artist, "cover")]).unwrap();
        }
        store.refresh_local_albums().unwrap();
        let albums = entity_page(&store, "album".into(), "同名".into(), 0).unwrap();
        assert_eq!(albums.items.len(), 1);
        assert_eq!(albums.items[0].id, "同名专辑");
        assert_eq!(albums.items[0].subtitle, "Band / Other");
        let songs = entity_tracks(&store, "album".into(), "local:b".into(), "".into(), 0).unwrap();
        assert_eq!(
            songs
                .items
                .iter()
                .map(|t| t.key.as_str())
                .collect::<Vec<_>>(),
            vec!["local:a", "local:c", "local:b"]
        );
    }
    #[test]
    fn album_titles_remain_raw_and_missing_album_artists_fall_back_to_singers() {
        let dir = crate::test_support::TestDir::new();
        let store = Store::open(&dir.0.join("library.sqlite")).unwrap();
        let mut tracks = vec![track("local:z", "B / A", ""), track("local:a", "A / C", "")];
        for (index, album) in [" Album", "Album", "album", "Album ", "local:literal"]
            .into_iter()
            .enumerate()
        {
            let mut song = track(&format!("local:{index}"), "Singer", "");
            song.album = album.into();
            tracks.push(song);
        }
        store.save_tracks(&tracks).unwrap();
        store.refresh_local_albums().unwrap();
        let album = entity_detail(&store, "album".into(), "同名专辑".into()).unwrap();
        assert_eq!(album.track_count, 2);
        assert_eq!(album.subtitle, "A / C / B");
        assert_eq!(
            entity_page(&store, "album".into(), "".into(), 0)
                .unwrap()
                .items
                .len(),
            6
        );
        assert_eq!(
            entity_detail(&store, "album".into(), "local:literal".into())
                .unwrap()
                .id,
            "local:literal"
        );
    }

    #[test]
    fn index_version_rebuild_removes_compound_ids_and_preserves_music_history_and_playlists() {
        let dir = crate::test_support::TestDir::new();
        let path = dir.0.join("library.sqlite");
        let store = Store::open(&path).unwrap();
        let song = track("local:a", "A", "");
        store.save_tracks(std::slice::from_ref(&song)).unwrap();
        store.record_playback("instance", &song, 10).unwrap();
        let playlist = create_playlist(&store, "保留歌单".into()).unwrap();
        add_playlist_track(&store, playlist.clone(), song.key.clone()).unwrap();
        {
            let db = store.0.lock().unwrap();
            db.execute(
                "UPDATE settings SET value='1' WHERE key='localIndexVersion'",
                [],
            )
            .unwrap();
            db.execute("INSERT INTO local_entities(kind,id,name,cover,subtitle) VALUES('album','[旧专辑,旧艺术家]','旧专辑','','')", []).unwrap();
            db.execute("INSERT INTO local_members(key,kind,id) VALUES('local:a','album','[旧专辑,旧艺术家]')", []).unwrap();
        }
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(
            entity_page(&store, "album".into(), "".into(), 0)
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            entity_detail(&store, "album".into(), "同名专辑".into())
                .unwrap()
                .track_count,
            1
        );
        assert!(store.track("local:a").is_ok());
        assert_eq!(store.playback_count().unwrap(), 1);
        assert_eq!(
            entity_tracks(&store, "playlist".into(), playlist, "".into(), 0)
                .unwrap()
                .items[0]
                .key,
            "local:a"
        );
        let db = store.0.lock().unwrap();
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM local_members WHERE id='[旧专辑,旧艺术家]'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn index_version_rebuild_discards_legacy_cover_tracks_instead_of_failing_startup() {
        let dir = crate::test_support::TestDir::new();
        let path = dir.0.join("library.sqlite");
        let store = Store::open(&path).unwrap();
        store.save_tracks(&[track("local:valid", "A", "")]).unwrap();
        {
            let db = store.0.lock().unwrap();
            let legacy = serde_json::json!({
                "key": "local:legacy",
                "title": "旧格式歌曲",
                "aliases": [],
                "artist": "A",
                "artists": [],
                "album": "旧专辑",
                "durationMs": 1000,
                "cover": "old-cover.jpg",
                "source": {
                    "kind": "local",
                    "path": "C:/music/legacy.mp3",
                    "neteaseId": null
                }
            });
            db.execute(
                "INSERT INTO tracks(key,source,title,artist,value) VALUES(?1,'local',?2,?3,?4)",
                rusqlite::params!["local:legacy", "旧格式歌曲", "A", legacy.to_string()],
            )
            .unwrap();
            db.execute(
                "UPDATE settings SET value='1' WHERE key='localIndexVersion'",
                [],
            )
            .unwrap();
        }
        drop(store);

        crate::infrastructure::diagnostics::test_log::start();
        let store = Store::open(&path).unwrap();
        let records = crate::infrastructure::diagnostics::test_log::take();
        let record = records
            .iter()
            .find(|record| record.contains("local.index.drop_invalid_track"))
            .expect("discarding an invalid track must preserve its decoding failure");
        assert!(record.contains("本地曲目索引解码失败 key=local:legacy"));
        assert!(record.contains("CoverImage"));
        assert!(record.contains("backtrace="));
        assert!(record.contains("local_library"));
        assert!(!record.contains("disabled backtrace"));
        assert!(store.track("local:valid").is_ok());
        assert!(store.track("local:legacy").is_err());
        let db = store.0.lock().unwrap();
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM tracks WHERE key='local:legacy'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT value FROM settings WHERE key='localIndexVersion'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "2"
        );
    }

    #[test]
    fn invalid_saved_local_preferences_fall_back_to_defaults() {
        let dir = crate::test_support::TestDir::new();
        let store = Store::open(&dir.0.join("library.sqlite")).unwrap();
        store.set_setting("localPreferences", "{invalid").unwrap();
        let db = store.0.lock().unwrap();
        assert_eq!(
            options_from_db(&db).unwrap().artist_separators,
            LocalPreferences::default().artist_separators
        );
    }

    #[test]
    fn invalid_embedded_cover_state_is_discarded_during_album_refresh() {
        let dir = crate::test_support::TestDir::new();
        let store = Store::open(&dir.0.join("library.sqlite")).unwrap();
        let cover = dir.0.join("covers").join("missing.png");
        let song = track("local:broken-cover-state", "A", &cover.to_string_lossy());
        store.save_tracks(&[song]).unwrap();
        store
            .0
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO local_cover_state(key,signature,embedded) VALUES(?1,'','{invalid')",
                ["local:broken-cover-state"],
            )
            .unwrap();
        store.refresh_local_albums().unwrap();
        assert_eq!(
            store
                .0
                .lock()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM local_cover_state WHERE key=?1",
                    ["local:broken-cover-state"],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn album_cover_uses_stable_member_order_after_reimport() {
        let dir = crate::test_support::TestDir::new();
        let covers = dir.0.join("covers");
        std::fs::create_dir_all(&covers).unwrap();
        let store = Store::open(&dir.0.join("library.sqlite")).unwrap();
        let mut tracks = Vec::new();
        for (key, number, red) in [("local:b", 2, 20), ("local:a", 1, 10)] {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                1,
                1,
                image::Rgb([red, 0, 0]),
            ))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
            let cover = super::super::cover_art::asset(bytes.get_ref(), &covers).unwrap();
            let song = track(key, "A", &cover);
            store
                .save_local_information(
                    key,
                    &LocalInformation {
                        track_number: Some(number),
                        ..Default::default()
                    },
                )
                .unwrap();
            store
                .0
                .lock()
                .unwrap()
                .execute(
                    "INSERT INTO local_cover_state(key,signature,embedded) VALUES(?1,'',?2)",
                    params![key, serde_json::to_string(&vec![cover]).unwrap()],
                )
                .unwrap();
            tracks.push(song);
        }
        store.save_tracks(&tracks).unwrap();
        store.refresh_local_albums().unwrap();
        let expected = tracks[1].cover.to_string();
        assert_eq!(
            entity_detail(&store, "album".into(), "同名专辑".into())
                .unwrap()
                .cover,
            expected
        );
        tracks.reverse();
        store.save_tracks(&tracks).unwrap();
        store.refresh_local_albums().unwrap();
        assert_eq!(
            entity_detail(&store, "album".into(), "同名专辑".into())
                .unwrap()
                .cover,
            expected
        );
    }

    #[test]
    fn album_members_keep_disc_track_title_key_order() {
        let dir = crate::test_support::TestDir::new();
        let store = Store::open(&dir.0.join("library.sqlite")).unwrap();
        for (key, disc, number, title) in [
            ("local:d", 2, 1, "A"),
            ("local:c", 1, 2, "A"),
            ("local:b", 1, 1, "B"),
            ("local:a", 1, 1, "A"),
        ] {
            store
                .save_local_information(
                    key,
                    &LocalInformation {
                        disc_number: Some(disc),
                        track_number: Some(number),
                        ..Default::default()
                    },
                )
                .unwrap();
            let mut song = track(key, "A", "");
            song.title = title.into();
            store.save_tracks(&[song]).unwrap();
        }
        store.refresh_local_albums().unwrap();
        let page = entity_tracks(&store, "album".into(), "同名专辑".into(), "".into(), 0).unwrap();
        assert_eq!(
            page.items
                .iter()
                .map(|song| song.key.as_str())
                .collect::<Vec<_>>(),
            vec!["local:a", "local:b", "local:c", "local:d"]
        );
    }

    #[test]
    fn music_search_includes_albums_and_pagination_does_not_drop_a_song() {
        let dir = crate::test_support::TestDir::new();
        let store = Store::open(&dir.0.join("library.sqlite")).unwrap();
        store
            .save_tracks(
                &(0..102)
                    .map(|i| track(&format!("local:{i:03}"), "A", "cover"))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        let first = entity_tracks(&store, "song".into(), "".into(), "同名专辑".into(), 0).unwrap();
        assert!(first.more);
        assert_eq!(first.items.len(), 100);
        let second =
            entity_tracks(&store, "song".into(), "".into(), "同名专辑".into(), 100).unwrap();
        assert!(!second.more);
        assert_eq!(second.items.len(), 2);
        assert_eq!(second.items[0].key, "local:100");
        let db = store.0.lock().unwrap();
        db.execute("INSERT INTO local_playlists(name) VALUES('空歌单')", [])
            .unwrap();
        drop(db);
        let playlists = entity_page(&store, "playlist".into(), "".into(), 0).unwrap();
        assert_eq!(playlists.items.len(), 1);
        assert_eq!(playlists.items[0].track_count, 0);
        let empty = entity_tracks(
            &store,
            "playlist".into(),
            playlists.items[0].id.clone(),
            "".into(),
            0,
        )
        .unwrap();
        assert!(empty.items.is_empty());
    }
    #[test]
    fn playlist_writes_prepend_new_tracks_deduplicate_and_never_delete_music() {
        let dir = crate::test_support::TestDir::new();
        let path = dir.0.join("library.sqlite");
        let store = Store::open(&path).unwrap();
        store
            .save_tracks(&[
                track("local:a", "A", "cover"),
                track("local:b", "B", "cover"),
            ])
            .unwrap();
        let id = create_playlist(&store, "收藏".into()).unwrap();
        add_playlist_track(&store, id.clone(), "local:b".into()).unwrap();
        add_playlist_track(&store, id.clone(), "local:a".into()).unwrap();
        add_playlist_track(&store, id.clone(), "local:b".into()).unwrap();
        rename_playlist(&store, id.clone(), "最喜欢".into()).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(
            entity_detail(&store, "playlist".into(), id.clone())
                .unwrap()
                .name,
            "最喜欢"
        );
        assert_eq!(
            entity_tracks(&store, "playlist".into(), id.clone(), "".into(), 0)
                .unwrap()
                .items
                .iter()
                .map(|t| t.key.as_str())
                .collect::<Vec<_>>(),
            vec!["local:a", "local:b"]
        );
        remove_playlist_track(&store, id.clone(), "local:b".into()).unwrap();
        assert_eq!(
            entity_detail(&store, "playlist".into(), id.clone())
                .unwrap()
                .track_count,
            1
        );
        delete_playlist(&store, id).unwrap();
        assert!(entity_page(&store, "playlist".into(), "".into(), 0)
            .unwrap()
            .items
            .is_empty());
        assert!(store.track("local:a").is_ok());
        assert!(store.track("local:b").is_ok());
    }
    #[test]
    fn separators_are_literal_and_deduplicated() {
        assert_eq!(split_artists(" A.*B.*A ", &[".*".into()]), vec!["A", "B"]);
        assert_eq!(split_artists("AC/DC", &[]), vec!["AC/DC"]);
    }
}
