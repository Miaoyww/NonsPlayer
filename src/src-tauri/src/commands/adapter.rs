use std::path::PathBuf;

use serde::Deserialize;
use tauri::State;

use crate::adapters::local::LocalAdapter;
use crate::adapters::AdapterMetadata;
use crate::AppState;

/// Adapter configuration passed from the frontend at startup.
#[derive(Debug, Clone, Deserialize)]
pub struct AdapterConfig {
    /// Local music directories to scan.
    #[serde(default)]
    pub local_music_dirs: Vec<String>,
    // Future fields:
    // pub netease_api_base: Option<String>,
    // pub qqmusic_enabled: bool,
}

#[tauri::command]
pub fn scan_local(
    music_dirs: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AdapterMetadata>, String> {
    let paths: Vec<PathBuf> = music_dirs.iter().map(PathBuf::from).collect();
    let local = LocalAdapter::new(paths).map_err(|e| e.to_string())?;
    state.adapters.register(local);
    Ok(state.adapters.list())
}

/// Initialize all adapters from config. Called once at app startup.
/// Returns metadata for all registered adapters.
#[tauri::command]
pub fn init_adapters(
    config: AdapterConfig,
    state: State<'_, AppState>,
) -> Result<Vec<AdapterMetadata>, String> {
    // Local adapter
    if !config.local_music_dirs.is_empty() {
        let paths: Vec<PathBuf> = config.local_music_dirs.iter().map(PathBuf::from).collect();
        match LocalAdapter::new(paths) {
            Ok(local) => {
                state.adapters.register(local);
            }
            Err(e) => {
                eprintln!("warning: failed to init local adapter: {}", e);
            }
        }
    }

    // Future: init netease, qqmusic, etc. from config

    Ok(state.adapters.list())
}

#[tauri::command]
pub fn list_adapters(state: State<'_, AppState>) -> Vec<AdapterMetadata> {
    state.adapters.list()
}
