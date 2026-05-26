use std::path::PathBuf;

use tauri::State;

use crate::adapters::local::LocalAdapter;
use crate::adapters::AdapterMetadata;
use crate::AppState;

#[tauri::command]
pub fn scan_local(
    music_dirs: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AdapterMetadata>, String> {
    let paths: Vec<PathBuf> = music_dirs.iter().map(PathBuf::from).collect();
    let local = LocalAdapter::new(paths).map_err(|e| e.to_string())?;
    // Register the adapter so it's available for subsequent music commands.
    state.adapters.register(local);
    Ok(state.adapters.list())
}

#[tauri::command]
pub fn list_adapters(state: State<'_, AppState>) -> Vec<AdapterMetadata> {
    state.adapters.list()
}
