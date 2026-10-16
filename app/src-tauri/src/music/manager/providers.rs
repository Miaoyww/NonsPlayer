//! Audio provider registry, independent of catalog/AccountManager registration.
use super::*;
use crate::music::provider::*;
use serde::Serialize;
#[cfg(test)]
mod tests;

pub(super) struct ProviderEntry {
    pub adapter: Arc<dyn ResourceProvider>,
    pub generation: Arc<AtomicU64>,
    catalog: Option<Arc<Entry>>,
    reads: Semaphore,
    playback: Semaphore,
}
impl ProviderEntry {
    fn session(&self) -> SessionContext {
        if let Some(catalog) = &self.catalog {
            return (catalog.session)();
        }
        SessionContext {
            source: self.adapter.descriptor().id.clone(),
            account: None,
            generation: 0,
        }
    }
    fn check(&self, context: &RequestContext) -> MusicResult<()> {
        let generation = self.generation.load(Ordering::Acquire);
        if generation.is_multiple_of(2) {
            return Err(ErrorCode::SourceUnavailable.into());
        }
        context.check(&self.session(), generation)
    }
}
pub(super) struct ProviderLease {
    entry: Arc<ProviderEntry>,
    origin: Option<(Arc<Entry>, RequestContext)>,
}
impl ProviderLease {
    pub(super) fn check(&self, lease: &ResourceLease, generation: u64) -> MusicResult<()> {
        if self
            .entry
            .generation
            .load(Ordering::Acquire)
            .is_multiple_of(2)
        {
            return Err(ErrorCode::SourceUnavailable.into());
        }
        if let Some((entry, context)) = &self.origin {
            // Resolution deadline does not expire already prepared resources.
            if entry.generation.load(Ordering::Acquire).is_multiple_of(2)
                || (entry.session)() != context.session
                || entry.generation.load(Ordering::Acquire) != context.adapter_generation
            {
                return Err(ErrorCode::StaleContext.into());
            }
        }
        lease.check(
            &self.entry.session(),
            self.entry.generation.load(Ordering::Acquire),
            generation,
        )
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProviderStatus {
    pub descriptor: ResourceProviderDescriptor,
    pub enabled: bool,
    pub loading: bool,
    pub load_error: Option<String>,
}

/// Present a catalog adapter's audio capability through the same resolver seam.
/// Keep its own account, instance generation and admission budget.
struct CatalogAudio {
    entry: Arc<Entry>,
    descriptor: ResourceProviderDescriptor,
}
impl ResourceProvider for CatalogAudio {
    fn descriptor(&self) -> &ResourceProviderDescriptor {
        &self.descriptor
    }
    fn resolve<'a>(
        &'a self,
        request: &'a ResourceRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async move {
            let _permit = self
                .entry
                .playback
                .try_acquire()
                .map_err(|_| ErrorCode::RateLimited)?;
            crate::infrastructure::diagnostics::observe(
                self.descriptor.id.as_str(),
                if request.track.reference.source == self.descriptor.id {
                    "resolvePlayback"
                } else {
                    "resolveResource"
                },
                self.entry.adapter.resolve_resource(request, context),
            )
            .await
        })
    }
}

impl AdapterManager {
    fn catalog_audio(entry: &Arc<Entry>) -> Arc<ProviderEntry> {
        let descriptor = entry.adapter.descriptor();
        Arc::new(ProviderEntry {
            adapter: Arc::new(CatalogAudio {
                entry: entry.clone(),
                descriptor: ResourceProviderDescriptor {
                    id: descriptor.source.clone(),
                    display_name: descriptor.display_name.clone(),
                    description: descriptor.description.clone(),
                    contract_version: descriptor.contract_version,
                    platforms: descriptor
                        .platforms
                        .iter()
                        .cloned()
                        .chain(std::iter::once(descriptor.source.clone()))
                        .collect(),
                    search_platforms: Default::default(),
                    cover_hosts: descriptor.cover_hosts.clone(),
                },
            }),
            generation: entry.generation.clone(),
            catalog: Some(entry.clone()),
            reads: Semaphore::new(4),
            playback: Semaphore::new(2),
        })
    }

    fn audio_candidates(&self, platform: &SourceId) -> MusicResult<Vec<Arc<ProviderEntry>>> {
        let mut candidates: Vec<_> = self
            .providers
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .values()
            .filter(|entry| {
                !entry.generation.load(Ordering::Acquire).is_multiple_of(2)
                    && entry.adapter.descriptor().platforms.contains(platform)
            })
            .cloned()
            .collect();
        for entry in self
            .entries
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .values()
        {
            let descriptor = entry.adapter.descriptor();
            if &descriptor.source == platform
                || entry.generation.load(Ordering::Acquire).is_multiple_of(2)
                || !descriptor.supports_resource_platform(platform)
            {
                continue;
            }
            candidates.push(Self::catalog_audio(entry));
        }
        candidates.sort_by(|left, right| {
            left.adapter
                .descriptor()
                .id
                .cmp(&right.adapter.descriptor().id)
        });
        Ok(candidates)
    }

    pub fn supports_audio_provider(&self, id: &str, platform: &SourceId) -> MusicResult<bool> {
        Ok(self
            .audio_candidates(platform)?
            .iter()
            .any(|entry| entry.adapter.descriptor().id.as_str() == id))
    }
    pub(crate) fn begin_resource_loading(
        &self,
        descriptor: ResourceProviderDescriptor,
    ) -> MusicResult<()> {
        descriptor.validate()?;
        self.provider_loading
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .insert(
                descriptor.id.clone(),
                ResourceProviderStatus {
                    descriptor,
                    enabled: false,
                    loading: true,
                    load_error: None,
                },
            );
        Ok(())
    }
    pub(crate) fn resource_loading_failed(&self, id: &SourceId, error: &str) {
        if let Ok(mut pending) = self.provider_loading.lock() {
            if let Some(status) = pending.get_mut(id) {
                status.loading = false;
                status.load_error = Some(crate::infrastructure::diagnostics::redact(error));
            }
        }
    }
    pub fn register_resource_provider(
        &self,
        adapter: Arc<dyn ResourceProvider>,
    ) -> MusicResult<()> {
        self.register_resource_provider_with_enabled(adapter, true)
    }
    pub(crate) fn register_resource_provider_with_enabled(
        &self,
        adapter: Arc<dyn ResourceProvider>,
        enabled: bool,
    ) -> MusicResult<()> {
        adapter.descriptor().validate()?;
        let id = adapter.descriptor().id.clone();
        let entries = self.entries.lock().map_err(|_| ErrorCode::Internal)?;
        let mut providers = self.providers.lock().map_err(|_| ErrorCode::Internal)?;
        if entries.contains_key(&id) || providers.contains_key(&id) || providers.len() >= 16 {
            return Err(ErrorCode::InvalidData.into());
        }
        providers.insert(
            id.clone(),
            Arc::new(ProviderEntry {
                adapter,
                generation: Arc::new(AtomicU64::new(if enabled { 1 } else { 2 })),
                catalog: None,
                reads: Semaphore::new(4),
                playback: Semaphore::new(2),
            }),
        );
        self.provider_loading
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .remove(&id);
        Ok(())
    }
    pub fn resource_provider_statuses(&self) -> MusicResult<Vec<ResourceProviderStatus>> {
        let mut statuses = self
            .providers
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .values()
            .map(|entry| ResourceProviderStatus {
                descriptor: entry.adapter.descriptor().clone(),
                enabled: !entry.generation.load(Ordering::Acquire).is_multiple_of(2),
                loading: false,
                load_error: None,
            })
            .collect::<Vec<_>>();
        for status in self
            .provider_loading
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .values()
        {
            if !statuses
                .iter()
                .any(|s| s.descriptor.id == status.descriptor.id)
            {
                statuses.push(status.clone());
            }
        }
        Ok(statuses)
    }
    pub fn set_resource_provider_enabled(&self, id: &SourceId, enabled: bool) -> MusicResult<()> {
        let entry = self
            .providers
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .get(id)
            .cloned()
            .ok_or(ErrorCode::NotFound)?;
        entry
            .generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current.is_multiple_of(2) == enabled).then_some(current + 1)
            })
            .ok();
        if !enabled {
            entry.adapter.revoke();
        }
        Ok(())
    }
    pub(super) fn provider_stamp(
        &self,
        id: &SourceId,
    ) -> MusicResult<Option<(SessionContext, u64)>> {
        let entry = self
            .providers
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .get(id)
            .cloned();
        entry
            .map(|e| {
                let generation = e.generation.load(Ordering::Acquire);
                if generation.is_multiple_of(2) {
                    return Err(ErrorCode::SourceUnavailable.into());
                }
                Ok((e.session(), generation))
            })
            .transpose()
    }
    async fn run_provider<T>(
        &self,
        entry: &ProviderEntry,
        context: &RequestContext,
        origin: Option<&(Arc<Entry>, RequestContext)>,
        future: impl Future<Output = MusicResult<T>>,
    ) -> MusicResult<T> {
        tokio::pin!(future);
        let deadline = tokio::time::sleep_until(context.deadline.into());
        tokio::pin!(deadline);
        let mut checks = tokio::time::interval(Duration::from_millis(10));
        loop {
            entry.check(context)?;
            if let Some((owner, scope)) = origin {
                owner.check(scope)?;
            }
            tokio::select! {
                biased;
                _ = &mut deadline => return Err(ErrorCode::DeadlineExceeded.into()),
                _ = checks.tick() => {},
                result = &mut future => {
                    entry.check(context)?;
                    if let Some((owner, scope)) = origin { owner.check(scope)?; }
                    return result;
                }
            }
        }
    }
    pub async fn search_resource_provider(
        &self,
        id: &SourceId,
        request: &ResourceSearchRequest,
    ) -> MusicResult<Page<MusicTrack>> {
        let entry = self
            .providers
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .get(id)
            .cloned()
            .ok_or(ErrorCode::NotFound)?;
        request.validate(entry.adapter.descriptor())?;
        let _permit = entry
            .reads
            .try_acquire()
            .map_err(|_| ErrorCode::RateLimited)?;
        let context = RequestContext {
            session: entry.session(),
            adapter_generation: entry.generation.load(Ordering::Acquire),
            deadline: Instant::now() + Duration::from_secs(12),
            cancellation: Default::default(),
        };
        let scope = serde_json::to_string(&(
            id,
            context.adapter_generation,
            &request.platform,
            &request.keyword,
            request.page.limit,
        ))
        .map_err(|_| ErrorCode::Internal)?;
        let mut inner = request.clone();
        if let Some(cursor) = &request.page.cursor {
            let (binding, raw): (String, String) =
                serde_json::from_str(cursor.as_str()).map_err(|_| ErrorCode::InvalidData)?;
            if binding != scope {
                return Err(ErrorCode::StaleContext.into());
            }
            inner.page.cursor = Some(Cursor::try_from(raw)?);
        }
        let mut page = self
            .run_provider(
                &entry,
                &context,
                None,
                entry.adapter.search(&inner, &context),
            )
            .await?;
        page.validate(&request.page)?;
        for track in &page.items {
            if track.reference.source != request.platform {
                return Err(ErrorCode::InvalidData.into());
            }
            track.validate(&track.reference)?;
            track
                .cover
                .validate_hosts(&entry.adapter.descriptor().cover_hosts)?;
        }
        encode_cursor(&mut page.next_cursor, &scope)?;
        // Account for the cursor envelope as well as the guest's page.
        page.validate(&request.page)?;
        Ok(page)
    }
    /// Explicit selection first, otherwise catalog first. Every operational failure
    /// advances once; cancellation/revocation never starts another network request.
    pub async fn resolve_with_providers(
        &self,
        request: &ResourceRequest,
        selected: Option<&str>,
        generation: u64,
    ) -> MusicResult<(Arc<ResourceTicket>, PlaybackSource)> {
        self.resolve_with_providers_notifying(request, selected, generation, &|_| {})
            .await
    }
    pub async fn resolve_with_providers_notifying(
        &self,
        request: &ResourceRequest,
        selected: Option<&str>,
        generation: u64,
        notify: &(dyn Fn(PlaybackSource) + Send + Sync),
    ) -> MusicResult<(Arc<ResourceTicket>, PlaybackSource)> {
        let platform = &request.track.reference.source;
        request.track.validate(&request.track.reference)?;
        let deadline = Instant::now() + Duration::from_secs(24);
        let origin = match self.entry(platform) {
            Ok(entry) => {
                let context = Self::context(&entry, Duration::from_secs(24));
                entry.check(&context)?;
                Some((entry, context))
            }
            Err(error) => {
                log::info!(target: "nons::music", "operation=resolve_origin source={} code={:?}; trying resource providers", platform.as_str(), error.code);
                None
            }
        };
        let providers = self.audio_candidates(platform)?;
        if selected.is_some_and(|id| {
            id != "official"
                && !providers
                    .iter()
                    .any(|p| p.adapter.descriptor().id.as_str() == id)
        }) {
            return Err(ErrorCode::SourceUnavailable.into());
        }
        let mut attempts = Vec::new();
        if let Some(id) = selected.filter(|id| *id != "official") {
            attempts.push(Some(id.to_owned()));
        }
        if origin.is_some() {
            attempts.push(None);
        }
        for p in &providers {
            let id = p.adapter.descriptor().id.as_str();
            if selected != Some(id) {
                attempts.push(Some(id.to_owned()));
            }
        }
        let mut last = ErrorCode::SourceUnavailable.into();
        for (attempt, id) in attempts.into_iter().enumerate() {
            if Instant::now() >= deadline {
                return Err(ErrorCode::DeadlineExceeded.into());
            }
            if let Some((entry, scope)) = &origin {
                entry.check(scope)?;
            }
            let display_name = id
                .as_ref()
                .and_then(|id| {
                    providers
                        .iter()
                        .find(|p| p.adapter.descriptor().id.as_str() == id)
                })
                .map(|p| p.adapter.descriptor().display_name.clone())
                .or_else(|| {
                    origin
                        .as_ref()
                        .map(|(e, _)| e.adapter.descriptor().display_name.clone())
                })
                .unwrap_or_default();
            let source = PlaybackSource {
                provider: id.clone().unwrap_or_else(|| "official".into()),
                display_name,
                platform: platform.clone(),
                changed: attempt > 0 || id.is_some(),
                track: request.track.reference.clone(),
                title: request.track.title.clone(),
                artist: request.track.artist.clone(),
            };
            notify(source.clone());
            let result = if let Some(id) = &id {
                let entry = providers
                    .iter()
                    .find(|p| p.adapter.descriptor().id.as_str() == id)
                    .ok_or(ErrorCode::SourceUnavailable)?
                    .clone();
                self.resolve_provider(entry, request, origin.clone(), deadline, generation)
                    .await
            } else {
                let entry = &origin.as_ref().ok_or(ErrorCode::SourceUnavailable)?.0;
                self.resolve_provider(
                    Self::catalog_audio(entry),
                    request,
                    origin.clone(),
                    deadline,
                    generation,
                )
                .await
            };
            if let Some(id) = &id {
                if providers.iter().any(|p| {
                    p.adapter.descriptor().id.as_str() == id
                        && p.generation.load(Ordering::Acquire).is_multiple_of(2)
                }) {
                    return Err(ErrorCode::SourceUnavailable.into());
                }
            }
            match result {
                Ok(ticket) => {
                    return Ok((ticket, source));
                }
                Err(error)
                    if matches!(error.code, ErrorCode::Cancelled | ErrorCode::StaleContext) =>
                {
                    return Err(error)
                }
                Err(error) => last = error,
            }
        }
        Err(last)
    }
    async fn resolve_provider(
        &self,
        entry: Arc<ProviderEntry>,
        request: &ResourceRequest,
        origin: Option<(Arc<Entry>, RequestContext)>,
        deadline: Instant,
        generation: u64,
    ) -> MusicResult<Arc<ResourceTicket>> {
        request.validate(entry.adapter.descriptor())?;
        let _permit = entry
            .playback
            .try_acquire()
            .map_err(|_| ErrorCode::RateLimited)?;
        let context = RequestContext {
            session: entry.session(),
            adapter_generation: entry.generation.load(Ordering::Acquire),
            deadline: deadline.min(Instant::now() + Duration::from_secs(12)),
            cancellation: Default::default(),
        };
        let resource = self
            .run_provider(
                &entry,
                &context,
                origin.as_ref(),
                entry.adapter.resolve(request, &context),
            )
            .await?;
        resource.metadata.require_full(now_ms())?;
        validate_access(&resource.access)?;
        let ttl = resource
            .metadata
            .expires_at_ms
            .map(|expiry| expiry.saturating_sub(now_ms()))
            .unwrap_or(60_000)
            .min(60_000);
        if ttl == 0 {
            return Err(ErrorCode::DeadlineExceeded.into());
        }
        let mut resources = self.resources.lock().map_err(|_| ErrorCode::Internal)?;
        resources.retain(|_, v| v.strong_count() > 0);
        if resources.len() >= 16 {
            return Err(ErrorCode::RateLimited.into());
        }
        let handle = ResourceHandle::try_from(format!(
            "resource-{}",
            self.next_handle.fetch_add(1, Ordering::Relaxed)
        ))?;
        let ticket = Arc::new(ResourceTicket {
            entry: None,
            provider: Some(ProviderLease {
                entry: entry.clone(),
                origin,
            }),
            lease: ResourceLease {
                handle,
                context,
                playback_generation: generation,
                valid_until: Instant::now() + Duration::from_millis(ttl),
            },
            resource,
        });
        ticket.check(generation)?;
        resources.insert(ticket.lease.handle.as_str().into(), Arc::downgrade(&ticket));
        Ok(ticket)
    }
}
