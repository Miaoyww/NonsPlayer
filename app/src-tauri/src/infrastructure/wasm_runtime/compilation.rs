//! Compiled code only: no Store, account, permission or guest state is persisted.
use crate::model::AppResult;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use wasmtime::{component::Component, Cache, CacheConfig, Config, Engine};

const SOFT_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
const THREADS: usize = 2;
static CACHE: OnceLock<Option<CodeCache>> = OnceLock::new();
static POOL: OnceLock<Result<ThreadPool, String>> = OnceLock::new();
// One compilation can use both workers; concurrent loads share this budget.
static COMPILE: Mutex<()> = Mutex::new(());

pub(super) struct CodeCache {
    cache: Cache,
    directory: PathBuf,
}
impl CodeCache {
    pub(super) fn new(directory: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let canonical = directory.canonicalize().map_err(|e| e.to_string())?;
        let mut config = CacheConfig::new();
        config
            .with_directory(&canonical)
            .with_files_total_size_soft_limit(SOFT_LIMIT_BYTES);
        Ok(Self {
            cache: Cache::new(config).map_err(|e| e.to_string())?,
            directory,
        })
    }
    fn status(&self) -> AppResult<crate::infrastructure::other_cache::Usage> {
        let files = crate::infrastructure::other_cache::resource_files(&self.directory, true)?;
        crate::infrastructure::other_cache::file_usage(&files)
    }
    fn clear(&self) -> AppResult<()> {
        for file in crate::infrastructure::other_cache::resource_files(&self.directory, true)? {
            crate::infrastructure::other_cache::checked_resource(&self.directory, &file)?;
            match std::fs::remove_file(file) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }
}

pub(crate) fn cache_status() -> AppResult<crate::infrastructure::other_cache::Usage> {
    let _guard = COMPILE.lock().map_err(|_| "WASM 编译锁不可用")?;
    CACHE
        .get()
        .and_then(Option::as_ref)
        .map_or_else(|| Ok(Default::default()), CodeCache::status)
}

pub(crate) fn clear_cache() -> AppResult<()> {
    let _guard = COMPILE.lock().map_err(|_| "WASM 编译锁不可用")?;
    CACHE
        .get()
        .and_then(Option::as_ref)
        .map_or(Ok(()), CodeCache::clear)
}

pub(crate) fn configure_cache(application_cache: &Path) {
    let cache = match CodeCache::new(application_cache.join("wasm-compiled-v1")) {
        Ok(cache) => Some(cache),
        Err(error) => {
            log::warn!(target: "nons::wasm", "stage=cache_init state=disabled reason={error}");
            None
        }
    };
    if CACHE.set(cache).is_err() {
        log::warn!(target: "nons::wasm", "stage=cache_init state=already_configured");
    }
}

pub(super) fn configure_engine(config: &mut Config) {
    config.parallel_compilation(true);
    if let Some(Some(cache)) = CACHE.get() {
        config.cache(Some(cache.cache.clone()));
    }
}

pub(crate) struct Compiled {
    pub component: Component,
    pub cache: &'static str,
    pub elapsed: Duration,
}

pub(crate) fn compile(engine: &Engine, bytes: &[u8]) -> AppResult<Compiled> {
    compile_with_cache(engine, bytes, CACHE.get().and_then(Option::as_ref))
}

fn compile_with_cache(
    engine: &Engine,
    bytes: &[u8],
    cache: Option<&CodeCache>,
) -> AppResult<Compiled> {
    let _guard = COMPILE
        .lock()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("wasm.compile.lock", error);
        })
        .map_err(|_| "WASM 编译锁不可用")?;
    let pool = POOL.get_or_init(|| {
        ThreadPoolBuilder::new()
            .num_threads(THREADS)
            .thread_name(|index| format!("nons-wasm-compile-{index}"))
            .build()
            .map_err(|e| e.to_string())
    });
    let pool = pool.as_ref().map_err(Clone::clone)?;
    let hits = cache.map(|c| c.cache.cache_hits()).unwrap_or(0);
    let misses = cache.map(|c| c.cache.cache_misses()).unwrap_or(0);
    let started = Instant::now();
    let result = pool
        .install(|| Component::new(engine, bytes))
        .map_err(|e| e.to_string());
    let elapsed = started.elapsed();
    let state = match cache {
        Some(cache) if cache.cache.cache_misses() > misses => "miss",
        Some(cache) if cache.cache.cache_hits() > hits => "hit",
        Some(_) => "unavailable",
        None => "disabled",
    };
    Ok(Compiled {
        component: result?,
        cache: state,
        elapsed,
    })
}

#[cfg(test)]
mod tests;
