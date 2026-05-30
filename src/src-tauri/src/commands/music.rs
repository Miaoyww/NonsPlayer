use std::sync::Arc;

use tauri::State;

use crate::adapters::{Adapter, MatchResult, PlaylistCategory, SearchResult, TopPlaylistGroup};
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

#[tauri::command]
pub async fn get_favorite_playlist(
    adapter: String,
    state: State<'_, AppState>,
) -> Result<Option<Playlist>, String> {
    get_adapter(&adapter, &state)?
        .get_favorite_playlist()
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

// -- Discover commands --

#[tauri::command]
pub async fn get_top_playlists(
    adapter: String,
    state: State<'_, AppState>,
) -> Result<Vec<TopPlaylistGroup>, String> {
    get_adapter(&adapter, &state)?
        .get_top_playlists()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_playlist_cats(
    adapter: String,
    state: State<'_, AppState>,
) -> Result<Vec<PlaylistCategory>, String> {
    get_adapter(&adapter, &state)?
        .get_playlist_cats()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_playlist_square(
    adapter: String,
    cat: String,
    order: String,
    limit: u32,
    offset: u32,
    high_quality: bool,
    state: State<'_, AppState>,
) -> Result<(Vec<Playlist>, usize), String> {
    get_adapter(&adapter, &state)?
        .get_playlist_square(&cat, &order, limit, offset, high_quality)
        .await
        .map_err(|e| e.to_string())
}

/// Fan out `match_song` to all registered adapters concurrently.
/// Returns aggregated results sorted by score (best first).
#[tauri::command]
pub async fn match_song_across_adapters(
    name: String,
    artist: String,
    state: State<'_, AppState>,
) -> Result<Vec<MatchResult>, String> {
    log::info!(
        "[match_song] fanning out: name=\"{}\" artist=\"{}\"",
        name, artist
    );

    let adapters: Vec<Arc<dyn Adapter>> = state
        .adapters
        .list()
        .into_iter()
        .filter_map(|m| {
            let a = state.adapters.get(&m.slug)?;
            log::info!("[match_song] dispatching to adapter: {}", m.slug);
            Some(a)
        })
        .collect();

    if adapters.is_empty() {
        log::info!("[match_song] no adapters registered, returning empty");
        return Ok(vec![]);
    }

    let handles: Vec<_> = adapters
        .iter()
        .map(|a| {
            let name = name.clone();
            let artist = artist.clone();
            let adapter = Arc::clone(a);
            let slug = adapter.metadata().slug.clone();
            tauri::async_runtime::spawn(async move {
                let result = adapter.match_song(&name, &artist).await;
                (slug, result)
            })
        })
        .collect();

    let mut results: Vec<MatchResult> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((slug, Ok(Some(r)))) => {
                log::info!(
                    "[match_song] {} → score={:.4} id={}",
                    slug, r.score, r.numeric_id
                );
                results.push(r);
            }
            Ok((slug, Ok(None))) => {
                log::info!("[match_song] {} → no match", slug);
            }
            Ok((slug, Err(e))) => {
                log::warn!("[match_song] {} → error: {}", slug, e);
            }
            Err(e) => log::warn!("[match_song] join error: {}", e),
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    log::info!(
        "[match_song] DONE: \"{}\" - \"{}\" → {} results across {} adapters",
        name,
        artist,
        results.len(),
        adapters.len()
    );

    Ok(results)
}
