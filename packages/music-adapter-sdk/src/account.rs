use super::{
    identity::{OpaqueId, SourceId},
    ErrorCode, MusicResult,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRef {
    pub source: SourceId,
    pub id: OpaqueId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRecord {
    pub reference: AccountRef,
    pub display_name: String,
    pub avatar: crate::cover::CoverImage,
}

/// Issued by AccountManager, never accepted from a plugin-supplied identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContext {
    pub source: SourceId,
    pub account: Option<OpaqueId>,
    pub generation: u64,
}

/// Deliberately no Serialize/Deserialize/Clone; Debug must not reveal credentials.
pub struct OpaqueCredential(Vec<u8>);
impl Drop for OpaqueCredential {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.0);
    }
}

impl OpaqueCredential {
    pub fn new(bytes: Vec<u8>) -> MusicResult<Self> {
        if bytes.is_empty() || bytes.len() > 64 * 1024 {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(Self(bytes))
    }
    /// For trusted adapter backend code only; the host must verify the instance scope first.
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for OpaqueCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpaqueCredential([redacted])")
    }
}

/// AccountManager owns storage/selection/generations; adapters own login and refresh.
/// Implementations must check source, account, instance and session generation on EVERY call.
/// These signatures are an internal backend contract, not an exported plugin capability.
pub trait AccountAccess: Send + Sync {
    fn read(&self, context: &super::adapter::RequestContext) -> MusicResult<OpaqueCredential>;
    fn replace(
        &self,
        context: &super::adapter::RequestContext,
        credential: OpaqueCredential,
    ) -> MusicResult<()>;
}

/// Trusted native backend bridge bound by the host to one provider.
/// The host verifies active instance/session before credential reads. No storage is exposed.
pub trait ProviderAccounts: Send + Sync {
    fn session(&self) -> SessionContext;
    fn credential(&self) -> MusicResult<Option<OpaqueCredential>>;
    fn read(
        &self,
        context: &super::adapter::RequestContext,
    ) -> MusicResult<Option<OpaqueCredential>>;
    fn invalidate<'a>(&'a self, expected: SessionContext) -> super::adapter::AdapterFuture<'a, ()>;
}
