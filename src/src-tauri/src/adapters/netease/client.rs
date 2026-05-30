use std::sync::Mutex;

use rand::Rng;

use crate::error::{Error, Result};

use super::crypto;

// ── Base hosts ──

pub const MUSIC_HOST: &str = "https://music.163.com";
pub const INTERFACE_HOST: &str = "https://interface.music.163.com";
pub const INTERFACE3_HOST: &str = "https://interface3.music.163.com";

// ── Crypto type ──

pub enum CryptoType {
    Weapi,
    Eapi,
    Api,
}

// ── User-Agent pool ──

const USER_AGENTS: &[&str] = &[
    // Mobile
    "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (Linux; Android 13; SM-G998B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/108.0.5359.112 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (Linux; Android 12; SM-G991B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Mobile Safari/537.36",
    // PC
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_0_1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_0_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.1 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:107.0) Gecko/20100101 Firefox/107.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_0_1; rv:107.0) Gecko/20100101 Firefox/107.0",
    "Mozilla/5.0 (X11; Linux i686; rv:107.0) Gecko/20100101 Firefox/107.0",
];

fn random_ua() -> &'static str {
    let idx = rand::thread_rng().gen_range(0..USER_AGENTS.len());
    USER_AGENTS[idx]
}

// ── Hardcoded anonymous token (from NeteaseCloudMusicApi config) ──

const DEFAULT_ANONYMOUS_TOKEN: &str = "bf8bfeabb1aa84f9c8c3906c04a04fb864322804c83f5d607e91a04eae463c9436bd1a17ec353cf780b396507a3f7464e8a60f4bbc019437993166e004087dd32d1490298caf655c2353e58daa0bc13cc7d5c198250968580b12c1b8817e3f5c807e650dd04abd3fb8130b7ae43fcc5b";

/// Base64-encoded username used to register an anonymous token.
const ANON_USERNAME_B64: &str = "MzEwMjcwYmY0Y2Y0ODcwMzU0ZDFkZmIxMmMzMGYyMTkgVlBaanMwNmtrb1BYMGxOVzVUMUJ3Zz09";

// ── Client struct ──

pub struct NeteaseClient {
    http: reqwest::Client,
    cookies: Mutex<String>,
    anonymous_token: Mutex<String>,
}

impl NeteaseClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            cookies: Mutex::new(String::new()),
            anonymous_token: Mutex::new(DEFAULT_ANONYMOUS_TOKEN.to_string()),
        }
    }

    /// Build the Cookie header for a request.
    fn build_cookie(&self) -> String {
        let mut parts: Vec<String> = vec![
            "__remember_me=true".into(),
            format!("NMTID={}", crypto::random_hex(16)),
            format!("_ntes_nuid={}", crypto::random_hex(16)),
        ];

        let saved = self.cookies.lock().unwrap().clone();
        if !saved.is_empty() {
            // Use user's cookies (contains MUSIC_U if logged in)
            parts.push(saved);
        } else {
            // Anonymous: inject MUSIC_A
            let anon = self.anonymous_token.lock().unwrap().clone();
            parts.push(format!("MUSIC_A={}", anon));
        }

        parts.join("; ")
    }

    /// Check if user is logged in (has MUSIC_U cookie).
    pub fn is_logged_in(&self) -> bool {
        self.cookies.lock().unwrap().contains("MUSIC_U")
    }

    /// Get the current cookie string (for exposing via Account).
    pub fn cookies_str(&self) -> String {
        self.cookies.lock().unwrap().clone()
    }

    /// Save cookies from a login/register response.
    pub fn save_cookies(&self, cookie_str: &str) {
        *self.cookies.lock().unwrap() = cookie_str.to_string();
    }

    /// Bootstrap anonymous token by calling /api/register/anonimous.
    pub async fn bootstrap_anonymous(&self) -> Result<()> {
        let data = serde_json::json!({ "username": ANON_USERNAME_B64 });
        let body = self
            .request(CryptoType::Weapi, &format!("{}/weapi/register/anonimous", MUSIC_HOST), &data)
            .await?;

        if let Some(cookie) = body["cookie"].as_str() {
            self.save_cookies(cookie);
            // Also cache the MUSIC_A part
            if let Some(ma) = cookie.split(';').find(|p| p.trim().starts_with("MUSIC_A=")) {
                let token = ma.trim().strip_prefix("MUSIC_A=").unwrap_or("");
                *self.anonymous_token.lock().unwrap() = token.to_string();
            }
        }
        Ok(())
    }

    /// Core request method. Handles encryption, cookie injection, and response parsing.
    ///
    /// - `crypto_type`: Weapi/Eapi/Api
    /// - `url`: Full upstream URL (e.g., "https://music.163.com/weapi/v3/song/detail")
    /// - `payload`: JSON object with the request parameters
    pub async fn request(
        &self,
        crypto_type: CryptoType,
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let ua = random_ua();
        let json_text = serde_json::to_string(payload)
            .map_err(|e| Error::Other(format!("json serialize: {}", e)))?;

        let (form_body, is_binary_response) = match crypto_type {
            CryptoType::Weapi => {
                let (params, enc_sec_key) = crypto::weapi(&json_text);
                (vec![("params".into(), params), ("encSecKey".into(), enc_sec_key)], false)
            }
            CryptoType::Eapi => {
                let url_path = Self::extract_path(url);
                let params = crypto::eapi(url_path, &json_text);
                (vec![("params".into(), params)], true)
            }
            CryptoType::Api => {
                // Plain form data from the payload object
                let params: Vec<(String, String)> = payload
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), val_to_form_string(v)))
                            .collect()
                    })
                    .unwrap_or_default();
                (params, false)
            }
        };

        let cookie = self.build_cookie();

        let form_pairs: Vec<(&str, &str)> = form_body
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let resp = self
            .http
            .post(url)
            .header("Cookie", &cookie)
            .header("User-Agent", ua)
            .header("Referer", MUSIC_HOST)
            .form(&form_pairs)
            .send()
            .await
            .map_err(Error::Http)?;

        if is_binary_response {
            // eapi response: binary, decrypt with AES-128-ECB
            let bytes = resp.bytes().await.map_err(|e| Error::Other(format!("read error: {}", e)))?;
            let decrypted = crypto::eapi_decrypt(&bytes);
            let json: serde_json::Value =
                serde_json::from_slice(&decrypted).map_err(|e| Error::Other(format!("parse error: {}", e)))?;
            Ok(json)
        } else {
            let text = resp.text().await.map_err(|e| Error::Other(format!("read error: {}", e)))?;
            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| Error::Other(format!("parse error: {}", e)))?;
            Ok(json)
        }
    }

    /// Convenience: request and check code == 200.
    pub async fn request_ok(&self, crypto_type: CryptoType, url: &str, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let body = self.request(crypto_type, url, payload).await?;
        let code = body["code"].as_i64().unwrap_or(-1);
        if code != 200 {
            let msg = body["message"]
                .as_str()
                .or_else(|| body["msg"].as_str())
                .unwrap_or("unknown error");
            return Err(Error::Other(format!("API error {}: {}", code, msg)));
        }
        Ok(body)
    }

    fn extract_path(url: &str) -> &str {
        // Extract path from full URL, e.g. "https://interface.music.163.com/eapi/cloudsearch/pc"
        // -> "/eapi/cloudsearch/pc"
        if let Some(pos) = url.find("://") {
            let after_scheme = &url[pos + 3..];
            if let Some(path_pos) = after_scheme.find('/') {
                return &after_scheme[path_pos..];
            }
        }
        url
    }
}

fn val_to_form_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => v.to_string(),
    }
}
