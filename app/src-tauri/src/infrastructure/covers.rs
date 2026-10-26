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

struct CoverCache {
    entries: Cache<(u64, String), Arc<CoverEntry>>,
    generation: tokio::sync::RwLock<u64>,
}

// Shared by WebUI runtime_cover and Android MediaSession. Failed reads are not cached.
fn cache() -> &'static CoverCache {
    static CACHE: OnceLock<CoverCache> = OnceLock::new();
    CACHE.get_or_init(CoverCache::new)
}

impl CoverCache {
    fn new() -> Self {
        Self {
            entries: Cache::builder()
                .max_capacity(32 * 1024 * 1024)
                .weigher(|key: &(u64, String), value: &Arc<CoverEntry>| {
                    (8 + key.1.len() + value.access.len() + value.data.len()) as u32
                })
                .time_to_live(Duration::from_secs(600))
                .build(),
            generation: tokio::sync::RwLock::new(0),
        }
    }

    async fn get(
        &self,
        url: String,
        access: &str,
        load: impl std::future::Future<Output = anyhow::Result<String>>,
    ) -> AppResult<Arc<CoverEntry>> {
        let generation = *self.generation.read().await;
        let key = (generation, url);
        if self
            .entries
            .get(&key)
            .await
            .is_some_and(|entry| entry.access != access)
        {
            self.entries.invalidate(&key).await;
        }
        let result = self
            .entries
            .try_get_with(key.clone(), async {
                load.await.map(|data| {
                    Arc::new(CoverEntry {
                        access: access.to_owned(),
                        data,
                    })
                })
            })
            .await;
        let current = self.generation.read().await;
        if *current != generation {
            // A late insertion has an unreachable old-generation key and is removed immediately.
            self.entries.invalidate(&key).await;
            return Err("封面缓存已清理，请重试".into());
        }
        result.map_err(|error| format!("{error:#}"))
    }

    async fn clear(&self) {
        let mut generation = self.generation.write().await;
        *generation = generation.wrapping_add(1);
        self.entries.invalidate_all();
        self.entries.run_pending_tasks().await;
    }

    async fn status(&self) -> super::other_cache::Usage {
        let generation = self.generation.read().await;
        self.entries.run_pending_tasks().await;
        let mut usage = super::other_cache::Usage::default();
        for (key, value) in &self.entries {
            if key.0 == *generation {
                usage.entries += 1;
                usage.used_bytes +=
                    (8 + key.1.len() + value.access.len() + value.data.len()) as u64;
            }
        }
        usage
    }
}

pub(crate) async fn cache_status() -> super::other_cache::Usage {
    cache().status().await
}

pub(crate) async fn clear_cache() {
    cache().clear().await;
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
    let result = cache()
        .get(url.clone(), &access, async {
            load(&url, backend, &access)
                .await
                .with_context(|| format!("runtime_cover request={request}"))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::report_error("covers.load", error);
                })
        })
        .await?;
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
    let generation = cache().generation.read().await;
    cache()
        .entries
        .invalidate(&(*generation, url.to_owned()))
        .await;
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
    async fn clearing_covers_rejects_old_loads_without_poisoning_a_new_request() {
        let cache = Arc::new(CoverCache::new());
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, blocked) = tokio::sync::oneshot::channel();
        let old_cache = cache.clone();
        let old = tokio::spawn(async move {
            old_cache
                .get("cover".into(), "access", async {
                    entered.send(()).unwrap();
                    blocked.await.unwrap();
                    Ok("old image".into())
                })
                .await
        });
        started.await.unwrap();
        cache.clear().await;
        assert_eq!(cache.status().await.entries, 0);
        let fresh = cache
            .get("cover".into(), "access", async { Ok("new image".into()) })
            .await
            .unwrap();
        assert_eq!(fresh.data, "new image");
        release.send(()).unwrap();
        assert!(old.await.unwrap().is_err());
        assert_eq!(cache.status().await.entries, 1);
        let retained = cache
            .get("cover".into(), "access", async {
                anyhow::bail!("must be cached")
            })
            .await
            .unwrap();
        assert_eq!(retained.data, "new image");
        cache.clear().await;
        assert_eq!(cache.status().await.used_bytes, 0);
    }
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
