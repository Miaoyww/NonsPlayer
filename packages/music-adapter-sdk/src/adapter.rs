use super::{
    account::SessionContext,
    identity::{EntityRef, MusicTrack, SourceId},
    resource::{PlaybackResource, ResolveRequest},
    ErrorCode, MusicResult,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Capability {
    Search,
    Browse,
    Account,
    UserLibrary,
    Favorites,
    PlaylistWrite,
    Recommendations,
    PrivateFm,
    Lyrics,
    Download,
}

/// Track read and playback resolution are mandatory, not optional capability groups.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDescriptor {
    pub source: SourceId,
    pub display_name: String,
    pub contract_version: u32,
    pub capabilities: BTreeSet<Capability>,
    /// Public artwork hosts, independent of playback resource/HTTP permissions.
    #[serde(default)]
    pub cover_hosts: BTreeSet<String>,
    /// Public entity links; the host URL-encodes the opaque ID into `{id}`.
    #[serde(default)]
    pub share_link_templates: BTreeMap<super::identity::EntityKind, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum OperationAvailability {
    Available,
    Unavailable { reason: super::ErrorCode },
}

impl SourceDescriptor {
    pub fn validate(&self) -> MusicResult<()> {
        if self.contract_version != super::CONTRACT_VERSION {
            return Err(ErrorCode::Unsupported.into());
        }
        if self.source.as_str() == "local"
            || self.display_name.trim().is_empty()
            || self.display_name.len() > 256
        {
            return Err(ErrorCode::InvalidData.into());
        }
        if self.cover_hosts.len() > 32
            || self
                .cover_hosts
                .iter()
                .any(|host| !crate::cover::valid_host_rule(host))
        {
            return Err(ErrorCode::InvalidData.into());
        }
        for template in self.share_link_templates.values() {
            if template.len() > 2048 || template.matches("{id}").count() != 1 {
                return Err(ErrorCode::InvalidData.into());
            }
            let parsed = url::Url::parse(&template.replace("{id}", "entity-id"))
                .map_err(|_| ErrorCode::InvalidData)?;
            if !matches!(parsed.scheme(), "http" | "https")
                || parsed.host_str().is_none()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || parsed
                    .host_str()
                    .is_some_and(|host| host.contains("entity-id"))
            {
                return Err(ErrorCode::InvalidData.into());
            }
        }
        Ok(())
    }
    /// Account/track restrictions are separate from what the adapter implements.
    pub fn availability(
        &self,
        capability: Capability,
        restriction: Option<ErrorCode>,
    ) -> OperationAvailability {
        match (self.capabilities.contains(&capability), restriction) {
            (false, _) => OperationAvailability::Unavailable {
                reason: ErrorCode::Unsupported,
            },
            (true, Some(reason)) => OperationAvailability::Unavailable { reason },
            (true, None) => OperationAvailability::Available,
        }
    }
}

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone)]
pub struct RequestContext {
    pub session: SessionContext,
    pub adapter_generation: u64,
    pub deadline: Instant,
    pub cancellation: Cancellation,
}

impl RequestContext {
    /// Admission and publication checks. Managers must also enforce an async timeout
    /// and cancellation while awaiting work, rather than relying on cooperative checks.
    pub fn check(
        &self,
        current_session: &SessionContext,
        current_adapter_generation: u64,
    ) -> MusicResult<()> {
        if self.cancellation.is_cancelled() {
            return Err(ErrorCode::Cancelled.into());
        }
        if Instant::now() >= self.deadline {
            return Err(ErrorCode::DeadlineExceeded.into());
        }
        if &self.session != current_session || self.adapter_generation != current_adapter_generation
        {
            return Err(ErrorCode::StaleContext.into());
        }
        Ok(())
    }
    pub fn check_reference(&self, reference: &EntityRef) -> MusicResult<()> {
        if reference.source != self.session.source
            || reference.kind != super::identity::EntityKind::Track
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

/// A cursor is not an entity ID and may encode arbitrary platform pagination state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Cursor(String);
impl Cursor {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Cursor {
    type Error = super::MusicError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 4096 {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest {
    pub cursor: Option<Cursor>,
    pub limit: u16,
}
impl PageRequest {
    pub fn validate(&self) -> MusicResult<()> {
        if self.limit == 0 || self.limit > 100 {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Cursor>,
}
impl<T: Serialize> Page<T> {
    pub fn validate(&self, request: &PageRequest) -> MusicResult<()> {
        request.validate()?;
        if self.items.len() > usize::from(request.limit)
            || serde_json::to_vec(self)
                .map_err(|_| super::MusicError::from(ErrorCode::InvalidData))?
                .len()
                > 2 * 1024 * 1024
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

/// Host-owned cache identity; actual results still use the existing cache services.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheScope {
    pub source: SourceId,
    pub account: Option<super::identity::OpaqueId>,
    pub session_generation: u64,
    pub adapter_generation: u64,
    pub operation: String,
    /// Canonicalized business parameters only, never credentials or resource URLs.
    pub parameters: String,
}

/// Suggestions only: the host clamps TTL/size and owns eviction and invalidation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheAdvice {
    pub max_age_ms: u64,
}

/// Published only after a successful write under a still-current request context.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteImpact {
    pub operations: BTreeSet<String>,
    pub entities: Vec<EntityRef>,
}

pub type LogoutAction = Pin<Box<dyn Future<Output = MusicResult<()>> + Send>>;
pub type AdapterFuture<'a, T> = Pin<Box<dyn Future<Output = MusicResult<T>> + Send + 'a>>;

/// Backend-only base contract. Optional groups acquire methods as they are migrated.
/// AdapterManager must validate DTO size/kind/source and context before publishing.
pub trait MusicAdapter: Send + Sync {
    fn descriptor(&self) -> &SourceDescriptor;
    fn prepare_logout<'a>(
        &'a self,
        _context: &'a RequestContext,
    ) -> AdapterFuture<'a, LogoutAction> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn business<'a>(
        &'a self,
        _request: &'a super::business::BusinessRequest,
        _context: &'a RequestContext,
    ) -> AdapterFuture<'a, super::business::BusinessResponse> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn account<'a>(
        &'a self,
        _request: &'a super::business::AccountRequest,
        _context: &'a RequestContext,
    ) -> AdapterFuture<'a, super::business::AccountOutcome> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn read_track<'a>(
        &'a self,
        reference: &'a EntityRef,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, MusicTrack>;
    fn resolve_playback<'a>(
        &'a self,
        request: &'a ResolveRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource>;
}

#[cfg(test)]
mod share_link_tests {
    use super::*;
    use crate::identity::EntityKind;

    #[test]
    fn validates_public_templates_and_defaults_legacy_descriptors() {
        let mut descriptor: SourceDescriptor = serde_json::from_str(
            r#"{"source":"fixture","displayName":"Fixture","contractVersion":1,"capabilities":[]}"#,
        )
        .unwrap();
        assert!(descriptor.share_link_templates.is_empty());
        assert!(descriptor.validate().is_ok());
        for template in [
            "https://example.com/song/{id}",
            "https://example.com/#/song?id={id}",
        ] {
            descriptor
                .share_link_templates
                .insert(EntityKind::Track, template.into());
            assert!(descriptor.validate().is_ok());
        }
        for template in [
            "https://example.com/song",
            "https://example.com/{id}/{id}",
            "file:///{id}",
            "https://user:secret@example.com/{id}",
            "https://{id}.example.com/song",
        ] {
            descriptor
                .share_link_templates
                .insert(EntityKind::Track, template.into());
            assert!(descriptor.validate().is_err(), "{template}");
        }
    }
}
