pub(crate) mod about;
#[cfg(target_os = "android")]
pub(crate) mod android;
#[cfg(desktop)]
pub(crate) mod autostart;
#[cfg(desktop)]
pub(crate) mod desktop;
pub(crate) mod fonts;
#[cfg(desktop)]
pub(crate) mod main_webview;
#[cfg(target_os = "android")]
#[path = "android/main_webview.rs"]
pub(crate) mod main_webview;
