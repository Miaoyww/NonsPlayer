//! Trusted native adapter, bundled by default. No dependency on the host crate.
use nons_music_adapter_sdk::legacy as model;
pub use nons_music_adapter_sdk::{
    account, adapter, business, identity, migration, resource, ErrorCode, MusicError, MusicResult,
    CONTRACT_VERSION,
};
mod cover;
mod netease;
mod netease_business;
pub mod platform;
pub use netease::{legacy, numeric_id, reference, session, NeteaseAdapter};
pub use netease_business::{kind, reference as entity_reference};
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
