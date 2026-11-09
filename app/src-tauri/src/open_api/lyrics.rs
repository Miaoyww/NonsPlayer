use super::{
    api::ApiError,
    dto::LyricsAsset,
    settings,
    ttml::{self, GeneratedTtml, LyricCue},
};
use crate::{
    application::Backend,
    infrastructure::diagnostics,
    lyrics::{local_lyrics, LocalLyricSource, LyricSkips, LyricSources},
    model::{Lyrics, Track, TrackSource},
};
use axum::http::StatusCode;
use futures_util::FutureExt;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    sync::{Arc, LazyLock, Mutex as StdMutex},
    time::Duration,
};
use tauri::Manager;
use tokio::sync::{Mutex, Notify, Semaphore};

static CONVERSIONS: Semaphore = Semaphore::const_new(2);
static IN_FLIGHT: LazyLock<Mutex<HashMap<String, Arc<Flight>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct Flight {
    done: Notify,
    result: StdMutex<Option<Result<Option<LyricsAsset>, Failure>>>,
}

#[derive(Clone)]
struct Failure {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl Failure {
    fn unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "unavailable",
            message: "歌词暂时不可用",
        }
    }

    fn stale() -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "stale_track",
            message: "当前曲目已变化",
        }
    }

    fn api_error(self) -> ApiError {
        ApiError {
            status: self.status,
            code: self.code,
            message: self.message,
        }
    }
}

pub(crate) async fn load(
    app: &tauri::AppHandle,
    track: Track,
) -> Result<Option<LyricsAsset>, ApiError> {
    let requested = serde_json::to_vec(&track).map_err(|error| {
        diagnostics::warn("open_api.lyrics.track.serialize", error);
        Failure::unavailable().api_error()
    })?;
    let (cache, cache_generation) = {
        let backend = app.state::<Backend>();
        (backend.cache.clone(), backend.cache.generation())
    };
    let mut fingerprint = Sha256::new();
    fingerprint.update(&requested);
    fingerprint.update(cache_generation.to_le_bytes());
    let key = format!("{:x}", fingerprint.finalize());
    let (flight, leader) = {
        let mut flights = IN_FLIGHT.lock().await;
        if let Some(flight) = flights.get(&key) {
            (flight.clone(), false)
        } else {
            let flight = Arc::new(Flight {
                done: Notify::new(),
                result: StdMutex::new(None),
            });
            flights.insert(key.clone(), flight.clone());
            (flight, true)
        }
    };

    if leader {
        let app = app.clone();
        let spawned = flight.clone();
        let spawned_key = key.clone();
        tauri::async_runtime::spawn(async move {
            let result = match tokio::time::timeout(
                Duration::from_secs(30),
                AssertUnwindSafe(load_inner(app, track, requested, cache, cache_generation))
                    .catch_unwind(),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(panic)) => {
                    diagnostics::warn(
                        "open_api.lyrics.panic",
                        panic
                            .downcast_ref::<&str>()
                            .copied()
                            .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                            .unwrap_or("lyrics loader panicked"),
                    );
                    Err(Failure::unavailable())
                }
                Err(_) => {
                    diagnostics::warn("open_api.lyrics.timeout", "lyrics load exceeded 30s");
                    Err(Failure::unavailable())
                }
            };
            if let Ok(mut slot) = spawned.result.lock() {
                *slot = Some(result);
            } else {
                diagnostics::warn("open_api.lyrics.flight.lock", "flight result lock poisoned");
            }
            {
                let mut flights = IN_FLIGHT.lock().await;
                if flights
                    .get(&spawned_key)
                    .is_some_and(|current| Arc::ptr_eq(current, &spawned))
                {
                    flights.remove(&spawned_key);
                }
            }
            spawned.done.notify_waiters();
        });
    }

    loop {
        let mut notified = Box::pin(flight.done.notified());
        notified.as_mut().enable();
        if let Ok(slot) = flight.result.lock() {
            if let Some(result) = slot.as_ref() {
                return result.clone().map_err(Failure::api_error);
            }
        }
        notified.await;
    }
}

async fn load_inner(
    app: tauri::AppHandle,
    track: Track,
    requested: Vec<u8>,
    cache: Arc<crate::lyrics::lyric_cache::LyricCache>,
    cache_generation: u64,
) -> Result<Option<LyricsAsset>, Failure> {
    let permit = tokio::time::timeout(Duration::from_secs(5), CONVERSIONS.acquire())
        .await
        .map_err(|_| Failure {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "busy",
            message: "歌词转换繁忙",
        })?
        .map_err(|_| Failure::unavailable())?;

    let (store, lyric_service) = {
        let backend = app.state::<Backend>();
        (backend.store.clone(), backend.lyrics.clone())
    };
    let key = track.key.clone();
    let settings_service = lyric_service.clone();
    let (correction, sources) = tauri::async_runtime::spawn_blocking(move || {
        Ok::<_, String>((
            settings_service.correction(&key)?,
            settings_service.sources()?,
        ))
    })
    .await
    .map_err(|error| {
        diagnostics::warn("open_api.lyrics.settings.join", error);
        Failure::unavailable()
    })?
    .map_err(|error| {
        diagnostics::warn("open_api.lyrics.settings", &error);
        Failure::unavailable()
    })?;

    let local_mode = matches!(track.source, TrackSource::Local { .. })
        && sources.local_source == LocalLyricSource::Local
        && correction.local_ttml.is_none()
        && correction.candidate.is_none()
        && correction.source.is_none();
    let outcome = if local_mode {
        load_local(track.clone()).await
    } else {
        load_online(app.clone(), track.clone(), sources.clone(), correction).await
    };
    let Some((lyrics, generated)) = outcome? else {
        return Ok(None);
    };
    let final_store = store.clone();
    let final_service = lyric_service.clone();
    let final_track = track.clone();
    let final_sources = sources.clone();
    let final_result = tauri::async_runtime::spawn_blocking(move || {
        if cache.generation() != cache_generation {
            return Ok::<_, String>(Err(Failure::stale()));
        }
        let current = final_store.track(&final_track.key)?;
        if serde_json::to_vec(&current).ok().as_deref() != Some(requested.as_slice()) {
            return Ok(Err(Failure::stale()));
        }
        let current_sources = final_service.sources()?;
        if current_sources.local_source != final_sources.local_source
            || current_sources.amll != final_sources.amll
            || current_sources.qq != final_sources.qq
        {
            return Ok(Err(Failure::stale()));
        }
        let correction = final_service.correction(&final_track.key)?;
        Ok(Ok((settings::read(&final_store)?, correction)))
    })
    .await
    .map_err(|error| {
        diagnostics::warn("open_api.lyrics.offset.join", error);
        Failure::unavailable()
    })?
    .map_err(|error| {
        diagnostics::warn("open_api.lyrics.offset", &error);
        Failure::unavailable()
    })??;
    let (global_offset, correction) = final_result;
    let effective_offset = global_offset + i64::from(correction.offset_ms.unwrap_or_default());
    drop(permit);
    Ok(Some(asset(lyrics, generated, effective_offset)))
}

async fn load_local(track: Track) -> Result<Option<(Lyrics, GeneratedTtml)>, Failure> {
    let duration = track.duration_ms;
    let cue = track.cue().map(|cue| LyricCue {
        start_ms: cue.start_ms,
        end_ms: cue.end_ms,
    });
    let mut skip_embedded = false;
    let mut skip_ttml = false;
    for attempt in 0..3 {
        let value = local_lyrics::get(track.clone(), skip_embedded, skip_ttml)
            .await
            .map_err(|error| {
                diagnostics::warn("open_api.lyrics.local.read", &error);
                Failure::unavailable()
            })?;
        let Some(value) = value else {
            return Ok(None);
        };
        match convert_blocking(value.clone(), duration, cue).await {
            Ok(generated) => return Ok(Some((value, generated))),
            Err(error) if attempt < 2 => {
                diagnostics::warn(
                    &format!("open_api.lyrics.local.parse.attempt{}", attempt + 1),
                    &error,
                );
                if value.format == "ttml" {
                    skip_embedded = true;
                    skip_ttml = true;
                } else if value.format == "lrc" && !skip_embedded {
                    skip_embedded = true;
                } else {
                    return Err(Failure::unavailable());
                }
            }
            Err(error) => {
                diagnostics::warn("open_api.lyrics.local.parse", error);
                return Err(Failure::unavailable());
            }
        }
    }
    Ok(None)
}

async fn load_online(
    app: tauri::AppHandle,
    track: Track,
    sources: LyricSources,
    correction: crate::lyrics::correction::Correction,
) -> Result<Option<(Lyrics, GeneratedTtml)>, Failure> {
    let (manual, selected) = match correction.source {
        Some(source) => (true, Some(source)),
        None => (
            correction.candidate.is_some() || correction.local_ttml.is_some(),
            correction.candidate.map(|candidate| candidate.source),
        ),
    };
    let mut skip_amll = manual || !sources.amll;
    let mut skip_qq = selected.map_or(!sources.qq, |source| {
        source != crate::lyrics::correction::Source::Qq
    });
    let mut skip_qrc = false;
    let mut skip_netease = selected == Some(crate::lyrics::correction::Source::Qq);
    let duration = track.duration_ms;
    let cue = matches!(track.source, TrackSource::Local { .. })
        .then(|| track.cue())
        .flatten()
        .map(|cue| LyricCue {
            start_ms: cue.start_ms,
            end_ms: cue.end_ms,
        });
    let lyric_service = {
        let backend = app.state::<Backend>();
        backend.lyrics.clone()
    };
    for attempt in 0..5 {
        let skips = LyricSkips {
            amll: skip_amll,
            qq: skip_qq,
            qrc: skip_qrc,
            netease: skip_netease,
        };
        let value = lyric_service
            .get(track.clone(), false, skips, app.clone())
            .await
            .map_err(|error| {
                diagnostics::warn("open_api.lyrics.online.fetch", &error);
                Failure::unavailable()
            })?;
        let Some(value) = value else {
            diagnostics::warn(
                "open_api.lyrics.online.absence",
                format!("source={} attempt={}", track.source_id(), attempt + 1),
            );
            return Err(Failure::unavailable());
        };
        match convert_blocking(value.clone(), duration, cue).await {
            Ok(generated) => return Ok(Some((value, generated))),
            Err(error) => {
                diagnostics::warn(
                    &format!("open_api.lyrics.online.parse.attempt{}", attempt + 1),
                    &error,
                );
                if manual && !(value.source == "qq" && value.format == "qrc" && !skip_qrc) {
                    return Err(Failure::unavailable());
                }
                match value.source.as_str() {
                    "amll" if !skip_amll => skip_amll = true,
                    "qq" if value.format == "qrc" && !skip_qrc => skip_qrc = true,
                    "qq" if !skip_qq => skip_qq = true,
                    "netease"
                        if matches!(track.source, TrackSource::Local { .. }) && !skip_netease =>
                    {
                        skip_netease = true
                    }
                    _ => return Err(Failure::unavailable()),
                }
            }
        }
    }
    Err(Failure::unavailable())
}

async fn convert_blocking(
    value: Lyrics,
    duration: u64,
    cue: Option<LyricCue>,
) -> Result<GeneratedTtml, ttml::ConversionError> {
    tauri::async_runtime::spawn_blocking(move || ttml::convert(&value, duration, cue))
        .await
        .map_err(|error| ttml::ConversionError {
            stage: ttml::ConvertStage::Generate,
            detail: error.to_string(),
        })?
}

fn asset(lyrics: Lyrics, generated: GeneratedTtml, offset: i64) -> LyricsAsset {
    let profile = "nons-amll-v1".to_owned();
    let mut etag_hash = Sha256::new();
    etag_hash.update(generated.content.as_bytes());
    etag_hash.update(profile.as_bytes());
    LyricsAsset {
        body: generated.content,
        source: lyrics.source,
        profile,
        effective_offset_ms: offset,
        etag: format!("{:x}", etag_hash.finalize()),
    }
}
