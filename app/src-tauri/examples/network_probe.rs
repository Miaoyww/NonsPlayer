//! Opt-in read-only live probe. Never prints cookies or expiring media URLs.
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
#[path = "support/netease.rs"]
#[allow(dead_code)]
mod netease;
#[path = "../src/playback/network.rs"]
mod network;
use gstreamer::{self as gst, prelude::*};

#[tokio::main]
async fn main() -> Result<(), String> {
    let api = std::sync::Arc::new(netease::probe_client()?);
    let adapters = std::sync::Arc::new(nons_lib::music::manager::AdapterManager::default());
    let session_api = api.clone();
    adapters
        .register(
            std::sync::Arc::new(nons_adapter_netease::NeteaseAdapter::new(api.clone())),
            std::sync::Arc::new(move || nons_adapter_netease::session(&session_api)),
        )
        .map_err(|e| e.to_string())?;
    let music = nons_lib::music::service::MusicService::new(adapters);
    let tracks = api.search("纯音乐", 0).await?;
    println!("Search returned {} tracks", tracks.len());
    let mut selected = None;
    for track in tracks.into_iter().take(3) {
        if let Ok(resource) = music.resolve(track, "standard", true, 0).await {
            selected = Some(resource.0);
            break;
        }
    }
    let resource = selected.ok_or("No accessible full track in the first three results")?;
    let address = url::Url::parse(&resource.uri).map_err(|e| e.to_string())?;
    println!(
        "Media transport: {} host={}",
        address.scheme(),
        address.host_str().unwrap_or_default()
    );
    let response = reqwest::Client::new()
        .get(&resource.uri)
        .header("Range", "bytes=0-1023")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|_| "Media HTTP request failed".to_string())?;
    println!("Media HTTP status: {}", response.status());
    println!(
        "HTTP accepts byte ranges: {:?}",
        response.headers().get("accept-ranges")
    );
    let lyrics = api.lyrics(resource.track.netease_id().unwrap()).await?;
    println!(
        "Resolved actual quality: {:?}; Netease lyrics: {}",
        resource.quality,
        lyrics.is_some()
    );
    // The statically linked source runs blocking GStreamer callbacks on its own
    // Tokio runtime; keep them off this probe's async runtime thread, as Engine does.
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        gst::init().map_err(|e| e.to_string())?;
        network::prefer_http_source();

        let player = gst::ElementFactory::make("playbin3")
            .build()
            .map_err(|e| e.to_string())?;
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", true)
            .build()
            .map_err(|e| e.to_string())?;
        player.set_property_from_str("flags", "audio+buffering");
        player.set_property("buffer-size", 4_194_304_i32);
        player.set_property("audio-sink", sink);
        player.set_property("uri", &resource.uri);
        player.connect("source-setup", false, |values| {
            let source = values[1].get::<gst::Element>().unwrap();
            println!("GStreamer source: {}", source.factory().unwrap().name());
            network::configure_source(&source);
            println!(
                "Manual system proxy applied: {}",
                source.find_property("proxy").is_some()
                    && source.property::<Option<String>>("proxy").is_some()
            );
            None
        });
        player
            .set_state(gst::State::Playing)
            .map_err(|e| e.to_string())?;
        let bus = player.bus().unwrap();
        let started = std::time::Instant::now();
        let mut sought = false;
        let outcome = loop {
            if started.elapsed() > std::time::Duration::from_secs(20) {
                break Err("Online decode or seek timed out".into());
            }
            let position = player
                .query_position::<gst::ClockTime>()
                .map(|p| p.mseconds());
            if sought && position.is_some_and(|p| (30_100..33_000).contains(&p)) {
                println!(
                    "Online seek reached 30 s and playback advanced: {} ms",
                    position.unwrap()
                );
                break Ok(());
            }
            if !sought
                && player
                    .query_position::<gst::ClockTime>()
                    .is_some_and(|p| p.mseconds() >= 500)
            {
                println!("Online resource decoded and advanced at least 500 ms");
                let mut query = gst::query::Seeking::new(gst::Format::Time);
                println!(
                    "Seeking query: {} {:?}",
                    player.query(&mut query),
                    query.result()
                );
                if player
                    .seek_simple(
                        gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                        gst::ClockTime::from_seconds(30),
                    )
                    .is_err()
                {
                    break Err("当前资源不支持定位".into());
                }
                sought = true;
            }
            if let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) {
                match message.view() {
                    gst::MessageView::Buffering(value) if value.percent() == 100 => {
                        println!("Buffering ready")
                    }
                    gst::MessageView::StateChanged(value)
                        if message.src().is_some_and(|s| s == &player) =>
                    {
                        println!("Pipeline state {:?}", value.current())
                    }
                    _ => {}
                }
                if let gst::MessageView::Error(error) = message.view() {
                    break Err(error.error().to_string());
                }
            }
        };
        let _ = player.set_state(gst::State::Null);
        outcome
    })
    .await
    .map_err(|e| e.to_string())?
}
