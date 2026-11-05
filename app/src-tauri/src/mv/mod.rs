//! Temporary per-track local MV association for the fullscreen player.
//! Remove `local_mv_links` and these commands when MV becomes a first-class media source.

use crate::{application::Backend, model::AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Manager, State};

mod proxy;
pub(crate) mod saved;
pub(crate) use proxy::MvProxy;
pub(crate) use saved::SavedMvs;

#[cfg(desktop)]
const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_OFFSET_MS: i64 = 10 * 60 * 1000;
const VIDEO_EXTENSIONS: [&str; 3] = ["mp4", "m4v", "webm"];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalMv {
    pub name: String,
    pub path: String,
    pub available: bool,
    pub offset_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MvBinding {
    pub source: String,
    pub id: String,
    pub title: String,
    pub offset_ms: i64,
    pub local: Option<LocalMv>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MvSearchGroup {
    pub source: String,
    pub display_name: String,
    pub items: Vec<crate::music::mv::MvCandidate>,
    pub next_cursor: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MvSearchInput {
    pub query: String,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedMv {
    pub source: String,
    pub local: bool,
}

fn valid_extension(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        VIDEO_EXTENSIONS
            .iter()
            .any(|allowed| extension.to_string_lossy().eq_ignore_ascii_case(allowed))
    })
}

#[cfg(desktop)]
fn validate_path(path: &Path) -> AppResult<()> {
    if !path.is_absolute()
        || path.to_string_lossy().len() > MAX_PATH_BYTES
        || !valid_extension(path)
        || !path.is_file()
    {
        return Err("只能选择 MP4、M4V 或 WebM 视频文件".into());
    }
    Ok(())
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "本地 MV".into())
}

fn inspect(
    app: &tauri::AppHandle,
    path: &Path,
    offset_ms: i64,
    authorize: bool,
) -> AppResult<LocalMv> {
    let available = path.is_absolute() && valid_extension(path) && path.is_file();
    if available && authorize {
        app.asset_protocol_scope()
            .allow_file(path)
            .map_err(|error| error.to_string())?;
    }
    Ok(LocalMv {
        name: name(path),
        path: path.to_string_lossy().into_owned(),
        available,
        offset_ms,
    })
}

#[tauri::command]
pub fn local_mv(
    key: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Option<LocalMv>> {
    backend.store.track(&key)?;
    let Some(link) = backend.store.local_mv_link(&key)? else {
        return Ok(None);
    };
    inspect(&app, Path::new(&link.path), link.offset_ms, true).map(Some)
}

#[tauri::command]
pub fn track_mv(
    key: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Option<MvBinding>> {
    backend.store.track(&key)?;
    if let Some(link) = backend.store.mv_link(&key)? {
        let source = crate::music::identity::SourceId::try_from(link.source.clone())
            .map_err(|error| error.to_string())?;
        let saved = backend
            .saved_mvs
            .saved_path(&source, &link.mv_id)?
            .is_some();
        let source_available = backend
            .music
            .adapters
            .mv_sources()
            .map_err(|error| error.to_string())?
            .iter()
            .any(|descriptor| descriptor.source == source);
        if saved || source_available {
            return Ok(Some(MvBinding {
                source: link.source,
                id: link.mv_id,
                title: link.title,
                offset_ms: link.offset_ms,
                local: None,
            }));
        }
    }
    let Some(link) = backend.store.local_mv_link(&key)? else {
        return Ok(None);
    };
    Ok(Some(MvBinding {
        source: "local".into(),
        id: String::new(),
        title: String::new(),
        offset_ms: link.offset_ms,
        local: Some(inspect(&app, Path::new(&link.path), link.offset_ms, true)?),
    }))
}

#[tauri::command]
pub async fn search_track_mvs(
    key: String,
    input: MvSearchInput,
    backend: State<'_, Backend>,
) -> AppResult<Vec<MvSearchGroup>> {
    let track = backend.store.track(&key)?;
    let mut query = input.query.trim().to_owned();
    if query.is_empty() {
        query = [track.title.as_str(), track.artist.as_str()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }
    if query.is_empty() {
        return Err("当前歌曲缺少可用于搜索 MV 的信息".into());
    }
    let cursor = input
        .cursor
        .map(crate::music::adapter::Cursor::try_from)
        .transpose()
        .map_err(|error| error.to_string())?;
    let request = crate::music::mv::MvSearchRequest {
        track: crate::music::mv::MvTrackQuery {
            title: track.title.clone(),
            artists: track
                .artists
                .iter()
                .map(|artist| artist.name.clone())
                .chain((!track.artist.is_empty()).then(|| track.artist.clone()))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            duration_ms: track.duration_ms,
        },
        query,
        page: crate::music::adapter::PageRequest { cursor, limit: 20 },
    };
    let sources = backend
        .music
        .adapters
        .mv_sources()
        .map_err(|error| error.to_string())?;
    let mut groups = Vec::new();
    for source in sources {
        let result = backend
            .music
            .adapters
            .search_mvs(&source.source, &request)
            .await;
        groups.push(match result {
            Ok(page) => MvSearchGroup {
                source: source.source.as_str().to_owned(),
                display_name: source.display_name,
                items: page.items,
                next_cursor: page.next_cursor.map(|cursor| cursor.as_str().to_owned()),
                error: None,
            },
            Err(error) => MvSearchGroup {
                source: source.source.as_str().to_owned(),
                display_name: source.display_name,
                items: vec![],
                next_cursor: None,
                error: Some(error.to_string()),
            },
        });
    }
    Ok(groups)
}

#[tauri::command]
pub async fn bind_mv(
    key: String,
    source: String,
    mv_id: String,
    title: String,
    backend: State<'_, Backend>,
) -> AppResult<MvBinding> {
    backend.store.track(&key)?;
    if title.trim().is_empty() || title.len() > 512 {
        return Err("MV 标题无效".into());
    }
    let source =
        crate::music::identity::SourceId::try_from(source).map_err(|error| error.to_string())?;
    let saved = backend.saved_mvs.saved_path(&source, &mv_id)?.is_some();
    let mut ticket = None;
    if !saved {
        let available = backend
            .music
            .adapters
            .mv_sources()
            .map_err(|error| error.to_string())?
            .iter()
            .any(|descriptor| descriptor.source == source);
        if !available {
            return Err("提供该 MV 的适配器当前不可用".into());
        }
        let reference = crate::music::identity::EntityRef {
            source: source.clone(),
            kind: crate::music::identity::EntityKind::Track,
            id: mv_id
                .clone()
                .try_into()
                .map_err(|error: crate::music::MusicError| error.to_string())?,
        };
        ticket = Some(
            backend
                .music
                .adapters
                .resolve_mv(&crate::music::mv::MvResolveRequest { mv: reference }, 0)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    backend
        .store
        .link_mv(&key, source.as_str(), &mv_id, &title)?;
    if let Some(ticket) = ticket {
        backend.saved_mvs.start_save(&source, &mv_id, ticket);
    }
    Ok(MvBinding {
        source: source.as_str().to_owned(),
        id: mv_id,
        title,
        offset_ms: 0,
        local: None,
    })
}

#[tauri::command]
pub async fn resolve_track_mv(key: String, backend: State<'_, Backend>) -> AppResult<ResolvedMv> {
    backend.store.track(&key)?;
    let link = backend.store.mv_link(&key)?.ok_or("当前歌曲未绑定 MV")?;
    let source = link
        .source
        .try_into()
        .map_err(|error: crate::music::MusicError| error.to_string())?;
    if let Some(path) = backend.saved_mvs.saved_path(&source, &link.mv_id)? {
        return Ok(ResolvedMv {
            source: path,
            local: true,
        });
    }
    let reference = crate::music::identity::EntityRef {
        source: source.clone(),
        kind: crate::music::identity::EntityKind::Track,
        id: link
            .mv_id
            .clone()
            .try_into()
            .map_err(|error: crate::music::MusicError| error.to_string())?,
    };
    let ticket = backend
        .music
        .adapters
        .resolve_mv(&crate::music::mv::MvResolveRequest { mv: reference }, 0)
        .await
        .map_err(|error| error.to_string())?;
    backend
        .saved_mvs
        .start_save(&source, &link.mv_id, ticket.clone());
    Ok(ResolvedMv {
        source: backend.mv_proxy.issue(ticket, 0)?,
        local: false,
    })
}

#[tauri::command]
pub fn unlink_mv_binding(key: String, backend: State<'_, Backend>) -> AppResult<()> {
    backend.store.track(&key)?;
    backend.store.unlink_mv(&key)
}

#[tauri::command]
pub fn set_mv_offset(key: String, offset_ms: i64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.store.track(&key)?;
    if !(-MAX_OFFSET_MS..=MAX_OFFSET_MS).contains(&offset_ms) {
        return Err("MV 偏移须在 ±10 分钟之间".into());
    }
    if backend.store.mv_link(&key)?.is_some() {
        backend.store.set_mv_offset(&key, offset_ms)
    } else {
        backend.store.set_local_mv_offset(&key, offset_ms)
    }
}

#[tauri::command]
pub async fn pick_local_mv(
    key: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Option<LocalMv>> {
    backend.store.track(&key)?;
    #[cfg(desktop)]
    {
        use tauri_plugin_dialog::DialogExt;

        let dialog_app = app.clone();
        let selected = tauri::async_runtime::spawn_blocking(move || {
            dialog_app
                .dialog()
                .file()
                .set_title("选择本地 MV")
                .add_filter("本地视频", &VIDEO_EXTENSIONS)
                .blocking_pick_file()
        })
        .await
        .map_err(|e| e.to_string())?;
        let Some(selected) = selected else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|e| e.to_string())?;
        validate_path(&path)?;
        let path_text = path.to_string_lossy().into_owned();
        Ok(Some(LocalMv {
            name: name(&path),
            path: path_text,
            available: true,
            offset_ms: 0,
        }))
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, backend);
        Err("当前平台不支持选择本地 MV".into())
    }
}

#[tauri::command]
pub fn link_local_mv(
    key: String,
    path: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<LocalMv> {
    backend.store.track(&key)?;
    #[cfg(desktop)]
    {
        let path = Path::new(&path);
        validate_path(path)?;
        app.asset_protocol_scope()
            .allow_file(path)
            .map_err(|error| error.to_string())?;
        let path_text = path.to_string_lossy().into_owned();
        backend.store.link_local_mv(&key, &path_text)?;
        Ok(LocalMv {
            name: name(path),
            path: path_text,
            available: true,
            offset_ms: 0,
        })
    }
    #[cfg(not(desktop))]
    {
        let _ = (path, app);
        Err("当前平台不支持关联本地 MV".into())
    }
}

#[tauri::command]
pub fn unlink_local_mv(key: String, backend: State<'_, Backend>) -> AppResult<()> {
    backend.store.track(&key)?;
    backend.store.unlink_local_mv(&key)
}

#[tauri::command]
pub fn set_local_mv_offset(
    key: String,
    offset_ms: i64,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.store.track(&key)?;
    if !(-MAX_OFFSET_MS..=MAX_OFFSET_MS).contains(&offset_ms) {
        return Err("MV 偏移须在 ±10 分钟之间".into());
    }
    backend.store.set_local_mv_offset(&key, offset_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mv_accepts_only_supported_video_extensions() {
        for path in ["video.mp4", "video.M4V", "video.webm"] {
            assert!(valid_extension(Path::new(path)), "{path}");
        }
        for path in ["video.mkv", "video.mp3", "video"] {
            assert!(!valid_extension(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn local_mv_link_is_an_independent_temporary_association() {
        let directory = tempfile::tempdir().unwrap();
        let store =
            crate::infrastructure::storage::Store::open(&directory.path().join("local-mv.sqlite3"))
                .unwrap();
        assert!(store.local_mv_link("track-a").unwrap().is_none());
        store
            .link_local_mv("track-a", r"C:\Videos\first.mp4")
            .unwrap();
        let first = store.local_mv_link("track-a").unwrap().unwrap();
        assert_eq!(first.path, r"C:\Videos\first.mp4");
        assert_eq!(first.offset_ms, 0);
        store.set_local_mv_offset("track-a", 2_500).unwrap();
        assert_eq!(
            store.local_mv_link("track-a").unwrap().unwrap().offset_ms,
            2_500
        );
        store
            .link_local_mv("track-a", r"C:\Videos\second.webm")
            .unwrap();
        let second = store.local_mv_link("track-a").unwrap().unwrap();
        assert_eq!(second.path, r"C:\Videos\second.webm");
        assert_eq!(second.offset_ms, 0);
        assert!(store.local_mv_link("track-b").unwrap().is_none());
        store.unlink_local_mv("track-a").unwrap();
        assert!(store.local_mv_link("track-a").unwrap().is_none());
        store.unlink_local_mv("track-a").unwrap();
    }

    #[test]
    fn existing_links_gain_a_zero_offset_column() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("old-local-mv.sqlite3");
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE local_mv_links (track_key TEXT PRIMARY KEY, path TEXT NOT NULL, updated_at INTEGER NOT NULL);
                     INSERT INTO local_mv_links(track_key,path,updated_at) VALUES('track-a','C:/Videos/old.mp4',0);",
                )
                .unwrap();
        }
        let store = crate::infrastructure::storage::Store::open(&path).unwrap();
        let link = store.local_mv_link("track-a").unwrap().unwrap();
        assert_eq!(link.offset_ms, 0);
        store.set_local_mv_offset("track-a", 400).unwrap();
        assert_eq!(
            store.local_mv_link("track-a").unwrap().unwrap().offset_ms,
            400
        );
    }

    #[test]
    fn adapter_mv_link_replaces_local_fallback_and_unlinks_both() {
        let directory = tempfile::tempdir().unwrap();
        let store = crate::infrastructure::storage::Store::open(
            &directory.path().join("remote-mv.sqlite3"),
        )
        .unwrap();
        store
            .link_local_mv("track-a", r"C:\Videos\local.mp4")
            .unwrap();
        store
            .link_mv("track-a", "bilibili", "BV1xx411c7mD", "MV")
            .unwrap();
        assert!(store.local_mv_link("track-a").unwrap().is_none());
        let link = store.mv_link("track-a").unwrap().unwrap();
        assert_eq!(link.source, "bilibili");
        assert_eq!(link.mv_id, "BV1xx411c7mD");
        store.set_mv_offset("track-a", -300).unwrap();
        assert_eq!(store.mv_link("track-a").unwrap().unwrap().offset_ms, -300);
        store.unlink_mv("track-a").unwrap();
        assert!(store.mv_link("track-a").unwrap().is_none());
    }
}
