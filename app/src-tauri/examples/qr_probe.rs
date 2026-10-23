//! Opt-in QR generation probe. Never prints keys, URLs, cookies, or image contents.
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
#[path = "../src/netease/mod.rs"]
#[allow(dead_code)]
mod netease;
use base64::Engine;

#[tokio::main]
async fn main() -> Result<(), String> {
    let qr = netease::probe_client()?.qr_login().await?;
    let image = qr
        .image
        .strip_prefix("data:image/svg+xml;base64,")
        .ok_or("QR generation returned an invalid image type")?;
    let svg = base64::engine::general_purpose::STANDARD
        .decode(image)
        .map_err(|_| "QR image is not valid base64")?;
    let svg = std::str::from_utf8(&svg).map_err(|_| "QR image is not valid UTF-8")?;
    if qr.key.is_empty() || !svg.contains("<svg") || !svg.contains("</svg>") {
        return Err("QR generation returned an invalid SVG result".into());
    }
    println!("QR key and SVG image generated successfully (contents redacted)");
    Ok(())
}
