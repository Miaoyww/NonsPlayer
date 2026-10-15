//! All legacy numeric DTO conversion is confined to the built-in provider boundary.
use super::{
    account::*,
    adapter::*,
    business::*,
    identity::*,
    netease::{numeric_id, NeteaseAdapter},
    ErrorCode, MusicResult,
};
use crate::{model::Track, platform as legacy};

pub fn kind(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Track => "track",
        EntityKind::Playlist => "playlist",
        EntityKind::Album => "album",
        EntityKind::Artist => "artist",
    }
}
pub fn reference(kind: EntityKind, id: u64) -> MusicResult<EntityRef> {
    Ok(EntityRef {
        source: SourceId::try_from("netease".to_owned())?,
        kind,
        id: OpaqueId::try_from(id.to_string())?,
    })
}
pub(super) fn track(mut value: Track) -> MusicResult<MusicTrack> {
    value.cover = crate::cover::image(&value.cover);
    super::migration::legacy_track(
        &serde_json::to_string(&value).map_err(|_| ErrorCode::InvalidData)?,
    )
    .map(|v| v.track)
}
pub(super) fn entity(value: legacy::Collection) -> MusicResult<MusicEntity> {
    let entity_kind = match value.kind.as_str() {
        "playlist" => EntityKind::Playlist,
        "album" => EntityKind::Album,
        "artist" => EntityKind::Artist,
        _ => return Err(ErrorCode::InvalidData.into()),
    };
    Ok(MusicEntity {
        reference: reference(entity_kind, value.id)?,
        name: value.name,
        cover: value.cover,
        subtitle: value.subtitle,
        track_count: value.track_count,
        creator: if value.creator_id == 0 {
            None
        } else {
            Some(AccountRef {
                source: SourceId::try_from("netease".to_owned())?,
                id: OpaqueId::try_from(value.creator_id.to_string())?,
            })
        },
        liked: value.liked,
        play_count: value.play_count,
        published_at: value.published_at,
        artists: value
            .artists
            .into_iter()
            .map(|c| {
                Ok(MusicCredit {
                    name: c.name,
                    reference: c
                        .id
                        .map(|id| reference(EntityKind::Artist, id))
                        .transpose()?,
                })
            })
            .collect::<MusicResult<_>>()?,
    })
}
fn record(profile: legacy::AccountProfile) -> MusicResult<AccountRecord> {
    Ok(AccountRecord {
        reference: AccountRef {
            source: SourceId::try_from("netease".to_owned())?,
            id: OpaqueId::try_from(profile.user_id.to_string())?,
        },
        display_name: profile.nickname,
        avatar: profile.avatar_url,
    })
}
fn library_summary(result: legacy::LibrarySummary) -> MusicResult<LibrarySummary> {
    Ok(LibrarySummary {
        account: record(result.profile)?,
        liked_playlist: result.liked_playlist.map(entity).transpose()?,
        liked_tracks: result
            .liked_tracks
            .into_iter()
            .map(track)
            .collect::<MusicResult<_>>()?,
        liked_error: result.liked_error.map(error),
    })
}

/// Stable public error codes; legacy human text is never passed through the uniform interface.
fn error(message: String) -> super::MusicError {
    log::warn!(target: "nons_adapter_netease::business", "platform_failure reason={message}");
    for code in [
        ErrorCode::Unauthenticated,
        ErrorCode::NotFound,
        ErrorCode::PermissionDenied,
        ErrorCode::RegionRestricted,
        ErrorCode::RateLimited,
        ErrorCode::Network,
        ErrorCode::Unsupported,
        ErrorCode::SourceUnavailable,
        ErrorCode::Cancelled,
        ErrorCode::DeadlineExceeded,
        ErrorCode::StaleContext,
        ErrorCode::InvalidData,
        ErrorCode::Internal,
    ] {
        let error = super::MusicError::from(code);
        if error.to_string() == message {
            return error;
        }
    }
    let code = if message.contains("登录") {
        ErrorCode::Unauthenticated
    } else if message.contains("超时") {
        ErrorCode::DeadlineExceeded
    } else if message.contains("网络") || message.contains("请求失败") {
        ErrorCode::Network
    } else if message.contains("缺失") {
        ErrorCode::NotFound
    } else if message.contains("只能") || message.contains("不能") {
        ErrorCode::PermissionDenied
    } else if message.contains("参数")
        || message.contains("无效")
        || message.contains("字符")
        || message.contains("搜索词")
    {
        ErrorCode::InvalidData
    } else {
        ErrorCode::Internal
    };
    code.into()
}
fn offset(request: &BusinessRequest) -> MusicResult<u32> {
    request
        .page()
        .and_then(|p| p.cursor.as_ref())
        .map(|v| {
            v.as_str()
                .parse::<u32>()
                .map_err(|_| ErrorCode::InvalidData.into())
        })
        .unwrap_or(Ok(0))
}
fn next(offset: u32, step: u32, more: bool) -> MusicResult<Option<Cursor>> {
    if more {
        Ok(Some(Cursor::try_from(
            offset
                .checked_add(step)
                .ok_or(ErrorCode::InvalidData)?
                .to_string(),
        )?))
    } else {
        Ok(None)
    }
}
fn tracks(
    values: Vec<Track>,
    request: &BusinessRequest,
    more: bool,
    total: Option<u64>,
    description: Option<String>,
    step: u32,
) -> MusicResult<BusinessResponse> {
    let limit = request.page().map_or(100, |p| usize::from(p.limit));
    let truncated = values.len() > limit;
    let items = values
        .into_iter()
        .take(limit)
        .map(track)
        .collect::<MusicResult<Vec<_>>>()?;
    Ok(BusinessResponse::Tracks(TrackPage {
        page: Page {
            items,
            next_cursor: next(offset(request)?, step.min(limit as u32), more || truncated)?,
        },
        total,
        description,
    }))
}
fn collections(
    page: legacy::CollectionPage,
    request: &BusinessRequest,
    step: u32,
) -> MusicResult<BusinessResponse> {
    // Filtered library pages advance by the raw platform page, including empty filtered pages.
    Ok(BusinessResponse::Entities(Page {
        items: page
            .items
            .into_iter()
            .map(entity)
            .collect::<MusicResult<_>>()?,
        next_cursor: next(offset(request)?, step, page.more)?,
    }))
}
pub(super) async fn business(
    adapter: &NeteaseAdapter,
    request: &BusinessRequest,
    context: &RequestContext,
) -> MusicResult<BusinessResponse> {
    let client = adapter.scoped_client(context).await?;
    let start = offset(request)?;
    let response = match request {
        BusinessRequest::Search {
            keyword,
            kind: EntityKind::Track,
            page,
        } => {
            if page.limit > 50 {
                return Err(ErrorCode::InvalidData.into());
            }
            let values = client.search(keyword, start).await.map_err(error)?;
            let more = values.len() == 50;
            tracks(values, request, more, None, None, u32::from(page.limit))?
        }
        BusinessRequest::Search {
            keyword,
            kind: entity_kind,
            page,
        } => {
            if page.limit != 30 {
                return Err(ErrorCode::InvalidData.into());
            }
            collections(
                client
                    .search_collections(keyword, kind(*entity_kind), start)
                    .await
                    .map_err(error)?,
                request,
                30,
            )?
        }
        BusinessRequest::Suggestions { keyword } => {
            BusinessResponse::Suggestions(client.search_suggestions(keyword).await.map_err(error)?)
        }
        BusinessRequest::Detail { entity: requested } => {
            let detail = client
                .music_entity_detail(kind(requested.kind), numeric_id(requested)?)
                .await
                .map_err(error)?;
            BusinessResponse::Detail(EntityDetail {
                item: entity(detail.item)?,
                description: detail.description,
                album_count: detail.album_count,
            })
        }
        BusinessRequest::Tracks { entity, page } => {
            let result = client
                .library_tracks(
                    kind(entity.kind),
                    numeric_id(entity)?,
                    start,
                    page.limit.into(),
                )
                .await
                .map_err(error)?;
            tracks(
                result.tracks,
                request,
                result.more,
                Some(result.total as u64),
                result.description,
                page.limit.into(),
            )?
        }
        BusinessRequest::ArtistAlbums { artist, page } => {
            if page.limit != 30 {
                return Err(ErrorCode::InvalidData.into());
            }
            collections(
                client
                    .artist_albums(numeric_id(artist)?, start)
                    .await
                    .map_err(error)?,
                request,
                30,
            )?
        }
        BusinessRequest::ArtistTracks { artist, page } => {
            let result = client
                .artist_tracks(numeric_id(artist)?, start)
                .await
                .map_err(error)?;
            tracks(
                result.tracks,
                request,
                result.more,
                Some(result.total as u64),
                result.description,
                page.limit.into(),
            )?
        }
        BusinessRequest::TrackInformation { track } => {
            let result = client
                .song_information(numeric_id(track)?)
                .await
                .map_err(error)?;
            BusinessResponse::Information(TrackInformation {
                artists: result
                    .artists
                    .into_iter()
                    .map(|v| {
                        Ok(MusicCredit {
                            name: v.name,
                            reference: v
                                .id
                                .map(|id| reference(EntityKind::Artist, id))
                                .transpose()?,
                        })
                    })
                    .collect::<MusicResult<_>>()?,
                album_reference: result
                    .album_id
                    .map(|id| reference(EntityKind::Album, id))
                    .transpose()?,
                published_at: result.published_at,
            })
        }
        BusinessRequest::LibrarySummary => {
            let result = client.library_summary().await.map_err(error)?;
            BusinessResponse::Summary(library_summary(result)?)
        }
        BusinessRequest::LibraryCollections {
            kind: entity_kind,
            filter,
            page,
        } => {
            if page.limit != 30 {
                return Err(ErrorCode::InvalidData.into());
            }
            collections(
                client
                    .library_collections(
                        kind(*entity_kind),
                        start,
                        match filter.as_str() {
                            "created" => "mine",
                            "collected" => "liked",
                            other => other,
                        },
                    )
                    .await
                    .map_err(error)?,
                request,
                30,
            )?
        }
        BusinessRequest::History { week, page } => {
            let result = client.library_history(*week, start).await.map_err(error)?;
            tracks(
                result.tracks,
                request,
                result.more,
                Some(result.total as u64),
                result.description,
                page.limit.into(),
            )?
        }
        BusinessRequest::Favorites => BusinessResponse::Favorites(
            client
                .liked_song_ids()
                .await
                .map_err(error)?
                .into_iter()
                .map(|id| reference(EntityKind::Track, id))
                .collect::<MusicResult<_>>()?,
        ),
        BusinessRequest::RecommendedPlaylists {
            section,
            category,
            order,
            page,
        } => {
            if page.limit != 30 {
                return Err(ErrorCode::InvalidData.into());
            }
            collections(
                client
                    .discovery_playlists(section, category, order, start)
                    .await
                    .map_err(error)?,
                request,
                30,
            )?
        }
        BusinessRequest::Categories => BusinessResponse::Categories(
            client
                .discovery_categories()
                .await
                .map_err(error)?
                .into_iter()
                .map(|v| Category {
                    name: v.name,
                    group: v.group,
                })
                .collect(),
        ),
        BusinessRequest::Radar => {
            BusinessResponse::Entity(entity(client.discovery_radar().await.map_err(error)?)?)
        }
        BusinessRequest::RecommendedTracks { kind } => tracks(
            client.discovery_tracks(kind).await.map_err(error)?,
            request,
            false,
            None,
            None,
            100,
        )?,
        _ => {
            let mut entities = Vec::new();
            match request {
                BusinessRequest::SetFavorite { track, liked } => {
                    client
                        .set_song_liked(numeric_id(track)?, *liked)
                        .await
                        .map_err(error)?;
                    entities.push(track.clone());
                }
                BusinessRequest::CreatePlaylist { name, private } => client
                    .library_create_playlist(name, *private)
                    .await
                    .map_err(error)?,
                BusinessRequest::UpdatePlaylist {
                    playlist,
                    name,
                    description,
                } => {
                    client
                        .update_library_playlist(numeric_id(playlist)?, name, description)
                        .await
                        .map_err(error)?;
                    entities.push(playlist.clone());
                }
                BusinessRequest::DeletePlaylist { playlist } => {
                    client
                        .delete_library_playlist(numeric_id(playlist)?)
                        .await
                        .map_err(error)?;
                    entities.push(playlist.clone());
                }
                BusinessRequest::PlaylistTrack {
                    playlist,
                    track,
                    add,
                } => {
                    if *add {
                        client
                            .add_playlist_song(numeric_id(playlist)?, numeric_id(track)?)
                            .await
                            .map_err(error)?;
                    } else {
                        client
                            .remove_playlist_song(numeric_id(playlist)?, numeric_id(track)?)
                            .await
                            .map_err(error)?;
                    }
                    entities.extend([playlist.clone(), track.clone()]);
                }
                BusinessRequest::Dislike { track } => {
                    client
                        .discovery_dislike(numeric_id(track)?)
                        .await
                        .map_err(error)?;
                    entities.push(track.clone());
                }
                _ => return Err(ErrorCode::Unsupported.into()),
            }
            BusinessResponse::Write(WriteImpact {
                operations: [
                    "favorites",
                    "librarySummary",
                    "libraryCollections",
                    "tracks",
                    "detail",
                    "recommendedPlaylists",
                    "recommendedTracks",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
                entities,
            })
        }
    };
    Ok(response)
}
pub(super) async fn account(
    client: &legacy::Netease,
    request: &AccountRequest,
    context: &RequestContext,
) -> MusicResult<AccountOutcome> {
    let mut credential = None;
    let mut migration = false;
    let presentation = match request {
        AccountRequest::BeginLogin { .. } => {
            let qr = client.qr_login().await.map_err(error)?;
            AccountPresentation::Challenge {
                key: qr.key,
                image: qr.image,
            }
        }
        AccountRequest::PollLogin { key } => {
            let (status, completion) = client.poll_completion(key).await.map_err(error)?;
            if let Some(secret) = completion {
                let profile = client
                    .profile_with_credential(&secret)
                    .await
                    .map_err(error)?
                    .ok_or(ErrorCode::Unauthenticated)?;
                credential = Some((record(profile)?, secret));
            }
            AccountPresentation::Progress {
                code: status.code,
                message: status.message,
            }
        }
        AccountRequest::Profile => {
            let profile = client
                .profile()
                .await
                .map_err(error)?
                .map(record)
                .transpose()?;
            if context.session.account.is_none() {
                if let Some(record) = &profile {
                    if let Some(cookie) = client.account_credentials().map_err(error)? {
                        credential =
                            Some((record.clone(), OpaqueCredential::new(cookie.into_bytes())?));
                        migration = true;
                    }
                }
            }
            AccountPresentation::Profile(profile)
        }
        AccountRequest::Logout => AccountPresentation::Logout(LogoutReport {
            local_cleared: false,
            local_error: None,
            remote_error: client.remote_logout().await.err(),
        }),
    };
    Ok(AccountOutcome {
        presentation,
        credential,
        migration,
    })
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    #[test]
    fn netease_summary_publishes_liked_playlist_reference_even_when_preview_fails() {
        let result: legacy::LibrarySummary = serde_json::from_value(serde_json::json!({
            "profile": { "userId": 42, "nickname": "测试", "avatarUrl": "" },
            "likedPlaylist": { "id": 12345, "kind": "playlist", "name": "我喜欢的音乐", "cover": "", "subtitle": "测试", "trackCount": 2901, "creatorId": 42, "liked": true, "playCount": null, "publishedAt": null, "artists": [] },
            "likedTracks": [], "likedError": "network"
        })).unwrap();
        let summary = library_summary(result).unwrap();
        let liked = summary.liked_playlist.as_ref().unwrap();
        assert_eq!(
            liked.reference,
            reference(EntityKind::Playlist, 12345).unwrap()
        );
        assert_eq!(liked.track_count, 2901);
        assert!(liked.liked);
        let wire = serde_json::to_value(BusinessResponse::Summary(summary)).unwrap();
        assert_eq!(
            wire["data"]["likedPlaylist"]["reference"]["source"],
            "netease"
        );
        assert_eq!(wire["data"]["likedPlaylist"]["reference"]["id"], "12345");
    }
}
