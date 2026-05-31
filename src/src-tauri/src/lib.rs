mod adapters;
mod commands;
mod error;
mod models;
mod player;
mod server;
mod services;

use std::process::Child;
use std::sync::{Arc, Mutex};

use adapters::AdapterManager;
use player::play_queue::PlayQueue;
use services::http;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

pub struct AppState {
    pub adapters: AdapterManager,
    pub http_client: reqwest::Client,
    pub play_queue: Arc<PlayQueue>,
}

/// Holds the Netease API server child process handle and port.
/// On drop, kills the child process.
pub struct ServerProcess {
    pub child: Mutex<Option<Child>>,
    pub port: Mutex<u16>,
}

impl ServerProcess {
    pub fn new() -> Self {
        ServerProcess {
            child: Mutex::new(None),
            port: Mutex::new(37562),
        }
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.kill();
                log::info!("[server] Netease API server stopped");
            }
        }
    }
}

#[tauri::command]
fn get_api_port(state: tauri::State<'_, ServerProcess>) -> u16 {
    state.port.lock().map(|g| *g).unwrap_or(37562)
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
    let play_queue = Arc::new(PlayQueue::new());

    let state = AppState {
        adapters,
        http_client,
        play_queue,
    };

    tauri::Builder::default()
        .manage(state)
        .manage(ServerProcess::new())
        .setup(|app| {
            // Start the Netease API Express server in the background
            let result = server::start_api_server();
            let server_handle = app.state::<ServerProcess>();
            if let Some((child, port)) = result {
                if let Ok(mut guard) = server_handle.child.lock() {
                    *guard = Some(child);
                }
                if let Ok(mut guard) = server_handle.port.lock() {
                    *guard = port;
                }
            }
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
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
            // server
            get_api_port,
            // player
            commands::player::play,
            commands::player::pause,
            commands::player::resume,
            commands::player::toggle_playback,
            commands::player::seek,
            commands::player::set_volume,
            commands::player::get_position,
            commands::player::get_duration,
            commands::player::get_player_state,
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
