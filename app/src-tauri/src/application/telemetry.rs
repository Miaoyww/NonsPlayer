//! One immutable launch snapshot per process; no account, music or machine identifiers.
use crate::{model::AppResult, plugins::PluginManager, storage::Store};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, State};
use tokio::sync::watch;

const ENDPOINT: &str = "https://nons-api.miaoyww.top/v1/event";
const ENABLED: &str = "anonymousTelemetryEnabled";

pub struct Telemetry {
    store: Arc<Store>,
    enabled: watch::Sender<bool>,
    mutation: Mutex<()>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Adapter {
    source: String,
    #[serde(rename = "displayName")]
    display_name: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Plugin {
    id: String,
    name: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Launch {
    event_id: String,
    device_id: String,
    platform: String,
    app_version: String,
    occurred_at: String,
    adapters: Vec<Adapter>,
    plugins: Vec<Plugin>,
}

impl Telemetry {
    pub fn new(store: Arc<Store>) -> AppResult<Arc<Self>> {
        let enabled = store.setting(ENABLED)?.as_deref() != Some("false");
        Ok(Arc::new(Self {
            store,
            enabled: watch::channel(enabled).0,
            mutation: Mutex::new(()),
        }))
    }

    fn installation_id(&self) -> AppResult<String> {
        let _guard = self
            .mutation
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "遥测设置锁不可用")?;
        if let Some(id) = self.store.setting("anonymousInstallationId")? {
            if uuid::Uuid::parse_str(&id).is_ok() {
                return Ok(id);
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.store.set_setting("anonymousInstallationId", &id)?;
        Ok(id)
    }

    pub async fn report(self: Arc<Self>, app: tauri::AppHandle, occurred_at: String) {
        // Development and probe processes never contribute production usage.
        if cfg!(debug_assertions) || cfg!(feature = "plugin-probe") {
            return;
        }
        if let Err(error) = self.report_inner(app, occurred_at).await {
            // Never log payloads, stable identifiers, headers or response bodies.
            log::warn!(target: "nons::telemetry", "launch_report_failed reason={error}");
        }
    }

    async fn report_inner(&self, app: tauri::AppHandle, occurred_at: String) -> AppResult<()> {
        let key = option_env!("NONS_TELEMETRY_INGEST_KEY").unwrap_or("");
        let enabled = self.enabled.subscribe();
        if !*enabled.borrow() || key.is_empty() {
            return Ok(());
        }
        let backend = app.state::<super::Backend>();
        let mut adapters = backend
            .music
            .adapters
            .statuses()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "适配器状态不可用")?
            .into_iter()
            .filter(|entry| entry.enabled)
            .map(|entry| Adapter {
                source: entry.descriptor.source.as_str().to_owned(),
                display_name: entry.descriptor.display_name,
            })
            .collect::<Vec<_>>();
        // Audio-only providers are adapters too; catalog adapters also expose providers.
        for entry in backend
            .music
            .adapters
            .resource_provider_statuses()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "音源状态不可用")?
        {
            if entry.enabled
                && !adapters
                    .iter()
                    .any(|a| a.source == entry.descriptor.id.as_str())
            {
                adapters.push(Adapter {
                    source: entry.descriptor.id.as_str().to_owned(),
                    display_name: entry.descriptor.display_name,
                });
            }
        }
        let plugins = app
            .state::<Arc<PluginManager>>()
            .list()?
            .into_iter()
            .filter(|entry| entry.enabled)
            .map(|entry| Plugin {
                id: entry.manifest.id,
                name: entry.manifest.name,
            })
            .collect::<Vec<_>>();
        if adapters.len() > 64 || plugins.len() > 64 {
            return Err("遥测组件数量超过接口上限".into());
        }
        let event = Launch {
            event_id: uuid::Uuid::new_v4().to_string(),
            device_id: self.installation_id()?,
            platform: platform().into(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            occurred_at,
            adapters,
            plugins,
        };
        let body = serde_json::to_vec(&event)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "遥测序列化失败")?;
        if body.len() > 65536 {
            return Err("遥测请求超过接口上限".into());
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "遥测网络初始化失败")?;
        transmit(&client, ENDPOINT, key, body, enabled, &[0, 15, 60, 300]).await
    }
}

async fn transmit(
    client: &reqwest::Client,
    endpoint: &str,
    key: &str,
    body: Vec<u8>,
    mut enabled: watch::Receiver<bool>,
    delays: &[u64],
) -> AppResult<()> {
    for (attempt, delay) in delays.iter().enumerate() {
        if !*enabled.borrow_and_update() {
            return Ok(());
        }
        tokio::select! {
            biased;
            _ = enabled.changed() => return Ok(()),
            _ = tokio::time::sleep(Duration::from_secs(*delay)) => {}
        }
        let response = tokio::select! {
            biased;
            _ = enabled.changed() => return Ok(()),
            result = client.post(endpoint).header("X-Ingest-Key", key)
                .header(reqwest::header::CONTENT_TYPE, "application/json").body(body.clone()).send() => result
        };
        match response {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => {
                log::warn!(target: "nons::telemetry", "operation=transmit attempt={} status={}", attempt + 1, response.status().as_u16());
                if !retryable(response.status().as_u16()) {
                    return Err("遥测请求被拒绝".into());
                }
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn_error("telemetry.transmit", &error);
            }
        }
    }
    Err("遥测重试结束".into())
}

fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "android") {
        "Android"
    } else if cfg!(target_os = "macos") {
        "MacOs"
    } else {
        "Linux"
    }
}
fn retryable(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

#[tauri::command]
pub fn telemetry_enabled(telemetry: State<'_, Arc<Telemetry>>) -> bool {
    *telemetry.enabled.borrow()
}
#[tauri::command]
pub fn set_telemetry_enabled(
    enabled: bool,
    telemetry: State<'_, Arc<Telemetry>>,
) -> AppResult<bool> {
    let _guard = telemetry
        .mutation
        .lock()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "遥测设置锁不可用")?;
    telemetry
        .store
        .set_setting(ENABLED, if enabled { "true" } else { "false" })?;
    if *telemetry.enabled.borrow() != enabled {
        telemetry.enabled.send_replace(enabled);
    }
    Ok(enabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    fn server(statuses: Vec<u16>) -> (String, std::thread::JoinHandle<Vec<Vec<u8>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1/event", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let task = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for status in statuses {
                let deadline = std::time::Instant::now() + Duration::from_secs(4);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "telemetry request timeout"
                            );
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("{error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut chunk = [0; 1024];
                loop {
                    let size = stream.read(&mut chunk).unwrap();
                    assert!(size > 0);
                    bytes.extend_from_slice(&chunk[..size]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                        assert!(headers.contains("x-ingest-key: test-key"));
                        let length: usize = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length: "))
                            .unwrap()
                            .parse()
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            requests.push(bytes[end + 4..end + 4 + length].to_vec());
                            break;
                        }
                    }
                }
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            }
            requests
        });
        (url, task)
    }
    #[tokio::test]
    async fn network_retries_preserve_the_entire_launch_snapshot() {
        let (url, server) = server(vec![503, 201]);
        let (enabled, receiver) = watch::channel(true);
        let body = br#"{"event_id":"same-id","plugins":[],"adapters":[]}"#.to_vec();
        transmit(
            &reqwest::Client::new(),
            &url,
            "test-key",
            body.clone(),
            receiver,
            &[0, 0],
        )
        .await
        .unwrap();
        drop(enabled);
        let requests = server.join().unwrap();
        assert_eq!(requests, vec![body.clone(), body]);
    }
    #[tokio::test]
    async fn rejection_is_not_retried_and_disabling_cancels_backoff() {
        let (url, server) = server(vec![401]);
        let (enabled, receiver) = watch::channel(true);
        assert!(transmit(
            &reqwest::Client::new(),
            &url,
            "test-key",
            b"{}".to_vec(),
            receiver,
            &[0, 0]
        )
        .await
        .is_err());
        assert_eq!(server.join().unwrap().len(), 1);
        let receiver = enabled.subscribe();
        let client = reqwest::Client::new();
        let request = transmit(
            &client,
            "http://127.0.0.1:1",
            "test-key",
            b"{}".to_vec(),
            receiver,
            &[60],
        );
        let cancel = async {
            tokio::task::yield_now().await;
            enabled.send_replace(false);
        };
        let (result, _) = tokio::join!(request, cancel);
        assert!(result.is_ok());
    }
    #[test]
    fn defaults_on_and_keeps_random_installation_id_across_restarts() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&dir.path().join("test.db")).unwrap());
        let telemetry = Telemetry::new(store.clone()).unwrap();
        assert!(*telemetry.enabled.borrow());
        let id = telemetry.installation_id().unwrap();
        assert_eq!(uuid::Uuid::parse_str(&id).unwrap().get_version_num(), 4);
        store.set_setting(ENABLED, "false").unwrap();
        let restored = Telemetry::new(store).unwrap();
        assert!(!*restored.enabled.borrow());
        assert_eq!(restored.installation_id().unwrap(), id);
    }
    #[test]
    fn retries_only_transient_failures() {
        for status in [429, 500, 503, 599] {
            assert!(retryable(status));
        }
        for status in [200, 201, 400, 401, 409, 413] {
            assert!(!retryable(status));
        }
    }
}
