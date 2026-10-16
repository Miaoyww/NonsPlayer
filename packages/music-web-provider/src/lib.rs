//! Shared ID-only music website protocol and bounded media validation for WASM guests.
#![cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
mod audio;
mod protocol;
#[cfg(test)]
mod tests;

use nons_music_adapter_sdk::provider::{ResourceProviderDescriptor, ResourceRequest};
use serde_json::{json, Value};

pub type AdapterResult<T> = std::result::Result<T, &'static str>;
/// Site identity and permissions remain owned by each adapter package.
#[derive(Clone, Copy)]
pub struct Config {
    pub base: &'static str,
    pub user_agent: &'static str,
    pub manifest: &'static str,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub url: String,
    pub method: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    pub fn check(&self) -> AdapterResult<()> {
        match self.status {
            200..=299 => Ok(()),
            401 | 403 => Err("permissionDenied"),
            404 => Err("notFound"),
            429 => Err("rateLimited"),
            _ => Err("network"),
        }
    }
}
pub trait Http {
    fn send(&self, request: Request) -> AdapterResult<Response>;
}

fn manifest(config: Config) -> Value {
    serde_json::from_str(config.manifest).expect("embedded manifest")
}
pub fn call(http: &impl Http, method: &str, args: &str, config: Config) -> String {
    let result = match method {
        "music.resource-descriptor" => Ok(manifest(config)["resourceProvider"].clone()),
        "music.resolve-resource" => resolve(http, args, config),
        _ => Err("unsupported"),
    };
    match result {
        Ok(data) => json!({"status":"ok", "data":data}).to_string(),
        Err(code) => {
            json!({"status":"error", "data":{"code":code,"retryAfterMs":null}}).to_string()
        }
    }
}
fn resolve(http: &impl Http, args: &str, config: Config) -> AdapterResult<Value> {
    if args.len() > 128 * 1024 {
        return Err("invalidData");
    }
    let input: Value = serde_json::from_str(args).map_err(|_| "invalidData")?;
    let request: ResourceRequest =
        serde_json::from_value(input["request"].clone()).map_err(|_| "invalidData")?;
    let descriptor: ResourceProviderDescriptor =
        serde_json::from_value(manifest(config)["resourceProvider"].clone())
            .map_err(|_| "internal")?;
    request.validate(&descriptor).map_err(|_| "invalidData")?;
    // Unknown duration cannot establish whether a returned resource is only a preview.
    if request.track.duration_ms == 0 {
        return Err("permissionDenied");
    }
    let requested = audio::requested_quality(&request.preferred_quality)?;
    let target = protocol::Target::new(&request.track)?;
    let mut client = protocol::Client::new(http, config);
    let url = client.locate(&target)?;
    let audio::InspectedAudio {
        url,
        quality,
        headers,
    } = audio::inspect(
        &mut client,
        url,
        request.track.duration_ms,
        requested,
        request.allow_downgrade,
    )?;
    Ok(
        json!({"metadata":{"actualQuality":quality,"extent":{"kind":"full"},"expiresAtMs":null},"url":url,"headers":headers}),
    )
}
