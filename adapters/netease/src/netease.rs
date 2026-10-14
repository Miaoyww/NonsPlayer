//! Built-in provider and legacy DTO compatibility stay at the platform boundary.
use super::{
    account::SessionContext, adapter::*, identity::*, resource::*, ErrorCode, MusicResult,
};
use crate::{
    model::{Track, TrackSource},
    platform::Netease,
};
use std::sync::Arc;

pub struct NeteaseAdapter {
    pub(super) client: Arc<Netease>,
    descriptor: SourceDescriptor,
    credential_queries: Arc<tokio::sync::Semaphore>,
    credential_playback: Arc<tokio::sync::Semaphore>,
}
impl NeteaseAdapter {
    pub(super) async fn scoped_client(&self, context: &RequestContext) -> MusicResult<Netease> {
        self.scoped_with_budget(context, false).await
    }
    async fn scoped_with_budget(
        &self,
        context: &RequestContext,
        playback: bool,
    ) -> MusicResult<Netease> {
        let budget = if playback {
            &self.credential_playback
        } else {
            &self.credential_queries
        };
        let permit = budget
            .clone()
            .try_acquire_owned()
            .map_err(|_| ErrorCode::RateLimited)?;
        let client = self.client.clone();
        let context = context.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            client.scoped(&context)
        })
        .await
        .map_err(|_| ErrorCode::Internal)?
    }
    async fn invalidate_on_error<T>(
        &self,
        result: MusicResult<T>,
        context: &RequestContext,
    ) -> MusicResult<T> {
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::Unauthenticated)
        {
            if let Some(accounts) = &self.client.accounts {
                let _ = accounts.invalidate(context.session.clone()).await;
            } else {
                self.client
                    .invalidate_session(context.session.generation)
                    .await;
            }
        }
        result
    }
    pub fn new(client: Arc<Netease>) -> Self {
        Self {
            client,
            credential_queries: Arc::new(tokio::sync::Semaphore::new(4)),
            credential_playback: Arc::new(tokio::sync::Semaphore::new(2)),
            descriptor: SourceDescriptor {
                source: SourceId::try_from("netease".to_owned()).expect("reserved source"),
                display_name: "网易云音乐".into(),
                contract_version: super::CONTRACT_VERSION,
                cover_hosts: ["*.music.126.net".into(), "*.music.163.com".into()]
                    .into_iter()
                    .collect(),
                share_link_templates: [
                    (
                        EntityKind::Track,
                        "https://music.163.com/#/song?id={id}".into(),
                    ),
                    (
                        EntityKind::Playlist,
                        "https://music.163.com/#/playlist?id={id}".into(),
                    ),
                    (
                        EntityKind::Album,
                        "https://music.163.com/#/album?id={id}".into(),
                    ),
                    (
                        EntityKind::Artist,
                        "https://music.163.com/#/artist?id={id}".into(),
                    ),
                ]
                .into_iter()
                .collect(),
                // Only the implemented optional groups are advertised.
                capabilities: [
                    Capability::Search,
                    Capability::Browse,
                    Capability::Account,
                    Capability::UserLibrary,
                    Capability::Favorites,
                    Capability::PlaylistWrite,
                    Capability::Recommendations,
                    Capability::PrivateFm,
                ]
                .into_iter()
                .collect(),
            },
        }
    }
}
pub fn numeric_id(reference: &EntityRef) -> MusicResult<u64> {
    let id = reference
        .id
        .as_str()
        .parse::<u64>()
        .map_err(|_| ErrorCode::InvalidData)?;
    if id == 0 || id > 9_007_199_254_740_991 || id.to_string() != reference.id.as_str() {
        return Err(ErrorCode::InvalidData.into());
    }
    Ok(id)
}
impl MusicAdapter for NeteaseAdapter {
    fn descriptor(&self) -> &SourceDescriptor {
        &self.descriptor
    }
    fn business<'a>(
        &'a self,
        request: &'a super::business::BusinessRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, super::business::BusinessResponse> {
        Box::pin(async move {
            let result = super::netease_business::business(self, request, context).await;
            self.invalidate_on_error(result, context).await
        })
    }
    fn account<'a>(
        &'a self,
        request: &'a super::business::AccountRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, super::business::AccountOutcome> {
        Box::pin(async move {
            super::netease_business::account(&self.scoped_client(context).await?, request, context)
                .await
        })
    }
    fn prepare_logout<'a>(
        &'a self,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, LogoutAction> {
        Box::pin(async move {
            let client = self.scoped_client(context).await?;
            Ok(Box::pin(async move { client.remote_logout().await }) as LogoutAction)
        })
    }
    fn read_track<'a>(
        &'a self,
        reference: &'a EntityRef,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, MusicTrack> {
        Box::pin(async move {
            context.check_reference(reference)?;
            let client = self.scoped_client(context).await?;
            let result = client.song(numeric_id(reference)?).await;
            let track = self.invalidate_on_error(result, context).await?;
            super::migration::legacy_track(
                &serde_json::to_string(&track).map_err(|_| ErrorCode::InvalidData)?,
            )
            .map(|v| v.track)
        })
    }
    fn resolve_playback<'a>(
        &'a self,
        request: &'a ResolveRequest,
        context: &'a RequestContext,
    ) -> AdapterFuture<'a, PlaybackResource> {
        Box::pin(async move {
            request.validate(context)?;
            let result = self
                .scoped_with_budget(context, true)
                .await?
                .resolve_resource(
                    numeric_id(&request.track)?,
                    &request.preferred_quality,
                    request.allow_downgrade,
                )
                .await;
            self.invalidate_on_error(result, context).await
        })
    }
}
pub fn session(client: &Netease) -> SessionContext {
    let source = SourceId::try_from("netease".to_owned()).unwrap();
    if let Some(accounts) = &client.accounts {
        return accounts.session();
    }
    SessionContext {
        source,
        account: None,
        generation: client.session_generation(),
    }
}
pub fn reference(id: u64) -> MusicResult<EntityRef> {
    Ok(EntityRef {
        source: SourceId::try_from("netease".to_owned())?,
        kind: EntityKind::Track,
        id: OpaqueId::try_from(id.to_string())?,
    })
}
pub fn legacy(track: MusicTrack) -> MusicResult<Track> {
    let id = numeric_id(&track.reference)?;
    Ok(Track {
        key: format!("netease:{id}"),
        title: track.title,
        aliases: track.aliases,
        artist: track.artist,
        artists: track
            .artists
            .into_iter()
            .map(|credit| crate::model::MusicCredit {
                name: credit.name,
                id: credit.reference.and_then(|r| r.id.as_str().parse().ok()),
            })
            .collect(),
        album: track.album,
        album_id: track
            .album_reference
            .and_then(|r| r.id.as_str().parse().ok()),
        duration_ms: track.duration_ms,
        cover: track.cover,
        source: TrackSource::Netease { id },
    })
}
