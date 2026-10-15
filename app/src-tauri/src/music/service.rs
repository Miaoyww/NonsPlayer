//! Playback routing for uniform online identities and compatible Netease/local queues.
use super::{
    manager::{AdapterManager, ResourceTicket},
    resource::ResolveRequest,
};
use crate::model::{AppResult, ResolvedTrack, Track, TrackSource};
use std::sync::Arc;

pub type ResolvedPlayback = (ResolvedTrack, Option<Arc<ResourceTicket>>);
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
                limit: 100,
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
                if seen.insert(track.reference.clone()) && tracks.len() < 1000 {
                    tracks.push(Track::from_music(track).map_err(|e| e.to_string())?);
                }
            }
            cursor = response.page.next_cursor;
            more = cursor.is_some();
            if !more || tracks.len() >= 1000 {
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
        if matches!(track.source, TrackSource::Local { .. }) {
            return crate::playback_resource::resolve(track)
                .await
                .map(|resolved| (resolved, None));
        }
        let reference = nons_music_adapter_sdk::migration::convert_track(track.clone())
            .map_err(|e| e.to_string())?
            .track
            .reference;
        let ticket = self
            .adapters
            .resolve_playback(
                &ResolveRequest {
                    track: reference,
                    preferred_quality: quality.into(),
                    allow_downgrade: downgrade,
                },
                generation,
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok((
            ResolvedTrack {
                decoded_audio: None,
                track,
                uri: ticket.uri().into(),
                quality: Some(ticket.quality().into()),
            },
            Some(ticket),
        ))
    }
}
