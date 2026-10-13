//! Compatibility imports for independent lyrics and legacy probes; platform code lives in its package.
#[cfg(target_os = "android")]
use crate::platform::android::credentials as keyring;
pub use nons_adapter_netease::platform::*;
#[allow(dead_code)]
pub fn probe_client() -> crate::model::AppResult<Netease> {
    let entry =
        keyring::Entry::new("NonsPlayer", "netease-session").map_err(|_| "系统凭据存储不可用")?;
    let cookie = match entry.get_password() {
        Ok(v) => Some(v),
        Err(keyring::Error::NoEntry) => None,
        Err(_) => return Err("无法读取系统凭据存储".into()),
    };
    Ok(Netease::with_cookie(cookie))
}
