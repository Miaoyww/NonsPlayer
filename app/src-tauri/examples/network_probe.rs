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
    Ok(())
}
