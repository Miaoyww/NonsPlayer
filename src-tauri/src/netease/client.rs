//! Minimal HTTP client for Netease Cloud Music API.
//!
//! Receives fully-prepared requests (URL, headers, body) from the TypeScript
//! frontend and forwards them via reqwest.  All encryption, header building,
//! and response processing happen in TypeScript; Rust only handles the raw
//! HTTP POST and Set-Cookie tracking.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Serialize;

// ── Cookie store ──────────────────────────────────────────────────────────

/// Simple cookie jar that persists as JSON.
pub type CookieJar = HashMap<String, String>;

pub fn cookie_jar_from_json(json: &str) -> Option<CookieJar> {
    serde_json::from_str(json).ok()
}

// ── Cookie persistence ────────────────────────────────────────────────────

fn cookie_jar_path() -> PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    };
    base.join("NonsPlayer").join("netease_cookies.json")
}

pub fn save_cookie_jar(jar: &CookieJar) {
    let path = cookie_jar_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(jar) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn load_cookie_jar() -> Option<CookieJar> {
    let path = cookie_jar_path();
    if path.exists() {
        if let Ok(json) = std::fs::read_to_string(&path) {
            return cookie_jar_from_json(&json);
        }
    }
    None
}

// ── Tunnel response ───────────────────────────────────────────────────────

/// The response returned to TypeScript from the HTTP tunnel.
#[derive(Serialize, Clone)]
pub struct TunnelResponse {
    pub status: u16,
    /// Base64-encoded response body (safe for binary data).
    pub body: String,
    /// JSON-serialized cookie jar after processing Set-Cookie headers.
    pub cookies: String,
}

// ── NeteaseClient ─────────────────────────────────────────────────────────

pub struct NeteaseClient {
    client: Client,
    cookies: Mutex<CookieJar>,
    csrf: Mutex<Option<String>>,
}

impl NeteaseClient {
    pub fn new(http_client: Client, initial_cookies: Option<CookieJar>) -> Self {
        let cookies = initial_cookies.unwrap_or_default();
        let csrf = cookies.get("__csrf").cloned();
        Self {
            client: http_client,
            cookies: Mutex::new(cookies),
            csrf: Mutex::new(csrf),
        }
    }

    pub fn cookie_jar(&self) -> CookieJar {
        self.cookies.lock().unwrap().clone()
    }

    pub fn set_cookies(&self, jar: CookieJar) {
        let mut cookies = self.cookies.lock().unwrap();
        *cookies = jar;
        if let Some(csrf) = cookies.get("__csrf") {
            *self.csrf.lock().unwrap() = Some(csrf.clone());
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Parse Set-Cookie headers from a response and update the cookie jar.
    fn update_cookies(&self, response: &reqwest::Response) {
        let mut cookies = self.cookies.lock().unwrap();
        for header in response.headers().get_all("set-cookie") {
            if let Ok(cookie_str) = header.to_str() {
                for part in cookie_str.split(';') {
                    let part = part.trim();
                    if let Some(eq_pos) = part.find('=') {
                        let name = part[..eq_pos].to_string();
                        let value = part[eq_pos + 1..].to_string();
                        if !name.is_empty()
                            && !name.eq_ignore_ascii_case("path")
                            && !name.eq_ignore_ascii_case("domain")
                            && !name.eq_ignore_ascii_case("expires")
                            && !name.eq_ignore_ascii_case("max-age")
                            && !name.eq_ignore_ascii_case("httponly")
                            && !name.eq_ignore_ascii_case("secure")
                            && !name.eq_ignore_ascii_case("samesite")
                        {
                            cookies.insert(name, value);
                        }
                    }
                }
            }
        }
        // Update cached CSRF
        if let Some(csrf) = cookies.get("__csrf") {
            *self.csrf.lock().unwrap() = Some(csrf.clone());
        }
    }

    // ── Tunnel ──────────────────────────────────────────────────────────

    /// Send a raw HTTP POST request.  The TypeScript frontend has already
    /// built the URL, headers, and body.  Rust just forwards them.
    ///
    /// Returns a `TunnelResponse` with status, base64-encoded body, and the
    /// updated cookie jar (so TypeScript can sync).
    pub async fn tunnel(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<TunnelResponse> {
        use base64::Engine;

        let mut req = self.client.post(url);

        // Add all headers from TypeScript
        for (k, v) in &headers {
            // reqwest may override certain headers internally; skip Host
            if k.eq_ignore_ascii_case("host") {
                continue;
            }
            req = req.header(k.as_str(), v.as_str());
        }

        // Set body if provided
        if let Some(b) = body {
            req = req.body(b.to_string());
        }

        let response = req
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        let status = response.status().as_u16();

        // Process Set-Cookie headers into the jar
        self.update_cookies(&response);

        // Read response body as bytes, base64-encode for safe transport
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow!("read response body: {}", e))?;
        let body_b64 = base64::engine::general_purpose::STANDARD.encode(&body_bytes);

        // Serialize current cookie jar
        let cookies_json =
            serde_json::to_string(&*self.cookies.lock().unwrap()).unwrap_or_else(|_| "{}".into());

        log::info!("[tunnel] {} {} → {}", status, url, body_bytes.len());

        Ok(TunnelResponse {
            status,
            body: body_b64,
            cookies: cookies_json,
        })
    }
}
