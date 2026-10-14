use crate::model::AppResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use moka::future::Cache;
use std::{sync::OnceLock, time::Duration};

// Shared by WebUI runtime_cover and Android MediaSession. Failed reads are not cached.
fn cache() -> &'static Cache<String, String> {
    static CACHE: OnceLock<Cache<String, String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(32 * 1024 * 1024)
            .weigher(|key: &String, value: &String| (key.len() + value.len()) as u32)
            .time_to_live(Duration::from_secs(600))
            .build()
    })
}

pub(crate) async fn fetch(url: String, backend: &crate::Backend) -> AppResult<String> {
    cache()
        .try_get_with(url.clone(), load(&url, backend))
        .await
        .map_err(|e| e.to_string())
}

async fn load(url: &str, backend: &crate::Backend) -> AppResult<String> {
    let parsed = url::Url::parse(url).map_err(|_| "封面地址无效")?;
    let host = parsed.host_str().unwrap_or_default();
    if !matches!(parsed.scheme(), "https" | "http")
        || !(host.ends_with(".music.126.net") || host.ends_with(".music.163.com"))
    {
        return Err("封面地址不受支持".into());
    }
    let _permit = backend
        .cover_requests
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
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
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let mime = image_mime(&content_type).ok_or("封面格式不支持")?;
    const LIMIT: usize = 4 * 1024 * 1024;
    if response.content_length().is_some_and(|n| n > LIMIT as u64) {
        return Err("封面文件过大".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("封面文件过大".into());
        }
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
    fn accepts_cdn_jpg_alias_and_emits_decodable_data_uri_type() {
        assert_eq!(image_mime("image/jpg"), Some("image/jpeg"));
        assert_eq!(image_mime("IMAGE/JPEG; charset=binary"), Some("image/jpeg"));
        assert_eq!(image_mime("image/png"), Some("image/png"));
        assert_eq!(image_mime("text/html"), None);
        assert_eq!(image_mime("application/octet-stream"), None);
    }
}
