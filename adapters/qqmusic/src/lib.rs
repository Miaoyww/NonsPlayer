//! QQ music protocol guest. Network, identity and lifetime are scoped by the Host.
pub mod auth;
pub mod catalog;
pub mod crypto;
pub mod device;
pub mod model;

#[cfg(target_arch = "wasm32")]
mod component;

use serde_json::{json, Value};

pub type Result<T> = std::result::Result<T, Error>;
pub const MAX_JSON: usize = 64 * 1024;

/// Only stable codes and static protocol stages cross the diagnostic boundary.
#[derive(Clone, Debug)]
pub struct Error {
    pub code: String,
    pub stage: String,
    pub class: String,
    pub retry_after_ms: Option<u64>,
}
impl Error {
    pub fn new(code: &str, stage: &str) -> Self {
        Self {
            code: code.into(),
            stage: stage.into(),
            class: "protocol".into(),
            retry_after_ms: None,
        }
    }
    pub fn retry_after(mut self, delay_ms: u64) -> Self {
        self.retry_after_ms = Some(delay_ms);
        self
    }
    pub fn with_class(mut self, class: String) -> Self {
        self.class = class;
        self
    }
    pub fn host(code: &str, stage: &str) -> Self {
        Self::new(
            match code {
                "cancelled" => "cancelled",
                "timeout" | "deadlineExceeded" => "deadlineExceeded",
                "staleContext" => "staleContext",
                "permissionDenied" => "permissionDenied",
                "unauthenticated" => "unauthenticated",
                "rateLimited" => "rateLimited",
                "invalidData" => "invalidData",
                "network" => "network",
                _ => "sourceUnavailable",
            },
            stage,
        )
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code, self.stage)
    }
}
impl std::error::Error for Error {}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
pub enum Event {
    Binary { socket: u32, data: Vec<u8> },
    Closed(u32),
}

/// Implementations must retain Host permission, cancellation and generation checks.
pub trait Host {
    fn http(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        timeout_ms: u32,
    ) -> Result<Response>;
    fn now_ms(&self) -> u64;
    fn random(&self, len: usize) -> Result<Vec<u8>>;
    fn state_get(&self, key: &str) -> Result<Option<String>>;
    fn state_set(&self, key: &str, value: &str) -> Result<()>;
    fn credential_read(&self) -> Result<Vec<u8>>;
    fn credential_publish(&self, metadata: &str, data: &[u8]) -> Result<()>;
    fn credential_replace(&self, data: &[u8]) -> Result<()>;
    fn credential_invalidate(&self) -> Result<()>;
    fn websocket_open(&self, url: &str, protocol: &str, timeout_ms: u32) -> Result<u32>;
    fn websocket_send(&self, socket: u32, data: &[u8]) -> Result<()>;
    fn websocket_close(&self, socket: u32) -> Result<()>;
    fn next_event(&self, timeout_ms: u32) -> Result<Option<Event>>;
    fn diagnostic(&self, stage: &str, class: &str, code: &str);
}

#[derive(Default)]
pub struct Adapter {
    auth: auth::Auth,
}
impl Adapter {
    pub fn shutdown(&mut self, host: &impl Host) {
        self.auth.shutdown(host);
    }
    pub fn call(&mut self, host: &impl Host, method: &str, args: &str) -> String {
        let result = self.dispatch(host, method, args).and_then(|value| {
            let encoded = json!({"status":"ok","data":value}).to_string();
            if encoded.len() > MAX_JSON {
                return Err(Error::new("invalidData", "transport.reply-size"));
            }
            Ok(encoded)
        });
        match result {
            Ok(encoded) => encoded,
            Err(error) => {
                host.diagnostic(&error.stage, &error.class, &error.code);
                json!({"status":"error","data":{"code":error.code,"retryAfterMs":error.retry_after_ms}}).to_string()
            }
        }
    }
    fn dispatch(&mut self, host: &impl Host, method: &str, args: &str) -> Result<Value> {
        if args.len() > MAX_JSON {
            return Err(Error::new("invalidData", "transport.args-size"));
        }
        if method == "music.descriptor" {
            let manifest: Value = serde_json::from_str(include_str!("../manifest.json"))
                .map_err(|_| Error::new("invalidData", "descriptor.json"))?;
            return Ok(manifest["descriptor"].clone());
        }
        let input: Value = serde_json::from_str(args)
            .map_err(|_| Error::new("invalidData", "transport.args-json"))?;
        if method == "runtime:event" {
            self.auth.event(host, &input)?;
            return Ok(Value::Null);
        }
        if input["session"]["source"] != "qqmusic" {
            return Err(Error::new("permissionDenied", "transport.source"));
        }
        let request = &input["request"];
        match method {
            "music.prepare-login" => {
                self.auth.prepare_login(host, request)?;
                Ok(Value::Null)
            }
            "music.read-track" => catalog::read_track(host, &mut self.auth, request),
            "music.business" => catalog::business(host, &mut self.auth, request),
            "music.account" => self.auth.account(host, request),
            "music.resolve-playback" => catalog::resolve_playback(host, &mut self.auth, request),
            _ => Err(Error::new("unsupported", "transport.method")),
        }
    }
}
