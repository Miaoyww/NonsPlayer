//! Native Android capabilities; business services keep their existing contracts.
pub(crate) mod credentials;
mod favorites;
pub(crate) mod playback;
pub(crate) mod window_layout;
use std::sync::OnceLock;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};

static HANDLE: OnceLock<PluginHandle<tauri::Wry>> = OnceLock::new();
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn init() -> TauriPlugin<tauri::Wry> {
    Builder::new("nons-android")
        .setup(|app, api| {
            let handle = api.register_android_plugin("com.miaoyww.nons", "NonsAndroidPlugin")?;
            HANDLE
                .set(handle)
                .inspect_err(|_| {
                    crate::infrastructure::diagnostics::warn(
                        "android.handle.initialize",
                        "Android 能力重复初始化",
                    );
                })
                .map_err(|_| "Android 能力重复初始化")?;
            APP.set(app.clone())
                .inspect_err(|_| {
                    crate::infrastructure::diagnostics::warn(
                        "android.app.initialize",
                        "Android 能力重复初始化",
                    );
                })
                .map_err(|_| "Android 能力重复初始化")?;
            Ok(())
        })
        .build()
}
