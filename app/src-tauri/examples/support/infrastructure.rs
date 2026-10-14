//! Shared dependencies of standalone probes; do not load the full app infrastructure.
#[path = "../../src/infrastructure/credentials.rs"]
pub(crate) mod credentials;
#[path = "../../src/infrastructure/directories.rs"]
pub(crate) mod directories;
