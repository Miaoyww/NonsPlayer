//! Playback routing for uniform online identities and compatible Netease/local queues.
use super::{
    manager::{AdapterManager, ResourceTicket},
    resource::ResolveRequest,
};
use crate::model::{AppResult, ResolvedTrack, Track, TrackSource};
use std::sync::Arc;

pub type ResolvedPlayback = (ResolvedTrack, Option<Arc<ResourceTicket>>);

pub struct MusicService {
    pub adapters: Arc<AdapterManager>,
    pub accounts: Option<Arc<super::accounts::AccountManager>>,
}
impl MusicService {
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
