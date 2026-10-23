mod aggregate;
mod application;
mod discovery;
mod infrastructure;
mod local;
mod lyrics;
mod model;
pub mod music;
mod netease;
mod performance;
mod platform;
mod playback;
mod plugins;
#[cfg(test)]
mod test_support;

use application::Backend;
use discovery::hitokoto;
use infrastructure::{storage, wasm_runtime};
use local::{encoded_audio, playback_resource};
use local::{library, local_folders, local_library};
use lyrics::{lyric_matching, netease_lyrics, qq_lyrics, qrc_decrypt, ttml_cache};
use platform::{about, fonts};
use playback::{audio, media, network, player};

pub use application::run;
