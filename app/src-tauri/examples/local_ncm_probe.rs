//! Explicit, local-only decoding probe. Uses a fake sink, never changes the user's queue.
#[path = "../src/local/encoded_audio.rs"]
#[allow(dead_code)]
mod encoded_audio;
#[path = "support/infrastructure.rs"]
#[allow(dead_code)]
mod infrastructure;
// Standalone probes do not redirect storage into the app performance workspace.
mod performance {
    pub fn root() -> Option<std::path::PathBuf> {
        None
    }
}
#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;
use gstreamer::{self as gst, prelude::*};

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("请提供本地 NCM 文件路径")?;
    let start = std::time::Instant::now();
    let (tags, info, cover) = encoded_audio::read_tags(&path)?;
    use lofty::prelude::*;
    println!(
        "Metadata: duration={}ms, NCM metadata={}, cover={} bytes",
        tags.properties().duration().as_millis(),
        info.is_some(),
        cover.len()
    );
    let lease = encoded_audio::prepare(&path)?;
    println!(
        "Decoded {} bytes in {:?}",
        std::fs::metadata(lease.path())
            .map_err(|e| e.to_string())?
            .len(),
        start.elapsed()
    );
    gst::init().map_err(|e| e.to_string())?;
    let sink = gst::ElementFactory::make("fakesink")
        .property("sync", false)
        .build()
        .map_err(|e| e.to_string())?;
    let player = gst::ElementFactory::make("playbin3")
        .property("audio-sink", &sink)
        .build()
        .map_err(|e| e.to_string())?;
    player.set_property_from_str("flags", "audio");
    player.set_property(
        "uri",
        url::Url::from_file_path(lease.path())
            .map_err(|_| "路径无效")?
            .as_str(),
    );
    player
        .set_state(gst::State::Playing)
        .map_err(|e| e.to_string())?;
    let message = player.bus().ok_or("总线不可用")?.timed_pop_filtered(
        gst::ClockTime::from_seconds(30),
        &[gst::MessageType::Eos, gst::MessageType::Error],
    );
    player
        .set_state(gst::State::Null)
        .map_err(|e| e.to_string())?;
    match message.as_ref().map(|m| m.view()) {
        Some(gst::MessageView::Eos(_)) => println!("GStreamer decoded the complete audio to EOS."),
        Some(gst::MessageView::Error(error)) => return Err(error.error().to_string()),
        _ => return Err("30 秒内未完成解码".into()),
    }
    let temporary = lease.path().to_owned();
    drop(lease);
    if temporary.exists() {
        return Err("临时播放音频未释放".into());
    }
    println!("Temporary audio released; original NCM preserved.");
    Ok(())
}
