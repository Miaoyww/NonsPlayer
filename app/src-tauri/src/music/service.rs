//! Playback routing for uniform online identities and compatible Netease/local queues.
use super::manager::{AdapterManager, ResourceTicket};
use crate::model::{AppResult, ResolvedTrack, Track, TrackSource};
use std::sync::Arc;

pub type ResolvedPlayback = (
    ResolvedTrack,
    Option<Arc<ResourceTicket>>,
    Option<super::manager::PlaybackSource>,
);
#[derive(Clone, Default)]
pub struct PlaybackSelection {
    pub provider: Option<String>,
    pub track: Option<Track>,
}
pub struct CollectionQueue {
    pub tracks: Vec<Track>,
    pub more: bool,
}

pub struct MusicService {
    pub adapters: Arc<AdapterManager>,
    pub accounts: Option<Arc<super::accounts::AccountManager>>,
}
impl MusicService {
    pub async fn collection_queue(
        &self,
        entity: &super::identity::EntityRef,
    ) -> AppResult<CollectionQueue> {
        self.collection_tracks(entity, 1000).await
    }
    pub(crate) async fn collection_first(
        &self,
        entity: &super::identity::EntityRef,
    ) -> AppResult<CollectionQueue> {
        self.collection_tracks(entity, 1).await
    }
    async fn collection_tracks(
        &self,
        entity: &super::identity::EntityRef,
        limit: usize,
    ) -> AppResult<CollectionQueue> {
        use super::{
            adapter::PageRequest,
            business::{BusinessRequest, BusinessResponse},
            identity::EntityKind,
        };
        if entity.kind == EntityKind::Track {
            return Err("曲目不能作为收藏展开".into());
        }
        let scope = self
            .adapters
            .cache_identity(&entity.source)
            .map_err(|e| e.to_string())?;
        let mut tracks = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut cursor = None;
        let mut more = false;
        for _ in 0..100 {
            let page = PageRequest {
                cursor: cursor.clone(),
                limit: limit.min(100) as u16,
            };
            let request = if entity.kind == EntityKind::Artist {
                BusinessRequest::ArtistTracks {
                    artist: entity.clone(),
                    page,
                }
            } else {
                BusinessRequest::Tracks {
                    entity: entity.clone(),
                    page,
                }
            };
            let BusinessResponse::Tracks(response) = self
                .adapters
                .business(&entity.source, &request)
                .await
                .map_err(|e| e.to_string())?
            else {
                return Err("收藏曲目响应无效".into());
            };
            if response.page.next_cursor.is_some() && response.page.next_cursor == cursor {
                return Err("音乐来源返回重复游标".into());
            }
            for track in response.page.items {
                if seen.insert(track.reference.clone()) && tracks.len() < limit {
                    tracks.push(Track::from_music(track).map_err(|e| e.to_string())?);
                }
            }
            cursor = response.page.next_cursor;
            more = cursor.is_some();
            if !more || tracks.len() >= limit {
                break;
            }
        }
        if self
            .adapters
            .cache_identity(&entity.source)
            .map_err(|e| e.to_string())?
            != scope
        {
            return Err("账号或音乐来源状态已变化".into());
        }
        Ok(CollectionQueue { tracks, more })
    }
    pub fn new(adapters: Arc<AdapterManager>) -> Self {
        Self {
            adapters,
            accounts: None,
        }
    }
    pub async fn resolve(
        &self,
        track: Track,
        quality: &str,
        downgrade: bool,
        generation: u64,
    ) -> AppResult<ResolvedPlayback> {
        self.resolve_selected(track, quality, downgrade, generation, None)
            .await
    }
    pub async fn resolve_selected(
        &self,
        original: Track,
        quality: &str,
        downgrade: bool,
        generation: u64,
        selection: Option<PlaybackSelection>,
    ) -> AppResult<ResolvedPlayback> {
        self.resolve_selected_notifying(
            original,
            quality,
            downgrade,
            generation,
            selection,
            &|_| {},
        )
        .await
    }
    pub async fn resolve_selected_notifying(
        &self,
        original: Track,
        quality: &str,
        downgrade: bool,
        generation: u64,
        selection: Option<PlaybackSelection>,
        notify: &(dyn Fn(super::manager::PlaybackSource) + Send + Sync),
    ) -> AppResult<ResolvedPlayback> {
        let original_key = original.key.clone();
        let mut track = selection
            .as_ref()
            .and_then(|s| s.track.clone())
            .unwrap_or(original);
        if matches!(track.source, TrackSource::Local { .. }) {
            return crate::playback_resource::resolve(track)
                .await
                .map(|resolved| (resolved, None, None));
        }
        let mut metadata = nons_music_adapter_sdk::migration::convert_track(track.clone())
            .map_err(|e| e.to_string())?
            .track;
        // Some searchable audio services omit duration. Hydrate only the selected
        // track through the ordinary catalog contract, never every search result.
        if metadata.duration_ms == 0 {
            if let Ok(enriched) =
                (self.adapters.read_track(&metadata.reference).await).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_success", error);
                })
            {
                metadata = enriched;
                track.duration_ms = metadata.duration_ms;
            }
        }
        let (ticket, mut source) = self
            .adapters
            .resolve_with_providers_notifying(
                &super::provider::ResourceRequest {
                    track: metadata,
                    preferred_quality: quality.into(),
                    allow_downgrade: downgrade,
                },
                selection.as_ref().and_then(|s| s.provider.as_deref()),
                generation,
                &|mut source| {
                    source.changed |= track.key != original_key;
                    notify(source);
                },
            )
            .await
            .map_err(|e| e.to_string())?;
        source.changed |= track.key != original_key;
        Ok((
            ResolvedTrack {
                decoded_audio: None,
                track,
                uri: ticket.uri().into(),
                quality: Some(ticket.quality().into()),
            },
            Some(ticket),
            Some(source),
        ))
    }
}
