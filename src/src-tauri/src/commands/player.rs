use tauri::{AppHandle, Emitter, State};

use crate::models::song::Song;
use crate::player::play_queue::{PlayMode, QueueItem};
use crate::AppState;

/// Load a list of songs into the backend queue (for state tracking).
/// Actual playback is handled entirely by the frontend Web Audio API.
#[tauri::command]
pub async fn play(
    adapter: String,
    _song_id: String,
    queue_songs: Vec<Song>,
    start_index: usize,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let queue = &state.play_queue;

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

    // Emit track-changed so the frontend can update the current song
    if let Some(item) = queue.current() {
        app.emit("track-changed", &item.song).ok();
    }

    Ok(())
}

#[tauri::command]
pub fn pause(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    // Playback is now frontend-driven; emit event for other listeners if needed
    app.emit(
        "player-state-changed",
        serde_json::json!({ "state": "paused" }),
    )
    .ok();
    let _ = state;
    Ok(())
}

#[tauri::command]
pub fn resume(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    app.emit(
        "player-state-changed",
        serde_json::json!({ "state": "playing" }),
    )
    .ok();
    let _ = state;
    Ok(())
}

#[tauri::command]
pub fn toggle_playback(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    app.emit(
        "player-state-changed",
        serde_json::json!({ "state": "toggled" }),
    )
    .ok();
    let _ = state;
    Ok(())
}

#[tauri::command]
pub fn seek(seconds: f64, state: State<'_, AppState>) -> Result<(), String> {
    let _ = (seconds, state);
    Ok(())
}

#[tauri::command]
pub fn set_volume(vol: f32, state: State<'_, AppState>) -> Result<(), String> {
    let _ = (vol, state);
    Ok(())
}

#[tauri::command]
pub fn get_position(state: State<'_, AppState>) -> Result<f64, String> {
    let _ = state;
    Ok(0.0)
}

#[tauri::command]
pub fn get_duration(state: State<'_, AppState>) -> Result<f64, String> {
    let _ = state;
    Ok(0.0)
}

#[tauri::command]
pub fn get_player_state(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let _ = state;
    Ok(serde_json::json!({
        "position": 0.0,
        "duration": 0.0,
        "isPlaying": false,
        "streaming": {
            "isStalled": false,
            "bytesDownloaded": 0,
            "bytesTotal": 0,
            "downloadPercent": 0.0,
            "stallCount": 0,
            "isLive": false
        }
    }))
}

#[tauri::command]
pub async fn next(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let item = state.play_queue.next();
    if let Some(item) = item {
        app.emit("track-changed", &item.song).ok();
    } else {
        app.emit(
            "player-state-changed",
            serde_json::json!({ "state": "stopped" }),
        )
        .ok();
    }
    Ok(())
}

#[tauri::command]
pub async fn prev(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let item = state.play_queue.prev();
    if let Some(item) = item {
        app.emit("track-changed", &item.song).ok();
    }
    Ok(())
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
