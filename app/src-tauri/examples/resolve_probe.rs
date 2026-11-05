//! Read-only probe for one NetEase playback resolution.
#[path = "support/infrastructure.rs"]
#[allow(dead_code)]
mod infrastructure;
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

#[tokio::main]
async fn main() -> Result<(), String> {
    struct Log;
    impl log::Log for Log {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.target().starts_with("nons") || metadata.target() == "probe"
        }
        fn log(&self, record: &log::Record<'_>) {
            println!("{} {} {}", record.target(), record.level(), record.args());
        }
        fn flush(&self) {}
    }
    log::set_boxed_logger(Box::new(Log)).map_err(|e| e.to_string())?;
    log::set_max_level(log::LevelFilter::Trace);
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: resolve_probe <track-id> [quality]")?
        .parse::<u64>()
        .map_err(|e| e.to_string())?;
    let quality = std::env::args().nth(2).unwrap_or_else(|| "exhigh".into());
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
    let reference = nons_adapter_netease::reference(id).map_err(|e| e.to_string())?;
    let metadata = music
        .adapters
        .read_track(&reference)
        .await
        .map_err(|e| format!("read_track: {e:?}"))?;
    println!(
        "track: {:?} {}",
        metadata.duration_ms,
        metadata.reference.id.as_str()
    );
    let track = model::Track::from_music(metadata).map_err(|e| e.to_string())?;
    for attempt in 1..=3 {
        match music.resolve(track.clone(), &quality, true, 0).await {
            Ok((resolved, _, _)) => {
                println!("resolved on {attempt}: quality={:?}", resolved.quality);
                break;
            }
            Err(error) => println!("attempt {attempt} failed: {error}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    Ok(())
}
