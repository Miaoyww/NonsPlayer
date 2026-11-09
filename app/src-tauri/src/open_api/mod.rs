mod api;
mod dto;
pub(crate) mod lyrics;
mod provider;
pub(crate) mod settings;
pub(crate) mod ttml;

use api::Context;
use dto::{Progress, Queue, State as PublicState};
use provider::BackendProvider;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{Emitter, EventId, Listener, Manager as TauriManager, State as TauriState};
use tokio::sync::{watch, Mutex as AsyncMutex};

const ENABLED: &str = "openApiEnabled";
const PORT: &str = "openApiPort";
const LAN: &str = "openApiLan";
const DEFAULT_PORT: u16 = 46321;
const STATUS_EVENT: &str = "open-api-status-changed";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub enabled: bool,
    pub port: u16,
    pub lan: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            lan: false,
        }
    }
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            Err("端口须在 1 至 65535 之间".into())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub config: Config,
    pub running: bool,
    pub addresses: Vec<String>,
    pub connections: usize,
    pub error: Option<String>,
}

struct LocalState {
    config: Config,
    error: Option<String>,
}

struct Running {
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
    listeners: Vec<EventId>,
}

pub struct Manager {
    app: tauri::AppHandle,
    store: Arc<crate::storage::Store>,
    provider: Arc<BackendProvider>,
    state: Mutex<LocalState>,
    running: Arc<AtomicBool>,
    runtime: Mutex<Option<Running>>,
    context: Mutex<Option<Arc<Context>>>,
    lifecycle: AsyncMutex<()>,
    offset: AsyncMutex<()>,
}

impl Manager {
    pub fn new(app: tauri::AppHandle, backend: &crate::Backend) -> Self {
        let config = load_config(&backend.store);
        let provider = Arc::new(BackendProvider::new(backend, app.clone()));
        Self {
            app,
            store: backend.store.clone(),
            provider,
            state: Mutex::new(LocalState {
                config,
                error: None,
            }),
            running: Arc::new(AtomicBool::new(false)),
            runtime: Mutex::new(None),
            context: Mutex::new(None),
            lifecycle: AsyncMutex::new(()),
            offset: AsyncMutex::new(()),
        }
    }

    fn config(&self) -> Result<Config, String> {
        self.state
            .lock()
            .map(|state| state.config.clone())
            .map_err(|_| "开放 API 配置锁不可用".into())
    }

    fn set_error(&self, value: Option<String>) {
        if let Ok(mut state) = self.state.lock() {
            state.error = value;
        }
    }

    pub fn status(&self) -> Status {
        let (config, error) = self
            .state
            .lock()
            .map(|state| (state.config.clone(), state.error.clone()))
            .unwrap_or_else(|_| (Config::default(), Some("开放 API 状态锁不可用".into())));
        let running = self.running.load(Ordering::Acquire);
        Status {
            addresses: if running {
                addresses(config.port, config.lan)
            } else {
                Vec::new()
            },
            connections: self
                .context
                .lock()
                .ok()
                .and_then(|context| context.as_ref().map(|context| context.connections()))
                .unwrap_or(0),
            config,
            running,
            error,
        }
    }

    pub async fn restore(&self) {
        let _guard = self.lifecycle.lock().await;
        if self.config().is_ok_and(|config| config.enabled) {
            if let Err(error) = self.start_locked().await {
                crate::infrastructure::diagnostics::warn("open_api.restore", &error);
                self.set_error(Some(error));
            }
        }
        self.emit_status();
    }

    pub async fn configure(&self, config: Config) -> Result<Status, String> {
        config.validate().inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.config.validate", error);
        })?;
        let _guard = self.lifecycle.lock().await;
        let previous = self.config()?;
        if config == previous && (self.running.load(Ordering::Acquire) || !config.enabled) {
            return Ok(self.status());
        }
        if let Err(error) = persist(&self.store, &config) {
            let _ = restore_persisted(&self.store, &previous).inspect_err(|rollback| {
                crate::infrastructure::diagnostics::warn("open_api.config.rollback", rollback);
            });
            return Err(error);
        }
        self.state
            .lock()
            .map_err(|_| "开放 API 配置锁不可用".to_string())?
            .config = config.clone();
        self.stop_locked().await;
        if config.enabled {
            if let Err(error) = self.start_locked().await {
                crate::infrastructure::diagnostics::warn("open_api.config.start", &error);
                self.set_error(Some(error));
            }
        } else {
            self.set_error(None);
        }
        self.emit_status();
        Ok(self.status())
    }

    async fn start_locked(&self) -> Result<(), String> {
        if self.running.load(Ordering::Acquire) {
            return Ok(());
        }
        let config = self.config()?;
        let host = if config.lan {
            Ipv4Addr::UNSPECIFIED
        } else {
            Ipv4Addr::LOCALHOST
        };
        let listener =
            tokio::net::TcpListener::bind(SocketAddr::new(IpAddr::V4(host), config.port))
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("open_api.bind", error);
                })
                .map_err(|_| "端口已被占用或监听地址不可用".to_string())?;
        let allowed = allowed_hosts(config.lan);
        let (shutdown, receiver) = watch::channel(false);
        let task_receiver = receiver.clone();
        let monitor_receiver = receiver.clone();
        let context = Arc::new(Context::new(
            self.provider.clone(),
            allowed,
            api::EVENT_CAPACITY,
            receiver.clone(),
        ));
        let listeners = attach_events(self.app.clone(), context.clone());
        let router = api::router(context.clone());
        let app = self.app.clone();
        let running = self.running.clone();
        running.store(true, Ordering::Release);
        let task_running = running.clone();
        let task = tokio::spawn(async move {
            let mut graceful = task_receiver;
            let result = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = graceful.changed().await;
                })
                .await;
            if let Err(error) = result {
                crate::infrastructure::diagnostics::warn("open_api.serve", error);
                if let Some(manager) = app.try_state::<Arc<Manager>>() {
                    manager.set_error(Some("开放 API 服务意外停止".into()));
                }
            }
            task_running.store(false, Ordering::Release);
            emit_status_for(&app);
        });
        monitor_connections(self.app.clone(), context.clone(), monitor_receiver);
        *self
            .context
            .lock()
            .map_err(|_| "开放 API 连接状态锁不可用".to_string())? = Some(context);
        *self
            .runtime
            .lock()
            .map_err(|_| "开放 API 生命周期锁不可用".to_string())? = Some(Running {
            shutdown,
            task,
            listeners,
        });
        self.set_error(None);
        self.emit_status();
        Ok(())
    }

    async fn stop_locked(&self) {
        let running = self
            .runtime
            .lock()
            .ok()
            .and_then(|mut runtime| runtime.take());
        self.running.store(false, Ordering::Release);
        if let Some(mut running) = running {
            for listener in running.listeners.drain(..) {
                self.app.unlisten(listener);
            }
            let _ = running.shutdown.send(true);
            if tokio::time::timeout(Duration::from_secs(3), &mut running.task)
                .await
                .is_err()
            {
                running.task.abort();
            }
        }
        if let Ok(mut context) = self.context.lock() {
            *context = None;
        }
    }

    pub async fn stop(&self) {
        let _guard = self.lifecycle.lock().await;
        self.stop_locked().await;
        self.emit_status();
    }

    fn emit_status(&self) {
        emit(&self.app, STATUS_EVENT, self.status());
    }

    pub async fn offset_lock(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.offset.lock().await
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        if let Ok(runtime) = self.runtime.get_mut() {
            if let Some(running) = runtime.take() {
                let _ = running.shutdown.send(true);
                running.task.abort();
            }
        }
    }
}

fn load_config(store: &crate::storage::Store) -> Config {
    let enabled = read_bool(store, ENABLED, false).unwrap_or(false);
    let lan = read_bool(store, LAN, false).unwrap_or(false);
    let port = match store
        .setting(PORT)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.config.port.read", error);
        })
        .ok()
        .flatten()
    {
        Some(value) => value
            .parse::<u16>()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("open_api.config.port.parse", error);
            })
            .ok()
            .filter(|port| *port > 0)
            .unwrap_or(DEFAULT_PORT),
        None => DEFAULT_PORT,
    };
    Config { enabled, port, lan }
}

fn read_bool(store: &crate::storage::Store, key: &str, fallback: bool) -> Result<bool, String> {
    match store.setting(key).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.config.bool.read", error);
    })? {
        Some(value) => value
            .parse::<bool>()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("open_api.config.bool.parse", error);
            })
            .or(Ok(fallback)),
        None => Ok(fallback),
    }
}

fn persist(store: &crate::storage::Store, config: &Config) -> Result<(), String> {
    let mut db = store.0.lock().map_err(|_| "配置锁不可用".to_string())?;
    let transaction = db.transaction().map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![ENABLED, if config.enabled { "true" } else { "false" }],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![PORT, config.port.to_string()],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![LAN, if config.lan { "true" } else { "false" }],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.config.persist", &error);
        error.to_string()
    })
}

fn restore_persisted(store: &crate::storage::Store, config: &Config) -> Result<(), String> {
    persist(store, config)
}

fn allowed_hosts(lan: bool) -> HashSet<String> {
    let mut hosts = HashSet::from(["127.0.0.1".into(), "localhost".into()]);
    if lan {
        if let Ok(interfaces) = if_addrs::get_if_addrs() {
            for interface in interfaces
                .into_iter()
                .filter(|interface| !interface.is_loopback())
            {
                if let IpAddr::V4(address) = interface.ip() {
                    hosts.insert(address.to_string());
                }
            }
        } else {
            crate::infrastructure::diagnostics::warn(
                "open_api.network_interfaces",
                "interface enumeration failed",
            );
        }
    }
    hosts
}

fn addresses(port: u16, lan: bool) -> Vec<String> {
    let mut values = vec![format!("http://127.0.0.1:{port}")];
    if lan {
        if let Ok(interfaces) = if_addrs::get_if_addrs() {
            values.extend(
                interfaces
                    .into_iter()
                    .filter(|interface| !interface.is_loopback())
                    .filter_map(|interface| match interface.ip() {
                        IpAddr::V4(address) => Some(format!("http://{address}:{port}")),
                        IpAddr::V6(_) => None,
                    }),
            );
        }
    }
    values.sort();
    values.dedup();
    values
}

fn emit(app: &tauri::AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(error) = app.emit(event, payload) {
        crate::infrastructure::diagnostics::warn("open_api.status.emit", error);
    }
}

fn emit_status_for(app: &tauri::AppHandle) {
    if let Some(manager) = app.try_state::<Arc<Manager>>() {
        emit(app, STATUS_EVENT, manager.status());
    }
}

fn attach_events(app: tauri::AppHandle, context: Arc<Context>) -> Vec<EventId> {
    let mut ids = Vec::new();
    let state_context = context.clone();
    ids.push(app.listen_any("player-state", move |event| {
        match serde_json::from_str::<crate::playback::publication::PlayerState>(event.payload()) {
            Ok(value) => state_context.publish("player.state", &PublicState::from_public(&value)),
            Err(error) => {
                crate::infrastructure::diagnostics::warn("open_api.event.player_state", error)
            }
        }
    }));
    let queue_context = context.clone();
    ids.push(app.listen_any("player-queue", move |event| {
        match serde_json::from_str::<crate::playback::publication::PlayerQueue>(event.payload()) {
            Ok(value) => queue_context.publish("player.queue", &Queue::from_public(&value)),
            Err(error) => {
                crate::infrastructure::diagnostics::warn("open_api.event.player_queue", error)
            }
        }
    }));
    let progress_context = context.clone();
    ids.push(app.listen_any("player-progress", move |event| {
        match serde_json::from_str::<crate::playback::publication::PlayerProgress>(event.payload())
        {
            Ok(value) => {
                progress_context.publish("player.progress", &Progress::from_public(&value))
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("open_api.event.player_progress", error)
            }
        }
    }));
    let lyric_context = context.clone();
    ids.push(app.listen_any("lyrics-updated", move |event| {
        match serde_json::from_str::<serde_json::Value>(event.payload()) {
            Ok(value) => {
                if let Some(key) = value.get("key").and_then(|value| value.as_str()) {
                    lyric_context.publish(
                        "lyrics.invalidated",
                        &serde_json::json!({"trackId": dto::public_track_id(key)}),
                    );
                }
            }
            Err(error) => crate::infrastructure::diagnostics::warn("open_api.event.lyrics", error),
        }
    }));
    ids.push(invalidate_key_event(
        app.clone(),
        context.clone(),
        "lyric-correction-changed",
    ));
    ids.push(invalidate_key_event(
        app.clone(),
        context.clone(),
        "lyric-offset-changed",
    ));
    ids.push(invalidate_current_event(
        app.clone(),
        context.clone(),
        "lyric-sources-changed",
    ));
    ids.push(invalidate_current_event(
        app.clone(),
        context.clone(),
        "lyric-global-offset-changed",
    ));
    ids.push(invalidate_current_event(
        app.clone(),
        context.clone(),
        "lyric-cache-changed",
    ));
    ids.push(invalidate_current_event(
        app.clone(),
        context.clone(),
        "local-library-updated",
    ));
    ids
}

fn invalidate_key_event(
    app: tauri::AppHandle,
    context: Arc<Context>,
    name: &'static str,
) -> EventId {
    app.listen_any(name, move |event| {
        match serde_json::from_str::<serde_json::Value>(event.payload()) {
            Ok(value) => {
                let key = value
                    .as_str()
                    .or_else(|| value.get("key").and_then(|value| value.as_str()));
                if let Some(key) = key {
                    context.publish(
                        "lyrics.invalidated",
                        &serde_json::json!({"trackId": dto::public_track_id(key)}),
                    );
                }
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("open_api.event.invalidate_key", error);
            }
        }
    })
}

fn invalidate_current_event(
    app: tauri::AppHandle,
    context: Arc<Context>,
    name: &'static str,
) -> EventId {
    app.listen_any(name, move |_| {
        let track = context.provider.bootstrap().ok().and_then(|bootstrap| {
            let index = bootstrap.state.index?;
            let key = bootstrap.queue.queue.get(index)?.key.clone();
            Some(dto::public_track_id(&key))
        });
        context.publish("lyrics.invalidated", &serde_json::json!({"trackId": track}));
    })
}

fn monitor_connections(
    app: tauri::AppHandle,
    context: Arc<Context>,
    mut shutdown: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let mut previous = context.connections();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = interval.tick() => {
                    let current = context.connections();
                    if current != previous {
                        previous = current;
                        emit_status_for(&app);
                    }
                }
            }
        }
    });
}

#[tauri::command]
pub fn open_api_status(manager: TauriState<'_, Arc<Manager>>) -> Status {
    manager.status()
}

#[tauri::command]
pub async fn open_api_configure(
    config: Config,
    manager: TauriState<'_, Arc<Manager>>,
) -> Result<Status, String> {
    manager.configure(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;

    #[test]
    fn configuration_persists_across_store_reopen_and_rejects_port_zero() {
        let root = TestDir::new();
        let path = root.0.join("nons.sqlite3");
        let store = crate::storage::Store::open(&path).unwrap();
        let config = Config {
            enabled: true,
            port: 46322,
            lan: true,
        };
        config.validate().unwrap();
        persist(&store, &config).unwrap();
        drop(store);

        let reopened = crate::storage::Store::open(&path).unwrap();
        assert_eq!(load_config(&reopened), config);
        assert!(Config {
            port: 0,
            ..Config::default()
        }
        .validate()
        .is_err());
    }
}
