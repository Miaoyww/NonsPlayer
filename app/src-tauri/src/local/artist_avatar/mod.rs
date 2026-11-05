mod matching;
mod store;
#[cfg(test)]
mod tests;

use crate::{
    application::Backend,
    infrastructure::covers::CoverRuntime,
    model::AppResult,
    music::{
        adapter::Capability,
        identity::{EntityKind, EntityRef, OpaqueId, SourceId},
        service::MusicService,
    },
    storage::Store,
};
use nons_music_adapter_sdk::cover::{CoverImage, CoverSize};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, State};

const DEFAULT_SOURCE: &str = "netease";
const DELAY_KEY: &str = "localArtistAvatarRequestDelayMs";
const DEFAULT_DELAY_MS: u64 = 1_500;
const MAX_DELAY_MS: u64 = 5_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ArtistAvatarStatus {
    Auto,
    Manual,
    NotFound,
    #[default]
    Pending,
    Default,
    Failed,
}

impl ArtistAvatarStatus {
    pub(super) fn from_storage(value: &str) -> Self {
        match value {
            "auto" => Self::Auto,
            "manual" => Self::Manual,
            "notFound" => Self::NotFound,
            "pending" => Self::Pending,
            "default" => Self::Default,
            _ => Self::Failed,
        }
    }

    pub(super) const fn storage(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::NotFound => "notFound",
            Self::Pending => "pending",
            Self::Default => "default",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct ArtistAvatarRecord {
    pub artist_id: String,
    pub status: ArtistAvatarStatus,
    pub source: Option<String>,
    pub platform_id: Option<String>,
    pub platform_name: Option<String>,
    pub cover_variants: Option<String>,
    pub cover_size: Option<String>,
    pub avatar_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub error: Option<String>,
    pub updated_at: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtistAvatarItem {
    pub id: String,
    pub name: String,
    pub cover: String,
    pub track_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ArtistAvatarStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtistAvatarPage {
    pub items: Vec<ArtistAvatarItem>,
    pub more: bool,
    pub total: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtistAvatarCandidate {
    pub source: String,
    pub platform_id: String,
    pub name: String,
    pub subtitle: String,
    pub track_count: u64,
    pub cover: CoverImage,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtistAvatarOptions {
    pub delay_ms: u64,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtistAvatarProgress {
    pub running: bool,
    pub delay_ms: u64,
    pub total: u64,
    pub processed: u64,
    pub matched: u64,
    pub pending: u64,
    pub not_found: u64,
    pub failed: u64,
    pub skipped: u64,
    pub current: Option<String>,
    pub error: Option<String>,
}

#[derive(Default)]
struct JobState {
    progress: ArtistAvatarProgress,
    generation: u64,
}

#[derive(Default)]
pub(crate) struct ArtistAvatarJob(Mutex<JobState>);

impl ArtistAvatarJob {
    fn status(&self) -> AppResult<ArtistAvatarProgress> {
        self.0
            .lock()
            .map(|state| state.progress.clone())
            .map_err(|_| "艺术家头像任务状态不可用".into())
    }

    fn begin(&self, total: u64, delay_ms: u64) -> AppResult<ArtistAvatarProgress> {
        let mut state = self.0.lock().map_err(|_| "艺术家头像任务状态不可用")?;
        if state.progress.running {
            return Err("艺术家头像补齐任务正在进行".into());
        }
        state.generation = state.generation.wrapping_add(1);
        state.progress = ArtistAvatarProgress {
            running: true,
            delay_ms,
            total,
            ..Default::default()
        };
        Ok(state.progress.clone())
    }

    fn publish(&self, app: &tauri::AppHandle, progress: &ArtistAvatarProgress) {
        if let Ok(mut state) = self.0.lock() {
            state.progress = progress.clone();
        }
        if let Err(error) = app.emit("local-artist-avatar-progress", progress) {
            crate::infrastructure::diagnostics::warn("local.artist_avatar.progress", error);
        }
    }
}

#[tauri::command]
pub(crate) fn local_artist_avatar_options(
    backend: State<'_, Backend>,
) -> AppResult<ArtistAvatarOptions> {
    Ok(ArtistAvatarOptions {
        delay_ms: request_delay(&backend.store)?,
    })
}

#[tauri::command]
pub(crate) fn local_artist_avatar_set_delay(
    delay_ms: u64,
    backend: State<'_, Backend>,
) -> AppResult<ArtistAvatarOptions> {
    validate_delay(delay_ms)?;
    backend
        .store
        .set_setting(DELAY_KEY, &delay_ms.to_string())?;
    Ok(ArtistAvatarOptions { delay_ms })
}

#[tauri::command]
pub(crate) async fn local_artist_avatars(
    keyword: String,
    offset: u32,
    backend: State<'_, Backend>,
) -> AppResult<ArtistAvatarPage> {
    let keyword = keyword.trim().to_owned();
    if keyword.len() > 256 {
        return Err("艺术家筛选词过长".into());
    }
    let store = backend.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (items, more, total) = store.artist_avatar_page(&keyword, offset)?;
        Ok(ArtistAvatarPage { items, more, total })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) fn local_artist_avatar_status(
    backend: State<'_, Backend>,
) -> AppResult<ArtistAvatarProgress> {
    backend.artist_avatars.status()
}

#[tauri::command]
pub(crate) async fn local_artist_avatar_candidates(
    keyword: String,
    source: Option<String>,
    backend: State<'_, Backend>,
) -> AppResult<Vec<ArtistAvatarCandidate>> {
    let source = source_id(source)?;
    validate_source(&backend.music, &source)?;
    let keyword = keyword.trim();
    if keyword.is_empty() || keyword.len() > 256 {
        return Err("请输入不超过 256 字节的艺术家名称".into());
    }
    let items = matching::search_candidates(&backend.music, &source, keyword).await?;
    Ok(items
        .into_iter()
        .map(|item| ArtistAvatarCandidate {
            source: item.reference.source.as_str().to_owned(),
            platform_id: item.reference.id.as_str().to_owned(),
            name: item.name,
            subtitle: item.subtitle,
            track_count: item.track_count,
            cover: item.cover,
        })
        .collect())
}

#[tauri::command]
pub(crate) async fn local_artist_avatar_start(
    artist_ids: Vec<String>,
    all: bool,
    keyword: Option<String>,
    source: Option<String>,
    force: Option<bool>,
    delay_ms: Option<u64>,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<ArtistAvatarProgress> {
    let source = source_id(source)?;
    validate_source(&backend.music, &source)?;
    let force = force.unwrap_or(false);
    let store = backend.store.clone();
    let delay_ms = match delay_ms {
        Some(value) => {
            validate_delay(value)?;
            store.set_setting(DELAY_KEY, &value.to_string())?;
            value
        }
        None => request_delay(&store)?,
    };
    let keyword = keyword.unwrap_or_default().trim().to_owned();
    if keyword.len() > 256 {
        return Err("艺术家筛选词过长".into());
    }
    let targets = {
        let store = store.clone();
        let keyword = keyword.clone();
        tauri::async_runtime::spawn_blocking(move || {
            store.artist_avatar_targets(&artist_ids, all, force, &keyword)
        })
        .await
        .map_err(|e| e.to_string())??
    };
    if targets.is_empty() {
        return Err("没有需要补齐头像的艺术家".into());
    }
    let initial = backend
        .artist_avatars
        .begin(targets.len() as u64, delay_ms)?;
    let response = initial.clone();
    let job = backend.artist_avatars.clone();
    let music = backend.music.clone();
    let avatars = backend.avatars.clone();
    let runtime = CoverRuntime::new(
        backend.music.adapters.clone(),
        backend.cover_client.clone(),
        backend.cover_requests.clone(),
        backend.online_covers.clone(),
    );
    let worker_app = app.clone();
    tauri::async_runtime::spawn(async move {
        run_job(
            &worker_app,
            &store,
            &music,
            &runtime,
            &avatars,
            &source,
            targets,
            force,
            &job,
            initial,
        )
        .await;
    });
    Ok(response)
}

#[tauri::command]
pub(crate) async fn local_artist_avatar_select(
    artist_id: String,
    source: Option<String>,
    platform_id: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    let source = source_id(source)?;
    validate_source(&backend.music, &source)?;
    let store = backend.store.clone();
    let artists = backend.avatars.clone();
    let runtime = CoverRuntime::new(
        backend.music.adapters.clone(),
        backend.cover_client.clone(),
        backend.cover_requests.clone(),
        backend.online_covers.clone(),
    );
    let music = backend.music.clone();
    select_avatar(
        &app,
        &store,
        &music,
        &runtime,
        &artists,
        &artist_id,
        &source,
        &platform_id,
    )
    .await
}

#[tauri::command]
pub(crate) fn local_artist_avatar_restore(
    artist_id: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    if backend.store.artist_avatar_name(&artist_id)?.is_none() {
        return Err("本地艺术家不存在".into());
    }
    backend.store.set_artist_avatar_default(&artist_id)?;
    emit_artist_updated(&app, &artist_id);
    emit_library_changed(&app);
    Ok(())
}

async fn run_job(
    app: &tauri::AppHandle,
    store: &Arc<Store>,
    music: &Arc<MusicService>,
    runtime: &CoverRuntime,
    avatars: &Path,
    source: &SourceId,
    targets: Vec<String>,
    force: bool,
    job: &Arc<ArtistAvatarJob>,
    mut progress: ArtistAvatarProgress,
) {
    let mut last_publish = Instant::now();
    let mut pacer = RequestPacer::new(Duration::from_millis(progress.delay_ms));
    for artist_id in targets {
        let started_at = crate::storage::now_millis();
        progress.current = Some(artist_id.clone());
        if last_publish.elapsed() >= Duration::from_millis(200) {
            job.publish(app, &progress);
            last_publish = Instant::now();
        }
        match process_auto(
            app, store, music, runtime, avatars, source, &artist_id, force, started_at, &mut pacer,
        )
        .await
        {
            Ok(Outcome::Matched) => progress.matched += 1,
            Ok(Outcome::Pending) => progress.pending += 1,
            Ok(Outcome::NotFound) => progress.not_found += 1,
            Ok(Outcome::Skipped) => progress.skipped += 1,
            Err(error) => {
                progress.failed += 1;
                crate::infrastructure::diagnostics::warn("local.artist_avatar.match", &error);
                if let Err(error) =
                    mark_failure(store, &artist_id, source, &error, force, started_at)
                {
                    crate::infrastructure::diagnostics::warn(
                        "local.artist_avatar.failure_record",
                        error,
                    );
                }
            }
        }
        progress.processed += 1;
        if last_publish.elapsed() >= Duration::from_millis(200) {
            job.publish(app, &progress);
            last_publish = Instant::now();
        }
    }
    progress.current = None;
    progress.running = false;
    job.publish(app, &progress);
    emit_library_changed(app);
}

struct RequestPacer {
    interval: Duration,
    last: Instant,
}

impl RequestPacer {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: Instant::now() - interval,
        }
    }

    async fn wait(&mut self) {
        if self.interval.is_zero() {
            return;
        }
        let elapsed = self.last.elapsed();
        if elapsed < self.interval {
            tokio::time::sleep(self.interval - elapsed).await;
        }
        self.last = Instant::now();
    }
}

enum Outcome {
    Matched,
    Pending,
    NotFound,
    Skipped,
}

async fn process_auto(
    app: &tauri::AppHandle,
    store: &Arc<Store>,
    music: &Arc<MusicService>,
    runtime: &CoverRuntime,
    avatars: &Path,
    source: &SourceId,
    artist_id: &str,
    force: bool,
    started_at: i64,
    pacer: &mut RequestPacer,
) -> AppResult<Outcome> {
    let name = store
        .artist_avatar_name(artist_id)?
        .ok_or("本地艺术家不存在")?;
    pacer.wait().await;
    let candidates = matching::search_candidates(music, source, &name).await?;
    match matching::decide_match(candidates, &name) {
        matching::MatchDecision::NotFound => {
            let record = ArtistAvatarRecord {
                artist_id: artist_id.to_owned(),
                status: ArtistAvatarStatus::NotFound,
                source: Some(source.as_str().to_owned()),
                error: Some("未找到匹配的艺术家".into()),
                ..Default::default()
            };
            if save_auto_result(store, &record, force, started_at)? {
                Ok(Outcome::NotFound)
            } else {
                Ok(Outcome::Skipped)
            }
        }
        matching::MatchDecision::Pending(count) => {
            let record = ArtistAvatarRecord {
                artist_id: artist_id.to_owned(),
                status: ArtistAvatarStatus::Pending,
                source: Some(source.as_str().to_owned()),
                error: Some(format!("找到 {count} 个候选艺术家，请手动确认")),
                ..Default::default()
            };
            if save_auto_result(store, &record, force, started_at)? {
                Ok(Outcome::Pending)
            } else {
                Ok(Outcome::Skipped)
            }
        }
        matching::MatchDecision::Exact(candidate) => {
            let platform_id = candidate.reference.id.as_str().to_owned();
            pacer.wait().await;
            let detail = matching::artist_detail(music, source, candidate.reference).await?;
            let cover = detail.cover;
            match matching::preferred_cover(&cover) {
                Some((url, size)) => {
                    pacer.wait().await;
                    let bytes = runtime.read(url.to_owned(), app).await?;
                    let (path, width, height) = store_avatar(avatars, bytes).await?;
                    let record = ArtistAvatarRecord {
                        artist_id: artist_id.to_owned(),
                        status: ArtistAvatarStatus::Auto,
                        source: Some(source.as_str().to_owned()),
                        platform_id: Some(platform_id),
                        platform_name: Some(detail.name),
                        cover_variants: Some(
                            serde_json::to_string(&cover).map_err(|e| e.to_string())?,
                        ),
                        cover_size: Some(size_name(size).to_owned()),
                        avatar_path: Some(path.clone()),
                        width: Some(width),
                        height: Some(height),
                        error: None,
                        updated_at: 0,
                    };
                    if save_auto_result(store, &record, force, started_at)? {
                        Ok(Outcome::Matched)
                    } else {
                        remove_unreferenced_avatar(store, avatars, &path)?;
                        Ok(Outcome::Skipped)
                    }
                }
                None => {
                    let record = ArtistAvatarRecord {
                        artist_id: artist_id.to_owned(),
                        status: ArtistAvatarStatus::NotFound,
                        source: Some(source.as_str().to_owned()),
                        platform_id: Some(platform_id),
                        platform_name: Some(detail.name),
                        error: Some("来源未提供艺术家头像".into()),
                        ..Default::default()
                    };
                    if save_auto_result(store, &record, force, started_at)? {
                        Ok(Outcome::NotFound)
                    } else {
                        Ok(Outcome::Skipped)
                    }
                }
            }
        }
    }
}

async fn select_avatar(
    app: &tauri::AppHandle,
    store: &Arc<Store>,
    music: &Arc<MusicService>,
    runtime: &CoverRuntime,
    avatars: &Path,
    artist_id: &str,
    source: &SourceId,
    platform_id: &str,
) -> AppResult<()> {
    if store.artist_avatar_name(artist_id)?.is_none() {
        return Err("本地艺术家不存在".into());
    }
    let reference = EntityRef {
        source: source.clone(),
        kind: EntityKind::Artist,
        id: OpaqueId::try_from(platform_id.to_owned())
            .map_err(|_| "平台艺术家 ID 无效".to_owned())?,
    };
    let detail = matching::artist_detail(music, source, reference).await?;
    let cover = detail.cover;
    let (url, size) = matching::preferred_cover(&cover).ok_or("来源未提供艺术家头像")?;
    let bytes = runtime.read(url.to_owned(), app).await?;
    let (path, width, height) = store_avatar(avatars, bytes).await?;
    let previous = store.artist_avatar_record(artist_id)?;
    let record = ArtistAvatarRecord {
        artist_id: artist_id.to_owned(),
        status: ArtistAvatarStatus::Manual,
        source: Some(source.as_str().to_owned()),
        platform_id: Some(platform_id.to_owned()),
        platform_name: Some(detail.name),
        cover_variants: Some(serde_json::to_string(&cover).map_err(|e| e.to_string())?),
        cover_size: Some(size_name(size).to_owned()),
        avatar_path: Some(path.clone()),
        width: Some(width),
        height: Some(height),
        error: None,
        updated_at: 0,
    };
    if let Err(error) = store.save_artist_avatar_replace(&record) {
        remove_unreferenced_avatar(store, avatars, &path)?;
        return Err(error);
    }
    if let Some(previous) = previous.and_then(|value| value.avatar_path) {
        remove_old_avatar(store, avatars, artist_id, &previous, &path)?;
    }
    emit_artist_updated(app, artist_id);
    emit_library_changed(app);
    Ok(())
}

async fn store_avatar(avatars: &Path, bytes: Vec<u8>) -> AppResult<(String, u32, u32)> {
    std::fs::create_dir_all(avatars).map_err(|e| e.to_string())?;
    let avatars = avatars.to_path_buf();
    tauri::async_runtime::spawn_blocking(move || {
        crate::local::cover_art::asset_with_size(&bytes, &avatars)
            .ok_or_else(|| "艺术家头像无法解码".to_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn mark_failure(
    store: &Store,
    artist_id: &str,
    source: &SourceId,
    error: &str,
    force: bool,
    started_at: i64,
) -> AppResult<()> {
    let current = store.artist_avatar_record(artist_id)?;
    if current
        .as_ref()
        .is_some_and(|value| value.updated_at > started_at)
    {
        return Ok(());
    }
    if current.as_ref().is_some_and(|value| {
        value
            .avatar_path
            .as_deref()
            .is_some_and(|path| !path.is_empty())
    }) {
        return store.set_artist_avatar_error(artist_id, error);
    }
    let record = ArtistAvatarRecord {
        artist_id: artist_id.to_owned(),
        status: ArtistAvatarStatus::Failed,
        source: Some(source.as_str().to_owned()),
        error: Some(error.to_owned()),
        ..Default::default()
    };
    if force {
        store.save_artist_avatar_replace(&record)?;
    } else {
        store.save_artist_avatar_auto(&record)?;
    }
    Ok(())
}

fn save_auto_result(
    store: &Store,
    record: &ArtistAvatarRecord,
    force: bool,
    started_at: i64,
) -> AppResult<bool> {
    if force {
        if let Some(current) = store.artist_avatar_record(&record.artist_id)? {
            if current.updated_at > started_at {
                return Ok(false);
            }
            if current.status == ArtistAvatarStatus::Manual
                && record.status != ArtistAvatarStatus::Auto
            {
                store.set_artist_avatar_error(
                    &record.artist_id,
                    record
                        .error
                        .as_deref()
                        .unwrap_or("重新匹配未得到可用的自动结果"),
                )?;
                return Ok(false);
            }
        }
        store.save_artist_avatar_replace(record)?;
        Ok(true)
    } else {
        store.save_artist_avatar_auto(record)
    }
}

fn remove_old_avatar(
    store: &Store,
    avatars: &Path,
    artist_id: &str,
    previous: &str,
    current: &str,
) -> AppResult<()> {
    if previous == current || previous.is_empty() {
        return Ok(());
    }
    let path = Path::new(previous);
    if !path.starts_with(avatars) || store.artist_avatar_path_references(artist_id, previous)? > 0 {
        return Ok(());
    }
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn remove_unreferenced_avatar(store: &Store, avatars: &Path, path: &str) -> AppResult<()> {
    let path_value = Path::new(path);
    if !path_value.starts_with(avatars) || store.artist_avatar_path_referenced(path)? > 0 {
        return Ok(());
    }
    match std::fs::remove_file(path_value) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn size_name(size: CoverSize) -> &'static str {
    match size {
        CoverSize::Small => "small",
        CoverSize::Normal => "normal",
        CoverSize::Large => "large",
        CoverSize::Exlarge => "exlarge",
        CoverSize::Original => "original",
    }
}

fn source_id(source: Option<String>) -> AppResult<SourceId> {
    SourceId::try_from(source.unwrap_or_else(|| DEFAULT_SOURCE.to_owned()))
        .map_err(|_| "音乐来源无效".to_owned())
}

fn request_delay(store: &Store) -> AppResult<u64> {
    let value = store
        .setting(DELAY_KEY)?
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DELAY_MS)
        .min(MAX_DELAY_MS);
    Ok(value)
}

fn validate_delay(delay_ms: u64) -> AppResult<()> {
    if delay_ms > MAX_DELAY_MS {
        return Err(format!("请求间隔不能超过 {MAX_DELAY_MS} 毫秒").into());
    }
    Ok(())
}

fn validate_source(music: &MusicService, source: &SourceId) -> AppResult<()> {
    let descriptors = music.adapters.descriptors().map_err(|e| e.to_string())?;
    let descriptor = descriptors
        .iter()
        .find(|descriptor| &descriptor.source == source)
        .ok_or("音乐来源不可用")?;
    if !descriptor.capabilities.contains(&Capability::Search)
        || !descriptor.capabilities.contains(&Capability::Browse)
    {
        return Err("当前音乐来源不支持艺术家头像匹配".into());
    }
    Ok(())
}

fn emit_artist_updated(app: &tauri::AppHandle, artist_id: &str) {
    if let Err(error) = app.emit(
        "local-artist-avatar-updated",
        serde_json::json!({"artistId": artist_id}),
    ) {
        crate::infrastructure::diagnostics::warn("local.artist_avatar.updated", error);
    }
}

fn emit_library_changed(app: &tauri::AppHandle) {
    if let Err(error) = app.emit(
        "local-library-updated",
        serde_json::json!({"scanning":false}),
    ) {
        crate::infrastructure::diagnostics::warn("local.artist_avatar.library_changed", error);
    }
}
