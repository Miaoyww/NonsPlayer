//! Native Android capabilities; business services keep their existing contracts.
pub(crate) mod credentials;
use std::sync::OnceLock;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};

static HANDLE: OnceLock<PluginHandle<tauri::Wry>> = OnceLock::new();

pub fn init() -> TauriPlugin<tauri::Wry> {
    Builder::new("nons-android")
        .setup(|_app, api| {
            let handle = api.register_android_plugin("com.miaoyww.nons", "NonsAndroidPlugin")?;
            HANDLE.set(handle).map_err(|_| "Android 能力重复初始化")?;
            Ok(())
        })
        .build()
}
