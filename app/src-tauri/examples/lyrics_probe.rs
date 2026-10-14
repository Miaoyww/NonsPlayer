//! Read-only online lyrics probe; optionally pass a local audio file.
#[path = "support/infrastructure.rs"]
#[allow(dead_code)]
mod infrastructure;
#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;
mod storage {
    pub use crate::model::MAX_LYRIC_BYTES;
}
#[path = "../src/lyrics/lyric_matching.rs"]
#[allow(dead_code)]
mod lyric_matching;
#[path = "../src/netease/mod.rs"]
#[allow(dead_code, unused_imports)]
mod netease;
#[path = "../src/lyrics/netease_lyrics.rs"]
mod netease_lyrics;
#[path = "../src/lyrics/qq_lyrics.rs"]
#[allow(dead_code)]
mod qq_lyrics;
#[path = "../src/lyrics/qrc_decrypt.rs"]
mod qrc_decrypt;

#[tokio::main]
async fn main() -> Result<(), String> {
    let track = if let Some(path) = std::env::args().nth(1) {
        use lofty::{prelude::*, probe::Probe};
        let tagged = Probe::open(&path)
            .map_err(|e| e.to_string())?
            .read()
            .map_err(|e| e.to_string())?;
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("Missing tags")?;
        model::Track {
            key: "local:probe".into(),
            title: tag.title().unwrap_or_default().into_owned(),
            artist: tag.artist().unwrap_or_default().into_owned(),
            album: tag.album().unwrap_or_default().into_owned(),
            duration_ms: tagged.properties().duration().as_millis() as u64,
            aliases: vec![],
            artists: vec![],
            album_id: None,
            cover: Default::default(),
            source: model::TrackSource::Local {
                path,
                netease_id: None,
            },
        }
    } else {
        model::Track {
            key: "local:probe".into(),
            title: "I Can't Fit In".into(),
            artist: "Marino".into(),
            album: "I Can't Fit In".into(),
            duration_ms: 128000,
            aliases: vec![],
            artists: vec![],
            album_id: None,
            cover: Default::default(),
            source: model::TrackSource::Local {
                path: String::new(),
                netease_id: None,
            },
        }
    };
    println!(
        "request: {} / {} / {} / {}ms",
        track.title, track.artist, track.album, track.duration_ms
    );
    let client = reqwest::Client::builder()
        .user_agent("NonsPlayer/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let separators = vec!["/".into(), "、".into(), ";".into()];
    let preferred = qq_lyrics::lookup(&client, &track, false, &separators).await?;
    if let Some(lyrics) = &preferred {
        println!(
            "QQ matched: {} score={:?} ({} bytes)",
            lyrics.format,
            lyrics.match_score,
            lyrics.content.len()
        );
    }
    let api = netease::probe_client()?;
    let fallback = netease_lyrics::lookup(&api, &track, &separators).await?;
    if let Some(lyrics) = &fallback {
        println!(
            "NetEase matched: {} score={:?} ({} bytes)",
            lyrics.format,
            lyrics.match_score,
            lyrics.content.len()
        );
    }
    let result = if preferred.as_ref().is_some_and(|value| {
        value.match_score.unwrap_or(0) >= lyric_matching::PREFERRED_MINIMUM_SCORE
    }) {
        preferred
    } else {
        lyric_matching::select(preferred, fallback)
    };
    let lyrics = result.ok_or("No online lyrics matched")?;
    println!("selected: {} / {}", lyrics.source, lyrics.format);
    Ok(())
}
