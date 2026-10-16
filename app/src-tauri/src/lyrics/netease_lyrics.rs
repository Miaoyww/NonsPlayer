//! AF-Media-Bar NetEaseSearchLyricsProvider / LyricsSearch policy (MIT).
use crate::{
    lyric_matching,
    model::{AppResult, Lyrics, Track},
    netease::Netease,
};
use std::time::Duration;

pub(crate) async fn lookup(
    api: &Netease,
    track: &Track,
    separators: &[String],
) -> AppResult<Option<Lyrics>> {
    tokio::time::timeout(Duration::from_secs(6), lookup_inner(api, track, separators))
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "网易云歌词搜索超时".to_string())?
}

async fn lookup_inner(
    api: &Netease,
    track: &Track,
    separators: &[String],
) -> AppResult<Option<Lyrics>> {
    if track.title.trim().is_empty() || track.artist.trim().is_empty() {
        return Ok(None);
    }
    let mut best = None;
    let mut failed = false;
    for query in lyric_matching::queries(track, separators) {
        let candidates = match api.search(&query, 0).await {
            Ok(value) => value,
            Err(error) => {
                crate::infrastructure::diagnostics::warn("error_fallback", error);
                failed = true;
                continue;
            }
        };
        for candidate in candidates {
            let artists = if candidate.artists.is_empty() {
                vec![candidate.artist.clone()]
            } else {
                candidate
                    .artists
                    .iter()
                    .map(|artist| artist.name.clone())
                    .collect()
            };
            let score = lyric_matching::score(
                track,
                &candidate.title,
                &artists,
                &candidate.album,
                candidate.duration_ms as f64 / 1000.0,
                separators,
            );
            if score >= lyric_matching::MINIMUM_SCORE
                && best.as_ref().is_none_or(|(_, previous)| score > *previous)
            {
                best = Some((candidate, score));
            }
        }
        if best.is_some() {
            break;
        }
    }
    let Some((candidate, score)) = best else {
        return if failed {
            Err("网易云歌词搜索失败".into())
        } else {
            Ok(None)
        };
    };
    let mut lyrics = api
        .lyrics(candidate.netease_id().ok_or("搜索结果缺少歌曲 ID")?)
        .await?;
    if let Some(value) = &mut lyrics {
        value.match_score = Some(score);
    }
    Ok(lyrics)
}
