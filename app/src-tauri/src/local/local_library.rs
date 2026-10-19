use crate::{
    model::{AppResult, MusicCredit, Track, TrackSource},
    storage::Store,
    Backend,
};
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
    value
        .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
        .unwrap_or_else(|| Ok(LocalPreferences::default()))
}
pub fn preferences(store: &Store) -> AppResult<LocalPreferences> {
    {
        let db = store
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
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
                crate::infrastructure::diagnostics::warn("translate_error", error);
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
        self.0.lock().inspect_err(|error| { crate::infrastructure::diagnostics::warn("translate_error", error); }).map_err(|_| "曲库锁不可用")?.execute("INSERT INTO local_metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, serde_json::to_string(value).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn rebuild_local_index(&self) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "曲库锁不可用")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        rebuild_index(&tx)?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn configure_local_preferences(&self, options: &LocalPreferences) -> AppResult<()> {
        let mut db = self
            .0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
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
fn rebuild_index(tx: &Connection) -> AppResult<()> {
    let options = options_from_db(tx)?;
    tx.execute("DELETE FROM local_members", [])
        .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare("SELECT value FROM tracks WHERE source='local' ORDER BY rowid")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let track: Track =
                serde_json::from_str(&row.get::<_, String>(0).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            index_track(tx, &track, &options)?;
        }
    }
    Ok(())
}
pub fn index_track(db: &Connection, track: &Track, options: &LocalPreferences) -> AppResult<()> {
    db.execute("DELETE FROM local_members WHERE key=?1", [&track.key])
        .map_err(|e| e.to_string())?;
    let metadata: Option<String> = db
        .query_row(
            "SELECT value FROM local_metadata WHERE key=?1",
            [&track.key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let album_artist = metadata
        .and_then(|s| {
            serde_json::from_str::<LocalInformation>(&s)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_result", error);
                })
                .ok()
        })
        .and_then(|m| m.album_artist)
        .unwrap_or_else(|| track.artist.clone());
    let album_id =
        serde_json::to_string(&(&track.album, &album_artist)).map_err(|e| e.to_string())?;
    let mut entities = split_artists(&track.artist, &options.artist_separators)
        .into_iter()
        .map(|name| ("artist", name.clone(), name, String::new()))
        .collect::<Vec<_>>();
    entities.push(("album", album_id, track.album.clone(), album_artist));
    for (kind, id, name, subtitle) in entities {
        // The first known cover remains the artist's avatar across scans and restarts.
        db.execute("INSERT OR IGNORE INTO local_entities(kind,id,name,cover,subtitle) VALUES(?1,?2,?3,?4,?5)", params![kind,id,name,track.cover.to_string(),subtitle]).map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO local_members(key,kind,id) VALUES(?1,?2,?3)",
            params![track.key, kind, id],
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
            crate::infrastructure::diagnostics::warn("translate_error", error);
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
    if kind == "album" && id.starts_with("local:") {
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
            crate::infrastructure::diagnostics::warn("translate_error", error);
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
            crate::infrastructure::diagnostics::warn("translate_error", error);
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
            .map(|name| MusicCredit { name, id: None })
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
        crate::infrastructure::diagnostics::warn("app.emit", error);
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
        crate::infrastructure::diagnostics::warn("app.emit", error);
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
            album_id: None,
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
        store.rebuild_local_index().unwrap();
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
    fn album_artist_disambiguates_names_and_track_links_resolve_the_album() {
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
        let albums = entity_page(&store, "album".into(), "同名".into(), 0).unwrap();
        assert_eq!(albums.items.len(), 2);
        let songs = entity_tracks(&store, "album".into(), "local:b".into(), "".into(), 0).unwrap();
        assert_eq!(
            songs
                .items
                .iter()
                .map(|t| t.key.as_str())
                .collect::<Vec<_>>(),
            vec!["local:a", "local:b"]
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
    fn playlist_writes_preserve_order_deduplicate_and_never_delete_music() {
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
            vec!["local:b", "local:a"]
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
