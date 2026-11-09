mod aggregate;
mod application;
mod discovery;
mod downloads;
mod infrastructure;
mod local;
mod lyrics;
mod model;
pub mod music;
mod mv;
mod netease;
#[cfg(desktop)]
mod open_api;
mod performance;
mod platform;
mod playback;
mod plugins;
#[cfg(test)]
mod test_support;

#[cfg(all(test, windows))]
#[link(name = "windows-test", kind = "static", modifiers = "+whole-archive")]
extern "C" {}

use application::Backend;
use discovery::hitokoto;
use infrastructure::{storage, wasm_runtime};
use local::{encoded_audio, playback_resource};
use local::{library, local_folders, local_library};
use lyrics::{lyric_matching, netease_lyrics, qq_lyrics, qrc_decrypt};
use platform::fonts;
use playback::{audio, media, player};

pub use application::run;
