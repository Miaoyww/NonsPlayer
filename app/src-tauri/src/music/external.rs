//! Backend-only v1 adapter bridge. Reuses the Component transport, never PluginManager.
use super::{adapter::*, business::*, identity::*, resource::*, ErrorCode, MusicResult};
use crate::{
    model::AppResult,
    wasm_runtime::{self as runtime, HostHandler, Runtime},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, io::Read, path::Path, pin::Pin, sync::Arc};
use wasmtime::{component::Component, Engine};
pub(crate) mod host;
mod sessions;

// Held inside the blocking closure, including when the loading future is dropped.
static COMPILATIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
struct NoCapabilities;
impl HostHandler for NoCapabilities {
    fn call<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<String>> + Send + 'a>> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
}

#[derive(Deserialize)]
#[serde(
    tag = "status",
    content = "data",
    rename_all = "camelCase",
    deny_unknown_fields
)]
enum Reply<T> {
    Ok(T),
    Error(super::MusicError),
}
// Transport DTO only: URLs never acquire Serialize on the domain resource type.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireResource {
    metadata: ResourceMetadata,
    url: String,
    headers: Vec<(String, String)>,
}

pub struct ExternalAdapter {
    descriptor: SourceDescriptor,
    engine: Engine,
    component: Component,
    reads: Arc<tokio::sync::Semaphore>,
    playback: tokio::sync::Semaphore,
    services: Option<Arc<host::Services>>,
    sessions: Arc<sessions::Sessions>,
}
impl ExternalAdapter {
    /// Explicit backend loading only. No auto-discovery, frontend, WASI or credential access.
    pub async fn load(path: &Path, expected: SourceDescriptor) -> MusicResult<Self> {
        Self::load_inner(path, expected, None).await
    }
    pub(crate) async fn load_with_host(
        path: &Path,
        services: Arc<host::Services>,
    ) -> MusicResult<Self> {
        Self::load_inner(path, services.descriptor.clone(), Some(services)).await
    }
    async fn load_inner(
        path: &Path,
        expected: SourceDescriptor,
        services: Option<Arc<host::Services>>,
    ) -> MusicResult<Self> {
        expected.validate()?;
        // The initial external ABI has no login, lyrics or download methods.
        if services.is_none()
            && expected
                .capabilities
                .iter()
                .any(|c| !matches!(c, Capability::Search | Capability::Browse))
        {
            return Err(ErrorCode::Unsupported.into());
        }
        let permit = COMPILATIONS
            .try_acquire()
            .map_err(|_| ErrorCode::RateLimited)?;
        let engine = runtime::engine().map_err(|error| {
            log::error!(target: "nons::adapter", "runtime_failure reason={error}");
            ErrorCode::Internal
        })?;
        let compile_engine = engine.clone();
        let path = path.to_owned();
        let component = tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            let mut bytes = Vec::new();
            std::fs::File::open(path)
                .map_err(|_| ErrorCode::InvalidData)?
                .take(runtime::Budget::ADAPTER.component_bytes as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| ErrorCode::InvalidData)?;
            if bytes.len() > runtime::Budget::ADAPTER.component_bytes {
                return Err(ErrorCode::InvalidData);
            }
            Component::new(&compile_engine, bytes).map_err(|_| ErrorCode::InvalidData)
        })
        .await
        .map_err(|_| ErrorCode::Internal)??;
        let adapter = Self {
            descriptor: expected,
            engine,
            component,
            reads: Arc::new(tokio::sync::Semaphore::new(4)),
            playback: tokio::sync::Semaphore::new(2),
            services,
            sessions: Arc::new(sessions::Sessions::default()),
        };
        let actual: SourceDescriptor = adapter
            .invoke("music.descriptor", "null", false, None)
            .await?;
        actual.validate()?;
        if serde_json::to_value(actual).map_err(|_| ErrorCode::InvalidData)?
            != serde_json::to_value(&adapter.descriptor).map_err(|_| ErrorCode::InvalidData)?
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(adapter)
    }

    async fn invoke<T: DeserializeOwned>(
        &self,
        method: &str,
        args: &str,
        playback: bool,
        context: Option<&RequestContext>,
    ) -> MusicResult<T> {
        let budget = if playback {
            &self.playback
        } else {
            self.reads.as_ref()
        };
        let _permit = budget.try_acquire().map_err(|_| ErrorCode::RateLimited)?;
        // Each request owns a fresh Store. Cancellation drops it, so no poisoned instance,
        // credentials, guest state or suspended execution survive into another session.
        // Compilation is shared; initialization and execution remain in the manager deadline.
        let handler: Arc<dyn HostHandler> = match (&self.services, context) {
            (Some(services), Some(context)) => {
                host::Context::new(services.clone(), context.clone(), false)
            }
            (Some(services), None) => host::Context::new(
                services.clone(),
                RequestContext {
                    session: services.accounts.session(&services.descriptor.source),
                    adapter_generation: 1,
                    deadline: std::time::Instant::now() + std::time::Duration::from_secs(15),
                    cancellation: Default::default(),
                },
                false,
            ),
            _ => Arc::new(NoCapabilities),
        };
        let mut runtime = Runtime::from_component_with_budget(
            self.engine.clone(),
            &self.component,
            handler,
            runtime::Budget::ADAPTER,
        )
        .await
        .map_err(|error| {
            log::error!(target: "nons::adapter", "runtime_failure reason={error}");
            ErrorCode::Internal
        })?;
        let response = runtime.call(method, args).await.map_err(|error| {
            log::error!(target: "nons::adapter", "runtime_failure reason={error}");
            ErrorCode::Internal
        })?;
        match serde_json::from_str::<Reply<T>>(&response).map_err(|_| ErrorCode::InvalidData)? {
            Reply::Ok(value) => Ok(value),
            Reply::Error(error) => Err(error),
        }
    }

    fn arguments(request: &impl Serialize, context: &RequestContext) -> MusicResult<String> {
        if context.cancellation.is_cancelled() {
            return Err(ErrorCode::Cancelled.into());
        }
        if std::time::Instant::now() >= context.deadline {
            return Err(ErrorCode::DeadlineExceeded.into());
        }
        serde_json::to_string(&serde_json::json!({
            "request": request,
            "session": context.session,
            "adapterGeneration": context.adapter_generation,
        }))
        .map_err(|_| ErrorCode::InvalidData.into())
    }
}
impl MusicAdapter for ExternalAdapter {
    fn cancel_login(&self, key: &str) {
        self.sessions.cancel(key);
    }
    fn revoke(&self) {
        self.sessions.revoke();
    }
    fn account<'a>(
        &'a self,
        request: &'a super::business::AccountRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, super::business::AccountOutcome> {
        Box::pin(async move { self.account_request(request, context).await })
    }
    fn descriptor(&self) -> &SourceDescriptor {
        &self.descriptor
    }
    fn read_track<'a>(
        &'a self,
        reference: &'a EntityRef,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, MusicTrack> {
        Box::pin(async move {
            context.check_reference(reference)?;
            self.invoke(
                "music.read-track",
                &Self::arguments(reference, context)?,
                false,
                Some(context),
            )
            .await
        })
    }
    fn business<'a>(
        &'a self,
        request: &'a BusinessRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, BusinessResponse> {
        Box::pin(async move {
            self.invoke(
                "music.business",
                &Self::arguments(request, context)?,
                false,
                Some(context),
            )
            .await
        })
    }
    fn resolve_playback<'a>(
        &'a self,
        request: &'a ResolveRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async move {
            request.validate(context)?;
            let wire: WireResource = self
                .invoke(
                    "music.resolve-playback",
                    &Self::arguments(request, context)?,
                    true,
                    Some(context),
                )
                .await?;
            if let Some(services) = &self.services {
                crate::infrastructure::http::checked_url(
                    &services.permissions.http_hosts,
                    &wire.url,
                )
                .map_err(|_| ErrorCode::PermissionDenied)?;
            }
            Ok(PlaybackResource {
                metadata: wire.metadata,
                access: ResourceAccess::Http(HttpAccess {
                    url: wire.url,
                    headers: wire.headers,
                }),
            })
        })
    }
}

#[cfg(test)]
mod acceptance;
#[cfg(test)]
mod tests;
