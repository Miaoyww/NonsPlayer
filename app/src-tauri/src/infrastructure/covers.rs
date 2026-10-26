use crate::model::AppResult;
use anyhow::Context;
use base64::{engine::general_purpose::STANDARD, Engine};
use moka::future::Cache;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock,
    },
    time::Duration,
};

pub(crate) const MAX_COVER_BYTES: u64 = 8 * 1024 * 1024;
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

fn check_size(bytes: u64) -> AppResult<()> {
    if bytes > MAX_COVER_BYTES {
        log::warn!(target: "nons::covers", "operation=covers.read stage=size_check code=coverTooLarge observed_bytes={bytes} limit_bytes={MAX_COVER_BYTES}");
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

#[tracing::instrument(target = "nons::covers", skip_all, fields(request = tracing::field::Empty))]
pub(crate) async fn fetch(url: String, backend: &crate::Backend) -> AppResult<String> {
    let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
    tracing::Span::current().record("request", request);
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
            load(&url, backend, &access)
                .await
                .with_context(|| format!("runtime_cover request={request}"))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::report_error("covers.load", error);
                })
                .map(|data| {
                    Arc::new(CoverEntry {
                        access: access.clone(),
                        data,
                    })
                })
        })
        .await
        .map_err(|e| format!("{e:#}"))?;
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

#[tracing::instrument(target = "nons::covers", skip_all)]
async fn load(url: &str, backend: &crate::Backend, access: &str) -> anyhow::Result<String> {
    let parsed = url::Url::parse(url).context("covers.url.parse")?;
    let _permit = backend
        .cover_requests
        .acquire()
        .await
        .context("covers.acquire")?;
    if backend
        .music
        .adapters
        .cover_access(url)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        != access
    {
        anyhow::bail!("封面来源状态已变化");
    }
    download(parsed, &backend.cover_client).await
}

#[tracing::instrument(target = "nons::covers", skip_all)]
async fn download(parsed: url::Url, client: &reqwest::Client) -> anyhow::Result<String> {
    let mut response = client
        .get(parsed)
        .send()
        .await
        .context("covers.http.send")?
        .error_for_status()
        .context("covers.http.status")?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .context("covers.response.content_type missing")?
        .to_str()
        .context("covers.response.content_type invalid")?
        .to_owned();
    let mime = image_mime(&content_type).context("封面格式不支持")?;
    if let Some(length) = response.content_length() {
        check_size(length).map_err(anyhow::Error::msg)?;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.context("covers.http.read_chunk")? {
        check_size(bytes.len() as u64 + chunk.len() as u64).map_err(anyhow::Error::msg)?;
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
    #[tokio::test]
    async fn failed_cover_transport_preserves_timeout_cause_and_stack() {
        crate::infrastructure::diagnostics::test_log::start();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(30))
            .build()
            .unwrap();
        let error = download(
            url::Url::parse(&format!("http://{address}/cover.png")).unwrap(),
            &client,
        )
        .await
        .unwrap_err();
        assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_timeout());
        crate::infrastructure::diagnostics::report_error("covers.load", &error);
        let output = crate::infrastructure::diagnostics::test_log::take().join("\n");
        assert!(output.contains("covers.http.send"), "{output}");
        assert!(output.contains("operation timed out"), "{output}");
        assert!(output.contains("backtrace_kind="), "{output}");
        assert!(output.contains("covers.rs:"), "{output}");
        assert!(!output.contains(&address.to_string()), "{output}");
    }
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
