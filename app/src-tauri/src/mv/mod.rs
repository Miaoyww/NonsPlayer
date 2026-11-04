//! Temporary per-track local MV association for the fullscreen player.
//! Remove `local_mv_links` and these commands when MV becomes a first-class media source.

use crate::{application::Backend, model::AppResult};
use serde::Serialize;
use std::path::Path;
use tauri::{Manager, State};

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
        app.asset_protocol_scope()
            .allow_file(&path)
            .map_err(|error| error.to_string())?;
        backend.store.link_local_mv(&key, &path_text)?;
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
}
