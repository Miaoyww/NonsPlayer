use super::cover_disk::{Asset, DiskCache};
use crate::model::AppResult;
use anyhow::Context;
use moka::future::Cache;
use std::{
    future::Future,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::Emitter;

const DISK_BUDGET: u64 = 256 * 1024 * 1024;
const FRESH_SECONDS: u64 = 24 * 60 * 60;
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

pub(crate) struct CoverCache {
    root: PathBuf,
    disk: tokio::sync::OnceCell<Arc<DiskCache>>,
    loads: Cache<(u64, String), Arc<Asset>>,
    refreshes: Cache<(u64, String), ()>,
}

struct Resolved {
    disk: Arc<DiskCache>,
    generation: u64,
    asset: Asset,
}

#[derive(Debug)]
struct Downloaded {
    bytes: Vec<u8>,
    mime: String,
}

async fn blocking<T: Send + 'static>(
    disk: Arc<DiskCache>,
    operation: impl FnOnce(&DiskCache) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    static IO: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);
    let permit = IO.acquire().await.map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        operation(&disk)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn fresh_at(asset: &Asset, now: u64) -> bool {
    asset.fetched_at <= now && now - asset.fetched_at < FRESH_SECONDS
}

fn now() -> u64 {
    crate::storage::now_seconds().max(0) as u64
}

impl CoverCache {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            disk: tokio::sync::OnceCell::new(),
            loads: Cache::builder()
                .max_capacity(512)
                .time_to_live(Duration::from_secs(600))
                .build(),
            refreshes: Cache::builder()
                .max_capacity(512)
                .time_to_live(Duration::from_secs(30))
                .build(),
        }
    }

    async fn disk(&self) -> AppResult<Arc<DiskCache>> {
        self.disk
            .get_or_try_init(|| async {
                let root = self.root.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    DiskCache::open(root, DISK_BUDGET).map(Arc::new)
                })
                .await
                .map_err(|e| e.to_string())?
            })
            .await
            .cloned()
    }

    async fn load<F, Fut>(
        self: &Arc<Self>,
        url: String,
        generation: u64,
        loader: F,
        timestamp: u64,
    ) -> AppResult<Arc<Asset>>
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = AppResult<Downloaded>> + Send + 'static,
    {
        let disk = self.disk().await?;
        let key = (generation, url.clone());
        let result = self
            .loads
            .try_get_with(key.clone(), async move {
                let lookup_url = url.clone();
                let lookup = blocking(disk.clone(), move |cache| cache.lookup(&lookup_url)).await?;
                if lookup.generation != generation {
                    return Err("封面缓存已失效，请重试".to_string());
                }
                if let Some(asset) = lookup.asset.filter(|asset| fresh_at(asset, timestamp)) {
                    return Ok(Arc::new(asset));
                }
                let downloaded = loader().await?;
                let asset = blocking(disk, move |cache| {
                    cache.publish(&url, &downloaded.bytes, &downloaded.mime, generation)
                })
                .await?;
                Ok::<_, String>(Arc::new(asset))
            })
            .await
            .map_err(|e| e.to_string());
        // Retain only in-flight coalescing; the persistent index is the source of truth.
        self.loads.invalidate(&key).await;
        result
    }

    async fn resolve_with<F, Fut, R>(
        self: &Arc<Self>,
        url: String,
        loader: F,
        refreshed: R,
        timestamp: u64,
    ) -> AppResult<Resolved>
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = AppResult<Downloaded>> + Send + 'static,
        R: Fn() + Send + Sync + 'static,
    {
        let disk = self.disk().await?;
        let lookup_url = url.clone();
        let lookup = blocking(disk.clone(), move |cache| cache.lookup(&lookup_url)).await?;
        let generation = lookup.generation;
        let asset = if let Some(asset) = lookup.asset {
            if !fresh_at(&asset, timestamp) {
                let cache = self.clone();
                let refresh_url = url.clone();
                self.refreshes
                    .get_with((generation, url), async move {
                        tauri::async_runtime::spawn(async move {
                            match cache.load(refresh_url, generation, loader, timestamp).await {
                                Ok(_) => refreshed(),
                                Err(error) => crate::infrastructure::diagnostics::warn(
                                    "covers.refresh",
                                    error,
                                ),
                            }
                        });
                    })
                    .await;
            }
            asset
        } else {
            (*self.load(url, generation, loader, timestamp).await?).clone()
        };
        if blocking(disk.clone(), |cache| cache.generation()).await? != generation {
            return Err("封面缓存已失效，请重试".into());
        }
        Ok(Resolved {
            disk,
            generation,
            asset,
        })
    }

    pub(crate) async fn clear(&self) -> AppResult<()> {
        blocking(self.disk().await?, |cache| cache.clear()).await?;
        self.loads.invalidate_all();
        self.refreshes.invalidate_all();
        Ok(())
    }

    pub(crate) async fn invalidate(&self, url: String) -> AppResult<()> {
        blocking(self.disk().await?, move |cache| cache.invalidate(&url)).await?;
        self.loads.invalidate_all();
        self.refreshes.invalidate_all();
        Ok(())
    }

    pub(crate) async fn status(&self) -> AppResult<super::other_cache::Usage> {
        blocking(self.disk().await?, |cache| cache.status()).await
    }
}

fn access(backend: &crate::Backend, url: &str) -> AppResult<String> {
    backend
        .music
        .adapters
        .cover_access(url)
        .map_err(|e| e.to_string())
}

async fn resolve(
    url: String,
    backend: &crate::Backend,
    app: &tauri::AppHandle,
) -> AppResult<Resolved> {
    let expected_access = access(backend, &url)?;
    let adapters = backend.music.adapters.clone();
    let client = backend.cover_client.clone();
    let requests = backend.cover_requests.clone();
    let request_url = url.clone();
    let load_access = expected_access.clone();
    let refresh_app = app.clone();
    let refresh_url = url.clone();
    let value = backend
        .online_covers
        .resolve_with(
            url.clone(),
            move || {
                let adapters = adapters.clone();
                let client = client.clone();
                let requests = requests.clone();
                let url = request_url.clone();
                let expected = load_access.clone();
                async move {
                    let _permit = requests.acquire().await.map_err(|e| e.to_string())?;
                    if adapters.cover_access(&url).map_err(|e| e.to_string())? != expected {
                        return Err("封面来源状态已变化".into());
                    }
                    let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
                    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
                    let result = download(parsed, &client)
                        .await
                        .with_context(|| format!("runtime_cover request={request}"))
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::report_error("covers.load", error)
                        })
                        .map_err(|error| format!("{error:#}"))?;
                    if adapters.cover_access(&url).map_err(|e| e.to_string())? != expected {
                        return Err("封面来源状态已变化".into());
                    }
                    Ok(result)
                }
            },
            move || {
                if let Err(error) = refresh_app.emit(
                    "runtime-cover-updated",
                    serde_json::json!({"url": refresh_url}),
                ) {
                    crate::infrastructure::diagnostics::warn("covers.updated", error);
                }
            },
            now(),
        )
        .await?;
    if access(backend, &url)? != expected_access {
        return Err("封面来源状态已变化".into());
    }
    Ok(value)
}

#[tracing::instrument(target = "nons::covers", skip_all)]
pub(crate) async fn fetch(
    url: String,
    backend: &crate::Backend,
    app: &tauri::AppHandle,
) -> AppResult<String> {
    Ok(resolve(url, backend, app)
        .await?
        .asset
        .path
        .to_string_lossy()
        .into_owned())
}

#[cfg(desktop)]
pub(crate) async fn read(
    url: String,
    backend: &crate::Backend,
    app: &tauri::AppHandle,
) -> AppResult<Vec<u8>> {
    let expected = access(backend, &url)?;
    for attempt in 0..2 {
        let resolved = resolve(url.clone(), backend, app).await?;
        let generation = resolved.generation;
        let asset = resolved.asset;
        let result = blocking(resolved.disk.clone(), move |cache| cache.read(&asset)).await;
        if access(backend, &url)? != expected
            || blocking(resolved.disk, |cache| cache.generation()).await? != generation
        {
            return Err("封面来源或缓存状态已变化".into());
        }
        match result {
            Ok(bytes) => return Ok(bytes),
            Err(_) if attempt == 0 => backend.online_covers.invalidate(url.clone()).await?,
            Err(error) => return Err(error),
        }
    }
    unreachable!()
}

#[tracing::instrument(target = "nons::covers", skip_all)]
async fn download(parsed: url::Url, client: &reqwest::Client) -> anyhow::Result<Downloaded> {
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
        .context("covers.response.content_type invalid")?;
    let mime = image_mime(content_type)
        .context("封面格式不支持")?
        .to_owned();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.context("covers.http.read_chunk")? {
        bytes.extend_from_slice(&chunk);
    }
    Ok(Downloaded { bytes, mime })
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
#[path = "covers_tests.rs"]
mod tests;
