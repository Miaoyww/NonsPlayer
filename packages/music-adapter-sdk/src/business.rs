//! Source-neutral optional business capabilities. Platform IDs and offsets stay in adapters.
use super::{account::*, adapter::*, identity::*, ErrorCode, MusicResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicEntity {
    pub reference: EntityRef,
    pub name: String,
    pub cover: crate::cover::CoverImage,
    pub subtitle: String,
    pub track_count: u64,
    pub creator: Option<AccountRef>,
    pub liked: bool,
    pub play_count: Option<u64>,
    pub published_at: Option<u64>,
    pub artists: Vec<MusicCredit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDetail {
    pub item: MusicEntity,
    pub description: Option<String>,
    pub album_count: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInformation {
    pub artists: Vec<MusicCredit>,
    pub album_reference: Option<EntityRef>,
    pub published_at: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub account: AccountRecord,
    pub liked_playlist: Option<MusicEntity>,
    pub liked_tracks: Vec<MusicTrack>,
    pub liked_error: Option<super::MusicError>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub group: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BusinessRequest {
    Search {
        keyword: String,
        kind: EntityKind,
        page: PageRequest,
    },
    Suggestions {
        keyword: String,
    },
    Detail {
        entity: EntityRef,
    },
    Tracks {
        entity: EntityRef,
        page: PageRequest,
    },
    ArtistAlbums {
        artist: EntityRef,
        page: PageRequest,
    },
    ArtistTracks {
        artist: EntityRef,
        page: PageRequest,
    },
    TrackInformation {
        track: EntityRef,
    },
    LibrarySummary,
    LibraryCollections {
        kind: EntityKind,
        filter: String,
        page: PageRequest,
    },
    History {
        week: bool,
        page: PageRequest,
    },
    Favorites,
    SetFavorite {
        track: EntityRef,
        liked: bool,
    },
    CreatePlaylist {
        name: String,
        private: bool,
    },
    UpdatePlaylist {
        playlist: EntityRef,
        name: String,
        description: String,
    },
    DeletePlaylist {
        playlist: EntityRef,
    },
    PlaylistTrack {
        playlist: EntityRef,
        track: EntityRef,
        add: bool,
    },
    RecommendedPlaylists {
        section: String,
        category: String,
        order: String,
        page: PageRequest,
    },
    Categories,
    Radar,
    RecommendedTracks {
        kind: String,
    },
    Dislike {
        track: EntityRef,
    },
}
impl BusinessRequest {
    pub fn capability(&self) -> Capability {
        match self {
            Self::Search { .. } | Self::Suggestions { .. } => Capability::Search,
            Self::Detail { .. }
            | Self::Tracks { .. }
            | Self::ArtistAlbums { .. }
            | Self::ArtistTracks { .. }
            | Self::TrackInformation { .. } => Capability::Browse,
            Self::LibrarySummary | Self::LibraryCollections { .. } | Self::History { .. } => {
                Capability::UserLibrary
            }
            Self::Favorites | Self::SetFavorite { .. } => Capability::Favorites,
            Self::CreatePlaylist { .. }
            | Self::UpdatePlaylist { .. }
            | Self::DeletePlaylist { .. }
            | Self::PlaylistTrack { .. } => Capability::PlaylistWrite,
            Self::Dislike { .. } => Capability::PrivateFm,
            Self::RecommendedTracks { kind } if kind == "fm" => Capability::PrivateFm,
            _ => Capability::Recommendations,
        }
    }
    pub fn requires_account(&self) -> bool {
        matches!(
            self.capability(),
            Capability::UserLibrary
                | Capability::Favorites
                | Capability::PlaylistWrite
                | Capability::PrivateFm
        ) || matches!(self, Self::RecommendedTracks { .. })
    }
    pub fn is_write(&self) -> bool {
        matches!(
            self,
            Self::SetFavorite { .. }
                | Self::CreatePlaylist { .. }
                | Self::UpdatePlaylist { .. }
                | Self::DeletePlaylist { .. }
                | Self::PlaylistTrack { .. }
                | Self::Dislike { .. }
        )
    }
    pub fn page(&self) -> Option<&PageRequest> {
        match self {
            Self::Search { page, .. }
            | Self::Tracks { page, .. }
            | Self::ArtistAlbums { page, .. }
            | Self::ArtistTracks { page, .. }
            | Self::LibraryCollections { page, .. }
            | Self::History { page, .. }
            | Self::RecommendedPlaylists { page, .. } => Some(page),
            _ => None,
        }
    }
    pub fn validate(&self, source: &SourceId) -> MusicResult<()> {
        if serde_json::to_vec(self)
            .map_err(|_| ErrorCode::InvalidData)?
            .len()
            > 16 * 1024
        {
            return Err(ErrorCode::InvalidData.into());
        }
        if let Some(page) = self.page() {
            page.validate()?;
        }
        let check = |r: &EntityRef, kind: EntityKind| {
            if &r.source == source && r.kind == kind {
                Ok(())
            } else {
                Err(ErrorCode::InvalidData.into())
            }
        };
        match self {
            Self::Search { keyword, .. } | Self::Suggestions { keyword }
                if keyword.trim().is_empty() || keyword.len() > 256 =>
            {
                Err(ErrorCode::InvalidData.into())
            }
            Self::Tracks { entity, .. } | Self::Detail { entity } => {
                if &entity.source != source || entity.kind == EntityKind::Track {
                    Err(ErrorCode::InvalidData.into())
                } else {
                    Ok(())
                }
            }
            Self::ArtistAlbums { artist, .. } | Self::ArtistTracks { artist, .. } => {
                check(artist, EntityKind::Artist)
            }
            Self::TrackInformation { track }
            | Self::SetFavorite { track, .. }
            | Self::Dislike { track } => check(track, EntityKind::Track),
            Self::PlaylistTrack {
                playlist, track, ..
            } => {
                check(playlist, EntityKind::Playlist)?;
                check(track, EntityKind::Track)
            }
            Self::UpdatePlaylist { playlist, .. } | Self::DeletePlaylist { playlist } => {
                check(playlist, EntityKind::Playlist)
            }
            Self::LibraryCollections { kind, .. } if *kind == EntityKind::Track => {
                Err(ErrorCode::InvalidData.into())
            }
            _ => Ok(()),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPage {
    #[serde(flatten)]
    pub page: Page<MusicTrack>,
    pub total: Option<u64>,
    pub description: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum BusinessResponse {
    Tracks(TrackPage),
    Entities(Page<MusicEntity>),
    Detail(EntityDetail),
    Entity(MusicEntity),
    Summary(LibrarySummary),
    Favorites(Vec<EntityRef>),
    Suggestions(Vec<String>),
    Information(TrackInformation),
    Categories(Vec<Category>),
    Write(WriteImpact),
}
impl BusinessResponse {
    pub fn validate_cover_hosts(
        &self,
        hosts: &std::collections::BTreeSet<String>,
    ) -> MusicResult<()> {
        match self {
            Self::Tracks(value) => {
                for track in &value.page.items {
                    track.cover.validate_hosts(hosts)?;
                }
            }
            Self::Entities(value) => {
                for entity in &value.items {
                    entity.cover.validate_hosts(hosts)?;
                }
            }
            Self::Entity(value) => value.cover.validate_hosts(hosts)?,
            Self::Detail(value) => value.item.cover.validate_hosts(hosts)?,
            Self::Summary(value) => {
                value.account.avatar.validate_hosts(hosts)?;
                if let Some(entity) = &value.liked_playlist {
                    entity.cover.validate_hosts(hosts)?;
                }
                for track in &value.liked_tracks {
                    track.cover.validate_hosts(hosts)?;
                }
            }
            _ => (),
        }
        Ok(())
    }
    pub fn validate(&self, request: &BusinessRequest, source: &SourceId) -> MusicResult<()> {
        if serde_json::to_vec(self)
            .map_err(|_| ErrorCode::InvalidData)?
            .len()
            > 2 * 1024 * 1024
        {
            return Err(ErrorCode::InvalidData.into());
        }
        let entity = |e: &MusicEntity| -> MusicResult<()> {
            e.cover.validate_adapter()?;
            if &e.reference.source == source
                && e.reference.kind != EntityKind::Track
                && e.creator.as_ref().is_none_or(|c| &c.source == source)
                && e.artists.iter().all(|c| {
                    c.reference
                        .as_ref()
                        .is_none_or(|r| &r.source == source && r.kind == EntityKind::Artist)
                })
            {
                Ok(())
            } else {
                Err(ErrorCode::InvalidData.into())
            }
        };
        let track = |t: &MusicTrack| {
            if &t.reference.source != source {
                return Err(ErrorCode::InvalidData.into());
            }
            t.cover.validate_adapter()?;
            t.validate(&t.reference)
        };
        match self {
            Self::Tracks(value) => {
                let p = &value.page;
                if let Some(page) = request.page() {
                    p.validate(page)?;
                } else if p.items.len() > 100 || p.next_cursor.is_some() {
                    return Err(ErrorCode::InvalidData.into());
                }
                for t in &p.items {
                    track(t)?;
                }
            }
            Self::Entities(p) => {
                p.validate(request.page().ok_or(ErrorCode::InvalidData)?)?;
                for e in &p.items {
                    entity(e)?;
                }
            }
            Self::Entity(e) => entity(e)?,
            Self::Detail(d) => {
                entity(&d.item)?;
                if let BusinessRequest::Detail { entity: requested } = request {
                    if &d.item.reference != requested {
                        return Err(ErrorCode::InvalidData.into());
                    }
                }
            }
            Self::Summary(s) => {
                s.account.avatar.validate_adapter()?;
                if &s.account.reference.source != source || s.liked_tracks.len() > 100 {
                    return Err(ErrorCode::InvalidData.into());
                }
                if let Some(e) = &s.liked_playlist {
                    entity(e)?;
                }
                for t in &s.liked_tracks {
                    track(t)?;
                }
            }
            Self::Favorites(items)
                if items.len() > 100_000
                    || items
                        .iter()
                        .any(|r| &r.source != source || r.kind != EntityKind::Track) =>
            {
                return Err(ErrorCode::InvalidData.into())
            }
            Self::Information(info)
                if info.artists.iter().any(|c| {
                    c.reference
                        .as_ref()
                        .is_some_and(|r| &r.source != source || r.kind != EntityKind::Artist)
                }) || info
                    .album_reference
                    .as_ref()
                    .is_some_and(|r| &r.source != source || r.kind != EntityKind::Album) =>
            {
                return Err(ErrorCode::InvalidData.into())
            }
            Self::Write(impact)
                if !request.is_write() || impact.entities.iter().any(|r| &r.source != source) =>
            {
                return Err(ErrorCode::InvalidData.into())
            }
            _ => (),
        }
        let valid = matches!(
            (request, self),
            (
                BusinessRequest::Search {
                    kind: EntityKind::Track,
                    ..
                } | BusinessRequest::Tracks { .. }
                    | BusinessRequest::ArtistTracks { .. }
                    | BusinessRequest::History { .. }
                    | BusinessRequest::RecommendedTracks { .. },
                Self::Tracks(_)
            ) | (
                BusinessRequest::Search { .. }
                    | BusinessRequest::ArtistAlbums { .. }
                    | BusinessRequest::LibraryCollections { .. }
                    | BusinessRequest::RecommendedPlaylists { .. },
                Self::Entities(_)
            ) | (BusinessRequest::Detail { .. }, Self::Detail(_))
                | (BusinessRequest::Radar, Self::Entity(_))
                | (BusinessRequest::LibrarySummary, Self::Summary(_))
                | (BusinessRequest::Favorites, Self::Favorites(_))
                | (BusinessRequest::Suggestions { .. }, Self::Suggestions(_))
                | (
                    BusinessRequest::TrackInformation { .. },
                    Self::Information(_)
                )
                | (BusinessRequest::Categories, Self::Categories(_))
        ) || request.is_write() && matches!(self, Self::Write(_));
        if valid {
            Ok(())
        } else {
            Err(ErrorCode::InvalidData.into())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum AccountRequest {
    BeginLogin {
        #[serde(default)]
        method: Option<String>,
    },
    PollLogin {
        key: String,
    },
    Profile,
    Logout,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AccountPresentation {
    Challenge { key: String, image: String },
    Progress { code: i64, message: String },
    Profile(Option<AccountRecord>),
    Logout(LogoutReport),
}
/// Backend-only completion; credentials can never be serialized through IPC.
pub struct AccountOutcome {
    pub presentation: AccountPresentation,
    pub credential: Option<(AccountRecord, OpaqueCredential)>,
    pub migration: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutReport {
    pub local_cleared: bool,
    pub local_error: Option<super::MusicError>,
    pub remote_error: Option<super::MusicError>,
}
