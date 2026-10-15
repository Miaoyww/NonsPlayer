//! Independent audio providers. Provider identity never replaces a track's platform.
use crate::{
    adapter::{AdapterFuture, Page, PageRequest, RequestContext},
    identity::{EntityKind, MusicTrack, SourceId},
    resource::PlaybackResource,
    ErrorCode, MusicResult, CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceProviderDescriptor {
    pub id: SourceId,
    pub display_name: String,
    pub contract_version: u32,
    pub platforms: BTreeSet<SourceId>,
    /// Only these platforms participate in search; results retain their platform IDs.
    #[serde(default)]
    pub search_platforms: BTreeSet<SourceId>,
    #[serde(default)]
    pub cover_hosts: BTreeSet<String>,
}
impl ResourceProviderDescriptor {
    pub fn validate(&self) -> MusicResult<()> {
        if self.contract_version != CONTRACT_VERSION {
            return Err(ErrorCode::Unsupported.into());
        }
        if self.id.as_str() == "local"
            || self.display_name.trim().is_empty()
            || self.display_name.len() > 256
            || self.platforms.is_empty()
            || self.platforms.len() > 16
            || self
                .platforms
                .iter()
                .any(|s| s.as_str() == "local" || s == &self.id)
            || !self.search_platforms.is_subset(&self.platforms)
            || self.cover_hosts.len() > 32
            || self
                .cover_hosts
                .iter()
                .any(|h| !crate::cover::valid_host_rule(h))
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRequest {
    pub track: MusicTrack,
    pub preferred_quality: String,
    pub allow_downgrade: bool,
}
impl ResourceRequest {
    pub fn validate(&self, descriptor: &ResourceProviderDescriptor) -> MusicResult<()> {
        self.track.validate(&self.track.reference)?;
        if self.track.reference.kind != EntityKind::Track
            || !descriptor.platforms.contains(&self.track.reference.source)
            || self.preferred_quality.is_empty()
            || self.preferred_quality.len() > 128
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceSearchRequest {
    pub platform: SourceId,
    pub keyword: String,
    pub page: PageRequest,
}
impl ResourceSearchRequest {
    pub fn validate(&self, descriptor: &ResourceProviderDescriptor) -> MusicResult<()> {
        self.page.validate()?;
        if !descriptor.search_platforms.contains(&self.platform) {
            return Err(ErrorCode::Unsupported.into());
        }
        if self.keyword.trim().is_empty() || self.keyword.len() > 1024 {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

pub trait ResourceProvider: Send + Sync {
    fn descriptor(&self) -> &ResourceProviderDescriptor;
    fn revoke(&self) {}
    fn resolve<'a>(
        &'a self,
        request: &'a ResourceRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource>;
    fn search<'a>(
        &'a self,
        _request: &'a ResourceSearchRequest,
        _context: &'a RequestContext,
    ) -> AdapterFuture<'a, Page<MusicTrack>> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_search_outside_supported_platforms_and_identity_aliases() {
        let mut descriptor: ResourceProviderDescriptor = serde_json::from_str(
            r#"{"id":"test-audio","displayName":"Test","contractVersion":1,"platforms":["netease"],"searchPlatforms":["qqmusic"]}"#,
        ).unwrap();
        assert!(descriptor.validate().is_err());
        descriptor.search_platforms.clear();
        assert!(descriptor.validate().is_ok());
        descriptor.platforms.insert(descriptor.id.clone());
        assert!(descriptor.validate().is_err());
    }
}
