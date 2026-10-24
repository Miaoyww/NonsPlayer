//! Probe credentials are provided explicitly, independently of application account storage.
pub use nons_adapter_netease::platform::*;
pub fn probe_client() -> Result<Netease, String> {
    Ok(Netease::with_cookie(
        std::env::var("NONS_PROBE_COOKIE").ok(),
    ))
}
