use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// Statistics for the TTML cache directory.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub count: u64,
    pub size_bytes: u64,
}

/// Resolve the lyric data directory: `{app_data_dir}/lyrics`.
/// Note: `app_data_dir()` already returns e.g. `%APPDATA%/NonsPlayer` on Windows.
fn lyric_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {}", e))?;
    Ok(data_dir.join("lyrics"))
}

/// Resolve the TTML cache directory: `{lyric_dir}/ttml`.
fn ttml_cache_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(lyric_dir(app_handle)?.join("ttml"))
}

/// Sanitize a song ID so it can be used as a safe filename.
/// Replaces characters that are invalid on Windows/Linux/macOS.
fn sanitize_filename(name: &str) -> String {
    name.replace(
        |c: char| {
            c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|'
        },
        "_",
    )
}

// ── TTML Cache Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn get_app_lyric_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    lyric_dir(&app_handle).map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn save_ttml_cache(
    app_handle: tauri::AppHandle,
    song_id: String,
    content: String,
) -> Result<(), String> {
    let dir = ttml_cache_dir(&app_handle)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create ttml cache dir: {}", e))?;

    let safe_name = sanitize_filename(&song_id);
    let path = dir.join(format!("{}.ttml", safe_name));
    fs::write(&path, content).map_err(|e| format!("write ttml cache: {}", e))?;

    log::info!("[lyric_cache] saved TTML cache: {}", song_id);
    Ok(())
}

#[tauri::command]
pub fn get_ttml_cache(
    app_handle: tauri::AppHandle,
    song_id: String,
) -> Result<Option<String>, String> {
    let dir = ttml_cache_dir(&app_handle)?;
    let safe_name = sanitize_filename(&song_id);
    let path = dir.join(format!("{}.ttml", safe_name));

    match fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => {
            // Corrupt or unreadable file — remove it so we don't retry forever
            let _ = fs::remove_file(&path);
            Err(format!("read ttml cache (file removed): {}", e))
        }
    }
}

#[tauri::command]
pub fn clear_ttml_cache(app_handle: tauri::AppHandle) -> Result<(), String> {
    let dir = ttml_cache_dir(&app_handle)?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("clear ttml cache: {}", e))?;
        fs::create_dir_all(&dir).map_err(|e| format!("recreate ttml cache dir: {}", e))?;
    }
    log::info!("[lyric_cache] cleared TTML cache");
    Ok(())
}

#[tauri::command]
pub fn delete_ttml_cache(
    app_handle: tauri::AppHandle,
    song_id: String,
) -> Result<(), String> {
    let dir = ttml_cache_dir(&app_handle)?;
    let safe_name = sanitize_filename(&song_id);
    let path = dir.join(format!("{}.ttml", safe_name));

    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("delete ttml cache: {}", e)),
    }
}

#[tauri::command]
pub fn get_ttml_cache_stats(app_handle: tauri::AppHandle) -> Result<CacheStats, String> {
    let dir = ttml_cache_dir(&app_handle)?;
    if !dir.exists() {
        return Ok(CacheStats { count: 0, size_bytes: 0 });
    }

    let mut count: u64 = 0;
    let mut size_bytes: u64 = 0;

    let entries = fs::read_dir(&dir).map_err(|e| format!("read ttml cache dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read entry: {}", e))?;
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                count += 1;
                size_bytes += meta.len();
            }
        }
    }

    Ok(CacheStats { count, size_bytes })
}

// ── Lyric Map Commands ────────────────────────────────────────────────

fn lyric_map_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(lyric_dir(app_handle)?.join("lyric_map.json"))
}

#[tauri::command]
pub fn save_lyric_map(app_handle: tauri::AppHandle, json: String) -> Result<(), String> {
    let dir = lyric_dir(&app_handle)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create lyric dir: {}", e))?;

    let path = lyric_map_path(&app_handle)?;
    fs::write(&path, &json).map_err(|e| format!("write lyric map: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_lyric_map(app_handle: tauri::AppHandle) -> Result<String, String> {
    let path = lyric_map_path(&app_handle)?;

    match fs::read_to_string(&path) {
        Ok(json) => Ok(json),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Return empty object if no map exists yet
            Ok("{}".to_string())
        }
        Err(e) => Err(format!("read lyric map: {}", e)),
    }
}
