mod assets;
mod audio_files;
mod capability;
#[cfg(desktop)]
mod clipboard;
#[cfg(target_os = "android")]
#[path = "android_clipboard.rs"]
mod clipboard;
mod configuration;
mod database;
mod diagnostics;
mod files;
mod http;
mod install;
pub mod manifest;
mod music;
#[cfg(feature = "plugin-probe")]
pub mod probe;
mod processes;
mod runtime;
mod secrets;
pub(crate) mod settings;
mod transfers;
mod upgrade;
mod windows;

use crate::{model::AppResult, player::Player};
use capability::Context;
use database::Database;
use manifest::Manifest;
use music::SongService;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub manifest: Manifest,
    pub enabled: bool,
    pub loaded: bool,
    pub generation: u64,
    pub error: Option<String>,
}
struct Record {
    descriptor: Descriptor,
    context: Context,
    runtime: Option<Arc<tokio::sync::Mutex<runtime::Runtime>>>,
    ready: bool,
}
#[derive(Clone, PartialEq, Eq)]
struct ClipboardChange {
    text: String,
    targets: Vec<(String, u64)>,
}
pub struct PluginManager {
    root: PathBuf,
    database: Arc<Database>,
    configurations: Arc<configuration::Configurations>,
    files: Arc<files::Files>,
    songs: Arc<SongService>,
    http: Arc<http::Http>,
    transfers: Arc<transfers::Transfers>,
    processes: Arc<processes::Processes>,
    windows: Arc<windows::Windows>,
    player: Arc<Player>,
    app: tauri::AppHandle,
    records: Mutex<BTreeMap<String, Record>>,
    operations: tokio::sync::Mutex<()>,
    engine: Mutex<Option<wasmtime::Engine>>,
    generation: AtomicU64,
    stopped: AtomicBool,
    watcher: Mutex<Option<clipboard::Shutdown>>,
    clipboard_sender: tokio::sync::watch::Sender<Option<ClipboardChange>>,
    lifecycle: tokio::sync::watch::Sender<()>,
}
impl PluginManager {
    pub(crate) fn shared_http(&self) -> Arc<crate::infrastructure::http::Http> {
        self.http.clone()
    }
    pub fn new(
        app: tauri::AppHandle,
        data: &Path,
        music: Arc<crate::music::service::MusicService>,
        player: Arc<Player>,
    ) -> AppResult<Arc<Self>> {
        let root = data.join("plugins");
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let (clipboard_sender, mut receiver) = tokio::sync::watch::channel(None);
        let database = Arc::new(Database::open(&data.join("plugins.sqlite3"))?);
        let configurations = Arc::new(configuration::Configurations::new(database.clone()));
        let files = Arc::new(files::Files::new(
            data.join("plugin-data"),
            database.clone(),
        ));
        let manager = Arc::new(Self {
            root,
            database,
            configurations,
            files,
            songs: Arc::new(SongService::new(music)),
            http: Arc::new(http::Http::new()?),
            transfers: Arc::new(transfers::Transfers::default()),
            processes: Arc::new(processes::Processes::default()),
            windows: Arc::new(windows::Windows::default()),
            player,
            app,
            records: Default::default(),
            operations: Default::default(),
            engine: Default::default(),
            generation: AtomicU64::new(1),
            stopped: AtomicBool::new(false),
            watcher: Default::default(),
            clipboard_sender,
            lifecycle: tokio::sync::watch::channel(()).0,
        });
        // Bundled artifacts are built outside the host and copied only once.
        if !manager.database.initialized()? {
            let bundled = manager
                .app
                .path()
                .resource_dir()
                .map_err(|e| e.to_string())?
                .join("bundled-plugins");
            if bundled.is_dir() {
                for entry in std::fs::read_dir(&bundled).map_err(|e| e.to_string())? {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(error) => {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.bundled_entry",
                                error,
                            );
                            continue;
                        }
                    };
                    let manifest = match Manifest::read(&entry.path()) {
                        Ok(manifest) => manifest,
                        Err(error) => {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.bundled_manifest",
                                error,
                            );
                            continue;
                        }
                    };
                    let destination = manager.root.join(&manifest.id);
                    if destination.exists() {
                        continue;
                    }
                    let staging = manager.root.join(format!(".bundled-{}", manifest.id));
                    if staging.exists() {
                        let _ = (std::fs::remove_dir_all(&staging)).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "std::fs::remove_dir_all",
                                error,
                            );
                        });
                    }
                    if install::install(&entry.path(), &staging)
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.bundled_install",
                                error,
                            )
                        })
                        .is_ok()
                        && std::fs::rename(&staging, destination)
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn(
                                    "plugins.bundled_publish",
                                    error,
                                )
                            })
                            .is_err()
                    {
                        let _ = (std::fs::remove_dir_all(&staging)).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "std::fs::remove_dir_all",
                                error,
                            );
                        });
                    }
                }
                manager.database.mark_initialized()?;
            }
        }
        manager.discover()?;
        let weak = Arc::downgrade(&manager);
        let mut changes = manager.configurations.changes.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                let ids = match changes.recv().await {
                    Ok(id) => vec![id],
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let Some(manager) = weak.upgrade() else {
                            break;
                        };
                        manager
                            .list()
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn(
                                    "plugins.configuration_reload",
                                    error,
                                )
                            })
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|p| p.manifest.configuration.is_some())
                            .map(|p| p.manifest.id)
                            .collect()
                    }
                    Err(_) => break,
                };
                let Some(manager) = weak.upgrade() else {
                    break;
                };
                if manager.stopped.load(Ordering::SeqCst) {
                    break;
                }
                for id in ids {
                    let _ = (manager
                        .app
                        .emit("plugin-config-changed", serde_json::json!({"pluginId":id})))
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "plugins.clipboard.emit_changed",
                            error,
                        );
                    });
                    let target = manager
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.config_changed.lookup_runtime",
                                error,
                            );
                        })
                        .ok()
                        .and_then(|r| {
                            r.get(&id)
                                .filter(|r| r.descriptor.loaded && r.runtime.is_some())
                                .map(|r| r.descriptor.generation)
                        });
                    if let Some(generation) = target {
                        if let Ok(snapshot) =
                            (manager.configurations.snapshot(&id)).inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn(
                                    "plugins.config_changed.read_snapshot",
                                    error,
                                );
                            })
                        {
                            if let Ok(args) = (serde_json::to_string(
                                &serde_json::json!({"revision":snapshot.revision,"pendingReload":snapshot.pending_reload}),
                            )).inspect_err(|error| { crate::infrastructure::diagnostics::warn("plugins.config_changed.serialize_event", error); }) {
                                let _ = (manager
                                    .invoke(&id, generation, "event:config-changed", &args)
                                    .await).inspect_err(|error| { crate::infrastructure::diagnostics::warn("plugins.config_changed.invoke", error); });
                            }
                        }
                    }
                }
            }
        });
        let weak = Arc::downgrade(&manager);
        tauri::async_runtime::spawn(async move {
            while receiver.changed().await.is_ok() {
                let change = receiver.borrow_and_update().clone();
                let Some(manager) = weak.upgrade() else {
                    break;
                };
                if manager.stopped.load(Ordering::SeqCst) {
                    break;
                }
                if let Some(change) = change {
                    // Release consumed plaintext without discarding a newer change.
                    manager.clipboard_sender.send_if_modified(|pending| {
                        if pending.as_ref() == Some(&change) {
                            *pending = None;
                        }
                        false
                    });
                    manager.deliver_clipboard(&change).await;
                }
            }
        });
        Ok(manager)
    }
    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, BTreeMap<String, Record>>> {
        self.records
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.registry.lock", error);
            })
            .map_err(|_| "插件注册表不可用".into())
    }
    fn context(&self, manifest: &Manifest, generation: u64) -> Context {
        Context {
            id: manifest.id.clone(),
            generation,
            permissions: manifest.permissions.iter().cloned().collect(),
            http_hosts: manifest.http_hosts.clone(),
            file_roots: manifest.file_roots.clone(),
            http: self.http.clone(),
            transfers: self.transfers.clone(),
            processes: self.processes.clone(),
            windows: self.windows.clone(),
            window_exports: manifest
                .contributes
                .windows
                .iter()
                .map(|w| (w.id.clone(), w.export.clone()))
                .collect(),
            active: Arc::new(AtomicBool::new(false)),
            database: self.database.clone(),
            configurations: self.configurations.clone(),
            files: self.files.clone(),
            songs: self.songs.clone(),
            player: self.player.clone(),
            app: self.app.clone(),
            event_budget: Arc::new(Mutex::new((Instant::now(), 0))),
            diagnostic_budget: Arc::new(Mutex::new((Instant::now(), 0))),
            requests: Arc::new(tokio::sync::Semaphore::new(16)),
        }
    }
    pub fn discover(&self) -> AppResult<()> {
        let mut records = self.lock()?;
        for entry in std::fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let id = entry.file_name().to_string_lossy().to_string();
            if !manifest::valid_id(&id) || records.contains_key(&id) {
                continue;
            }
            let meta = std::fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if manifest::is_link(&meta) || !meta.is_dir() {
                continue;
            }
            let (manifest, configuration) = match Manifest::read_configured(&entry.path()) {
                Ok((manifest, configuration)) if manifest.id == id => (manifest, configuration),
                Ok(_) => {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.discover",
                        "manifest ID does not match directory",
                    );
                    continue;
                }
                Err(error) => {
                    crate::infrastructure::diagnostics::warn("plugins.discover", error);
                    continue;
                }
            };
            let generation = self.generation.fetch_add(1, Ordering::SeqCst);
            self.configurations.register(&id, configuration)?;
            let context = self.context(&manifest, generation);
            let descriptor = Descriptor {
                enabled: self.database.enabled(&id, &manifest.authorization())?,
                loaded: false,
                generation,
                error: None,
                manifest,
            };
            records.insert(
                id,
                Record {
                    descriptor,
                    context,
                    runtime: None,
                    ready: false,
                },
            );
        }
        Ok(())
    }
    pub fn list(&self) -> AppResult<Vec<Descriptor>> {
        Ok(self
            .lock()?
            .values()
            .map(|r| r.descriptor.clone())
            .collect())
    }
    fn notify(&self) {
        let _ = (self.app.emit("plugins-changed", ())).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("self.app.emit", error);
        });
    }
    pub async fn startup(self: &Arc<Self>) {
        if let Ok(plugins) = (self.list()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.startup.list", error);
        }) {
            for plugin in plugins {
                if plugin.enabled {
                    let _ = (self.action(&plugin.manifest.id, "load", false).await).inspect_err(
                        |error| {
                            crate::infrastructure::diagnostics::warn("self.action", error);
                        },
                    );
                }
            }
        }
    }
    #[cfg(feature = "plugin-probe")]
    pub async fn install(self: &Arc<Self>, source: PathBuf) -> AppResult<()> {
        self.install_reviewed(source, false).await
    }
    fn get_engine(&self) -> AppResult<wasmtime::Engine> {
        let mut engine = self
            .engine
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.engine.lock", error);
            })
            .map_err(|_| "WASM Engine 不可用")?;
        if engine.is_none() {
            let value = runtime::engine()?;
            let weak = value.weak();
            std::thread::Builder::new()
                .name("nons-plugin-epoch".into())
                .spawn(move || {
                    while let Some(engine) = weak.upgrade() {
                        engine.increment_epoch();
                        drop(engine);
                        std::thread::sleep(Duration::from_millis(10));
                    }
                })
                .map_err(|e| e.to_string())?;
            *engine = Some(value);
        }
        engine.clone().ok_or_else(|| "WASM Engine 不可用".into())
    }
    pub async fn action(
        self: &Arc<Self>,
        id: &str,
        action: &str,
        confirmed: bool,
    ) -> AppResult<()> {
        let _operation = self.operations.lock().await;
        if !manifest::valid_id(id) {
            return Err("插件 ID 无效".into());
        }
        if !self.lock()?.contains_key(id) {
            return Err("插件未安装".into());
        }
        match action {
            "enable" => {
                if !confirmed {
                    return Err("请先确认插件权限和前端信任说明".into());
                }
                let scope = self
                    .lock()?
                    .get(id)
                    .ok_or("插件未安装")?
                    .descriptor
                    .manifest
                    .authorization();
                self.database.authorize(id, &scope)?;
                self.lock()?
                    .get_mut(id)
                    .ok_or("插件未安装")?
                    .descriptor
                    .enabled = true;
                self.load(id).await?;
            }
            "load" => {
                if !self.lock()?.get(id).ok_or("插件未安装")?.descriptor.enabled {
                    return Err("插件未启用".into());
                }
                self.load(id).await?;
            }
            "disable" => {
                self.database.enable(id, false)?;
                self.lock()?
                    .get_mut(id)
                    .ok_or("插件未安装")?
                    .descriptor
                    .enabled = false;
                self.unload(id).await?;
            }
            "unload" => {
                self.unload(id).await?;
            }
            "reload" => {
                self.unload(id).await?;
                if self.lock()?.get(id).ok_or("插件未安装")?.descriptor.enabled {
                    self.load(id).await?;
                }
            }
            "uninstall" | "uninstall-keep-data" => {
                self.database.enable(id, false)?;
                self.lock()?
                    .get_mut(id)
                    .ok_or("插件未安装")?
                    .descriptor
                    .enabled = false;
                self.unload(id).await?;
                let files = self.files.clone();
                let plugin_id = id.to_string();
                let program = self.root.join(id);
                let database = self.database.clone();
                let keep = action == "uninstall-keep-data";
                tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
                    files.revoke(&plugin_id, None)?;
                    secrets::clear(&plugin_id, &database)?;
                    std::fs::remove_dir_all(program).map_err(|e| e.to_string())?;
                    if !keep {
                        files.clear_data(&plugin_id)?;
                        database.remove(&plugin_id)?;
                    }
                    Ok(())
                })
                .await
                .map_err(|e| e.to_string())??;
                self.configurations.register(id, None)?;
                self.lock()?.remove(id);
            }
            _ => return Err("未知插件操作".into()),
        }
        self.sync_watcher()?;
        self.notify();
        Ok(())
    }
    async fn load(self: &Arc<Self>, id: &str) -> AppResult<()> {
        if self.lock()?.get(id).ok_or("插件未安装")?.descriptor.loaded {
            return Ok(());
        }
        let (manifest, configuration) = Manifest::read_configured(&self.root.join(id))?;
        if manifest.backend.is_some()
            && self
                .lock()?
                .values()
                .filter(|r| r.descriptor.loaded && r.runtime.is_some())
                .count()
                >= 4
        {
            return Err("同时加载的 WASM 插件最多 4 个".into());
        }
        let old = self
            .lock()?
            .get(id)
            .ok_or("插件未安装")?
            .descriptor
            .manifest
            .clone();
        if manifest.id != id
            || manifest.permissions != old.permissions
            || manifest.http_hosts != old.http_hosts
            || manifest.file_roots != old.file_roots
            || manifest.version != old.version
        {
            return Err("插件权限或版本已改变，请重新安装并授权".into());
        }
        if !self.database.enabled(id, &manifest.authorization())? {
            return Err("插件权限范围已改变，请重新确认授权".into());
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst);
        self.configurations.register(id, configuration)?;
        self.configurations.loaded(id)?;
        let context = self.context(&manifest, generation);
        context.active.store(true, Ordering::SeqCst);
        let runtime = if let Some(entry) = &manifest.backend {
            match runtime::Runtime::load(
                self.get_engine()?,
                &manifest::checked_file(&self.root.join(id), entry)?,
                context.clone(),
            )
            .await
            {
                Ok(runtime) => Some(Arc::new(tokio::sync::Mutex::new(runtime))),
                Err(error) => {
                    context.active.store(false, Ordering::SeqCst);
                    self.configurations.unloaded(id);
                    self.files.close_instance(id, generation);
                    self.processes.revoke(id, generation);
                    self.windows.revoke(id, generation);
                    self.lock()?
                        .get_mut(id)
                        .ok_or("插件未安装")?
                        .descriptor
                        .error = Some(error.clone());
                    self.notify();
                    return Err(error);
                }
            }
        } else {
            None
        };
        let mut records = self.lock()?;
        let record = records.get_mut(id).ok_or("插件未安装")?;
        record.context = context;
        record.runtime = runtime;
        record.ready = manifest.frontend.is_none();
        record.descriptor.manifest = manifest;
        record.descriptor.loaded = true;
        record.descriptor.generation = generation;
        record.descriptor.error = None;
        Ok(())
    }
    async fn unload(&self, id: &str) -> AppResult<()> {
        let runtime = {
            let mut records = self.lock()?;
            let record = records.get_mut(id).ok_or("插件未安装")?;
            record.context.active.store(false, Ordering::SeqCst);
            self.files.close_instance(id, record.descriptor.generation);
            self.processes.revoke(id, record.descriptor.generation);
            self.windows.revoke(id, record.descriptor.generation);
            self.configurations.unloaded(id);
            record.descriptor.loaded = false;
            record.ready = false;
            record.runtime.take()
        };
        self.notify();
        if let Some(runtime) = runtime {
            let _ = (tokio::time::timeout(Duration::from_secs(1), async {
                runtime.lock().await.shutdown().await
            })
            .await)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("shutdown_timeout", error);
                });
        }
        Ok(())
    }
    pub fn scoped(&self, id: &str, generation: u64) -> AppResult<Context> {
        let records = self.lock()?;
        let record = records.get(id).ok_or("插件未安装")?;
        if !record.descriptor.loaded || record.descriptor.generation != generation {
            return Err("插件加载代次已失效".into());
        }
        record.context.check(None)?;
        Ok(record.context.clone())
    }
    pub async fn call(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        method: &str,
        args: &str,
    ) -> AppResult<String> {
        if method.starts_with("event:") {
            return Err("SDK 不允许发送宿主事件".into());
        }
        self.invoke(id, generation, method, args).await
    }
    async fn invoke(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        method: &str,
        args: &str,
    ) -> AppResult<String> {
        let context = self.scoped(id, generation)?;
        let started = std::time::Instant::now();
        let _permit = context
            .requests
            .clone()
            .try_acquire_owned()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.invoke.admission", error);
            })
            .map_err(|_| "插件调用过于频繁")?;
        let runtime = self
            .lock()?
            .get(id)
            .and_then(|r| r.runtime.clone())
            .ok_or("插件没有 WASM Backend")?;
        let result = tokio::time::timeout(Duration::from_secs(6), async {
            let mut runtime = runtime.lock().await;
            context.check(None)?;
            let result = runtime.call(method, args).await;
            if runtime.faulted {
                if let Err(error) = &result {
                    self.fault(id, generation, error)?;
                }
            }
            result
        })
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.invoke.queue_timeout", error);
        })
        .map_err(|_| "插件调用队列超时".to_string())
        .and_then(|r| r);
        match result {
            Ok(value) => {
                context.check(None)?;
                Ok(value)
            }
            Err(error) => {
                log::warn!(target: "nons::plugins", "operation=plugins.invoke stage=backend_call plugin={} generation={} method={} elapsed_ms={} reason={}", id, generation, crate::infrastructure::diagnostics::redact(method), started.elapsed().as_millis(), error);
                Err(error)
            }
        }
    }
    pub fn fault(self: &Arc<Self>, id: &str, generation: u64, error: &str) -> AppResult<()> {
        if let Some(record) = self
            .lock()?
            .get_mut(id)
            .filter(|r| r.descriptor.loaded && r.descriptor.generation == generation)
        {
            record.context.active.store(false, Ordering::SeqCst);
            self.files.close_instance(id, generation);
            self.processes.revoke(id, generation);
            self.windows.revoke(id, generation);
            self.configurations.unloaded(id);
            record.descriptor.loaded = false;
            record.descriptor.error = Some(error.chars().take(512).collect());
            record.runtime = None;
            record.ready = false;
        }
        self.lifecycle.send_replace(());
        self.sync_watcher()?;
        self.notify();
        Ok(())
    }
    pub fn ready(self: &Arc<Self>, id: &str, generation: u64) -> AppResult<()> {
        self.scoped(id, generation)?.check(Some("ui"))?;
        {
            let mut records = self.lock()?;
            let record = records.get_mut(id).ok_or("插件未安装")?;
            if !record.descriptor.loaded || record.descriptor.generation != generation {
                return Err("插件加载代次已失效".into());
            }
            record.ready = true;
        }
        let result = self.sync_watcher();
        if let Err(error) = &result {
            self.fault(id, generation, error)?;
        }
        self.lifecycle.send_replace(());
        result
    }
    #[cfg(feature = "plugin-probe")]
    pub(crate) fn frontend_detached(self: &Arc<Self>) -> AppResult<()> {
        for record in self.lock()?.values_mut() {
            record.ready = false;
        }
        self.sync_watcher()
    }
    pub fn has_clipboard_subscribers(&self) -> bool {
        self.lock().is_ok_and(|records| {
            records.values().any(|r| {
                r.descriptor.loaded && r.ready && r.context.permissions.contains("clipboard:read")
            })
        })
    }
    fn sync_watcher(self: &Arc<Self>) -> AppResult<()> {
        let mut watcher = self
            .watcher
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.clipboard.sync_watcher.lock",
                    error,
                );
            })
            .map_err(|_| "剪贴板监听不可用")?;
        if self.has_clipboard_subscribers() && watcher.is_none() {
            *watcher = Some(clipboard::start(self)?);
        }
        if !self.has_clipboard_subscribers() {
            self.clipboard_sender.send_replace(None);
            if let Some(watcher) = watcher.take() {
                watcher.stop();
            }
        }
        if !self.lock()?.values().any(|record| record.runtime.is_some()) {
            *self
                .engine
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("plugins.engine.release.lock", error);
                })
                .map_err(|_| "WASM Engine 不可用")? = None;
        }
        Ok(())
    }
    fn queue_clipboard(&self, text: String) {
        if text.len() > clipboard::MAX_TEXT {
            return;
        }
        let targets = self
            .lock()
            .map(|records| {
                records
                    .values()
                    .filter(|r| {
                        r.descriptor.loaded
                            && r.ready
                            && r.context.permissions.contains("clipboard:read")
                    })
                    .map(|r| (r.context.id.clone(), r.context.generation))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !targets.is_empty() {
            self.clipboard_sender
                .send_replace(Some(ClipboardChange { text, targets }));
        }
    }
    async fn deliver_clipboard(self: &Arc<Self>, change: &ClipboardChange) {
        for (id, generation) in &change.targets {
            if self.scoped(id, *generation).is_err() {
                continue;
            }
            let _ = (self.app.emit(
                "plugin-clipboard-changed",
                serde_json::json!({"pluginId":id,"generation":generation,"text":change.text}),
            ))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("self.app.emit", error);
            });
            let _ = (self
                .invoke(
                    id,
                    *generation,
                    "event:clipboard-text",
                    &serde_json::json!({"text":change.text}).to_string(),
                )
                .await)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("plugins.event.invoke", error);
                });
        }
    }
    fn resource(
        &self,
        request: &tauri::http::Request<Vec<u8>>,
    ) -> AppResult<assets::ResourceResponse> {
        let uri = request.uri();
        let decoded = percent_encoding::percent_decode_str(uri.path())
            .decode_utf8()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.asset.decode_path", error);
            })
            .map_err(|_| "插件资源编码无效")?;
        let path = decoded.trim_start_matches('/');
        let mut parts = path.splitn(3, '/');
        let id = parts.next().ok_or("插件资源无效")?;
        let generation = parts
            .next()
            .and_then(|v| {
                v.parse::<u64>()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "plugins.asset.parse_generation",
                            error,
                        );
                    })
                    .ok()
            })
            .ok_or("资源代次无效")?;
        let relative = parts.next().ok_or("资源路径无效")?;
        let context = self.scoped(id, generation)?;
        context.check(Some("ui"))?;
        if request.method() != tauri::http::Method::GET
            && request.method() != tauri::http::Method::HEAD
        {
            return Ok(assets::ResourceResponse::method_not_allowed());
        }
        if let Some(module) = relative.strip_prefix("_host/") {
            let (bytes, mime) = host_module(module)?;
            return Ok(assets::ResourceResponse::from_bytes(
                bytes,
                mime,
                request.method(),
            ));
        }
        let file = {
            let records = self.lock()?;
            let manifest = &records.get(id).ok_or("插件未安装")?.descriptor.manifest;
            if manifest.frontend.as_deref() != Some(relative) && !relative.starts_with("assets/") {
                return Err("资源不在公开范围".into());
            }
            manifest::checked_file(&self.root.join(id), relative)?
        };
        let response = assets::read(&file, request.method(), request.headers())?;
        context.check(None)?;
        Ok(response)
    }
    pub async fn account_changed(&self) {
        self.songs.clear().await;
    }
    pub(crate) async fn music_cache_status(
        &self,
    ) -> AppResult<crate::infrastructure::other_cache::Usage> {
        self.songs.cache_status().await
    }
    pub(crate) async fn clear_music_cache(&self) {
        self.songs.clear().await;
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.lifecycle.send_replace(());
        if let Ok(records) = (self.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.stop.registry_lock", error);
        }) {
            for record in records.values() {
                record.context.active.store(false, Ordering::SeqCst);
                self.files
                    .close_instance(&record.descriptor.manifest.id, record.descriptor.generation);
                self.processes
                    .revoke(&record.context.id, record.context.generation);
                self.windows
                    .revoke(&record.context.id, record.context.generation);
            }
        }
        if let Ok(mut watcher) = (self.watcher.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.stop.watcher_lock", error);
        }) {
            if let Some(watcher) = watcher.take() {
                watcher.stop();
            }
        }
        if let Ok(mut engine) = (self.engine.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.stop.engine_lock", error);
        }) {
            *engine = None;
        }
        self.clipboard_sender.send_replace(None);
    }
}
fn host_module(module: &str) -> AppResult<(Vec<u8>, &'static str)> {
    let (object, exports) = match module {
        "react.mjs" => ("react", "Children,Fragment,Profiler,StrictMode,Suspense,Component,PureComponent,createContext,createElement,cloneElement,isValidElement,forwardRef,memo,lazy,startTransition,useActionState,useCallback,useContext,useDebugValue,useDeferredValue,useEffect,useId,useImperativeHandle,useInsertionEffect,useLayoutEffect,useMemo,useOptimistic,useReducer,useRef,useState,useSyncExternalStore,useTransition,use,act,cache,version"),
        "jsx-runtime.mjs" => ("jsx", "Fragment,jsx,jsxs"),
        "sdk.mjs" => ("sdk", "reportCaughtError,usePluginClipboard,usePluginHttp,useMusicSource,usePluginBackend,usePluginEvent,usePluginStorage,usePluginConfig,usePluginFiles,usePluginProcesses,usePluginWindows,usePluginNavigate,usePluginRoute,usePlayer,useTheme,useCoverSource,useSongPlayback,SongArtists,SongLikeButton,SongIdentity,DownloadedSongList,MusicPageHeader,Button,Progress,Input,Icon,Tabs,TabsList,TabsTab,TabsPanel,TabsPanels"),
        _ => return Err("未知宿主桥接模块".into()),
    };
    Ok((format!("const host=globalThis.__NONS_PLUGIN_HOST__.{object};export default host;export const {{{exports}}}=host;").into_bytes(), "text/javascript"))
}

#[tauri::command]
pub fn plugin_list(manager: State<'_, Arc<PluginManager>>) -> AppResult<Vec<Descriptor>> {
    manager.list()
}
#[tauri::command]
pub fn plugin_open_folder(manager: State<'_, Arc<PluginManager>>) -> AppResult<()> {
    manager
        .app
        .opener()
        .open_path(manager.root.to_string_lossy(), None::<&str>)
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub fn plugin_discover(manager: State<'_, Arc<PluginManager>>) -> AppResult<Vec<Descriptor>> {
    manager.discover()?;
    manager.notify();
    manager.list()
}
#[tauri::command]
pub async fn plugin_package_preview(
    path: String,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<Manifest> {
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.preview(Path::new(&path)))
        .await
        .map_err(|error| error.to_string())?
}
#[tauri::command]
pub async fn plugin_install(
    path: String,
    confirmed: Option<bool>,
    expected_manifest: Option<Manifest>,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<()> {
    let confirmed = confirmed.unwrap_or(false);
    if confirmed && expected_manifest.is_none() {
        return Err("确认安装前必须预览插件包并提供 expectedManifest".into());
    }
    manager
        .inner()
        .install_reviewed_expected(path.into(), confirmed, expected_manifest)
        .await
}
#[tauri::command]
pub async fn plugin_action(
    id: String,
    action: String,
    confirmed: bool,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<()> {
    manager.inner().action(&id, &action, confirmed).await
}
#[tauri::command]
pub async fn plugin_call(
    id: String,
    generation: u64,
    method: String,
    args: String,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<String> {
    manager.inner().call(&id, generation, &method, &args).await
}
#[tauri::command]
pub async fn plugin_host_call(
    id: String,
    generation: u64,
    operation: String,
    args: String,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<String> {
    let context = manager.scoped(&id, generation)?;
    let started = std::time::Instant::now();
    let _permit = context
        .requests
        .clone()
        .try_acquire_owned()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.host_call.admission", error);
        })
        .map_err(|_| "插件调用过于频繁")?;
    context.call(&operation, &args).await.inspect_err(|error| {
        log::warn!(target: "nons::plugins", "operation=plugins.host_call stage=capability_call plugin={} generation={} capability={} elapsed_ms={} reason={}", id, generation, crate::infrastructure::diagnostics::redact(&operation), started.elapsed().as_millis(), error);
    })
}
#[tauri::command]
pub fn plugin_ready(
    id: String,
    generation: u64,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<()> {
    manager.inner().ready(&id, generation)
}
#[tauri::command]
pub fn plugin_fault(
    id: String,
    generation: u64,
    error: String,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<()> {
    manager.inner().fault(&id, generation, &error)
}

pub fn protocol(
    context: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let origin = request
        .headers()
        .get("origin")
        .and_then(|v| {
            v.to_str()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("plugins.asset.parse_origin", error);
                })
                .ok()
        })
        .unwrap_or("");
    let allowed = matches!(
        origin,
        "http://tauri.localhost"
            | "https://tauri.localhost"
            | "tauri://localhost"
            | "http://localhost:1420"
            | "http://plugin.localhost"
            | "https://plugin.localhost"
            | "plugin://localhost"
    );
    let result = if allowed || origin.is_empty() {
        context
            .app_handle()
            .try_state::<Arc<PluginManager>>()
            .ok_or("插件服务尚未就绪".to_string())
            .and_then(|manager| manager.resource(&request))
    } else {
        Err("插件资源来源无效".into())
    };
    let mut builder = tauri::http::Response::builder()
        .header("Cache-Control", "no-store")
        .header("X-Content-Type-Options", "nosniff");
    if allowed {
        builder = builder.header("Access-Control-Allow-Origin", origin);
    }
    match result {
        Ok(response) => {
            let mut builder = builder
                .status(response.status)
                .header("Content-Type", response.content_type)
                .header("Content-Length", response.content_length)
                .header("Accept-Ranges", "bytes");
            if response.status == tauri::http::StatusCode::METHOD_NOT_ALLOWED {
                builder = builder.header("Allow", "GET, HEAD");
            }
            if let Some(content_range) = response.content_range {
                builder = builder.header("Content-Range", content_range);
            }
            builder.body(response.body).expect("valid response")
        }
        Err(error) => {
            crate::infrastructure::diagnostics::warn("plugins.resource", error);
            builder
                .status(403)
                .body(Vec::new())
                .expect("valid response")
        }
    }
}
