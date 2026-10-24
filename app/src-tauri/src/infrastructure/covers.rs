use crate::model::AppResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use moka::future::Cache;
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

pub(crate) const MAX_COVER_BYTES: u64 = 8 * 1024 * 1024;

fn check_size(bytes: u64) -> AppResult<()> {
    if bytes > MAX_COVER_BYTES {
        return Err("封面文件过大".into());
    }
    Ok(())
}

struct CoverEntry {
    access: String,
    data: String,
}

// Shared by WebUI runtime_cover and Android MediaSession. Failed reads are not cached.
fn cache() -> &'static Cache<String, Arc<CoverEntry>> {
    static CACHE: OnceLock<Cache<String, Arc<CoverEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(32 * 1024 * 1024)
            .weigher(|key: &String, value: &Arc<CoverEntry>| {
                (key.len() + value.access.len() + value.data.len()) as u32
            })
            .time_to_live(Duration::from_secs(600))
            .build()
    })
}

pub(crate) async fn fetch(url: String, backend: &crate::Backend) -> AppResult<String> {
    let access = backend
        .music
        .adapters
        .cover_access(&url)
        .map_err(|e| e.to_string())?;
    if cache()
        .get(&url)
        .await
        .is_some_and(|entry| entry.access != access)
    {
        cache().invalidate(&url).await;
    }
    let result = cache()
        .try_get_with(url.clone(), async {
            load(&url, backend, &access).await.map(|data| {
                Arc::new(CoverEntry {
                    access: access.clone(),
                    data,
                })
            })
        })
        .await
        .map_err(|e| e.to_string())?;
    if result.access != access
        || backend
            .music
            .adapters
            .cover_access(&url)
            .map_err(|e| e.to_string())?
            != access
    {
        return Err("封面来源状态已变化".into());
    }
    Ok(result.data.clone())
}

async fn load(url: &str, backend: &crate::Backend, access: &str) -> AppResult<String> {
    let parsed = url::Url::parse(url)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "封面地址无效")?;
    let _permit = backend
        .cover_requests
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    if backend
        .music
        .adapters
        .cover_access(url)
        .map_err(|e| e.to_string())?
        != access
    {
        return Err("封面来源状态已变化".into());
    }
    let mut response = backend
        .cover_client
        .get(parsed)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| {
            h.to_str()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_result", error);
                })
                .ok()
        })
        .unwrap_or_default()
        .to_owned();
    let mime = image_mime(&content_type).ok_or("封面格式不支持")?;
    if let Some(length) = response.content_length() {
        check_size(length)?;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        check_size(bytes.len() as u64 + chunk.len() as u64)?;
        bytes.extend_from_slice(&chunk);
    }
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

pub(crate) async fn invalidate(url: &str) {
    cache().invalidate(url).await;
}

fn image_mime(value: &str) -> Option<&'static str> {
    match value
        .split(';')
        .next()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/jpeg" | "image/jpg" | "image/pjpeg" => Some("image/jpeg"),
        "image/png" | "image/x-png" => Some("image/png"),
        "image/webp" => Some("image/webp"),
        "image/gif" => Some("image/gif"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cover_size_accepts_eight_mib_and_rejects_larger_responses() {
        assert!(check_size(4 * 1024 * 1024 + 1).is_ok());
        assert!(check_size(8 * 1024 * 1024).is_ok());
        assert_eq!(check_size(8 * 1024 * 1024 + 1), Err("封面文件过大".into()));
        assert!(check_size(u64::MAX).is_err());
    }
    #[test]
    fn accepts_cdn_jpg_alias_and_emits_decodable_data_uri_type() {
        assert_eq!(image_mime("image/jpg"), Some("image/jpeg"));
        assert_eq!(image_mime("IMAGE/JPEG; charset=binary"), Some("image/jpeg"));
        assert_eq!(image_mime("image/png"), Some("image/png"));
        assert_eq!(image_mime("text/html"), None);
        assert_eq!(image_mime("application/octet-stream"), None);
    }
}
