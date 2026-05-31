use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tauri::Manager;

const CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

/// Resolve the lyric cache directory: `{app_data_dir}/lyrics`.
fn lyric_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {}", e))?;
    Ok(data_dir.join("lyrics"))
}

/// Sanitize a song ID so it can be used as a safe filename.
fn sanitize(name: &str) -> String {
    name.replace(
        |c: char| {
            c == '/'
                || c == '\\'
                || c == ':'
                || c == '*'
                || c == '?'
                || c == '"'
                || c == '<'
                || c == '>'
                || c == '|'
        },
        "_",
    )
}

// ── Cache Commands ────────────────────────────────────────────────────

#[tauri::command]
pub fn save_lyric_cache(
    app_handle: tauri::AppHandle,
    song_id: String,
    format: String,
    content: String,
) -> Result<(), String> {
    let dir = lyric_dir(&app_handle)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create lyric dir: {}", e))?;

    let safe_name = sanitize(&song_id);
    let path = dir.join(format!("{}.{}", safe_name, format));
    fs::write(&path, content).map_err(|e| format!("write lyric cache: {}", e))
}

#[tauri::command]
pub fn get_lyric_cache(
    app_handle: tauri::AppHandle,
    song_id: String,
    format: String,
) -> Result<Option<String>, String> {
    let dir = lyric_dir(&app_handle)?;
    let safe_name = sanitize(&song_id);
    let path = dir.join(format!("{}.{}", safe_name, format));

    let meta = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("stat lyric cache: {}", e)),
    };

    // Check TTL
    if let Ok(modified) = meta.modified() {
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default()
            .as_secs();
        if age > CACHE_TTL_SECS {
            let _ = fs::remove_file(&path);
            return Ok(None);
        }
    }

    match fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content)),
        Err(e) => {
            let _ = fs::remove_file(&path);
            Err(format!("read lyric cache (file removed): {}", e))
        }
    }
}

#[tauri::command]
pub fn clear_lyric_cache(app_handle: tauri::AppHandle) -> Result<(), String> {
    let dir = lyric_dir(&app_handle)?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("clear lyric cache: {}", e))?;
        fs::create_dir_all(&dir).map_err(|e| format!("recreate lyric dir: {}", e))?;
    }
    Ok(())
}
