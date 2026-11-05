use super::{
    adapter::{PageRequest, RequestContext},
    cover::CoverImage,
    identity::{EntityKind, EntityRef},
    resource::{PlaybackResource, ResolveRequest},
    ErrorCode, MusicResult,
};
use serde::{Deserialize, Serialize};

const MAX_MV_DURATION_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MvTrackQuery {
    pub title: String,
    #[serde(default)]
    pub artists: Vec<String>,
    #[serde(default)]
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MvSearchRequest {
    pub track: MvTrackQuery,
    pub query: String,
    pub page: PageRequest,
}
impl MvSearchRequest {
    pub fn validate(&self) -> MusicResult<()> {
        self.page.validate()?;
        if self.query.trim().is_empty()
            || self.query.len() > 256
            || self.track.title.len() > 256
            || self.track.artists.len() > 8
            || self.track.artists.iter().any(|artist| artist.len() > 128)
            || self.track.duration_ms > MAX_MV_DURATION_MS
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MvCandidate {
    pub reference: EntityRef,
    pub title: String,
    pub uploader: String,
    pub duration_ms: u64,
    pub cover: CoverImage,
}
impl MvCandidate {
    pub fn validate(&self, source: &super::identity::SourceId) -> MusicResult<()> {
        if &self.reference.source != source
            || self.reference.kind != EntityKind::Track
            || self.title.trim().is_empty()
            || self.title.len() > 512
            || self.uploader.len() > 256
            || self.duration_ms > MAX_MV_DURATION_MS
        {
            return Err(ErrorCode::InvalidData.into());
        }
        self.cover.validate_adapter()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MvResolveRequest {
    pub mv: EntityRef,
}
impl MvResolveRequest {
    pub fn validate(&self, context: &RequestContext) -> MusicResult<()> {
        if self.mv.source != context.session.source || self.mv.kind != EntityKind::Track {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
    pub fn as_playback(&self) -> ResolveRequest {
        ResolveRequest {
            track: self.mv.clone(),
            preferred_quality: "source".into(),
            allow_downgrade: true,
        }
    }
}

pub type MvResource = PlaybackResource;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        account::SessionContext,
        adapter::Cancellation,
        identity::{OpaqueId, SourceId},
    };
    use std::time::{Duration, Instant};

    fn source() -> SourceId {
        "bilibili".to_owned().try_into().unwrap()
    }
    fn reference(kind: EntityKind) -> EntityRef {
        EntityRef {
            source: source(),
            kind,
            id: OpaqueId::try_from("BV1xx411c7mD".to_owned()).unwrap(),
        }
    }
    fn context() -> RequestContext {
        RequestContext {
            session: SessionContext {
                source: source(),
                account: None,
                generation: 1,
            },
            adapter_generation: 1,
            deadline: Some(Instant::now() + Duration::from_secs(10)),
            cancellation: Cancellation::default(),
        }
    }

    #[test]
    fn validates_search_and_resolution_references() {
        let search = MvSearchRequest {
            track: MvTrackQuery {
                title: "Song".into(),
                artists: vec!["Singer".into()],
                duration_ms: 180_000,
            },
            query: "Song Singer".into(),
            page: PageRequest {
                cursor: None,
                limit: 20,
            },
        };
        search.validate().unwrap();
        let resolve = MvResolveRequest {
            mv: reference(EntityKind::Track),
        };
        resolve.validate(&context()).unwrap();
        assert_eq!(resolve.as_playback().track, resolve.mv);
    }

    #[test]
    fn rejects_wrong_mv_kind_and_oversized_query() {
        let mut resolve = MvResolveRequest {
            mv: reference(EntityKind::Playlist),
        };
        assert!(resolve.validate(&context()).is_err());
        resolve.mv.kind = EntityKind::Track;
        let mut search = MvSearchRequest {
            track: MvTrackQuery {
                title: "Song".into(),
                artists: vec![],
                duration_ms: 1,
            },
            query: "x".repeat(257),
            page: PageRequest {
                cursor: None,
                limit: 1,
            },
        };
        assert!(search.validate().is_err());
        search.query = "Song".into();
        assert!(search.validate().is_ok());
    }
}
