use crate::{
    about, audio_runtime, fonts, hitokoto, library, local_folders, local_library, lyrics, model,
    netease, player, plugins, storage, ttml_cache,
};

use crate::platform::{autostart, desktop, main_webview};
use model::{AppResult, Lyrics, OutputDevice, PlayerSnapshot, Track, TrackSource};
use netease::{AccountProfile, CollectionPage, LibrarySummary, LoginStatus, QrLogin, TrackPage};
use player::{Command, Player};
use std::{path::PathBuf, sync::Arc};
use tauri::{Manager, State};

pub(crate) struct Backend {
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
    backend: State<'_, Backend>,
) -> crate::music::MusicResult<crate::music::business::BusinessResponse> {
    if request.is_write() {
        return Err(crate::music::ErrorCode::PermissionDenied.into());
    }
    if backend.music.adapters.cache_identity(&source)? != scope {
        return Err(crate::music::ErrorCode::StaleContext.into());
    }
    let response = backend.music.adapters.business(&source, &request).await?;
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
    let response = backend.music.adapters.business(&source, &request).await?;
    let _ = app.emit("music-changed", &source);
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
async fn runtime_cover(url: String, backend: State<'_, Backend>) -> AppResult<String> {
    use base64::Engine;
    let parsed = url::Url::parse(&url).map_err(|_| "封面地址无效")?;
    let host = parsed.host_str().unwrap_or_default();
    if !matches!(parsed.scheme(), "https" | "http")
        || !(host.ends_with(".music.126.net") || host.ends_with(".music.163.com"))
    {
        return Err("封面地址不受支持".into());
    }
    let _permit = backend
        .cover_requests
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    let mut response = backend
        .cover_client
        .get(parsed)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .to_string();
    if !["image/jpeg", "image/png", "image/webp", "image/gif"].contains(&mime.as_str()) {
        return Err("封面格式不支持".into());
    }
    const LIMIT: usize = 4 * 1024 * 1024;
    if response.content_length().is_some_and(|n| n > LIMIT as u64) {
        return Err("封面文件过大".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("封面文件过大".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[tauri::command]
fn player_snapshot(backend: State<'_, Backend>) -> AppResult<PlayerSnapshot> {
    backend.player.snapshot()
}

#[tauri::command]
async fn search_music(
    keyword: String,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<Vec<Track>> {
    let mut tracks = backend.music.search(&keyword, offset.min(10_000)).await?;
    for track in &mut tracks {
        if track.cover.is_empty() {
            track.cover = backend.fallback_cover.clone();
        }
    }
    backend.store.save_tracks(&tracks)?;
    Ok(tracks)
}

#[tauri::command]
async fn liked_song_ids(backend: State<'_, Backend>) -> AppResult<Vec<u64>> {
    backend.music.liked_song_ids().await
}

#[tauri::command]
async fn search_suggestions(
    keyword: String,
    backend: State<'_, Backend>,
) -> AppResult<Vec<String>> {
    backend.music.search_suggestions(&keyword).await
}

#[tauri::command]
async fn search_collections(
    keyword: String,
    kind: String,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<netease::CollectionPage> {
    backend
        .music
        .search_collections(&keyword, &kind, offset)
        .await
}

#[tauri::command]
async fn set_song_liked(id: u64, liked: bool, backend: State<'_, Backend>) -> AppResult<()> {
    backend.music.set_song_liked(id, liked).await
}

#[tauri::command]
async fn song_information(
    key: String,
    backend: State<'_, Backend>,
) -> AppResult<netease::SongInformation> {
    let track = backend.store.track(&key)?;
    if let TrackSource::Netease { id } = track.source {
        backend.music.song_information(id).await
    } else {
        Ok(netease::SongInformation {
            artists: vec![netease::SongCredit {
                name: track.artist,
                id: None,
            }],
            album_id: None,
            published_at: None,
        })
    }
}

#[tauri::command]
async fn remove_playlist_song(
    playlist_id: u64,
    song_id: u64,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend
        .music
        .remove_playlist_song(playlist_id, song_id)
        .await
}

#[tauri::command]
fn remove_queue_track(index: usize, key: String, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Remove(index, key))
}

#[tauri::command]
async fn music_library(backend: State<'_, Backend>) -> AppResult<LibrarySummary> {
    let mut summary = backend.music.library_summary().await?;
    save_library_tracks(&mut summary.liked_tracks, &backend)?;
    Ok(summary)
}

#[tauri::command]
async fn discovery_playlists(
    section: String,
    category: String,
    order: String,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<CollectionPage> {
    backend
        .music
        .discovery_playlists(&section, &category, &order, offset)
        .await
}
#[tauri::command]
async fn discovery_categories(
    backend: State<'_, Backend>,
) -> AppResult<Vec<netease::PlaylistCategory>> {
    backend.music.discovery_categories().await
}
#[tauri::command]
async fn discovery_radar(backend: State<'_, Backend>) -> AppResult<netease::Collection> {
    backend.music.discovery_radar().await
}
#[tauri::command]
async fn discovery_tracks(kind: String, backend: State<'_, Backend>) -> AppResult<Vec<Track>> {
    let mut tracks = backend.music.discovery_tracks(&kind).await?;
    save_library_tracks(&mut tracks, &backend)?;
    Ok(tracks)
}
#[tauri::command]
async fn discovery_dislike(id: u64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.music.discovery_dislike(id).await
}

#[tauri::command]
async fn library_collections(
    kind: String,
    offset: u32,
    filter: String,
    backend: State<'_, Backend>,
) -> AppResult<CollectionPage> {
    backend
        .music
        .library_collections(&kind, offset, &filter)
        .await
}

#[tauri::command]
async fn library_tracks(
    kind: String,
    id: u64,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<TrackPage> {
    let mut page = backend.music.library_tracks(&kind, id, offset, 100).await?;
    save_library_tracks(&mut page.tracks, &backend)?;
    Ok(page)
}

#[tauri::command]
async fn music_entity_detail(
    kind: String,
    id: u64,
    backend: State<'_, Backend>,
) -> AppResult<netease::EntityDetail> {
    backend.music.music_entity_detail(&kind, id).await
}
#[tauri::command]
async fn artist_albums(
    id: u64,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<CollectionPage> {
    backend.music.artist_albums(id, offset).await
}
#[tauri::command]
async fn artist_tracks(id: u64, offset: u32, backend: State<'_, Backend>) -> AppResult<TrackPage> {
    let mut page = backend.music.artist_tracks(id, offset).await?;
    save_library_tracks(&mut page.tracks, &backend)?;
    Ok(page)
}

#[tauri::command]
async fn library_history(
    week: bool,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<TrackPage> {
    let mut page = backend.music.library_history(week, offset).await?;
    save_library_tracks(&mut page.tracks, &backend)?;
    Ok(page)
}

fn save_library_tracks(tracks: &mut [Track], backend: &Backend) -> AppResult<()> {
    for track in tracks.iter_mut() {
        if track.cover.is_empty() {
            track.cover = backend.fallback_cover.clone();
        }
    }
    backend.store.save_tracks(tracks)
}

#[tauri::command]
async fn play_library_collection(
    kind: String,
    id: u64,
    key: Option<String>,
    backend: State<'_, Backend>,
) -> AppResult<bool> {
    if id == 0 || !matches!(kind.as_str(), "playlist" | "album" | "artist") {
        return Err("收藏参数无效".into());
    }
    let track = key
        .as_deref()
        .map(|key| backend.store.track(key))
        .transpose()?;
    let (reply, completion) = tokio::sync::oneshot::channel();
    backend.player.send(Command::Collection {
        kind,
        id,
        track,
        fallback_cover: backend.fallback_cover.clone(),
        reply,
    })?;
    // Superseded requests are silent; their actor-owned preparation task drops the sender.
    completion.await.unwrap_or(Ok(false))
}

#[tauri::command]
async fn create_library_playlist(
    name: String,
    private: bool,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.music.library_create_playlist(&name, private).await
}

#[tauri::command]
async fn append_library_collection(
    kind: String,
    id: u64,
    backend: State<'_, Backend>,
) -> AppResult<bool> {
    let mut page = backend.music.library_queue(&kind, id).await?;
    if page.tracks.is_empty() {
        return Err("这个收藏还没有可播放的歌曲".into());
    }
    save_library_tracks(&mut page.tracks, &backend)?;
    backend.player.send(Command::PlayNext(page.tracks))?;
    Ok(page.more)
}

#[tauri::command]
async fn add_playlist_song(
    playlist_id: u64,
    song_id: u64,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.music.add_playlist_song(playlist_id, song_id).await
}

#[tauri::command]
async fn update_library_playlist(
    id: u64,
    name: String,
    description: String,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend
        .music
        .update_library_playlist(id, &name, &description)
        .await
}

#[tauri::command]
async fn delete_library_playlist(id: u64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.music.delete_library_playlist(id).await
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
    if keys.is_empty() || keys.len() > 1000 || index >= keys.len() {
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
fn append_queue(keys: Vec<String>, backend: State<'_, Backend>) -> AppResult<()> {
    if keys.is_empty() || keys.len() > 1000 {
        return Err("待添加的歌曲无效".into());
    }
    let tracks = keys
        .iter()
        .map(|key| backend.store.track(key))
        .collect::<AppResult<Vec<_>>>()?;
    backend.player.send(Command::PlayNext(tracks))
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
fn player_seek(position_ms: u64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Seek(position_ms))
}
#[tauri::command]
fn player_volume(volume: f64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Volume(volume))
}
#[tauri::command]
fn player_device(device_id: Option<String>, backend: State<'_, Backend>) -> AppResult<()> {
    backend.player.send(Command::Device(device_id))
}
#[tauri::command]
fn audio_output_modes() -> Vec<model::AudioOutputMode> {
    crate::playback::output::available_modes()
}
#[tauri::command]
async fn set_audio_output_mode(
    mode: model::AudioOutputMode,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    let player = backend.player.clone();
    tauri::async_runtime::spawn_blocking(move || player.set_output_mode(mode))
        .await
        .map_err(|e| e.to_string())?
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
fn lyric_sources(backend: State<'_, Backend>) -> AppResult<lyrics::LyricSources> {
    backend.lyrics.sources()
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
fn set_lyric_endpoints(endpoints: Vec<String>, backend: State<'_, Backend>) -> AppResult<()> {
    if endpoints.is_empty()
        || endpoints.len() > 3
        || endpoints.iter().any(|e| {
            e.len() > 2048
                || !e.contains("{id}")
                || !url::Url::parse(&e.replace("{id}", "1"))
                    .is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some())
        })
    {
        return Err("设置 1–3 个 HTTPS 歌词地址，使用 {id} 表示歌曲 ID".into());
    }
    backend.store.set_setting(
        "lyricEndpoints",
        &serde_json::to_string(&endpoints).map_err(|e| e.to_string())?,
    )
}

#[tauri::command]
async fn qr_login(backend: State<'_, Backend>) -> AppResult<QrLogin> {
    backend.music.qr_login().await
}
#[tauri::command]
async fn poll_login(
    key: String,
    backend: State<'_, Backend>,
    plugins: State<'_, Arc<plugins::PluginManager>>,
) -> AppResult<LoginStatus> {
    let result = backend.music.poll_login(&key).await?;
    if result.code == 803 {
        plugins.account_changed().await;
    }
    Ok(result)
}
#[tauri::command]
async fn login_session(
    backend: State<'_, Backend>,
    plugins: State<'_, Arc<plugins::PluginManager>>,
) -> AppResult<bool> {
    let result = backend.music.session().await?;
    if !result {
        plugins.account_changed().await;
    }
    Ok(result)
}
#[tauri::command]
async fn account_profile(backend: State<'_, Backend>) -> AppResult<Option<AccountProfile>> {
    backend.music.profile().await
}
#[tauri::command]
async fn logout(
    backend: State<'_, Backend>,
    plugins: State<'_, Arc<plugins::PluginManager>>,
) -> AppResult<crate::music::business::LogoutReport> {
    let result = backend.music.logout().await?;
    plugins.account_changed().await;
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    if std::env::args().any(|arg| arg == "--remove-autostart") {
        if let Err(error) = autostart::cleanup() {
            eprintln!("清理开机自启失败：{error}");
            std::process::exit(1);
        }
        return;
    }
    #[cfg(not(feature = "plugin-probe"))]
    let context = tauri::generate_context!();
    #[cfg(feature = "plugin-probe")]
    let context = {
        let mut context = tauri::generate_context!();
        context.config_mut().app.windows[0].visible = false;
        context
    };
    let app = tauri::Builder::default()
        // Claim the instance before other plugins or backend initialization.
        .plugin(desktop::single_instance())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(autostart::ENTRY_NAME)
                .build(),
        )
        .register_uri_scheme_protocol("plugin", plugins::protocol)
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(main_webview::background_shortcut)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(not(feature = "plugin-probe"))]
            let data = app.path().app_data_dir()?;
            #[cfg(feature = "plugin-probe")]
            let data = plugins::probe::directory();
            std::fs::create_dir_all(&data)?;
            audio_runtime::configure(&app.path().app_cache_dir()?)?;
            let covers = app.path().app_cache_dir()?.join("covers");
            std::fs::create_dir_all(&covers)?;
            let fallback = covers.join("default.png");
            if !fallback.exists() {
                std::fs::write(&fallback, include_bytes!("../../icons/128x128.png"))?;
            }
            let store = Arc::new(storage::Store::open(&data.join("nons.sqlite3"))?);
            if store.setting("localIndexVersion")?.as_deref() != Some("1") {
                store.rebuild_local_index()?;
                store.set_setting("localIndexVersion", "1")?;
            }
            let accounts = Arc::new(crate::music::accounts::AccountManager::new(
                store.clone(),
                Arc::new(crate::music::accounts::SystemCredentials),
            )?);
            let account_events = app.handle().clone();
            accounts.on_change(Arc::new(move |source| {
                use tauri::Emitter;
                let _ = account_events.emit("music-account-changed", source);
            }));
            let (music, netease) = crate::music::bundled::service(accounts)?;
            let music = Arc::new(music);
            app.manage(Arc::new(crate::music::settings::AdapterSettings::new(
                store.clone(),
                music.adapters.clone(),
            )?));
            let cache = Arc::new(ttml_cache::TtmlCache::new(
                store.clone(),
                app.path().app_cache_dir()?,
            )?);
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
            #[cfg(feature = "plugin-probe")]
            plugins::probe::start(plugin_manager.clone());
            tauri::async_runtime::spawn(async move {
                plugin_manager.startup().await;
            });
            app.manage(Backend {
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
            desktop::setup(app)?;
            main_webview::setup(app)?;
            Ok(())
        })
        .on_window_event(desktop::on_window_event)
        .invoke_handler(tauri::generate_handler![
            autostart::autostart_status,
            autostart::set_autostart,
            desktop::close_behavior,
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
            music_sources,
            music_context,
            music_query,
            music_write,
            music_read_track,
            music_account,
            music_accounts,
            music_current_account,
            music_select_account,
            music_remove_account,
            search_music,
            search_suggestions,
            search_collections,
            music_library,
            song_information,
            remove_playlist_song,
            remove_queue_track,
            library_collections,
            discovery_playlists,
            discovery_categories,
            discovery_radar,
            discovery_tracks,
            discovery_dislike,
            library_tracks,
            music_entity_detail,
            artist_albums,
            artist_tracks,
            library_history,
            play_library_collection,
            create_library_playlist,
            append_library_collection,
            add_playlist_song,
            update_library_playlist,
            delete_library_playlist,
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
            play_queue,
            play_private_fm,
            append_private_fm,
            append_queue,
            player_action,
            player_jump,
            player_seek,
            player_volume,
            player_device,
            audio_output_modes,
            set_audio_output_mode,
            set_playback_options,
            output_devices,
            track_lyrics,
            lyric_sources,
            set_lyric_sources,
            bind_local_lyrics,
            music_options,
            set_music_options,
            set_lyric_endpoints,
            qr_login,
            poll_login,
            login_session,
            account_profile,
            liked_song_ids,
            set_song_liked,
            logout
        ])
        .build(context)
        .expect("无法启动 NonsPlayer");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            main_webview::stop(app);
            app.state::<Arc<plugins::PluginManager>>().stop();
            app.state::<Backend>().player.shutdown();
        }
    });
}
