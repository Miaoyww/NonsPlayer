//! Tauri commands for the Netease HTTP tunnel.
//!
//! The frontend handles all encryption, URL building, and header construction.
//! Rust only does the raw HTTP POST and tracks Set-Cookie headers.

use std::collections::HashMap;

use tauri::State;
use tokio::sync::Mutex;

use crate::netease::{save_cookie_jar, CookieJar, NeteaseClient, TunnelResponse};

/// Managed state holding the NeteaseClient.
pub struct NeteaseClientState {
    pub client: Mutex<NeteaseClient>,
}

/// Raw HTTP tunnel — TypeScript prepares everything, Rust just sends.
///
/// - `url`: full URL (built by TypeScript)
/// - `headers`: all request headers including Cookie, User-Agent, etc.
/// - `body`: POST body (url-encoded form string)
///
/// Returns a `TunnelResponse` with status, base64 body, and updated cookies.
#[tauri::command]
pub async fn netease_tunnel(
    state: State<'_, NeteaseClientState>,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
) -> Result<TunnelResponse, String> {
    let client = state.client.lock().await;
    let result = client
        .tunnel(&url, headers, body.as_deref())
        .await
        .map_err(|e| {
            log::error!("[netease_tunnel] {}: {}", url, e);
            e.to_string()
        })?;

    log::info!("Tunnel Success: {} → {}", url, result.status);

    // Persist cookies after every request
    save_cookie_jar(&client.cookie_jar());

    Ok(result)
}

/// Get the current cookies from the NeteaseClient as a JSON string.
#[tauri::command]
pub async fn netease_get_cookies(
    state: State<'_, NeteaseClientState>,
) -> Result<String, String> {
    let client = state.client.lock().await;
    let jar = client.cookie_jar();
    serde_json::to_string(&jar).map_err(|e| e.to_string())
}

/// Set cookies on the NeteaseClient from a JSON string.
#[tauri::command]
pub async fn netease_set_cookies(
    state: State<'_, NeteaseClientState>,
    cookies: String,
) -> Result<(), String> {
    let jar: CookieJar = serde_json::from_str(&cookies).map_err(|e| e.to_string())?;
    let client = state.client.lock().await;
    client.set_cookies(jar);
    Ok(())
}
