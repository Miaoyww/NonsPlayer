use std::sync::Arc;

use tauri::State;

use crate::adapters::{Adapter, SearchResult};
use crate::models::{album::Album, artist::Artist, playlist::Playlist, song::Song};
use crate::AppState;

fn get_adapter(
    adapter_name: &str,
    state: &AppState,
) -> Result<Arc<dyn Adapter>, String> {
    match state.adapters.get(adapter_name) {
        Some(a) => Ok(a),
        None => {
            let available: Vec<String> = state.adapters.list().into_iter().map(|m| m.slug).collect();
            log::warn!(
                "[get_adapter] unknown adapter \"{}\", available: {:?}",
                adapter_name,
                available
            );
            Err(format!("unknown adapter: {}", adapter_name))
        }
    }
}

#[tauri::command]
pub async fn get_song(
    adapter: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Song, String> {
    get_adapter(&adapter, &state)?
        .get_song(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_songs(
    adapter: String,
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Song>, String> {
    get_adapter(&adapter, &state)?
        .get_songs(&ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_song_url(
    adapter: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    get_adapter(&adapter, &state)?
        .get_song_url(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_lyric(
    adapter: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    get_adapter(&adapter, &state)?
        .get_lyric(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_like(
    adapter: String,
    id: String,
    like: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    get_adapter(&adapter, &state)?
        .toggle_like(&id, like)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_album(
    adapter: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Album, String> {
    get_adapter(&adapter, &state)?
        .get_album(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_artist(
    adapter: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Artist, String> {
    get_adapter(&adapter, &state)?
        .get_artist(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_playlist(
    adapter: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Playlist, String> {
    get_adapter(&adapter, &state)?
        .get_playlist(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search(
    adapter: String,
    keyword: String,
    state: State<'_, AppState>,
) -> Result<SearchResult, String> {
    log::info!("[search] adapter=\"{}\", keyword=\"{}\"", adapter, keyword);
    let result = get_adapter(&adapter, &state)?
        .search(&keyword)
        .await
        .map_err(|e| e.to_string())?;
    log::info!(
        "[search] {} songs, {} albums, {} artists, {} playlists",
        result.songs.len(),
        result.albums.len(),
        result.artists.len(),
        result.playlists.len()
    );
    Ok(result)
}

// -- Account commands --

#[tauri::command]
pub async fn login_qr_url(
    adapter: String,
    state: State<'_, AppState>,
) -> Result<(String, String), String> {
    get_adapter(&adapter, &state)?
        .login_qr_url()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_login(
    adapter: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<crate::adapters::LoginStatus, String> {
    get_adapter(&adapter, &state)?
        .check_login(&key)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_account(
    adapter: String,
    state: State<'_, AppState>,
) -> Result<crate::models::account::Account, String> {
    get_adapter(&adapter, &state)?
        .get_account()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_user_playlists(
    adapter: String,
    state: State<'_, AppState>,
) -> Result<Vec<Playlist>, String> {
    get_adapter(&adapter, &state)?
        .get_user_playlists()
        .await
        .map_err(|e| e.to_string())
}

// -- Recommend commands --

#[tauri::command]
pub async fn get_recommended_playlists(
    adapter: String,
    count: u32,
    state: State<'_, AppState>,
) -> Result<Vec<Playlist>, String> {
    get_adapter(&adapter, &state)?
        .get_recommended_playlists(count)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_daily_recommended(
    adapter: String,
    state: State<'_, AppState>,
) -> Result<Vec<Song>, String> {
    get_adapter(&adapter, &state)?
        .get_daily_recommended()
        .await
        .map_err(|e| e.to_string())
}
