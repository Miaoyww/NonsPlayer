//! Host-owned music routing, lifecycle and accounts.
pub use nons_music_adapter_sdk::{
    account, adapter, business, identity, provider, resource, ErrorCode, MusicError, MusicResult,
    CONTRACT_VERSION,
};
pub mod accounts;
pub(crate) mod adapter_install;
pub(crate) mod bundled;
pub mod collections;
pub mod external;
pub mod manager;
pub(crate) mod provider_commands;
pub mod service;
pub(crate) mod settings;
pub(crate) mod source_preferences;
#[cfg(test)]
mod tests;
