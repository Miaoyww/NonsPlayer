//! Source-scoped bindings for generic Host capabilities. No platform protocol here.
use super::*;
use crate::infrastructure::{
    http::{BinaryRequest, Http},
    websocket::Sockets,
};
use crate::music::{
    account::{AccountAccess, AccountRecord, OpaqueCredential},
    accounts::AccountManager,
    manager::AdapterManager,
};
use crate::wasm_runtime::host_io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, Weak,
};

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Permissions {
    pub http_hosts: Vec<String>,
    pub websocket_hosts: Vec<String>,
    pub system: bool,
    pub state: bool,
    pub credentials: bool,
}
pub(crate) struct Services {
    pub accounts: Arc<AccountManager>,
    pub adapters: Weak<AdapterManager>,
    pub store: Arc<crate::storage::Store>,
    pub http: Arc<Http>,
    pub permissions: Permissions,
    pub descriptor: SourceDescriptor,
    state_lock: Mutex<()>,
}
impl Services {
    pub fn new(
        accounts: Arc<AccountManager>,
        adapters: Weak<AdapterManager>,
        store: Arc<crate::storage::Store>,
        http: Arc<Http>,
        permissions: Permissions,
        descriptor: SourceDescriptor,
    ) -> AppResult<Self> {
        for hosts in [&permissions.http_hosts, &permissions.websocket_hosts] {
            if hosts.len() > 32
                || hosts.iter().any(|host| {
                    !nons_music_adapter_sdk::cover::valid_host_rule(host) || host == "*"
                })
            {
                return Err("适配器网络授权范围无效".into());
            }
        }
        Ok(Self {
            accounts,
            adapters,
            store,
            http,
            permissions,
            descriptor,
            state_lock: Mutex::new(()),
        })
    }
}
pub(crate) struct Context {
    pub services: Arc<Services>,
    context: Mutex<RequestContext>,
    pub active: Arc<AtomicBool>,
    sockets: Sockets,
    pub pending: Mutex<Option<(AccountRecord, OpaqueCredential)>>,
    login: bool,
    diagnostics: Mutex<(std::time::Instant, u32)>,
}
impl Context {
    pub fn new(services: Arc<Services>, context: RequestContext, login: bool) -> Arc<Self> {
        let active = Arc::new(AtomicBool::new(true));
        Arc::new(Self {
            services,
            context: Mutex::new(context),
            sockets: Sockets::new(active.clone()),
            active,
            pending: Mutex::new(None),
            login,
            diagnostics: Mutex::new((std::time::Instant::now(), 0)),
        })
    }
    pub fn check(&self) -> AppResult<RequestContext> {
        if !self.active.load(Ordering::Acquire) {
            return Err("staleContext".into());
        }
        let context = self.context.lock().map_err(|_| "internal")?.clone();
        let manager = self
            .services
            .adapters
            .upgrade()
            .ok_or("sourceUnavailable")?;
        let (session, generation) = manager
            .stamp(&self.services.descriptor.source)
            .map_err(|e| e.to_string())?;
        context
            .check(&session, generation)
            .map_err(|e| e.to_string())?;
        Ok(context)
    }
    pub fn renew(&self, next: RequestContext) -> AppResult<()> {
        let mut current = self.context.lock().map_err(|_| "internal")?;
        if current.session != next.session || current.adapter_generation != next.adapter_generation
        {
            return Err("staleContext".into());
        }
        *current = next;
        Ok(())
    }
    pub async fn release(&self) {
        self.active.store(false, Ordering::Release);
        self.sockets.clear().await;
        if let Ok(mut pending) = self.pending.lock() {
            pending.take();
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}
impl HostHandler for Context {
    fn system_capabilities(&self) -> bool {
        self.services.permissions.system
    }
    fn call<'a>(
        &'a self,
        operation: &'a str,
        args: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<String>> + Send + 'a>> {
        Box::pin(async move {
            self.check()?;
            if operation != "diagnostic.report" || args.len() > 1024 {
                return Err("permissionDenied".into());
            }
            let value: std::collections::BTreeMap<String, String> =
                serde_json::from_str(args).map_err(|_| "invalidData")?;
            if value.len() > 3
                || value.iter().any(|(key, value)| {
                    !matches!(key.as_str(), "stage" | "class" | "code")
                        || value.len() > 64
                        || !value
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
                })
            {
                return Err("invalidData".into());
            }
            let mut budget = self.diagnostics.lock().map_err(|_| "internal")?;
            if budget.0.elapsed() >= std::time::Duration::from_secs(1) {
                *budget = (std::time::Instant::now(), 0);
            }
            if budget.1 >= 16 {
                return Err("rateLimited".into());
            }
            budget.1 += 1;
            log::warn!(target: "nons::adapter", "source={} stage={} class={} code={}", self.services.descriptor.source.as_str(), value.get("stage").map(String::as_str).unwrap_or("-"), value.get("class").map(String::as_str).unwrap_or("-"), value.get("code").map(String::as_str).unwrap_or("-"));
            Ok("null".into())
        })
    }
    fn http_request(
        &self,
        value: host_io::Request,
    ) -> Pin<Box<dyn Future<Output = AppResult<host_io::Response>> + Send + '_>> {
        Box::pin(async move {
            let context = self.check()?;
            let remaining = context
                .deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_millis()
                .min(15000) as u32;
            let response = self
                .services
                .http
                .binary_request(
                    &self.services.permissions.http_hosts,
                    BinaryRequest {
                        url: value.url,
                        method: value.method,
                        headers: value
                            .headers
                            .into_iter()
                            .map(|h| (h.name, h.value))
                            .collect(),
                        body: value.body,
                        timeout_ms: value.timeout_ms.min(remaining),
                    },
                    &self.active,
                )
                .await?;
            self.check()?;
            Ok(host_io::Response {
                status: response.status,
                headers: response
                    .headers
                    .into_iter()
                    .map(|(name, value)| host_io::Header { name, value })
                    .collect(),
                body: response.body,
            })
        })
    }
    fn websocket_open(
        &self,
        url: String,
        protocol: String,
        timeout: u32,
    ) -> Pin<Box<dyn Future<Output = AppResult<u32>> + Send + '_>> {
        Box::pin(async move {
            self.check()?;
            let id = self
                .sockets
                .open(
                    &self.services.permissions.websocket_hosts,
                    url,
                    protocol,
                    timeout,
                )
                .await?;
            self.check()?;
            Ok(id)
        })
    }
    fn websocket_send(
        &self,
        socket: u32,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + '_>> {
        Box::pin(async move {
            self.check()?;
            self.sockets.send(socket, data).await
        })
    }
    fn websocket_close(
        &self,
        socket: u32,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + '_>> {
        Box::pin(async move { self.sockets.close(socket).await })
    }
    fn next_event(
        &self,
        timeout: u32,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<host_io::Event>>> + Send + '_>> {
        Box::pin(async move {
            self.check()?;
            let value = self.sockets.next_event(timeout).await?;
            self.check()?;
            Ok(value)
        })
    }
    fn credential_read(&self) -> Pin<Box<dyn Future<Output = AppResult<Vec<u8>>> + Send + '_>> {
        Box::pin(async move {
            let context = self.check()?;
            if !self.services.permissions.credentials {
                return Err("permissionDenied".into());
            }
            if context.session.account.is_none() {
                return Ok(Vec::new());
            }
            let accounts = self.services.accounts.clone();
            let bytes = accounts
                .blocking(move |a| {
                    a.scoped_access(&context)?
                        .read(&context)
                        .map(|c| c.expose().to_vec())
                })
                .await
                .map_err(|e| e.to_string())?;
            self.check()?;
            Ok(bytes)
        })
    }
    fn credential_invalidate(&self) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + '_>> {
        Box::pin(async move {
            let context = self.check()?;
            if !self.services.permissions.credentials
                || self.login
                || context.session.account.is_none()
            {
                return Err("permissionDenied".into());
            }
            self.services
                .accounts
                .clone()
                .blocking(move |accounts| {
                    accounts.scoped_access(&context)?;
                    accounts.invalidate(&context.session)
                })
                .await
                .map_err(|error| error.to_string())
        })
    }
    fn credential_replace(
        &self,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + '_>> {
        Box::pin(async move {
            let context = self.check()?;
            if !self.services.permissions.credentials {
                return Err("permissionDenied".into());
            }
            let credential = OpaqueCredential::new(data).map_err(|e| e.to_string())?;
            let accounts = self.services.accounts.clone();
            accounts
                .blocking(move |a| a.scoped_access(&context)?.replace(&context, credential))
                .await
                .map_err(|e| e.to_string())?;
            // AccountManager validated the write and advanced the session generation.
            // The caller's old business result is rejected by AdapterManager.
            Ok(())
        })
    }
    fn credential_publish(
        &self,
        metadata: String,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + '_>> {
        Box::pin(async move {
            let context = self.check()?;
            if !self.login || !self.services.permissions.credentials || metadata.len() > 16384 {
                return Err("permissionDenied".into());
            }
            let record: AccountRecord =
                serde_json::from_str(&metadata).map_err(|_| "invalidData")?;
            if record.reference.source != context.session.source || record.display_name.len() > 1024
            {
                return Err("invalidData".into());
            }
            record
                .avatar
                .validate_hosts(&self.services.descriptor.cover_hosts)
                .map_err(|e| e.to_string())?;
            let credential = OpaqueCredential::new(data).map_err(|e| e.to_string())?;
            let mut pending = self.pending.lock().map_err(|_| "internal")?;
            if pending.is_some() {
                return Err("invalidData".into());
            }
            *pending = Some((record, credential));
            Ok(())
        })
    }
    fn state_get(
        &self,
        key: String,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<String>>> + Send + '_>> {
        Box::pin(async move {
            self.check()?;
            if !self.services.permissions.state
                || key.is_empty()
                || key.len() > 64
                || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return Err("permissionDenied".into());
            }
            let services = self.services.clone();
            let value = tauri::async_runtime::spawn_blocking(move || {
                let _guard = services.state_lock.lock().map_err(|_| "状态锁不可用")?;
                let name = format!(
                    "musicAdapterStateV1:{}",
                    services.descriptor.source.as_str()
                );
                let map: std::collections::BTreeMap<String, String> = services
                    .store
                    .setting(&name)?
                    .map(|s| serde_json::from_str(&s).map_err(|_| String::from("适配器状态损坏")))
                    .transpose()?
                    .unwrap_or_default();
                Ok::<_, String>(map.get(&key).cloned())
            })
            .await
            .map_err(|_| "状态读取失败")??;
            self.check()?;
            Ok(value)
        })
    }
    fn state_set(
        &self,
        key: String,
        value: String,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + '_>> {
        Box::pin(async move {
            let context = self.check()?;
            if !self.services.permissions.state
                || key.is_empty()
                || key.len() > 64
                || value.len() > 8192
                || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return Err("permissionDenied".into());
            }
            let services = self.services.clone();
            let active = self.active.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let _guard = services.state_lock.lock().map_err(|_| "状态锁不可用")?;
                let check = || -> AppResult<()> {
                    if !active.load(Ordering::Acquire) {
                        return Err("staleContext".into());
                    }
                    let manager = services.adapters.upgrade().ok_or("sourceUnavailable")?;
                    let (session, generation) = manager
                        .stamp(&context.session.source)
                        .map_err(|e| e.to_string())?;
                    context
                        .check(&session, generation)
                        .map_err(|e| e.to_string())
                };
                check()?;
                let name = format!(
                    "musicAdapterStateV1:{}",
                    services.descriptor.source.as_str()
                );
                let previous = services.store.setting(&name)?;
                let mut map: std::collections::BTreeMap<String, String> = previous
                    .clone()
                    .map(|s| serde_json::from_str(&s).map_err(|_| String::from("适配器状态损坏")))
                    .transpose()?
                    .unwrap_or_default();
                map.insert(key, value);
                let value = serde_json::to_string(&map).map_err(|_| "状态编码失败")?;
                if map.len() > 16 || value.len() > 65536 {
                    return Err("适配器状态超过额度".into());
                }
                services.store.set_setting(&name, &value)?;
                if let Err(error) = check() {
                    services
                        .store
                        .set_setting(&name, previous.as_deref().unwrap_or("{}"))?;
                    return Err(error);
                }
                Ok(())
            })
            .await
            .map_err(|_| "状态保存失败")??;
            self.check()?;
            Ok(())
        })
    }
}
