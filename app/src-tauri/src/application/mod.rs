use crate::{
    about, fonts, hitokoto, library, local_folders, local_library, lyrics, model, player, plugins,
    storage, ttml_cache,
};

use crate::platform::main_webview;
#[cfg(desktop)]
use crate::platform::{autostart, desktop};
use model::{AppResult, Lyrics, OutputDevice, Track, TrackSource};
use player::{Command, Player};
use std::{path::PathBuf, sync::Arc};
use tauri::{Manager, State};

mod cover_export;
mod telemetry;
#[cfg(desktop)]
mod updates;

pub(crate) struct Backend {
    pub(crate) aggregate: Arc<crate::aggregate::SyncJobs>,
    pub(crate) store: Arc<storage::Store>,
    pub(crate) music: Arc<crate::music::service::MusicService>,
    pub(crate) lyrics: Arc<lyrics::LyricService>,
    pub(crate) player: Arc<Player>,
    pub(crate) covers: PathBuf,
    pub(crate) fallback_cover: String,
    pub(crate) cache: Arc<ttml_cache::TtmlCache>,
    pub(crate) folders: Arc<local_folders::LocalFolders>,
    pub(crate) cover_client: reqwest::Client,
    pub(crate) cover_requests: tokio::sync::Semaphore,
}

#[tauri::command]
fn music_sources(
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<Vec<crate::music::adapter::SourceDescriptor>> {
    backend.music.adapters.descriptors()
}
#[tauri::command]
fn music_context(
    source: crate::music::identity::SourceId,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<String> {
    backend.music.adapters.cache_identity(&source)
}
#[tauri::command]
async fn music_query(
    source: crate::music::identity::SourceId,
    request: crate::music::business::BusinessRequest,
    scope: String,
    refresh: Option<bool>,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<crate::music::business::BusinessResponse> {
    if request.is_write() {
        return Err(crate::music::ErrorCode::PermissionDenied.into());
    }
    if backend.music.adapters.cache_identity(&source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    let response = backend
        .music
        .query(&source, &request, refresh.unwrap_or(false))
        .await?;
    if backend.music.adapters.cache_identity(&source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    Ok(response)
}
#[tauri::command]
async fn music_read_track(
    reference: crate::music::identity::EntityRef,
    scope: String,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<crate::music::identity::MusicTrack> {
    if backend.music.adapters.cache_identity(&reference.source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    let track = backend.music.adapters.read_track(&reference).await?;
    if backend.music.adapters.cache_identity(&reference.source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    Ok(track)
}
#[tauri::command]
fn music_queue_tracks(
    source: crate::music::identity::SourceId,
    scope: String,
    tracks: Vec<crate::music::identity::MusicTrack>,
    backend: State<'_, Backend>,
) -> Result<Vec<Track>, crate::infrastructure::diagnostics::CommandError> {
    if tracks.len() > 100
        || tracks.iter().any(|track| track.reference.source != source)
        || serde_json::to_vec(&tracks)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "music.queue_tracks.serialize_batch",
                    error,
                );
            })
            .map_err(|_| crate::music::ErrorCode::InvalidData)?
            .len()
            > 2 * 1024 * 1024
    {
        return Err(crate::music::ErrorCode::InvalidData.into());
    }
    if backend.music.adapters.cache_identity(&source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    let tracks = tracks
        .into_iter()
        .map(Track::from_music)
        .collect::<crate::music::MusicResult<Vec<_>>>()?;
    backend.store.save_tracks(&tracks).map_err(|error| {
        crate::infrastructure::diagnostics::internal_error("save_queue_tracks", &error)
    })?;
    Ok(tracks)
}

#[tauri::command]
async fn music_stored_tracks(
    source: crate::music::identity::SourceId,
    scope: String,
    tracks: Vec<crate::music::identity::MusicTrack>,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<Vec<Track>> {
    backend
        .music
        .collections
        .as_ref()
        .ok_or(crate::music::ErrorCode::Internal)?
        .stored_tracks(&source, &scope, tracks)
        .await
}

#[tauri::command]
async fn music_queue_track(
    reference: crate::music::identity::EntityRef,
    scope: String,
    backend: State<'_, Backend>,
) -> Result<Track, crate::infrastructure::diagnostics::CommandError> {
    if backend.music.adapters.cache_identity(&reference.source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    let music = backend.music.adapters.read_track(&reference).await?;
    let track = Track::from_music(music)?;
    if backend.music.adapters.cache_identity(&reference.source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    backend
        .store
        .save_tracks(std::slice::from_ref(&track))
        .map_err(|error| {
            crate::infrastructure::diagnostics::internal_error("save_queue_track", &error)
        })?;
    Ok(track)
}

#[tauri::command]
async fn music_write(
    source: crate::music::identity::SourceId,
    request: crate::music::business::BusinessRequest,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<crate::music::business::BusinessResponse> {
    use tauri::Emitter;
    if !request.is_write() {
        return Err(crate::music::ErrorCode::InvalidData.into());
    }
    let response = backend.music.query(&source, &request, false).await?;
    let _ = (app.emit("music-changed", &source)).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("app.emit", error);
    });
    Ok(response)
}
#[tauri::command]
async fn music_account(
    source: crate::music::identity::SourceId,
    request: crate::music::business::AccountRequest,
    backend: State<'_, Backend>,
    plugins: State<'_, Arc<plugins::PluginManager>>,
) -> crate::music::MusicResult<crate::music::business::AccountPresentation> {
    let accounts = backend
        .music
        .accounts
        .as_ref()
        .ok_or(crate::music::ErrorCode::Internal)?;
    let before = accounts.session(&source);
    let result = backend
        .music
        .adapters
        .account(&source, &request, accounts)
        .await;
    if accounts.session(&source) != before {
        plugins.account_changed().await;
    }
    result
}
#[tauri::command]
fn music_current_account(
    source: crate::music::identity::SourceId,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<Option<crate::music::account::AccountRecord>> {
    backend
        .music
        .accounts
        .as_ref()
        .ok_or(crate::music::ErrorCode::Internal)?
        .current_record(&source)
}

#[tauri::command]
fn music_cancel_login(
    source: crate::music::identity::SourceId,
    key: String,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<()> {
    backend.music.adapters.cancel_login(&source, &key)
}
#[tauri::command]
fn music_accounts(
    source: crate::music::identity::SourceId,
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<Vec<crate::music::account::AccountRecord>> {
    backend
        .music
        .accounts
        .as_ref()
        .ok_or(crate::music::ErrorCode::Internal)?
        .records(&source)
}
#[tauri::command]
async fn music_select_account(
    reference: crate::music::account::AccountRef,
    backend: State<'_, Backend>,
    plugins: State<'_, Arc<plugins::PluginManager>>,
) -> crate::music::MusicResult<()> {
    let accounts = backend
        .music
        .accounts
        .as_ref()
        .ok_or(crate::music::ErrorCode::Internal)?;
    let selected = reference.clone();
    accounts
        .blocking(move |accounts| accounts.select(&selected))
        .await?;
    plugins.account_changed().await;

    Ok(())
}
#[tauri::command]
async fn music_remove_account(
    reference: crate::music::account::AccountRef,
    backend: State<'_, Backend>,
    plugins: State<'_, Arc<plugins::PluginManager>>,
) -> crate::music::MusicResult<()> {
    let accounts = backend
        .music
        .accounts
        .as_ref()
        .ok_or(crate::music::ErrorCode::Internal)?;
    let before = accounts.session(&reference.source);
    let removed = reference.clone();
    let result = accounts
        .blocking(move |accounts| accounts.remove(&removed))
        .await;
    if result.is_ok()
        || accounts
            .records(&reference.source)?
            .iter()
            .all(|record| record.reference != reference)
    {
        if let Some(collections) = &backend.music.collections {
            collections.remove_account(reference.clone()).await?;
        }
    }
    if accounts.session(&reference.source) != before {
        plugins.account_changed().await;
    }
    result
}

#[tauri::command]
async fn discovery_hitokoto(backend: State<'_, Backend>) -> AppResult<String> {
    hitokoto::fetch(&backend.cover_client).await
}

#[tauri::command]
async fn local_cache_status(backend: State<'_, Backend>) -> AppResult<ttml_cache::CacheStatus> {
    let cache = backend.cache.clone();
    tauri::async_runtime::spawn_blocking(move || cache.status())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_local_cache_options(
    options: ttml_cache::CacheOptions,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    let cache = backend.cache.clone();
    tauri::async_runtime::spawn_blocking(move || cache.configure(options))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn clear_local_cache(backend: State<'_, Backend>) -> AppResult<()> {
    let cache = backend.cache.clone();
    tauri::async_runtime::spawn_blocking(move || cache.clear())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn local_options(backend: State<'_, Backend>) -> AppResult<serde_json::Value> {
    Ok(
        serde_json::json!({"showCovers": backend.store.setting("showLocalCovers")?.as_deref() != Some("false")}),
    )
}
#[tauri::command]
fn set_local_options(show_covers: bool, backend: State<'_, Backend>) -> AppResult<()> {
    backend.store.set_setting(
        "showLocalCovers",
        if show_covers { "true" } else { "false" },
    )
}
#[tauri::command]
async fn music_folders(backend: State<'_, Backend>) -> AppResult<Vec<local_folders::MusicFolder>> {
    let folders = backend.folders.clone();
    tauri::async_runtime::spawn_blocking(move || folders.list())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn add_music_folder(path: String, backend: State<'_, Backend>) -> AppResult<()> {
    let folders = backend.folders.clone();
    tauri::async_runtime::spawn_blocking(move || folders.add(path))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn remove_music_folder(path: String, backend: State<'_, Backend>) -> AppResult<()> {
    let folders = backend.folders.clone();
    tauri::async_runtime::spawn_blocking(move || folders.remove(path))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn rescan_music_folders(backend: State<'_, Backend>) {
    backend.folders.rescan();
}
#[tauri::command]
fn runtime_cover_access(url: String, backend: State<'_, Backend>) -> AppResult<String> {
    backend
        .music
        .adapters
        .cover_access(&url)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn runtime_cover(url: String, backend: State<'_, Backend>) -> AppResult<String> {
    crate::infrastructure::covers::fetch(url, &backend).await
}
#[tauri::command]
async fn runtime_cover_invalidate(url: String) {
    crate::infrastructure::covers::invalidate(&url).await;
}

#[tauri::command]
fn player_snapshot(
    backend: State<'_, Backend>,
) -> AppResult<crate::playback::publication::PlayerBootstrap> {
    backend.player.bootstrap()
}

#[tauri::command]
fn remove_queue_track(index: usize, key: String, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Remove(index, key))
}

fn save_library_tracks(tracks: &mut [Track], backend: &Backend) -> AppResult<()> {
    for track in tracks.iter_mut() {
        if track.cover.is_empty() {
            track.cover = backend.fallback_cover.clone().into();
        }
    }
    backend.store.save_tracks(tracks)
}

#[tauri::command]
async fn music_play_collection(
    entity: crate::music::identity::EntityRef,
    next: bool,
    key: Option<String>,
    backend: State<'_, Backend>,
) -> AppResult<bool> {
    if entity.kind == crate::music::identity::EntityKind::Track {
        return Err("收藏参数无效".into());
    }
    if next {
        let mut page = backend.music.collection_queue(&entity).await?;
        if entity.kind != crate::music::identity::EntityKind::Playlist
            || backend.music.collections.is_none()
        {
            save_library_tracks(&mut page.tracks, &backend)?;
        }
        backend.player.send(Command::PlayNext(page.tracks))?;
        return Ok(page.more);
    }
    let scope = backend
        .music
        .adapters
        .cache_identity(&entity.source)
        .map_err(|e| e.to_string())?;
    let cached = backend.music.cached_collection(&entity).await?;
    let track = if let Some(key) = key {
        let store = backend.store.clone();
        Some(
            tauri::async_runtime::spawn_blocking(move || store.track(&key))
                .await
                .map_err(|error| error.to_string())??,
        )
    } else {
        None
    };
    let (reply, completion) = tokio::sync::oneshot::channel();
    if track
        .as_ref()
        .is_some_and(|track| track.source_id() != entity.source.as_str())
    {
        return Err("曲目与收藏来源不一致".into());
    }
    backend.player.send(Command::Collection {
        entity,
        track: track.map(Box::new),
        cached,
        scope,
        fallback_cover: backend.fallback_cover.clone(),
        reply,
    })?;
    // Superseded requests are silent; their actor-owned preparation task drops the sender.
    completion.await.unwrap_or(Ok(false))
}

#[tauri::command]
async fn local_music(
    keyword: String,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<Vec<Track>> {
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || store.local_tracks(&keyword, offset))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn import_music(
    paths: Vec<String>,
    backend: State<'_, Backend>,
) -> AppResult<library::ImportReport> {
    let store = backend.store.clone();
    let covers = backend.covers.clone();
    let fallback = backend.fallback_cover.clone();
    tauri::async_runtime::spawn_blocking(move || {
        library::import(paths, store, covers, fallback, None)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn play_queue(keys: Vec<String>, index: usize, backend: State<'_, Backend>) -> AppResult<()> {
    if keys.is_empty() || keys.len() > 10000 || index >= keys.len() {
        return Err("播放队列无效".into());
    }
    let tracks = keys
        .iter()
        .map(|key| backend.store.track(key))
        .collect::<AppResult<Vec<_>>>()?;
    backend.player.send(Command::Queue(tracks, index))
}

#[tauri::command]
fn play_private_fm(keys: Vec<String>, backend: State<'_, Backend>) -> AppResult<()> {
    if keys.is_empty() || keys.len() > 1000 {
        return Err("播放队列无效".into());
    }
    let tracks = keys
        .iter()
        .map(|key| backend.store.track(key))
        .collect::<AppResult<Vec<_>>>()?;
    backend.player.send(Command::FmQueue(tracks))
}

#[tauri::command]
fn append_private_fm(
    session: u64,
    queue_len: usize,
    keys: Vec<String>,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if keys.is_empty() || keys.len() > 1000 {
        return Err("待添加的歌曲无效".into());
    }
    let tracks = keys
        .iter()
        .map(|key| backend.store.track(key))
        .collect::<AppResult<Vec<_>>>()?;
    backend.player.send(Command::AppendFm {
        session,
        queue_len,
        tracks,
    })
}

#[tauri::command]
fn append_queue(
    keys: Vec<String>,
    play: Option<bool>,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if keys.is_empty() || keys.len() > 10000 {
        return Err("待添加的歌曲无效".into());
    }
    let tracks = keys
        .iter()
        .map(|key| backend.store.track(key))
        .collect::<AppResult<Vec<_>>>()?;
    backend.player.send(if play.unwrap_or(false) {
        Command::PlayNextNow(tracks)
    } else {
        Command::PlayNext(tracks)
    })
}

#[tauri::command]
fn player_action(action: &str, backend: State<'_, Backend>) -> AppResult<()> {
    let command = match action {
        "toggle" => Command::Toggle,
        "pause" => Command::Pause,
        "resume" => Command::Resume,
        "stop" => Command::Stop,
        "clear" => Command::Clear,
        "next" => Command::Next,
        "previous" => Command::Previous,
        "repeat" => Command::Repeat,
        "shuffle" => Command::Shuffle,
        _ => return Err("播放操作无效".into()),
    };
    backend.player.send(command)
}

#[tauri::command]
fn player_jump(index: usize, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Jump(index))
}
#[tauri::command]
fn player_retry(revision: u64, key: String, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Retry { revision, key })
}
#[tauri::command]
fn player_seek(position_ms: u64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Seek(position_ms))
}
#[tauri::command]
fn player_volume(volume: f64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Volume(volume))
}
#[tauri::command]
async fn player_device(device_id: Option<String>, backend: State<'_, Backend>) -> AppResult<()> {
    let player = backend.player.clone();
    tauri::async_runtime::spawn_blocking(move || player.set_device(device_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_playback_engine(
    engine: model::PlaybackEngine,
    webview: tauri::Webview,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if webview.label() != "main" {
        return Err("播放引擎仅允许主窗口控制".into());
    }
    let player = backend.player.clone();
    tauri::async_runtime::spawn_blocking(move || player.set_engine(engine))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn web_audio_ready(
    webview: tauri::Webview,
    backend: State<'_, Backend>,
) -> AppResult<crate::playback::web_audio::Control> {
    if webview.label() != "main" {
        return Err("音频执行端仅允许主窗口连接".into());
    }
    let player = backend.player.clone();
    tauri::async_runtime::spawn_blocking(move || player.web_ready())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn web_audio_feedback(
    feedback: crate::playback::web_audio::Feedback,
    webview: tauri::Webview,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if webview.label() != "main" {
        return Err("音频执行端仅允许主窗口连接".into());
    }
    backend.player.send(Command::WebFeedback(feedback))
}
#[tauri::command]
async fn set_playback_options(
    options: model::PlaybackOptions,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    let player = backend.player.clone();
    tauri::async_runtime::spawn_blocking(move || player.set_options(options))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn output_devices(backend: State<'_, Backend>) -> AppResult<Vec<OutputDevice>> {
    let player = backend.player.clone();
    tauri::async_runtime::spawn_blocking(move || player.devices())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Flat IPC arguments retain the existing source-skip contract.
async fn track_lyrics(
    key: String,
    refresh: bool,
    skip_amll: bool,
    skip_qq: bool,
    skip_qrc: Option<bool>,
    skip_netease: Option<bool>,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Option<Lyrics>> {
    let track = backend.store.track(&key)?;
    backend
        .lyrics
        .get(
            track,
            refresh,
            lyrics::LyricSkips {
                amll: skip_amll,
                qq: skip_qq,
                qrc: skip_qrc.unwrap_or(false),
                netease: skip_netease.unwrap_or(false),
            },
            app,
        )
        .await
}

#[tauri::command]
fn bind_local_lyrics(
    key: String,
    netease_id: Option<u64>,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if netease_id == Some(0) {
        return Err("网易云歌曲 ID 无效".into());
    }
    let mut track = backend.store.track(&key)?;
    match &mut track.source {
        TrackSource::Local { netease_id: id, .. } => *id = netease_id,
        _ => return Err("仅本地音乐需要手动绑定歌词".into()),
    }
    backend.store.save_tracks(&[track])
}

#[tauri::command]
async fn local_track_lyrics(
    key: String,
    skip_embedded: Option<bool>,
    backend: State<'_, Backend>,
) -> AppResult<Option<Lyrics>> {
    let track = backend.store.track(&key)?;
    lyrics::local_lyrics::get(track, skip_embedded.unwrap_or(false)).await
}

#[tauri::command]
fn lyric_sources(backend: State<'_, Backend>) -> AppResult<lyrics::LyricSources> {
    backend.lyrics.sources()
}

#[tauri::command]
fn android_media_lyrics(
    key: String,
    lines: Vec<crate::lyrics::media_metadata::Line>,
) -> AppResult<()> {
    crate::lyrics::media_metadata::validate(&lines)?;
    #[cfg(target_os = "android")]
    {
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        crate::platform::android::playback::call(
            "mediaLyrics",
            serde_json::json!({"key": key, "sequence": sequence, "lines": lines}),
        )?;
    }
    #[cfg(not(target_os = "android"))]
    let _ = key;
    Ok(())
}

#[tauri::command]
fn android_media_status() -> AppResult<serde_json::Value> {
    if !cfg!(debug_assertions) {
        return Err("媒体诊断仅供调试构建使用".into());
    }
    #[cfg(target_os = "android")]
    return crate::platform::android::playback::debug_status();
    #[cfg(not(target_os = "android"))]
    Err("仅 Android 支持媒体诊断".into())
}

#[tauri::command]
fn set_lyric_sources(sources: lyrics::LyricSources, backend: State<'_, Backend>) -> AppResult<()> {
    backend.lyrics.set_sources(sources)
}

#[tauri::command]
fn music_options(backend: State<'_, Backend>) -> AppResult<serde_json::Value> {
    Ok(
        serde_json::json!({ "quality": backend.store.setting("quality")?.unwrap_or_else(|| "exhigh".into()), "allowDowngrade": backend.store.setting("allowDowngrade")?.as_deref() != Some("false") }),
    )
}

#[tauri::command]
fn set_music_options(
    quality: String,
    allow_downgrade: bool,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if !["standard", "higher", "exhigh", "lossless", "hires"].contains(&quality.as_str()) {
        return Err("音质设置无效".into());
    }
    backend.store.set_setting("quality", &quality)?;
    backend.store.set_setting(
        "allowDowngrade",
        if allow_downgrade { "true" } else { "false" },
    )
}

#[tauri::command]
fn set_lyric_endpoints(
    endpoints: Vec<String>,
    platform: Option<String>,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.lyrics.set_endpoints(platform, endpoints)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    crate::performance::start_clock();
    #[cfg(windows)]
    if std::env::args().any(|arg| arg == "--remove-autostart") {
        if let Err(error) = autostart::cleanup() {
            eprintln!("清理开机自启失败：{error}");
            std::process::exit(1);
        }
        return;
    }
    #[cfg(not(feature = "plugin-probe"))]
    let mut context = tauri::generate_context!();
    #[cfg(feature = "plugin-probe")]
    let mut context = {
        let mut context = tauri::generate_context!();
        context.config_mut().app.windows[0].visible = false;
        context
    };
    if let Err(error) = crate::infrastructure::directories::initialize() {
        eprintln!("初始化程序数据目录失败：{error}");
        std::process::exit(1);
    }
    #[cfg(windows)]
    for window in &mut context.config_mut().app.windows {
        if window.label == "main" {
            // Playback also starts from native SMTC while the page is hidden.
            window.additional_browser_args = Some(format!(
                "{} --autoplay-policy=no-user-gesture-required",
                window
                    .additional_browser_args
                    .as_deref()
                    .unwrap_or("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection")
            ));
        }
    }
    if crate::infrastructure::directories::portable().is_some() {
        // Absolute WebView paths must be supplied through the builder, not WindowConfig.
        for window in &mut context.config_mut().app.windows {
            window.create = false;
        }
    }
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = if crate::performance::root().is_some() {
        builder
    } else {
        builder.plugin(desktop::single_instance())
    };
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(autostart::ENTRY_NAME)
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(main_webview::background_shortcut)
                .build(),
        )
        .on_window_event(desktop::on_window_event);
    #[cfg(target_os = "android")]
    let builder = builder.plugin(crate::platform::android::init());
    let app = builder
        .register_uri_scheme_protocol("plugin", plugins::protocol)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            crate::infrastructure::diagnostics::initialize(app.handle());
            #[cfg(windows)]
            if let Some(data) = crate::infrastructure::directories::portable() {
                for config in &app.config().app.windows {
                    tauri::WebviewWindowBuilder::from_config(app, config)?
                        .initialization_script(if crate::performance::root().is_some() { "window.__NONS_PERFORMANCE__ = true;" } else { "" })
                        .data_directory(data.join("webview"))
                        .build()?;
                }
            }
            #[cfg(not(feature = "plugin-probe"))]
            let data = match crate::infrastructure::directories::portable() {
                Some(data) => data.to_path_buf(),
                None => app.path().app_data_dir()?,
            };
            let cache_directory = match crate::infrastructure::directories::portable() {
                Some(data) => data.join("cache"),
                None => app.path().app_cache_dir()?,
            };
            #[cfg(feature = "plugin-probe")]
            let data = plugins::probe::directory();
            std::fs::create_dir_all(&data)?;
            log::info!(target: "nons::lifecycle", "application_started version={} logs={}", env!("CARGO_PKG_VERSION"), match crate::infrastructure::directories::portable() { Some(data) => data.join("logs"), None => app.path().app_log_dir()? }.display());
            std::panic::set_hook(Box::new(move |info| {
                log::error!(target: "nons::panic", "{}", info);
                log::logger().flush();
            }));
            std::fs::create_dir_all(&cache_directory)?;
            crate::wasm_runtime::configure_compilation_cache(&cache_directory);
            let covers = cache_directory.join("covers");
            std::fs::create_dir_all(&covers)?;
            app.asset_protocol_scope().allow_directory(&covers, true)?;
            let fallback = covers.join("default.png");
            if !fallback.exists() {
                std::fs::write(&fallback, include_bytes!("../../icons/128x128.png"))?;
            }
            let store = Arc::new(storage::Store::open(&data.join("nons.sqlite3"))?);
            if crate::performance::root().is_some() {
                store.set_setting("anonymousTelemetryEnabled", "false")?;
                store.set_setting("updateAutoCheck", "false")?;
                store.set_setting("lyricSources", r#"{"localSource":"local","amll":false,"qq":false}"#)?;
            }
            let telemetry = telemetry::Telemetry::new(store.clone())?;
            app.manage(telemetry.clone());
            let launch_time = time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
            let accounts = Arc::new(crate::music::accounts::AccountManager::new(
                store.clone(),
                Arc::new(crate::music::accounts::SystemCredentials),
            )?);
            let account_events = app.handle().clone();
            accounts.on_change(Arc::new(move |source| {
                use tauri::Emitter;
                let _ = (account_events.emit("music-account-changed", source)).inspect_err(|error| { crate::infrastructure::diagnostics::warn("account_events.emit", error); });
            }));
            let (mut music, netease) = crate::music::bundled::service(accounts)?;
            let collections=crate::music::collections::Collections::new(store.clone(),music.adapters.clone());
            let collection_events=app.handle().clone();
            collections.on_change(Arc::new(move |entity| {
                use tauri::Emitter;
                let _=collection_events.emit("music-collection-updated",entity).inspect_err(|error|crate::infrastructure::diagnostics::warn("collection.notify",error));
            }));
            music.collections=Some(collections);
            let music = Arc::new(music);
            app.manage(Arc::new(crate::music::settings::AdapterSettings::new(
                store.clone(),
                music.adapters.clone(),
            )?));
            app.manage(Arc::new(
                crate::music::source_preferences::SourceSettings::new(
                    store.clone(),
                    music.adapters.clone(),
                )?,
            ));
            let cache = Arc::new(ttml_cache::TtmlCache::new(store.clone(), cache_directory)?);
            let lyrics = Arc::new(lyrics::LyricService::new(
                store.clone(),
                netease.clone(),
                cache.clone(),
            )?);
            let folders = local_folders::LocalFolders::new(
                store.clone(),
                covers.clone(),
                fallback.to_string_lossy().to_string(),
                app.handle().clone(),
            )?;
            #[cfg(windows)]
            let hwnd = app
                .get_webview_window("main")
                .ok_or("主窗口不可用")?
                .hwnd()?
                .0 as isize;
            #[cfg(not(windows))]
            let hwnd = 0;
            let player = Arc::new(Player::start(
                app.handle().clone(),
                store.clone(),
                music.clone(),
                hwnd,
            )?);
            let plugin_manager = plugins::PluginManager::new(
                app.handle().clone(),
                &data,
                music.clone(),
                player.clone(),
            )?;
            app.manage(plugin_manager.clone());
            let component_directory = if cfg!(debug_assertions) {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bundled-adapters")
            } else {
                app.path().resource_dir()?.join("bundled-adapters")
            };
            let component_music = music.clone();
            let component_packages = crate::music::bundled::prepare_components(&component_directory, &music.adapters);
            let component_store = store.clone();
            let component_http = plugin_manager.shared_http();
            let component_settings = app
                .state::<Arc<crate::music::settings::AdapterSettings>>()
                .inner()
                .clone();
            let component_app = app.handle().clone();
            let adapters_startup = tauri::async_runtime::spawn(async move {
                use tauri::Emitter;
                let result = match component_packages {
                  Ok(packages) => crate::music::bundled::load_components(
                    packages,
                    &component_music,
                    component_store,
                    component_http,
                    &component_settings,
                )
                .await,
                  Err(error) => Err(error),
                };
                if let Err(error) = result {
                    log::error!(target: "nons::adapter", "load_components_failed directory={} reason={error}", component_directory.display());
                    let _ = (component_app.emit("adapter-load-failed", error)).inspect_err(|error| { crate::infrastructure::diagnostics::warn("component_app.emit", error); });
                }
                let _ = (component_app.emit("adapters-changed", ())).inspect_err(|error| { crate::infrastructure::diagnostics::warn("component_app.emit", error); });
            });
            #[cfg(feature = "plugin-probe")]
            plugins::probe::start(plugin_manager.clone());
            let plugins_startup = tauri::async_runtime::spawn(async move {
                plugin_manager.startup().await;
            });
            app.manage(Backend {
                aggregate: Arc::new(crate::aggregate::SyncJobs::default()),
                store,
                music,
                lyrics,
                player,
                covers,
                fallback_cover: fallback.to_string_lossy().to_string(),
                cache,
                folders,
                cover_client: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(8))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()?,
                cover_requests: tokio::sync::Semaphore::new(8),
            });
            let telemetry_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let (adapters, plugins) = tokio::join!(adapters_startup, plugins_startup);
                if let Err(error) = adapters {
                    crate::infrastructure::diagnostics::warn("adapters.startup.join", error);
                }
                if let Err(error) = plugins {
                    crate::infrastructure::diagnostics::warn("plugins.startup.join", error);
                }
                crate::performance::startup_ready();
                telemetry.report(telemetry_app, launch_time).await;
            });
            #[cfg(desktop)]
            {
                let updates = updates::Updates::new(updates::auto_check_enabled(&app.state::<Backend>().store)?);
                app.manage(updates.clone());
                let update_app = app.handle().clone();
                if !cfg!(debug_assertions) && !cfg!(feature = "plugin-probe") {
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        let _ = (updates.check_on_startup(&update_app).await).inspect_err(|error| { crate::infrastructure::diagnostics::warn("updates.check_on_startup", error); });
                    });
                }
                desktop::setup(app)?;
                main_webview::setup(app)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::aggregate::aggregate_playlists,
            crate::aggregate::aggregate_playlist,
            crate::aggregate::aggregate_play,
            crate::aggregate::aggregate_tracks,
            crate::aggregate::aggregate_write,
            crate::aggregate::sync::aggregate_sync,
            crate::aggregate::sync::aggregate_sync_status,
            crate::aggregate::sync::aggregate_sync_cancel,
            telemetry::telemetry_enabled,
            telemetry::set_telemetry_enabled,
            #[cfg(desktop)]
            updates::update_status,
            #[cfg(desktop)]
            updates::set_update_auto_check,
            #[cfg(desktop)]
            updates::update_check,
            #[cfg(desktop)]
            updates::update_download,
            #[cfg(desktop)]
            updates::update_install,
            #[cfg(desktop)]
            updates::update_open_download,
            crate::music::provider_commands::resource_provider_list,
            crate::music::provider_commands::resource_provider_set_enabled,
            crate::music::provider_commands::resource_search,
            crate::music::provider_commands::player_select_source,
            crate::infrastructure::diagnostics::diagnostic_report,
            #[cfg(target_os = "android")]
            crate::platform::android::window_layout::android_window_layout,
            #[cfg(desktop)]
            autostart::autostart_status,
            #[cfg(desktop)]
            autostart::set_autostart,
            #[cfg(desktop)]
            desktop::close_behavior,
            #[cfg(desktop)]
            desktop::set_close_behavior,
            main_webview::main_webview_ready,
            main_webview::main_sleep_shortcuts,
            about::open_devtools,
            discovery_hitokoto,
            #[cfg(feature = "plugin-probe")]
            plugins::probe::plugin_probe_report,
            plugins::plugin_list,
            plugins::plugin_open_folder,
            plugins::settings::plugin_settings,
            plugins::plugin_discover,
            plugins::plugin_install,
            plugins::plugin_action,
            plugins::plugin_call,
            plugins::plugin_host_call,
            plugins::plugin_ready,
            plugins::plugin_fault,
            fonts::system_fonts,
            player_snapshot,
            crate::music::settings::adapter_list,
            crate::music::settings::adapter_set_enabled,
            crate::music::settings::adapter_reload,
            crate::music::source_preferences::music_source_preferences,
            crate::music::source_preferences::music_set_source_preferences,
            crate::music::source_preferences::music_source_intent,
            crate::music::source_preferences::music_use_source,
            music_cancel_login,
            music_sources,
            music_context,
            music_query,
            music_write,
            music_read_track,
            music_queue_track,
            music_queue_tracks,
            music_stored_tracks,
            music_account,
            music_accounts,
            music_current_account,
            music_select_account,
            music_remove_account,
            remove_queue_track,
            music_play_collection,
            local_music,
            local_library::local_entities,
            local_library::local_entity_detail,
            local_library::local_entity_tracks,
            local_library::local_track_information,
            local_library::local_preferences,
            local_library::set_local_preferences,
            local_library::create_local_playlist,
            local_library::rename_local_playlist,
            local_library::delete_local_playlist,
            local_library::add_local_playlist_track,
            local_library::remove_local_playlist_track,
            import_music,
            local_cache_status,
            set_local_cache_options,
            clear_local_cache,
            local_options,
            set_local_options,
            music_folders,
            add_music_folder,
            remove_music_folder,
            rescan_music_folders,
            runtime_cover,
            cover_export::cover_export,
            runtime_cover_access,
            runtime_cover_invalidate,
            play_queue,
            play_private_fm,
            append_private_fm,
            append_queue,
            player_action,
            player_jump,
            player_retry,
            crate::performance::performance_probe,
            player_seek,
            player_volume,
            player_device,
            set_playback_options,
            set_playback_engine,
            web_audio_ready,
            web_audio_feedback,
            output_devices,
            track_lyrics,
            crate::lyrics::correction::lyric_correction,
            crate::lyrics::correction::lyric_candidates,
            crate::lyrics::correction::preview_lyric_candidate,
            crate::lyrics::correction::set_lyric_correction,
            local_track_lyrics,
            lyric_sources,
            android_media_lyrics,
            android_media_status,
            set_lyric_sources,
            bind_local_lyrics,
            music_options,
            set_music_options,
            set_lyric_endpoints
        ])
        .build(context)
        .expect("无法启动 NonsPlayer");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            #[cfg(desktop)]
            app.state::<Arc<updates::Updates>>().cleanup();
            #[cfg(desktop)]
            main_webview::stop(app);
            app.state::<Arc<plugins::PluginManager>>().stop();
            app.state::<Backend>().player.shutdown();
        }
    });
}
