//! Frontend endpoints expose metadata and selections, never URLs or credentials.
use super::{identity::SourceId, manager::ResourceProviderStatus, provider::ResourceSearchRequest};
use crate::{
    application::Backend,
    model::{AppResult, Track},
    player::Command,
};
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub(crate) fn resource_provider_list(
    backend: State<'_, Backend>,
) -> AppResult<Vec<ResourceProviderStatus>> {
    backend
        .music
        .adapters
        .resource_provider_statuses()
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) async fn resource_provider_set_enabled(
    id: SourceId,
    enabled: bool,
    settings: State<'_, std::sync::Arc<super::settings::AdapterSettings>>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    let settings = settings.inner().clone();
    let changed = id.clone();
    tauri::async_runtime::spawn_blocking(move || settings.set_resource_enabled(id, enabled))
        .await
        .map_err(|_| "音源设置任务失败")??;
    use tauri::Emitter;
    let _ = app.emit("adapters-changed", changed);
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchPage {
    items: Vec<Track>,
    next_cursor: Option<super::adapter::Cursor>,
}
#[tauri::command]
pub(crate) async fn resource_search(
    id: SourceId,
    request: ResourceSearchRequest,
    backend: State<'_, Backend>,
) -> AppResult<SearchPage> {
    let scope = backend
        .music
        .adapters
        .stamp(&id)
        .map_err(|e| e.to_string())?;
    let page = backend
        .music
        .adapters
        .search_resource_provider(&id, &request)
        .await
        .map_err(|e| e.to_string())?;
    let mut tracks = page
        .items
        .into_iter()
        .map(Track::from_music)
        .collect::<super::MusicResult<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    for track in &mut tracks {
        if track.duration_ms == 0 {
            if let Ok(existing) = backend.store.track(&track.key) {
                if existing.source_id() == track.source_id() && existing.duration_ms > 0 {
                    *track = existing;
                }
            }
        }
    }
    if backend
        .music
        .adapters
        .stamp(&id)
        .map_err(|e| e.to_string())?
        != scope
    {
        return Err("音源状态已变化".into());
    }
    backend.store.save_tracks(&tracks)?;
    Ok(SearchPage {
        items: tracks,
        next_cursor: page.next_cursor,
    })
}
#[tauri::command]
pub(crate) async fn player_select_source(
    revision: u64,
    provider: Option<String>,
    alternate_key: Option<String>,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    let snapshot = backend.player.snapshot()?;
    if snapshot.revision != revision {
        return Err("播放歌曲已变化，请重新选择".into());
    }
    let current = snapshot.current().ok_or("没有正在播放的歌曲")?;
    if matches!(current.source, crate::model::TrackSource::Local { .. }) {
        return Err("本地歌曲不切换在线音源".into());
    }
    let track = alternate_key
        .map(|key| backend.store.track(&key))
        .transpose()?;
    let target = track.as_ref().unwrap_or(current);
    if let Some(id) = provider.as_deref().filter(|id| *id != "official") {
        if !backend
            .music
            .adapters
            .supports_audio_provider(
                id,
                &SourceId::try_from(target.source_id().to_owned()).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?
        {
            return Err("所选音源不可用或不支持这个平台".into());
        }
    }
    if matches!(target.source, crate::model::TrackSource::Local { .. }) {
        return Err("请选择在线歌曲".into());
    }
    let (reply, response) = tokio::sync::oneshot::channel();
    backend.player.send(Command::SelectSource {
        revision,
        selection: super::service::PlaybackSelection { provider, track },
        reply,
    })?;
    response.await.map_err(|_| "音源切换任务已结束")?
}
