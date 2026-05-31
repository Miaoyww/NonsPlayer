mod adapters;
mod commands;
mod error;
mod models;
mod services;

use adapters::AdapterManager;
use services::http;
use tauri_plugin_log::{Target, TargetKind};

pub struct AppState {
    pub adapters: AdapterManager,
    pub http_client: reqwest::Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Capture panics to a file for debugging
    let _ = std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {:?}", info);
        eprintln!("{}", msg);
        let _ = std::fs::write(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crash.log"),
            &msg,
        );
    }));

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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([Target::new(TargetKind::Stdout), Target::new(TargetKind::Webview)])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            // adapter management
            commands::adapter::scan_local,
            commands::adapter::init_adapters,
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
            commands::music::get_favorite_playlist,
            // recommend
            commands::music::get_recommended_playlists,
            commands::music::get_daily_recommended,
            // discover
            commands::music::get_top_playlists,
            commands::music::get_playlist_cats,
            commands::music::get_playlist_square,
            // lyric cache
            commands::lyric_cache::save_lyric_cache,
            commands::lyric_cache::get_lyric_cache,
            commands::lyric_cache::clear_lyric_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
