mod adapters;
mod commands;
mod error;
mod models;
mod services;

use adapters::AdapterManager;
use services::http;

pub struct AppState {
    pub adapters: AdapterManager,
    pub http_client: reqwest::Client,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let adapters = AdapterManager::new();
    let http_client = http::create_client();

    let state = AppState {
        adapters,
        http_client,
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            // adapter management
            commands::adapter::scan_local,
            commands::adapter::list_adapters,
            // music
            commands::music::get_song,
            commands::music::get_songs,
            commands::music::get_song_url,
            commands::music::get_lyric,
            commands::music::toggle_like,
            commands::music::get_album,
            commands::music::get_artist,
            commands::music::get_playlist,
            commands::music::search,
            // account
            commands::music::login_qr_url,
            commands::music::check_login,
            commands::music::get_account,
            commands::music::get_user_playlists,
            // recommend
            commands::music::get_recommended_playlists,
            commands::music::get_daily_recommended,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
