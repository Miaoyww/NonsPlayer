//! Deprecated host DTO/offset bridge. New code uses business requests and source references.
use super::{account::*, adapter::*, business::*, identity::*, service::MusicService};
use crate::{
    model::{AppResult, Track},
    netease as old,
};
fn reference(kind: &str, id: u64) -> AppResult<EntityRef> {
    let kind = match kind {
        "track" => EntityKind::Track,
        "playlist" => EntityKind::Playlist,
        "album" => EntityKind::Album,
        "artist" => EntityKind::Artist,
        _ => return Err("音乐实体类型无效".into()),
    };
    nons_adapter_netease::entity_reference(kind, id).map_err(|e| e.to_string())
}
fn kind(value: &str) -> AppResult<EntityKind> {
    Ok(reference(value, 1)?.kind)
}
fn page(offset: u32, limit: u16) -> AppResult<PageRequest> {
    Ok(PageRequest {
        cursor: if offset == 0 {
            None
        } else {
            Some(Cursor::try_from(offset.to_string()).map_err(|e| e.to_string())?)
        },
        limit,
    })
}
fn profile(record: AccountRecord) -> AppResult<old::AccountProfile> {
    Ok(old::AccountProfile {
        user_id: record
            .reference
            .id
            .as_str()
            .parse()
            .map_err(|_| "兼容账号 ID 无效")?,
        nickname: record.display_name,
        avatar_url: record.avatar,
    })
}
fn entity(value: MusicEntity) -> AppResult<old::Collection> {
    Ok(old::Collection {
        id: nons_adapter_netease::numeric_id(&value.reference).map_err(|e| e.to_string())?,
        kind: nons_adapter_netease::kind(value.reference.kind).to_owned(),
        name: value.name,
        cover: value.cover,
        subtitle: value.subtitle,
        track_count: value.track_count,
        creator_id: value
            .creator
            .map(|a| a.id.as_str().parse::<u64>())
            .transpose()
            .map_err(|_| "兼容账号 ID 无效")?
            .unwrap_or(0),
        liked: value.liked,
        play_count: value.play_count,
        published_at: value.published_at,
        artists: value
            .artists
            .into_iter()
            .map(old_credit)
            .collect::<AppResult<_>>()?,
    })
}
fn old_credit(value: MusicCredit) -> AppResult<crate::model::MusicCredit> {
    Ok(crate::model::MusicCredit {
        name: value.name,
        id: value
            .reference
            .map(|r| nons_adapter_netease::numeric_id(&r).map_err(|e| e.to_string()))
            .transpose()?,
    })
}
fn legacy(response: BusinessResponse) -> AppResult<serde_json::Value> {
    let value = match response {
        BusinessResponse::Tracks(p) => serde_json::to_value(old::TrackPage {
            description: p.description,
            tracks: p
                .page
                .items
                .into_iter()
                .map(|t| nons_adapter_netease::legacy(t).map_err(|e| e.to_string()))
                .collect::<AppResult<_>>()?,
            total: p.total.unwrap_or(0) as usize,
            more: p.page.next_cursor.is_some(),
        }),
        BusinessResponse::Entities(p) => serde_json::to_value(old::CollectionPage {
            items: p.items.into_iter().map(entity).collect::<AppResult<_>>()?,
            more: p.next_cursor.is_some(),
        }),
        BusinessResponse::Entity(e) => serde_json::to_value(entity(e)?),
        BusinessResponse::Detail(d) => serde_json::to_value(old::EntityDetail {
            item: entity(d.item)?,
            description: d.description,
            album_count: d.album_count,
        }),
        BusinessResponse::Summary(s) => serde_json::to_value(old::LibrarySummary {
            profile: profile(s.account)?,
            liked_playlist: s.liked_playlist.map(entity).transpose()?,
            liked_tracks: s
                .liked_tracks
                .into_iter()
                .map(|t| nons_adapter_netease::legacy(t).map_err(|e| e.to_string()))
                .collect::<AppResult<_>>()?,
            liked_error: s.liked_error.map(|e| e.to_string()),
        }),
        BusinessResponse::Favorites(items) => serde_json::to_value(
            items
                .iter()
                .map(|r| nons_adapter_netease::numeric_id(r).map_err(|e| e.to_string()))
                .collect::<AppResult<Vec<_>>>()?,
        ),
        BusinessResponse::Suggestions(items) => serde_json::to_value(items),
        BusinessResponse::Categories(items) => serde_json::to_value(items),
        BusinessResponse::Information(info) => serde_json::to_value(old::SongInformation {
            artists: info
                .artists
                .into_iter()
                .map(|c| {
                    let c = old_credit(c)?;
                    Ok(old::SongCredit {
                        name: c.name,
                        id: c.id,
                    })
                })
                .collect::<AppResult<_>>()?,
            album_id: info
                .album_reference
                .map(|r| nons_adapter_netease::numeric_id(&r).map_err(|e| e.to_string()))
                .transpose()?,
            published_at: info.published_at,
        }),
        BusinessResponse::Write(_) => Ok(serde_json::Value::Null),
    };
    value.map_err(|e| e.to_string())
}
impl MusicService {
    pub fn legacy_stamp(&self) -> AppResult<(super::account::SessionContext, u64)> {
        self.adapters
            .stamp(&super::identity::SourceId::try_from("netease".to_owned()).unwrap())
            .map_err(|e| e.to_string())
    }
    pub async fn legacy_track(&self, id: u64) -> AppResult<Track> {
        let reference = nons_adapter_netease::reference(id).map_err(|e| e.to_string())?;
        let track = self
            .adapters
            .read_track(&reference)
            .await
            .map_err(|e| e.to_string())?;
        nons_adapter_netease::legacy(track).map_err(|e| e.to_string())
    }

    async fn old_call<T: serde::de::DeserializeOwned>(
        &self,
        request: BusinessRequest,
    ) -> AppResult<T> {
        let source = SourceId::try_from("netease".to_owned()).unwrap();
        let result = self
            .adapters
            .business_legacy(&source, &request)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::from_value(legacy(result)?).map_err(|e| e.to_string())
    }
    pub async fn search(&self, keyword: &str, offset: u32) -> AppResult<Vec<Track>> {
        Ok(self
            .old_call::<old::TrackPage>(BusinessRequest::Search {
                keyword: keyword.into(),
                kind: EntityKind::Track,
                page: page(offset, 50)?,
            })
            .await?
            .tracks)
    }
    pub async fn search_suggestions(&self, keyword: &str) -> AppResult<Vec<String>> {
        self.old_call(BusinessRequest::Suggestions {
            keyword: keyword.into(),
        })
        .await
    }
    pub async fn search_collections(
        &self,
        keyword: &str,
        entity_kind: &str,
        offset: u32,
    ) -> AppResult<old::CollectionPage> {
        self.old_call(BusinessRequest::Search {
            keyword: keyword.into(),
            kind: kind(entity_kind)?,
            page: page(offset, 30)?,
        })
        .await
    }
    pub async fn liked_song_ids(&self) -> AppResult<Vec<u64>> {
        self.old_call(BusinessRequest::Favorites).await
    }
    pub async fn set_song_liked(&self, id: u64, liked: bool) -> AppResult<()> {
        self.old_call(BusinessRequest::SetFavorite {
            track: reference("track", id)?,
            liked,
        })
        .await
    }
    pub async fn song_information(&self, id: u64) -> AppResult<old::SongInformation> {
        self.old_call(BusinessRequest::TrackInformation {
            track: reference("track", id)?,
        })
        .await
    }
    pub async fn library_summary(&self) -> AppResult<old::LibrarySummary> {
        self.old_call(BusinessRequest::LibrarySummary).await
    }
    pub async fn library_collections(
        &self,
        entity_kind: &str,
        offset: u32,
        filter: &str,
    ) -> AppResult<old::CollectionPage> {
        self.old_call(BusinessRequest::LibraryCollections {
            kind: kind(entity_kind)?,
            filter: filter.into(),
            page: page(offset, 30)?,
        })
        .await
    }
    pub async fn library_tracks(
        &self,
        entity_kind: &str,
        id: u64,
        offset: u32,
        limit: usize,
    ) -> AppResult<old::TrackPage> {
        self.old_call(BusinessRequest::Tracks {
            entity: reference(entity_kind, id)?,
            page: page(offset, u16::try_from(limit).map_err(|_| "分页数量无效")?)?,
        })
        .await
    }
    pub async fn music_entity_detail(
        &self,
        entity_kind: &str,
        id: u64,
    ) -> AppResult<old::EntityDetail> {
        self.old_call(BusinessRequest::Detail {
            entity: reference(entity_kind, id)?,
        })
        .await
    }
    pub async fn artist_albums(&self, id: u64, offset: u32) -> AppResult<old::CollectionPage> {
        self.old_call(BusinessRequest::ArtistAlbums {
            artist: reference("artist", id)?,
            page: page(offset, 30)?,
        })
        .await
    }
    pub async fn artist_tracks(&self, id: u64, offset: u32) -> AppResult<old::TrackPage> {
        self.old_call(BusinessRequest::ArtistTracks {
            artist: reference("artist", id)?,
            page: page(offset, 100)?,
        })
        .await
    }
    pub async fn library_history(&self, week: bool, offset: u32) -> AppResult<old::TrackPage> {
        self.old_call(BusinessRequest::History {
            week,
            page: page(offset, 100)?,
        })
        .await
    }
    pub async fn library_create_playlist(&self, name: &str, private: bool) -> AppResult<()> {
        self.old_call(BusinessRequest::CreatePlaylist {
            name: name.into(),
            private,
        })
        .await
    }
    pub async fn update_library_playlist(
        &self,
        id: u64,
        name: &str,
        description: &str,
    ) -> AppResult<()> {
        self.old_call(BusinessRequest::UpdatePlaylist {
            playlist: reference("playlist", id)?,
            name: name.into(),
            description: description.into(),
        })
        .await
    }
    pub async fn delete_library_playlist(&self, id: u64) -> AppResult<()> {
        self.old_call(BusinessRequest::DeletePlaylist {
            playlist: reference("playlist", id)?,
        })
        .await
    }
    pub async fn add_playlist_song(&self, playlist_id: u64, song_id: u64) -> AppResult<()> {
        self.old_call(BusinessRequest::PlaylistTrack {
            playlist: reference("playlist", playlist_id)?,
            track: reference("track", song_id)?,
            add: true,
        })
        .await
    }
    pub async fn remove_playlist_song(&self, playlist_id: u64, song_id: u64) -> AppResult<()> {
        self.old_call(BusinessRequest::PlaylistTrack {
            playlist: reference("playlist", playlist_id)?,
            track: reference("track", song_id)?,
            add: false,
        })
        .await
    }
    pub async fn discovery_playlists(
        &self,
        section: &str,
        category: &str,
        order: &str,
        offset: u32,
    ) -> AppResult<old::CollectionPage> {
        self.old_call(BusinessRequest::RecommendedPlaylists {
            section: section.into(),
            category: category.into(),
            order: order.into(),
            page: page(offset, 30)?,
        })
        .await
    }
    pub async fn discovery_categories(&self) -> AppResult<Vec<old::PlaylistCategory>> {
        self.old_call(BusinessRequest::Categories).await
    }
    pub async fn discovery_radar(&self) -> AppResult<old::Collection> {
        self.old_call(BusinessRequest::Radar).await
    }
    pub async fn discovery_tracks(&self, entity_kind: &str) -> AppResult<Vec<Track>> {
        Ok(self
            .old_call::<old::TrackPage>(BusinessRequest::RecommendedTracks {
                kind: entity_kind.into(),
            })
            .await?
            .tracks)
    }
    pub async fn discovery_dislike(&self, id: u64) -> AppResult<()> {
        self.old_call(BusinessRequest::Dislike {
            track: reference("track", id)?,
        })
        .await
    }
    pub async fn library_queue(&self, entity_kind: &str, id: u64) -> AppResult<old::TrackPage> {
        let stamp = self.legacy_stamp()?;
        let mut result = old::TrackPage {
            description: None,
            tracks: Vec::new(),
            total: 0,
            more: false,
        };
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(12);
        for offset in (0..1000).step_by(100) {
            let page = tokio::time::timeout_at(
                deadline,
                self.library_tracks(entity_kind, id, offset, 100),
            )
            .await
            .map_err(|_| "整组歌曲读取超时，请重试")??;
            result.total = page.total;
            result.description = page.description;
            result.more = page.more;
            result.tracks.extend(page.tracks);
            if self.legacy_stamp()? != stamp {
                return Err("账号或音乐来源状态已变化".into());
            }
            if !result.more {
                break;
            }
        }
        Ok(result)
    }
    pub async fn qr_login(&self) -> AppResult<old::QrLogin> {
        match self.account_call(AccountRequest::BeginLogin).await? {
            AccountPresentation::Challenge { key, image } => Ok(old::QrLogin { key, image }),
            _ => Err("登录响应无效".into()),
        }
    }
    pub async fn poll_login(&self, key: &str) -> AppResult<old::LoginStatus> {
        match self
            .account_call(AccountRequest::PollLogin { key: key.into() })
            .await?
        {
            AccountPresentation::Progress { code, message } => {
                Ok(old::LoginStatus { code, message })
            }
            _ => Err("登录响应无效".into()),
        }
    }
    pub fn profile_snapshot(&self) -> AppResult<Option<old::AccountProfile>> {
        let source = SourceId::try_from("netease".to_owned()).unwrap();
        let before = self.adapters.stamp(&source).map_err(|e| e.to_string())?;
        let record = self
            .accounts
            .as_ref()
            .ok_or("账号管理器不可用")?
            .current_record(&source)
            .map_err(|e| e.to_string())?;
        if self.adapters.stamp(&source).map_err(|e| e.to_string())? != before {
            return Err("账号或音乐来源状态已变化".into());
        }
        record.map(profile).transpose()
    }
    pub async fn profile(&self) -> AppResult<Option<old::AccountProfile>> {
        match self.account_call(AccountRequest::Profile).await? {
            AccountPresentation::Profile(record) => record.map(profile).transpose(),
            _ => Err("账号响应无效".into()),
        }
    }
    pub async fn session(&self) -> AppResult<bool> {
        Ok(self.profile().await?.is_some())
    }
    pub async fn logout(&self) -> AppResult<LogoutReport> {
        match self.account_call(AccountRequest::Logout).await? {
            AccountPresentation::Logout(report) => Ok(report),
            _ => Err("退出响应无效".into()),
        }
    }
    async fn account_call(&self, request: AccountRequest) -> AppResult<AccountPresentation> {
        self.adapters
            .account(
                &SourceId::try_from("netease".to_owned()).unwrap(),
                &request,
                self.accounts.as_ref().ok_or("账号管理器未初始化")?,
            )
            .await
            .map_err(|e| e.to_string())
    }
}
