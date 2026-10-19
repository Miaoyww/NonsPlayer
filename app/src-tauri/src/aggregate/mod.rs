//! Device-owned playlists. Remote relationships are durable user data, not caches.
mod store;
pub(crate) mod sync;
#[cfg(test)]
mod tests;
use crate::{model::AppResult, Backend};
pub(crate) use store::{migrate, playlist, playlists, tracks, write, Playlist, ReadPage, Write};
pub(crate) use sync::SyncJobs;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn aggregate_play(
    id: String,
    key: Option<String>,
    next: Option<bool>,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<u32> {
    let store = backend.store.clone();
    let (tracks, skipped) = tauri::async_runtime::spawn_blocking(move || store::queue(&store, &id))
        .await
        .map_err(|e| e.to_string())??;
    if tracks.is_empty() {
        if skipped > 0 && !next.unwrap_or(false) {
            backend.player.send(crate::player::Command::Stop)?;
        }
        return Err("歌单中没有可播放的歌曲".into());
    }
    let index = match key {
        Some(key) => tracks
            .iter()
            .position(|t| t.key == key)
            .ok_or("所选歌曲已移除、已排除或不可用")?,
        None => 0,
    };
    backend.player.send(if next.unwrap_or(false) {
        crate::player::Command::PlayNext(tracks)
    } else {
        crate::player::Command::Queue(tracks, index)
    })?;
    if skipped > 0 {
        let _ = app.emit(
            "playback-notice",
            format!("已跳过 {skipped} 首已退出曲库的歌曲。"),
        );
    }
    Ok(skipped)
}

#[tauri::command]
pub async fn aggregate_playlists(backend: State<'_, Backend>) -> AppResult<Vec<Playlist>> {
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || playlists(&store))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn aggregate_playlist(id: String, backend: State<'_, Backend>) -> AppResult<Playlist> {
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || playlist(&store, &id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn aggregate_tracks(
    id: String,
    offset: u32,
    excluded: Option<bool>,
    backend: State<'_, Backend>,
) -> AppResult<ReadPage> {
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        tracks(&store, &id, offset, excluded.unwrap_or(false))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn aggregate_write(
    request: Write,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<String> {
    let store = backend.store.clone();
    let jobs = backend.aggregate.clone();
    let id = tauri::async_runtime::spawn_blocking(move || {
        // Remove completed links before cancelling their outstanding task.
        let mut active = jobs.active.lock().map_err(|_| "同步状态不可用")?;
        let result = write(&store, &request)?;
        if matches!(request, Write::Delete { .. } | Write::Unlink { .. }) {
            if let Some(job) = active.remove(&result) {
                job.cancel.send_replace(true);
            }
        }
        Ok::<_, String>(result)
    })
    .await
    .map_err(|e| e.to_string())??;
    changed(&app, &id);
    Ok(id)
}
pub(crate) fn changed(app: &tauri::AppHandle, id: &str) {
    if let Err(error) = app.emit("aggregate-playlist-updated", id) {
        crate::infrastructure::diagnostics::warn("aggregate.emit", error);
    }
}
