//! Backend registry, admission budgets and revocable resource ownership.
use super::{
    account::SessionContext, adapter::*, identity::*, resource::*, ErrorCode, MusicResult,
};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, Weak,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
type SessionProvider = Arc<dyn Fn() -> SessionContext + Send + Sync>;
struct Entry {
    adapter: Arc<dyn MusicAdapter>,
    session: SessionProvider,
    generation: AtomicU64,
    reads: Semaphore,
    playback: Semaphore,
    accounts: Semaphore,
    account_waiters: Semaphore,
    login_challenges: Mutex<BTreeMap<String, (SessionContext, u64, Instant)>>,
    revision: AtomicU64,
}
impl Entry {
    fn check(&self, context: &RequestContext) -> MusicResult<()> {
        if self.generation.load(Ordering::Acquire).is_multiple_of(2) {
            return Err(ErrorCode::SourceUnavailable.into());
        }
        context.check(&(self.session)(), self.generation.load(Ordering::Acquire))
    }
}
#[derive(Default)]
pub struct AdapterManager {
    entries: Mutex<BTreeMap<SourceId, Arc<Entry>>>,
    resources: Mutex<BTreeMap<String, Weak<ResourceTicket>>>,
    next_handle: AtomicU64,
}
/// This ownership token is backend-only. Access is never serialized or logged.
pub struct ResourceTicket {
    entry: Arc<Entry>,
    lease: ResourceLease,
    resource: PlaybackResource,
}
impl std::fmt::Debug for ResourceTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceTicket([redacted])")
    }
}
impl ResourceTicket {
    /// Lightweight atomic checks are safe at the gapless handoff; no network/IO.
    pub fn check(&self, playback_generation: u64) -> MusicResult<()> {
        if self
            .entry
            .generation
            .load(Ordering::Acquire)
            .is_multiple_of(2)
        {
            return Err(ErrorCode::SourceUnavailable.into());
        }
        self.lease.check(
            &(self.entry.session)(),
            self.entry.generation.load(Ordering::Acquire),
            playback_generation,
        )
    }
    pub(crate) fn uri(&self) -> &str {
        match &self.resource.access {
            ResourceAccess::Http(http) => &http.url,
        }
    }
    pub(crate) fn quality(&self) -> &str {
        &self.resource.metadata.actual_quality
    }
}
impl AdapterManager {
    pub fn cancel_login(&self, source: &SourceId, key: &str) -> MusicResult<()> {
        let entry = self.entry(source)?;
        entry
            .login_challenges
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .remove(key);
        entry.adapter.cancel_login(key);
        Ok(())
    }
    pub fn statuses(&self) -> MusicResult<Vec<AdapterStatus>> {
        Ok(self
            .entries
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .values()
            .map(|entry| AdapterStatus {
                descriptor: entry.adapter.descriptor().clone(),
                enabled: !entry.generation.load(Ordering::Acquire).is_multiple_of(2),
            })
            .collect())
    }
    pub fn register(
        &self,
        adapter: Arc<dyn MusicAdapter>,
        session: SessionProvider,
    ) -> MusicResult<()> {
        adapter.descriptor().validate()?;
        let source = adapter.descriptor().source.clone();
        if (session)().source != source {
            return Err(ErrorCode::InvalidData.into());
        }
        let mut entries = self.entries.lock().map_err(|_| ErrorCode::Internal)?;
        if entries.contains_key(&source) {
            return Err(ErrorCode::InvalidData.into());
        }
        entries.insert(
            source,
            Arc::new(Entry {
                adapter,
                session,
                generation: AtomicU64::new(1),
                reads: Semaphore::new(4),
                playback: Semaphore::new(2),
                accounts: Semaphore::new(1),
                account_waiters: Semaphore::new(4),
                login_challenges: Default::default(),
                revision: AtomicU64::new(0),
            }),
        );
        Ok(())
    }
    pub fn set_enabled(&self, source: &SourceId, enabled: bool) -> MusicResult<()> {
        let entry = self.entry(source)?;
        entry
            .generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current.is_multiple_of(2) == enabled).then_some(current + 1)
            })
            .ok();
        if !enabled {
            entry.adapter.revoke();
            entry
                .login_challenges
                .lock()
                .map_err(|_| ErrorCode::Internal)?
                .clear();
        }
        Ok(())
    }
    /// Internal built-in reload; package loading is introduced with external ABI.
    pub fn reload(&self, source: &SourceId) -> MusicResult<()> {
        let entry = self.entry(source)?;
        entry.generation.fetch_add(2, Ordering::AcqRel);
        entry.adapter.revoke();
        entry
            .login_challenges
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .clear();
        Ok(())
    }
    fn entry(&self, source: &SourceId) -> MusicResult<Arc<Entry>> {
        self.entries
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .get(source)
            .cloned()
            .ok_or_else(|| ErrorCode::SourceUnavailable.into())
    }
    pub fn stamp(&self, source: &SourceId) -> MusicResult<(SessionContext, u64)> {
        let entry = self.entry(source)?;
        if entry.generation.load(Ordering::Acquire).is_multiple_of(2) {
            return Err(ErrorCode::SourceUnavailable.into());
        }
        Ok(((entry.session)(), entry.generation.load(Ordering::Acquire)))
    }
    fn context(entry: &Entry, budget: Duration) -> RequestContext {
        RequestContext {
            session: (entry.session)(),
            adapter_generation: entry.generation.load(Ordering::Acquire),
            deadline: Instant::now() + budget,
            cancellation: Cancellation::default(),
        }
    }
    async fn run<T>(
        &self,
        entry: &Entry,
        context: &RequestContext,
        future: impl Future<Output = MusicResult<T>>,
    ) -> MusicResult<T> {
        entry.check(context)?;
        tokio::pin!(future);
        let deadline = tokio::time::sleep_until(context.deadline.into());
        tokio::pin!(deadline);
        let mut checks = tokio::time::interval(Duration::from_millis(10));
        loop {
            tokio::select! {
                biased;
                _ = &mut deadline => return Err(ErrorCode::DeadlineExceeded.into()),
                _ = checks.tick() => entry.check(context)?,
                result = &mut future => { entry.check(context)?; return result; }
            }
        }
    }
    pub fn cache_identity(&self, source: &SourceId) -> MusicResult<String> {
        let (session, generation) = self.stamp(source)?;
        let entry = self.entry(source)?;
        serde_json::to_string(&(session, generation, entry.revision.load(Ordering::Acquire)))
            .map_err(|_| ErrorCode::Internal.into())
    }
    /// Authorization is checked even for public cache hits. Stamp changes on disable/reload.
    pub fn cover_access(&self, url: &str) -> MusicResult<String> {
        let entries = self.entries.lock().map_err(|_| ErrorCode::Internal)?;
        let mut owners = entries
            .values()
            .filter_map(|entry| {
                let generation = entry.generation.load(Ordering::Acquire);
                let descriptor = entry.adapter.descriptor();
                (!generation.is_multiple_of(2)
                    && nons_music_adapter_sdk::cover::checked_url(url, &descriptor.cover_hosts)
                        .is_ok())
                .then(|| (descriptor.source.clone(), generation))
            })
            .collect::<Vec<_>>();
        if owners.is_empty() {
            return Err(ErrorCode::PermissionDenied.into());
        }
        owners.sort();
        serde_json::to_string(&owners).map_err(|_| ErrorCode::InvalidData.into())
    }
    pub fn descriptors(&self) -> MusicResult<Vec<SourceDescriptor>> {
        Ok(self
            .entries
            .lock()
            .map_err(|_| ErrorCode::Internal)?
            .values()
            .map(|e| e.adapter.descriptor().clone())
            .collect())
    }
    pub async fn business(
        &self,
        source: &SourceId,
        request: &super::business::BusinessRequest,
    ) -> MusicResult<super::business::BusinessResponse> {
        use super::business::*;
        request.validate(source)?;
        let entry = self.entry(source)?;
        let context = Self::context(&entry, Duration::from_secs(12));
        entry.check(&context)?;
        if !entry
            .adapter
            .descriptor()
            .capabilities
            .contains(&request.capability())
        {
            return Err(ErrorCode::Unsupported.into());
        }
        if request.requires_account() && context.session.account.is_none() {
            return Err(ErrorCode::Unauthenticated.into());
        }
        let _permit = entry
            .reads
            .try_acquire()
            .map_err(|_| ErrorCode::RateLimited)?;
        let revision = entry.revision.load(Ordering::Acquire);
        let scope = format!("{}-{revision}", cursor_scope(request, &context)?);
        let internal = decode_cursor(request, &scope)?;
        let mut result = self
            .run(
                &entry,
                &context,
                entry.adapter.business(&internal, &context),
            )
            .await?;
        result.validate(request, source)?;
        result.validate_cover_hosts(&entry.adapter.descriptor().cover_hosts)?;
        if let BusinessResponse::Summary(summary) = &result {
            if context.session.account.as_ref() != Some(&summary.account.reference.id) {
                return Err(ErrorCode::InvalidData.into());
            }
        }
        if !request.is_write() && entry.revision.load(Ordering::Acquire) != revision {
            return Err(ErrorCode::StaleContext.into());
        }
        match &mut result {
            BusinessResponse::Tracks(p) => encode_cursor(&mut p.page.next_cursor, &scope)?,
            BusinessResponse::Entities(p) => encode_cursor(&mut p.next_cursor, &scope)?,
            _ => (),
        }
        entry.check(&context)?;
        if request.is_write() {
            entry.revision.fetch_add(1, Ordering::AcqRel);
        }
        Ok(result)
    }
    pub async fn account(
        &self,
        source: &SourceId,
        request: &super::business::AccountRequest,
        accounts: &Arc<super::accounts::AccountManager>,
    ) -> MusicResult<super::business::AccountPresentation> {
        use super::business::*;
        let entry = self.entry(source)?;
        if serde_json::to_vec(request)
            .map_err(|_| ErrorCode::InvalidData)?
            .len()
            > 16 * 1024
        {
            return Err(ErrorCode::InvalidData.into());
        }
        let deadline = Instant::now() + Duration::from_secs(12);
        let _waiter = entry
            .account_waiters
            .try_acquire()
            .map_err(|_| ErrorCode::RateLimited)?;
        let _permit = tokio::time::timeout_at(deadline.into(), entry.accounts.acquire())
            .await
            .map_err(|_| ErrorCode::DeadlineExceeded)?
            .map_err(|_| ErrorCode::Internal)?;
        let mut context = Self::context(&entry, Duration::from_secs(12));
        context.deadline = deadline;
        entry.check(&context)?;
        if !entry
            .adapter
            .descriptor()
            .capabilities
            .contains(&Capability::Account)
        {
            return Err(ErrorCode::Unsupported.into());
        }
        if let AccountRequest::PollLogin { key } = request {
            // A challenge is meaningful only within this account and adapter generation.
            let mut challenges = entry
                .login_challenges
                .lock()
                .map_err(|_| ErrorCode::Internal)?;
            challenges.retain(|key, (_, _, expires)| {
                let valid = Instant::now() < *expires;
                if !valid {
                    entry.adapter.cancel_login(key);
                }
                valid
            });
            let (session, generation, _) = challenges.get(key).ok_or(ErrorCode::StaleContext)?;
            if session != &context.session || *generation != context.adapter_generation {
                return Err(ErrorCode::StaleContext.into());
            }
        }
        if let AccountRequest::BeginLogin {
            method: Some(method),
        } = request
        {
            if !entry
                .adapter
                .descriptor()
                .login_methods
                .iter()
                .any(|candidate| &candidate.id == method)
            {
                return Err(ErrorCode::Unsupported.into());
            }
        }
        if matches!(request, AccountRequest::BeginLogin { .. }) {
            let mut challenges = entry
                .login_challenges
                .lock()
                .map_err(|_| ErrorCode::Internal)?;
            challenges.retain(|key, (_, _, expires)| {
                let valid = Instant::now() < *expires;
                if !valid {
                    entry.adapter.cancel_login(key);
                }
                valid
            });
            if challenges.len() >= 4 {
                return Err(ErrorCode::RateLimited.into());
            }
        }
        if matches!(request, AccountRequest::Logout) {
            // Deselect locally. A platform logout could revoke the saved credential,
            // preventing the user from selecting this account again.
            entry.check(&context)?;
            let worker = accounts.clone();
            let expected = context.session.clone();
            let local_error = worker
                .blocking(move |accounts| accounts.clear_expected(&expected))
                .await
                .err();
            let local_cleared = accounts.session(source) != context.session
                && accounts.session(source).account.is_none();
            if !local_cleared {
                return Ok(AccountPresentation::Logout(LogoutReport {
                    local_cleared,
                    local_error,
                    remote_error: None,
                }));
            }
            return Ok(AccountPresentation::Logout(LogoutReport {
                local_cleared,
                local_error,
                remote_error: None,
            }));
        }
        let outcome = self
            .run(&entry, &context, entry.adapter.account(request, &context))
            .await?;
        if serde_json::to_vec(&outcome.presentation)
            .map_err(|_| ErrorCode::InvalidData)?
            .len()
            > 64 * 1024
        {
            return Err(ErrorCode::InvalidData.into());
        }
        entry.check(&context)?;
        if let AccountPresentation::Profile(Some(record)) = &outcome.presentation {
            record
                .avatar
                .validate_hosts(&entry.adapter.descriptor().cover_hosts)?;
        }
        if let Some((record, _)) = &outcome.credential {
            record
                .avatar
                .validate_hosts(&entry.adapter.descriptor().cover_hosts)?;
        }
        let valid = match (request, &outcome.presentation) {
            (AccountRequest::BeginLogin { .. }, AccountPresentation::Challenge { key, image }) => {
                !key.is_empty()
                    && key.len() <= 256
                    && image.len() <= 64 * 1024
                    && outcome.credential.is_none()
            }
            (AccountRequest::PollLogin { .. }, AccountPresentation::Progress { code, .. }) => {
                (*code == 803) == outcome.credential.is_some()
            }
            (AccountRequest::Profile, AccountPresentation::Profile(record)) => {
                record.as_ref().is_none_or(|r| {
                    &r.reference.source == source
                        && (context.session.account.is_none()
                            || context.session.account.as_ref() == Some(&r.reference.id))
                })
            }
            _ => false,
        };
        if !valid {
            return Err(ErrorCode::InvalidData.into());
        }
        if let AccountPresentation::Challenge { key, .. } = &outcome.presentation {
            let mut challenges = entry
                .login_challenges
                .lock()
                .map_err(|_| ErrorCode::Internal)?;
            challenges.retain(|key, (_, _, expires)| {
                let valid = Instant::now() < *expires;
                if !valid {
                    entry.adapter.cancel_login(key);
                }
                valid
            });
            if challenges.len() >= 4 {
                entry.adapter.cancel_login(key);
                return Err(ErrorCode::RateLimited.into());
            }
            challenges.insert(
                key.clone(),
                (
                    context.session.clone(),
                    context.adapter_generation,
                    Instant::now() + Duration::from_secs(180),
                ),
            );
        }
        if let Some((record, credential)) = outcome.credential {
            let worker = accounts.clone();
            let mutation_context = context.clone();
            let migration = outcome.migration;
            let login_entry = entry.clone();
            let login_key = match request {
                AccountRequest::PollLogin { key } => Some(key.clone()),
                _ => None,
            };
            worker
                .blocking(move |accounts| {
                    // Cancellation also guards the backend persistence boundary, not
                    // only the WebUI callback. Keep the challenge locked until saved.
                    let challenges = login_entry
                        .login_challenges
                        .lock()
                        .map_err(|_| ErrorCode::Internal)?;
                    if login_key
                        .as_ref()
                        .is_some_and(|key| !challenges.contains_key(key))
                    {
                        return Err(ErrorCode::Cancelled.into());
                    }
                    accounts.accept_context(&mutation_context, record, credential, migration)
                })
                .await?;
        } else if matches!(outcome.presentation, AccountPresentation::Profile(None))
            && context.session.account.is_some()
        {
            let expected = context.session.clone();
            accounts
                .blocking(move |accounts| accounts.invalidate(&expected))
                .await?;
        }
        if let AccountRequest::PollLogin { key } = request {
            if matches!(
                outcome.presentation,
                AccountPresentation::Progress {
                    code: 803 | 800,
                    ..
                }
            ) {
                entry
                    .login_challenges
                    .lock()
                    .map_err(|_| ErrorCode::Internal)?
                    .remove(key);
                entry.adapter.cancel_login(key);
            }
        }
        Ok(outcome.presentation)
    }
    pub async fn read_track(&self, reference: &EntityRef) -> MusicResult<MusicTrack> {
        let entry = self.entry(&reference.source)?;
        let context = Self::context(&entry, Duration::from_secs(3));
        context.check_reference(reference)?;
        // Reject overload rather than create an unbounded waiting queue.
        let _permit = entry
            .reads
            .try_acquire()
            .map_err(|_| ErrorCode::RateLimited)?;
        let track = self
            .run(
                &entry,
                &context,
                entry.adapter.read_track(reference, &context),
            )
            .await?;
        track.validate(reference)?;
        track
            .cover
            .validate_hosts(&entry.adapter.descriptor().cover_hosts)?;
        entry.check(&context)?;
        Ok(track)
    }
    pub async fn resolve_playback(
        &self,
        request: &ResolveRequest,
        playback_generation: u64,
    ) -> MusicResult<Arc<ResourceTicket>> {
        let entry = self.entry(&request.track.source)?;
        let context = Self::context(&entry, Duration::from_secs(12));
        request.validate(&context)?;
        let _permit = entry
            .playback
            .try_acquire()
            .map_err(|_| ErrorCode::RateLimited)?;
        let resource = self
            .run(
                &entry,
                &context,
                entry.adapter.resolve_playback(request, &context),
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
        entry.check(&context)?;
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
            entry: entry.clone(),
            lease: ResourceLease {
                handle,
                context,
                playback_generation,
                valid_until: Instant::now() + Duration::from_millis(ttl),
            },
            resource,
        });
        resources.insert(ticket.lease.handle.as_str().into(), Arc::downgrade(&ticket));
        ticket.check(playback_generation)?;
        Ok(ticket)
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterStatus {
    pub descriptor: SourceDescriptor,
    pub enabled: bool,
}
fn cursor_scope(
    request: &super::business::BusinessRequest,
    context: &RequestContext,
) -> MusicResult<String> {
    use sha2::{Digest, Sha256};
    let mut value = serde_json::to_value(request).map_err(|_| ErrorCode::InvalidData)?;
    if let Some(page) = value.get_mut("page") {
        page["cursor"] = serde_json::Value::Null;
    }
    let bytes = serde_json::to_vec(&(
        value,
        &context.session.source,
        &context.session.account,
        context.session.generation,
        context.adapter_generation,
    ))
    .map_err(|_| ErrorCode::InvalidData)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn decode_cursor(
    request: &super::business::BusinessRequest,
    scope: &str,
) -> MusicResult<super::business::BusinessRequest> {
    let mut value = serde_json::to_value(request).map_err(|_| ErrorCode::InvalidData)?;
    if let Some(page) = value.get_mut("page") {
        if let Some(cursor) = page["cursor"].as_str() {
            let (binding, raw): (String, String) =
                serde_json::from_str(cursor).map_err(|_| ErrorCode::InvalidData)?;
            if binding != scope {
                return Err(ErrorCode::StaleContext.into());
            }
            page["cursor"] = serde_json::Value::String(raw);
        }
    }
    serde_json::from_value(value).map_err(|_| ErrorCode::InvalidData.into())
}
fn encode_cursor(cursor: &mut Option<Cursor>, scope: &str) -> MusicResult<()> {
    if let Some(raw) = cursor.take() {
        *cursor = Some(Cursor::try_from(
            serde_json::to_string(&(scope, raw.as_str())).map_err(|_| ErrorCode::InvalidData)?,
        )?);
    }
    Ok(())
}

fn validate_access(access: &ResourceAccess) -> MusicResult<()> {
    let ResourceAccess::Http(http) = access;
    if http.url.len() > 8192 {
        return Err(ErrorCode::InvalidData.into());
    }
    let url = url::Url::parse(&http.url).map_err(|_| ErrorCode::InvalidData)?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(ErrorCode::InvalidData.into());
    }
    // The phase-two URI-only audio transport cannot inject credential headers.
    // Reject these explicitly until the controlled transport implements them.
    if !http.headers.is_empty() {
        return Err(ErrorCode::Unsupported.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::resource::{HttpAccess, PlaybackExtent, ResourceMetadata};
    use super::*;
    struct TestAdapter {
        descriptor: SourceDescriptor,
        blocked: bool,
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
        calls: Arc<AtomicU64>,
        wrong_identity: bool,
        expiry: Option<u64>,
    }
    impl MusicAdapter for TestAdapter {
        fn account<'a>(
            &'a self,
            request: &'a super::super::business::AccountRequest,
            _: &'a RequestContext,
        ) -> AdapterFuture<'a, super::super::business::AccountOutcome> {
            use super::super::{account::*, business::*};
            Box::pin(async move {
                if matches!(request, AccountRequest::BeginLogin { .. }) {
                    return Ok(AccountOutcome {
                        presentation: AccountPresentation::Challenge {
                            key: "challenge".into(),
                            image: "data:image/png;base64,dGVzdA==".into(),
                        },
                        credential: None,
                        migration: false,
                    });
                }
                if !matches!(request, AccountRequest::PollLogin { .. }) {
                    return Err(ErrorCode::Unsupported.into());
                }
                self.entered.notify_one();
                self.release.notified().await;
                Ok(AccountOutcome {
                    presentation: AccountPresentation::Progress {
                        code: 803,
                        message: "登录成功".into(),
                    },
                    credential: Some((
                        AccountRecord {
                            reference: AccountRef {
                                source: source(),
                                id: OpaqueId::try_from("listener".to_owned())?,
                            },
                            display_name: "Listener".into(),
                            avatar: Default::default(),
                        },
                        OpaqueCredential::new(b"test-secret".to_vec())?,
                    )),
                    migration: false,
                })
            })
        }
        fn descriptor(&self) -> &SourceDescriptor {
            &self.descriptor
        }
        fn business<'a>(
            &'a self,
            request: &'a super::super::business::BusinessRequest,
            context: &'a RequestContext,
        ) -> AdapterFuture<'a, super::super::business::BusinessResponse> {
            use super::super::business::*;
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.entered.notify_one();
                if self.blocked && !request.is_write() {
                    self.release.notified().await;
                }
                match request {
                    BusinessRequest::Search { .. } => {
                        let mut track = adapter(false)
                            .read_track(&reference_for("opaque:01/track"), context)
                            .await?;
                        if self.wrong_identity {
                            track.reference.source =
                                SourceId::try_from("wrong".to_owned()).unwrap();
                        }
                        Ok(BusinessResponse::Tracks(TrackPage {
                            page: Page {
                                items: vec![track],
                                next_cursor: Some(Cursor::try_from(
                                    "provider:next/page".to_owned(),
                                )?),
                            },
                            total: None,
                            description: None,
                        }))
                    }
                    BusinessRequest::Suggestions { keyword } => {
                        Ok(BusinessResponse::Suggestions(vec![keyword.clone()]))
                    }
                    request if request.is_write() => Ok(BusinessResponse::Write(WriteImpact {
                        operations: ["favorites".to_owned()].into_iter().collect(),
                        entities: vec![],
                    })),
                    _ => Err(ErrorCode::Unsupported.into()),
                }
            })
        }
        fn read_track<'a>(
            &'a self,
            reference: &'a EntityRef,
            _: &'a RequestContext,
        ) -> AdapterFuture<'a, MusicTrack> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.entered.notify_one();
                if self.blocked {
                    std::future::pending::<()>().await;
                }
                Ok(MusicTrack {
                    reference: if self.wrong_identity {
                        reference_for("other")
                    } else {
                        reference.clone()
                    },
                    title: "song".into(),
                    aliases: vec![],
                    artist: "artist".into(),
                    artists: vec![],
                    album: "album".into(),
                    album_reference: None,
                    duration_ms: 1000,
                    cover: Default::default(),
                    associations: vec![],
                })
            })
        }
        fn resolve_playback<'a>(
            &'a self,
            _: &'a ResolveRequest,
            _: &'a RequestContext,
        ) -> AdapterFuture<'a, PlaybackResource> {
            Box::pin(async move {
                Ok(PlaybackResource {
                    metadata: ResourceMetadata {
                        actual_quality: "test".into(),
                        extent: PlaybackExtent::Full,
                        expires_at_ms: self.expiry,
                    },
                    access: ResourceAccess::Http(HttpAccess {
                        url: "https://example.com/audio?secret=hidden".into(),
                        headers: vec![],
                    }),
                })
            })
        }
    }
    fn source() -> SourceId {
        SourceId::try_from("test".to_owned()).unwrap()
    }
    fn reference_for(id: &str) -> EntityRef {
        EntityRef {
            source: source(),
            kind: EntityKind::Track,
            id: OpaqueId::try_from(id.to_owned()).unwrap(),
        }
    }
    fn adapter(blocked: bool) -> TestAdapter {
        TestAdapter {
            descriptor: SourceDescriptor {
                source: source(),
                display_name: "test".into(),
                contract_version: super::super::CONTRACT_VERSION,
                login_methods: Vec::new(),
                cover_hosts: Default::default(),
                share_link_templates: Default::default(),
                capabilities: Default::default(),
            },
            blocked,
            entered: Default::default(),
            release: Default::default(),
            calls: Default::default(),
            wrong_identity: false,
            expiry: None,
        }
    }
    fn setup(adapter: TestAdapter) -> (Arc<AdapterManager>, Arc<AtomicU64>, Arc<TestAdapter>) {
        let manager = Arc::new(AdapterManager::default());
        let generation = Arc::new(AtomicU64::new(0));
        let session_generation = generation.clone();
        let adapter = Arc::new(adapter);
        manager
            .register(
                adapter.clone(),
                Arc::new(move || SessionContext {
                    source: source(),
                    account: None,
                    generation: session_generation.load(Ordering::Acquire),
                }),
            )
            .unwrap();
        (manager, generation, adapter)
    }
    fn request() -> ResolveRequest {
        ResolveRequest {
            track: reference_for("A:non-numeric/01"),
            preferred_quality: "test".into(),
            allow_downgrade: false,
        }
    }

    #[test]
    fn cover_permissions_follow_declared_hosts_and_adapter_instances_not_accounts() {
        let mut provider = adapter(false);
        provider.descriptor.cover_hosts = ["*.art.example".into()].into_iter().collect();
        let (manager, accounts, _) = setup(provider);
        let url = "https://images.art.example/cover.jpg";
        let initial = manager.cover_access(url).unwrap();
        assert!(manager
            .cover_access("https://other.example/cover.jpg")
            .is_err());
        accounts.fetch_add(1, Ordering::AcqRel);
        assert_eq!(manager.cover_access(url).unwrap(), initial);
        manager.set_enabled(&source(), false).unwrap();
        assert!(manager.cover_access(url).is_err());
        manager.set_enabled(&source(), true).unwrap();
        assert_ne!(manager.cover_access(url).unwrap(), initial);
    }
    #[tokio::test]
    async fn registry_validates_and_never_overwrites_a_source() {
        let (manager, _, provider) = setup(adapter(false));
        assert_eq!(
            manager
                .register(
                    provider,
                    Arc::new(|| SessionContext {
                        source: source(),
                        account: None,
                        generation: 0
                    })
                )
                .unwrap_err()
                .code,
            ErrorCode::InvalidData
        );
        let track = manager.read_track(&request().track).await.unwrap();
        assert_eq!(track.reference.id.as_str(), "A:non-numeric/01");
        manager.set_enabled(&source(), false).unwrap();
        assert_eq!(
            manager.read_track(&request().track).await.unwrap_err().code,
            ErrorCode::SourceUnavailable
        );
        manager.set_enabled(&source(), true).unwrap();
        manager.read_track(&request().track).await.unwrap();
        let mut malformed = adapter(false);
        malformed.wrong_identity = true;
        assert_eq!(
            setup(malformed)
                .0
                .read_track(&request().track)
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidData
        );
    }

    #[tokio::test]
    async fn cancelled_login_cannot_publish_late_credentials() {
        use super::super::{accounts::*, business::*};
        struct Secrets;
        impl CredentialStore for Secrets {
            fn read(&self, _: &str) -> MusicResult<Option<Vec<u8>>> {
                Ok(None)
            }
            fn write(&self, _: &str, _: &[u8]) -> MusicResult<()> {
                panic!("cancelled login must not write credentials")
            }
            fn delete(&self, _: &str) -> MusicResult<()> {
                Ok(())
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            crate::storage::Store::open(&directory.path().join("accounts.sqlite3")).unwrap(),
        );
        let accounts = Arc::new(AccountManager::new(store, Arc::new(Secrets)).unwrap());
        let mut provider = adapter(false);
        provider.descriptor.capabilities.insert(Capability::Account);
        let (manager, _, provider) = setup(provider);
        accounts.bind(&manager);
        manager
            .account(
                &source(),
                &AccountRequest::BeginLogin { method: None },
                &accounts,
            )
            .await
            .unwrap();
        let worker = manager.clone();
        let worker_accounts = accounts.clone();
        let poll = tokio::spawn(async move {
            worker
                .account(
                    &source(),
                    &AccountRequest::PollLogin {
                        key: "challenge".into(),
                    },
                    &worker_accounts,
                )
                .await
        });
        provider.entered.notified().await;
        manager.cancel_login(&source(), "challenge").unwrap();
        provider.release.notify_one();
        assert_eq!(
            poll.await.unwrap().err().unwrap().code,
            ErrorCode::Cancelled
        );
        assert!(accounts.records(&source()).unwrap().is_empty());
        assert!(accounts.session(&source()).account.is_none());
    }
    #[tokio::test]
    async fn pending_queries_are_cancelled_without_adapter_cooperation() {
        for change in 0..3 {
            let (manager, session, adapter) = setup(adapter(true));
            let worker = manager.clone();
            let query = tokio::spawn(async move { worker.read_track(&request().track).await });
            adapter.entered.notified().await;
            let expected = match change {
                0 => {
                    manager.set_enabled(&source(), false).unwrap();
                    ErrorCode::SourceUnavailable
                }
                1 => {
                    session.fetch_add(1, Ordering::AcqRel);
                    ErrorCode::StaleContext
                }
                _ => {
                    manager.reload(&source()).unwrap();
                    ErrorCode::StaleContext
                }
            };
            let result = tokio::time::timeout(Duration::from_secs(1), query)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(result.unwrap_err().code, expected);
        }
    }
    #[tokio::test]
    async fn playback_has_a_reserved_budget_when_reads_are_full() {
        let (manager, _, adapter) = setup(adapter(true));
        let mut jobs = vec![];
        for _ in 0..4 {
            let worker = manager.clone();
            jobs.push(tokio::spawn(async move {
                worker.read_track(&request().track).await
            }));
            adapter.entered.notified().await;
        }
        assert_eq!(
            manager.read_track(&request().track).await.unwrap_err().code,
            ErrorCode::RateLimited
        );
        let resource = manager.resolve_playback(&request(), 7).await.unwrap();
        resource.check(7).unwrap();
        assert_eq!(resource.check(8).unwrap_err().code, ErrorCode::StaleContext);
        manager.set_enabled(&source(), false).unwrap();
        for job in jobs {
            assert_eq!(
                job.await.unwrap().unwrap_err().code,
                ErrorCode::SourceUnavailable
            );
        }
    }
    #[tokio::test]
    async fn deadline_and_explicit_cancellation_drop_pending_futures() {
        let (manager, _, _) = setup(adapter(false));
        let entry = manager.entry(&source()).unwrap();
        let context = AdapterManager::context(&entry, Duration::from_millis(20));
        let result = manager
            .run::<()>(&entry, &context, std::future::pending())
            .await;
        assert_eq!(result.unwrap_err().code, ErrorCode::DeadlineExceeded);
        let context = AdapterManager::context(&entry, Duration::from_secs(1));
        context.cancellation.cancel();
        assert_eq!(
            manager
                .run::<()>(&entry, &context, std::future::pending())
                .await
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        );
    }
    #[tokio::test]
    async fn resource_ownership_is_bounded_revocable_and_redacted() {
        let (manager, session, _) = setup(adapter(false));
        let mut resources = vec![];
        for _ in 0..16 {
            resources.push(manager.resolve_playback(&request(), 7).await.unwrap());
        }
        assert_eq!(
            manager
                .resolve_playback(&request(), 7)
                .await
                .unwrap_err()
                .code,
            ErrorCode::RateLimited
        );
        resources.pop();
        let ticket = manager.resolve_playback(&request(), 7).await.unwrap();
        assert!(!format!("{ticket:?}").contains("hidden"));
        session.fetch_add(1, Ordering::AcqRel);
        assert_eq!(ticket.check(7).unwrap_err().code, ErrorCode::StaleContext);
        manager.set_enabled(&source(), false).unwrap();
        assert_eq!(
            ticket.check(7).unwrap_err().code,
            ErrorCode::SourceUnavailable
        );
        manager.set_enabled(&source(), true).unwrap();
        assert_eq!(ticket.check(7).unwrap_err().code, ErrorCode::StaleContext);
    }
    #[tokio::test]
    async fn provider_expiry_is_checked_again_before_handoff() {
        let mut provider = adapter(false);
        provider.expiry = Some(now_ms() + 100);
        let (manager, _, _) = setup(provider);
        let resource = manager.resolve_playback(&request(), 7).await.unwrap();
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(
            resource.check(7).unwrap_err().code,
            ErrorCode::DeadlineExceeded
        );
        assert_eq!(
            manager
                .resolve_playback(&request(), 7)
                .await
                .unwrap_err()
                .code,
            ErrorCode::DeadlineExceeded
        );
    }
    #[test]
    fn unsafe_or_unimplemented_access_is_rejected() {
        for url in [
            "file:///tmp/music",
            "https://user:secret@example.com/a",
            "https://example.com/a#fragment",
        ] {
            assert_eq!(
                validate_access(&ResourceAccess::Http(HttpAccess {
                    url: url.into(),
                    headers: vec![]
                }))
                .unwrap_err()
                .code,
                ErrorCode::InvalidData
            );
        }
        assert_eq!(
            validate_access(&ResourceAccess::Http(HttpAccess {
                url: "https://example.com/a".into(),
                headers: vec![("Authorization".into(), "secret".into())]
            }))
            .unwrap_err()
            .code,
            ErrorCode::Unsupported
        );
    }
    fn business_adapter(blocked: bool) -> TestAdapter {
        let mut provider = adapter(blocked);
        provider.descriptor.capabilities = [Capability::Search, Capability::Favorites]
            .into_iter()
            .collect();
        provider
    }
    fn search(keyword: &str, cursor: Option<Cursor>) -> super::super::business::BusinessRequest {
        super::super::business::BusinessRequest::Search {
            keyword: keyword.into(),
            kind: EntityKind::Track,
            page: PageRequest { cursor, limit: 1 },
        }
    }
    #[tokio::test]
    async fn business_capabilities_references_and_account_requirements_are_enforced() {
        use super::super::business::*;
        let (manager, _, _) = setup(adapter(false));
        assert_eq!(
            manager
                .business(&source(), &search("q", None))
                .await
                .unwrap_err()
                .code,
            ErrorCode::Unsupported
        );
        let (manager, _, _) = setup(business_adapter(false));
        assert_eq!(
            manager
                .business(&source(), &BusinessRequest::Favorites)
                .await
                .unwrap_err()
                .code,
            ErrorCode::Unauthenticated
        );
        let request = BusinessRequest::SetFavorite {
            track: EntityRef {
                source: SourceId::try_from("other".to_owned()).unwrap(),
                ..reference_for("abc")
            },
            liked: true,
        };
        assert_eq!(
            manager
                .business(&source(), &request)
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidData
        );
        let BusinessResponse::Tracks(page) = manager
            .business(&source(), &search("q", None))
            .await
            .unwrap()
        else {
            panic!("track page");
        };
        assert_eq!(page.page.items[0].reference.id.as_str(), "opaque:01/track");
        let mut malformed = business_adapter(false);
        malformed.wrong_identity = true;
        assert_eq!(
            setup(malformed)
                .0
                .business(&source(), &search("q", None))
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidData
        );
    }
    #[tokio::test]
    async fn cursors_are_bound_to_query_session_and_instance_without_parsing_provider_tokens() {
        use super::super::business::*;
        let (manager, session, _) = setup(business_adapter(false));
        let BusinessResponse::Tracks(page) = manager
            .business(&source(), &search("first", None))
            .await
            .unwrap()
        else {
            panic!("track page");
        };
        let cursor = page.page.next_cursor.unwrap();
        manager
            .business(&source(), &search("first", Some(cursor.clone())))
            .await
            .unwrap();
        assert_eq!(
            manager
                .business(&source(), &search("different", Some(cursor.clone())))
                .await
                .unwrap_err()
                .code,
            ErrorCode::StaleContext
        );
        session.fetch_add(1, Ordering::AcqRel);
        assert_eq!(
            manager
                .business(&source(), &search("first", Some(cursor.clone())))
                .await
                .unwrap_err()
                .code,
            ErrorCode::StaleContext
        );
        manager.reload(&source()).unwrap();
        assert_eq!(
            manager
                .business(&source(), &search("first", Some(cursor)))
                .await
                .unwrap_err()
                .code,
            ErrorCode::StaleContext
        );
    }
    #[tokio::test]
    async fn successful_writes_reject_older_inflight_queries_and_change_cache_identity() {
        use super::super::business::*;
        let manager = Arc::new(AdapterManager::default());
        let provider = Arc::new(business_adapter(true));
        manager
            .register(
                provider.clone(),
                Arc::new(|| SessionContext {
                    source: source(),
                    account: Some(OpaqueId::try_from("account:A".to_owned()).unwrap()),
                    generation: 1,
                }),
            )
            .unwrap();
        let before = manager.cache_identity(&source()).unwrap();
        let query_manager = manager.clone();
        let job =
            tokio::spawn(
                async move { query_manager.business(&source(), &search("q", None)).await },
            );
        provider.entered.notified().await;
        let request = BusinessRequest::SetFavorite {
            track: reference_for("track:01"),
            liked: true,
        };
        manager.business(&source(), &request).await.unwrap();
        assert_ne!(before, manager.cache_identity(&source()).unwrap());
        provider.release.notify_one();
        assert_eq!(
            job.await.unwrap().unwrap_err().code,
            ErrorCode::StaleContext
        );
    }
}
