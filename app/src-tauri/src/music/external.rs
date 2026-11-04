//! Backend-only v1 adapter bridge. Reuses the Component transport, never PluginManager.
use super::{
    adapter::*, business::*, identity::*, lyric::*, mv::*, resource::*, ErrorCode, MusicResult,
};
use crate::{
    model::AppResult,
    wasm_runtime::{self as runtime, HostHandler, Runtime},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, path::Path, pin::Pin, sync::Arc};
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
    #[serde(default)]
    backup_urls: Vec<String>,
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
        Self::load_inner(path, expected, None, None).await
    }
    pub(crate) async fn load_with_host(
        path: &Path,
        services: Arc<host::Services>,
    ) -> MusicResult<Self> {
        Self::load_inner(path, services.descriptor.clone(), Some(services), None).await
    }
    async fn load_inner(
        path: &Path,
        expected: SourceDescriptor,
        services: Option<Arc<host::Services>>,
        resource_descriptor: Option<&super::provider::ResourceProviderDescriptor>,
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
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("adapter.compile_admission", error);
            })
            .map_err(|_| ErrorCode::RateLimited)?;
        let engine_started = std::time::Instant::now();
        let engine = runtime::engine().map_err(|error| {
            log::error!(target: "nons::adapter", "runtime_failure reason={error}");
            ErrorCode::Internal
        })?;
        log::info!(target: "nons::adapter", "source={} stage=create_engine elapsed_ms={}", expected.source.as_str(), engine_started.elapsed().as_millis());
        let compile_engine = engine.clone();
        let path = path.to_owned();
        let compile_source = expected.source.clone();
        let component = tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            let read_started = std::time::Instant::now();
            let bytes = std::fs::read(&path)
                .inspect_err(|error| { crate::infrastructure::diagnostics::warn("adapter.component_read", error); }).map_err(|_| ErrorCode::InvalidData)?;
            log::info!(target: "nons::adapter", "source={} stage=read_component bytes={} elapsed_ms={}", compile_source.as_str(), bytes.len(), read_started.elapsed().as_millis());
            runtime::compile_component(&compile_engine, &bytes).map(|compiled| {
                log::info!(target: "nons::adapter", "source={} stage=compile_component cache={} elapsed_ms={}", compile_source.as_str(), compiled.cache, compiled.elapsed.as_millis());
                compiled.component
            }).map_err(|error| {
                log::error!(target: "nons::adapter", "stage=compile_component reason={error:#}");
                ErrorCode::InvalidData
            })
        })
        .await
        .inspect_err(|error| { crate::infrastructure::diagnostics::warn("adapter.compile_join", error); }).map_err(|_| ErrorCode::Internal)??;
        let adapter = Self {
            descriptor: expected,
            engine,
            component,
            reads: Arc::new(tokio::sync::Semaphore::new(4)),
            playback: tokio::sync::Semaphore::new(2),
            services,
            sessions: Arc::new(sessions::Sessions::default()),
        };
        let initialize_started = std::time::Instant::now();
        let (actual, expected) = if let Some(descriptor) = resource_descriptor {
            let actual: super::provider::ResourceProviderDescriptor = adapter
                .invoke("music.resource-descriptor", "null", false, None)
                .await?;
            actual.validate()?;
            (
                serde_json::to_value(actual),
                serde_json::to_value(descriptor),
            )
        } else {
            let actual: SourceDescriptor = adapter
                .invoke("music.descriptor", "null", false, None)
                .await?;
            actual.validate()?;
            (
                serde_json::to_value(actual),
                serde_json::to_value(&adapter.descriptor),
            )
        };
        log::info!(target: "nons::adapter", "source={} stage=initialize elapsed_ms={}", adapter.descriptor.source.as_str(), initialize_started.elapsed().as_millis());
        if actual
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("adapter.descriptor_actual_encode", error);
            })
            .map_err(|_| ErrorCode::InvalidData)?
            != expected
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "adapter.descriptor_expected_encode",
                        error,
                    );
                })
                .map_err(|_| ErrorCode::InvalidData)?
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
        let _permit = budget
            .try_acquire()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("adapter.invoke_admission", error);
            })
            .map_err(|_| ErrorCode::RateLimited)?;
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
        let mut runtime = Runtime::from_component(self.engine.clone(), &self.component, handler)
            .await
            .map_err(|error| {
                log::error!(target: "nons::adapter", "runtime_failure reason={error}");
                ErrorCode::Internal
            })?;
        let response = runtime.call(method, args).await.map_err(|error| {
            log::error!(target: "nons::adapter", "runtime_failure reason={error}");
            ErrorCode::Internal
        })?;
        match serde_json::from_str::<Reply<T>>(&response)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("adapter.reply_decode", error);
            })
            .map_err(|_| ErrorCode::InvalidData)?
        {
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
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("adapter.arguments_encode", error);
        })
        .map_err(|_| ErrorCode::InvalidData.into())
    }

    fn wire_resource(&self, wire: WireResource, operation: &str) -> MusicResult<PlaybackResource> {
        let mut urls =
            std::iter::once(wire.url.as_str()).chain(wire.backup_urls.iter().map(String::as_str));
        if let Some(services) = &self.services {
            for url in urls.by_ref() {
                crate::infrastructure::http::checked_playback_url(
                    &services.permissions.http_hosts,
                    url,
                )
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(operation, error);
                })
                .map_err(|_| ErrorCode::PermissionDenied)?;
            }
        }
        let primary = HttpAccess {
            url: wire.url,
            headers: wire.headers,
        };
        let access = if wire.backup_urls.is_empty() {
            ResourceAccess::Http(primary)
        } else {
            ResourceAccess::HttpFallback {
                primary,
                backup_urls: wire.backup_urls,
            }
        };
        Ok(PlaybackResource {
            metadata: wire.metadata,
            access,
        })
    }
}

/// Uses the shared Component executor and Host, without registering a music platform.
pub(crate) struct ResourceComponent {
    runtime: ExternalAdapter,
    descriptor: super::provider::ResourceProviderDescriptor,
}
impl ResourceComponent {
    pub(crate) async fn load(
        path: &Path,
        descriptor: super::provider::ResourceProviderDescriptor,
        services: Arc<host::Services>,
    ) -> MusicResult<Self> {
        descriptor.validate()?;
        if services.permissions.credentials
            || services.permissions.state
            || !services.permissions.websocket_hosts.is_empty()
        {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let runtime = ExternalAdapter::load_inner(
            path,
            services.descriptor.clone(),
            Some(services),
            Some(&descriptor),
        )
        .await?;
        Ok(Self {
            runtime,
            descriptor,
        })
    }
}
impl super::provider::ResourceProvider for ResourceComponent {
    fn descriptor(&self) -> &super::provider::ResourceProviderDescriptor {
        &self.descriptor
    }
    fn resolve<'a>(
        &'a self,
        request: &'a super::provider::ResourceRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async move {
            request.validate(&self.descriptor)?;
            let wire: WireResource = self
                .runtime
                .invoke(
                    "music.resolve-resource",
                    &ExternalAdapter::arguments(request, context)?,
                    true,
                    Some(context),
                )
                .await?;
            self.runtime
                .wire_resource(wire, "adapter.resource_url_permissions")
        })
    }
    fn search<'a>(
        &'a self,
        request: &'a super::provider::ResourceSearchRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, Page<MusicTrack>> {
        Box::pin(async move {
            request.validate(&self.descriptor)?;
            self.runtime
                .invoke(
                    "music.search-resources",
                    &ExternalAdapter::arguments(request, context)?,
                    false,
                    Some(context),
                )
                .await
        })
    }
}
impl MusicAdapter for ExternalAdapter {
    fn resolve_resource<'a>(
        &'a self,
        request: &'a super::provider::ResourceRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async move {
            if !self
                .descriptor
                .supports_resource_platform(&request.track.reference.source)
            {
                return Err(ErrorCode::Unsupported.into());
            }
            if request.track.reference.source == self.descriptor.source {
                return self
                    .resolve_playback(
                        &ResolveRequest {
                            track: request.track.reference.clone(),
                            preferred_quality: request.preferred_quality.clone(),
                            allow_downgrade: request.allow_downgrade,
                        },
                        context,
                    )
                    .await;
            }
            let wire: WireResource = self
                .invoke(
                    "music.resolve-resource",
                    &Self::arguments(request, context)?,
                    true,
                    Some(context),
                )
                .await?;
            self.wire_resource(wire, "adapter.alternate_resource_url_permissions")
        })
    }
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
    fn search_mvs<'a>(
        &'a self,
        request: &'a MvSearchRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, Page<MvCandidate>> {
        Box::pin(async move {
            request.validate()?;
            let page: Page<MvCandidate> = self
                .invoke(
                    "music.search-mvs",
                    &Self::arguments(request, context)?,
                    false,
                    Some(context),
                )
                .await?;
            page.validate(&request.page)?;
            for candidate in &page.items {
                candidate.validate(&self.descriptor.source)?;
                candidate
                    .cover
                    .validate_hosts(&self.descriptor.cover_hosts)?;
            }
            Ok(page)
        })
    }
    fn lyric_match<'a>(
        &'a self,
        request: &'a LyricMatchRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, LyricMatchSubject> {
        Box::pin(async move {
            request.validate(&request.track.reference)?;
            if request.track.reference.source != self.descriptor.source {
                return Err(ErrorCode::Unsupported.into());
            }
            let subject: LyricMatchSubject = self
                .invoke(
                    "music.lyric-match",
                    &Self::arguments(request, context)?,
                    false,
                    Some(context),
                )
                .await?;
            subject.validate()?;
            Ok(subject)
        })
    }
    fn resolve_mv<'a>(
        &'a self,
        request: &'a MvResolveRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async move {
            request.validate(context)?;
            let wire: WireResource = self
                .invoke(
                    "music.resolve-mv",
                    &Self::arguments(request, context)?,
                    true,
                    Some(context),
                )
                .await?;
            self.wire_resource(wire, "adapter.mv_url_permissions")
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
            self.wire_resource(wire, "adapter.playback_url_permissions")
        })
    }
}

#[cfg(test)]
mod acceptance;
#[cfg(test)]
mod tests;
