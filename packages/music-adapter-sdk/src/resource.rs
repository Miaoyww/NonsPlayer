use super::{adapter::RequestContext, identity::EntityRef, ErrorCode, MusicResult};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRequest {
    pub track: EntityRef,
    pub preferred_quality: String,
    pub allow_downgrade: bool,
}
impl ResolveRequest {
    pub fn validate(&self, context: &RequestContext) -> MusicResult<()> {
        context.check_reference(&self.track)?;
        if self.preferred_quality.is_empty() || self.preferred_quality.len() > 128 {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PlaybackExtent {
    Full,
    Preview { start_ms: u64, end_ms: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMetadata {
    pub actual_quality: String,
    pub extent: PlaybackExtent,
    /// Absolute Unix milliseconds; None means the provider did not supply an expiry.
    pub expires_at_ms: Option<u64>,
}

/// Backend-only transport data. No serialization; Debug always redacts the entire access.
/// AdapterManager validates access before registering the backend-only resource.
pub struct HttpAccess {
    pub url: String,
    pub headers: Vec<(String, String)>,
}
impl std::fmt::Debug for HttpAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HttpAccess([redacted])")
    }
}

#[derive(Debug)]
pub enum ResourceAccess {
    Http(HttpAccess),
    HttpFallback {
        primary: HttpAccess,
        backup_urls: Vec<String>,
    },
}

/// Returned only to the host. It must never become a queue item or ordinary plugin DTO.
#[derive(Debug)]
pub struct PlaybackResource {
    pub metadata: ResourceMetadata,
    pub access: ResourceAccess,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ResourceHandle(String);
impl ResourceHandle {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ResourceHandle {
    type Error = super::MusicError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(Self(value))
    }
}

/// Controlled resource ownership; this is internal and must not be accepted as IPC authority.
pub struct ResourceLease {
    pub handle: ResourceHandle,
    pub context: RequestContext,
    pub playback_generation: u64,
    /// Host cap also bounds resources whose platform expiry is unknown.
    pub valid_until: Instant,
}
impl ResourceLease {
    pub fn check(
        &self,
        session: &super::account::SessionContext,
        adapter_generation: u64,
        playback_generation: u64,
    ) -> MusicResult<()> {
        // The request deadline governs resolution, not the lifetime of a prepared resource.
        if self.context.cancellation.is_cancelled() {
            return Err(ErrorCode::Cancelled.into());
        }
        if &self.context.session != session || self.context.adapter_generation != adapter_generation
        {
            return Err(ErrorCode::StaleContext.into());
        }
        if self.playback_generation != playback_generation {
            return Err(ErrorCode::StaleContext.into());
        }
        if Instant::now() >= self.valid_until {
            return Err(ErrorCode::DeadlineExceeded.into());
        }
        Ok(())
    }
}

impl ResourceMetadata {
    pub fn require_full(&self, now_ms: u64) -> MusicResult<()> {
        if self.actual_quality.is_empty() || self.actual_quality.len() > 128 {
            return Err(ErrorCode::InvalidData.into());
        }
        if !matches!(self.extent, PlaybackExtent::Full) {
            return Err(ErrorCode::PermissionDenied.into());
        }
        if self.expires_at_ms.is_some_and(|expiry| expiry <= now_ms) {
            return Err(ErrorCode::DeadlineExceeded.into());
        }
        Ok(())
    }
}
