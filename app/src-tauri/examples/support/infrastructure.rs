//! Shared dependencies of standalone probes; do not load the full app infrastructure.
#[path = "../../src/infrastructure/credentials.rs"]
pub(crate) mod credentials;
#[path = "diagnostics.rs"]
pub(crate) mod diagnostics;
#[path = "../../src/infrastructure/directories.rs"]
pub(crate) mod directories;
