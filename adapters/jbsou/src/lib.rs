//! ID-only audio provider. Protocol, anonymous cookies and media checks stay in the guest.
#![cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
mod audio;
#[cfg(target_arch = "wasm32")]
mod component;
mod protocol;
#[cfg(test)]
mod tests;

use nons_music_adapter_sdk::provider::{ResourceProviderDescriptor, ResourceRequest};
use serde_json::{json, Value};

pub(crate) type AdapterResult<T> = std::result::Result<T, &'static str>;
pub(crate) const BASE: &str = "https://www.jbsou.cn/";
pub(crate) const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36";

#[derive(Clone, Debug)]
pub(crate) struct Request {
    pub url: String,
    pub method: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
pub(crate) struct Response {
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
pub(crate) trait Http {
    fn send(&self, request: Request) -> AdapterResult<Response>;
}

pub(crate) fn manifest() -> Value {
    serde_json::from_str(include_str!("../manifest.json")).expect("embedded manifest")
}
pub(crate) fn call(http: &impl Http, method: &str, args: &str) -> String {
    let result = match method {
        "music.resource-descriptor" => Ok(manifest()["resourceProvider"].clone()),
        "music.resolve-resource" => resolve(http, args),
        _ => Err("unsupported"),
    };
    match result {
        Ok(data) => json!({"status":"ok", "data":data}).to_string(),
        Err(code) => {
            json!({"status":"error", "data":{"code":code,"retryAfterMs":null}}).to_string()
        }
    }
}
fn resolve(http: &impl Http, args: &str) -> AdapterResult<Value> {
    if args.len() > 128 * 1024 {
        return Err("invalidData");
    }
    let input: Value = serde_json::from_str(args).map_err(|_| "invalidData")?;
    let request: ResourceRequest =
        serde_json::from_value(input["request"].clone()).map_err(|_| "invalidData")?;
    let descriptor: ResourceProviderDescriptor =
        serde_json::from_value(manifest()["resourceProvider"].clone()).map_err(|_| "internal")?;
    request.validate(&descriptor).map_err(|_| "invalidData")?;
    // Unknown duration cannot establish whether a returned resource is only a preview.
    if request.track.duration_ms == 0 {
        return Err("permissionDenied");
    }
    let requested = audio::requested_quality(&request.preferred_quality)?;
    let target = protocol::Target::new(&request.track)?;
    let mut client = protocol::Client::new(http);
    let url = client.locate(&target)?;
    let (url, quality, headers) = audio::inspect(
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
