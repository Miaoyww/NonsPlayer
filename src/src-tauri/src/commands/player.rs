use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::adapters::Adapter;
use crate::models::song::Song;
use crate::player::engine::PlayerState;
use crate::player::play_queue::{PlayMode, QueueItem};
use crate::AppState;

fn get_adapter(
    adapter_slug: &str,
    state: &AppState,
) -> Result<Arc<dyn Adapter>, String> {
    state
        .adapters
        .get(adapter_slug)
        .ok_or_else(|| format!("unknown adapter: {}", adapter_slug))
}

/// Load a list of songs into the queue and start playing from the specified index.
#[tauri::command]
pub async fn play(
    adapter: String,
    _song_id: String,
    queue_songs: Vec<Song>,   // full playlist / album to load into the queue
    start_index: usize,       // which position to start at
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let queue = &state.play_queue;

    // Build queue items from the provided list
    let items: Vec<QueueItem> = queue_songs
        .into_iter()
        .map(|s| {
            let slug = if s.adapter_slug.is_empty() {
                adapter.clone()
            } else {
                s.adapter_slug.clone()
            };
            QueueItem {
                song: s,
                adapter_slug: slug,
            }
        })
        .collect();

    queue.set_list(items);
    queue.jump_to(start_index);

    // Play the current item
    play_current(&state, &app).await
}

/// Play the item at the current queue position.
async fn play_current(state: &AppState, app: &AppHandle) -> Result<(), String> {
    let item = state
        .play_queue
        .current()
        .ok_or_else(|| "queue is empty".to_string())?;

    let adapter = get_adapter(&item.adapter_slug, state)?;
    let url = adapter
        .get_song_url(&item.song.id)
        .await
        .map_err(|e| e.to_string())?;

    // Determine if this is a local file or remote URL
    if url.starts_with("file://") {
        let path = url.strip_prefix("file://").unwrap_or(&url);
        state.player_engine.play_file(path).map_err(|e| e.to_string())?;
    } else {
        state.player_engine.play_url(&url).map_err(|e| e.to_string())?;
    }

    // Notify frontend of track change
    app.emit("track-changed", &item.song).ok();

    // Notify frontend of player state
    app.emit(
        "player-state-changed",
        serde_json::json!({ "state": "playing" }),
    )
    .ok();

    Ok(())
}

#[tauri::command]
pub fn pause(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.player_engine.pause();
    app.emit(
        "player-state-changed",
        serde_json::json!({ "state": "paused" }),
    )
    .ok();
    Ok(())
}

#[tauri::command]
pub fn resume(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.player_engine.resume();
    app.emit(
        "player-state-changed",
        serde_json::json!({ "state": "playing" }),
    )
    .ok();
    Ok(())
}

#[tauri::command]
pub fn toggle_playback(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.player_engine.toggle_playback();
    let st = match state.player_engine.get_state() {
        PlayerState::Playing => "playing",
        PlayerState::Paused => "paused",
        _ => "stopped",
    };
    app.emit("player-state-changed", serde_json::json!({ "state": st })).ok();
    Ok(())
}

#[tauri::command]
pub fn seek(seconds: f64, state: State<'_, AppState>) -> Result<(), String> {
    state.player_engine.seek(seconds);
    Ok(())
}

#[tauri::command]
pub fn set_volume(vol: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.player_engine.set_volume(vol);
    Ok(())
}

#[tauri::command]
pub fn get_position(state: State<'_, AppState>) -> Result<f64, String> {
    Ok(state.player_engine.get_position())
}

#[tauri::command]
pub fn get_duration(state: State<'_, AppState>) -> Result<f64, String> {
    Ok(state.player_engine.get_duration())
}

#[tauri::command]
pub async fn next(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let item = state.play_queue.next();
    match item {
        Some(_) => play_current(&state, &app).await,
        None => {
            state.player_engine.stop();
            app.emit(
                "player-state-changed",
                serde_json::json!({ "state": "stopped" }),
            )
            .ok();
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn prev(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let item = state.play_queue.prev();
    match item {
        Some(_) => play_current(&state, &app).await,
        None => Ok(()), // stay at current track
    }
}

#[tauri::command]
pub fn set_play_mode(
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mode = match mode.as_str() {
        "sequential" => PlayMode::Sequential,
        "shuffle" => PlayMode::Shuffle,
        "single_loop" => PlayMode::SingleLoop,
        "list_loop" => PlayMode::ListLoop,
        other => return Err(format!("unknown play mode: {}", other)),
    };

    if mode == PlayMode::Shuffle {
        state.play_queue.reshuffle();
    }
    state.play_queue.set_mode(mode);
    Ok(())
}

#[tauri::command]
pub fn get_play_mode(state: State<'_, AppState>) -> Result<String, String> {
    let mode = match state.play_queue.get_mode() {
        PlayMode::Sequential => "sequential",
        PlayMode::Shuffle => "shuffle",
        PlayMode::SingleLoop => "single_loop",
        PlayMode::ListLoop => "list_loop",
    };
    Ok(mode.to_string())
}

#[tauri::command]
pub fn get_queue(state: State<'_, AppState>) -> Result<Vec<Song>, String> {
    Ok(state
        .play_queue
        .get_all()
        .into_iter()
        .map(|item| item.song)
        .collect())
}

#[tauri::command]
pub fn get_current_song(state: State<'_, AppState>) -> Result<Option<Song>, String> {
    Ok(state.play_queue.current().map(|item| item.song))
}
