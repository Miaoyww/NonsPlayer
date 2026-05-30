mod adapters;
mod commands;
mod error;
mod models;
mod player;
mod services;

use std::sync::Arc;

use adapters::AdapterManager;
use player::engine::BassEngine;
use player::ffi::BassLib;
use player::play_queue::PlayQueue;
use services::http;
use tauri_plugin_log::{Target, TargetKind};

pub struct AppState {
    pub adapters: AdapterManager,
    pub http_client: reqwest::Client,
    pub player_engine: Option<Arc<BassEngine>>,
    pub play_queue: Arc<PlayQueue>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
    let player_engine = match BassLib::load() {
        Ok(bass) => {
            eprintln!("[startup] BASS library loaded successfully");
            let engine = BassEngine::new(Arc::new(bass));
            match engine {
                Ok(e) => {
                    eprintln!("[startup] BASS engine initialized");
                    Some(Arc::new(e))
                }
                Err(e) => {
                    eprintln!("[startup] BASS engine init failed: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("[startup] BASS library load failed: {}", e);
            None
        }
    };
    let play_queue = Arc::new(PlayQueue::new());

    let state = AppState {
        adapters,
        http_client,
        player_engine,
        play_queue,
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([Target::new(TargetKind::Webview)])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            greet,
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
            // player
            commands::player::play,
            commands::player::pause,
            commands::player::resume,
            commands::player::toggle_playback,
            commands::player::seek,
            commands::player::set_volume,
            commands::player::get_position,
            commands::player::get_duration,
            commands::player::next,
            commands::player::prev,
            commands::player::set_play_mode,
            commands::player::get_play_mode,
            commands::player::get_queue,
            commands::player::get_current_song,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
