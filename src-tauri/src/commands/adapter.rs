use std::path::PathBuf;

use serde::Deserialize;
use tauri::State;

use crate::adapters::local::LocalAdapter;
use crate::adapters::AdapterMetadata;
use crate::AppState;

/// Adapter configuration passed from the frontend at startup.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterConfig {
    /// Local music directories to scan.
    #[serde(default)]
    pub local_music_dirs: Vec<String>,
}

#[tauri::command]
pub fn scan_local(
    music_dirs: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AdapterMetadata>, String> {
    log::info!("[scan_local] dirs={:?}", music_dirs);
    let paths: Vec<PathBuf> = music_dirs.iter().map(PathBuf::from).collect();
    let local = LocalAdapter::new(paths).map_err(|e| e.to_string())?;
    state.adapters.register(local);
    Ok(state.adapters.list())
}

/// Initialize all adapters from config. Called once at app startup.
/// Returns metadata for all registered adapters.
///
/// Note: The "netease" adapter is handled entirely on the frontend via
/// `netease_request` / `netease_get_cookies` / `netease_set_cookies` commands.
/// No Rust-side adapter registration is needed.
#[tauri::command]
pub async fn init_adapters(
    config: AdapterConfig,
    state: State<'_, AppState>,
) -> Result<Vec<AdapterMetadata>, String> {
    log::info!(
        "[init_adapters] local_dirs={:?}",
        config.local_music_dirs,
    );

    // Local adapter
    if !config.local_music_dirs.is_empty() {
        let paths: Vec<PathBuf> = config.local_music_dirs.iter().map(PathBuf::from).collect();
        match LocalAdapter::new(paths) {
            Ok(local) => {
                state.adapters.register(local);
            }
            Err(e) => {
                log::warn!("[init_adapters] failed to init local adapter: {}", e);
            }
        }
    } else {
        log::info!("[init_adapters] no local music dirs configured, skipping local adapter");
    }

    let list = state.adapters.list();
    log::info!("[init_adapters] done, {} adapter(s) registered", list.len());
    Ok(list)
}

#[tauri::command]
pub fn list_adapters(state: State<'_, AppState>) -> Vec<AdapterMetadata> {
    state.adapters.list()
}
