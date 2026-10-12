//! Host-owned music routing, lifecycle, accounts and compatibility.
pub use nons_music_adapter_sdk::{
    account, adapter, business, identity, resource, ErrorCode, MusicError, MusicResult,
    CONTRACT_VERSION,
};
pub mod accounts;
pub(crate) mod bundled;
pub(crate) mod compatibility;
pub mod external;
pub mod manager;
pub mod migration;
pub mod service;
pub(crate) mod settings;
pub(crate) mod source_preferences;
#[cfg(test)]
mod tests;
