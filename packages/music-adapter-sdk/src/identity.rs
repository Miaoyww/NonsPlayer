use super::{ErrorCode, MusicError};
use serde::{Deserialize, Serialize};

/// Stable platform identity, independent of adapter package name/version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SourceId(String);

impl SourceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SourceId {
    type Error = MusicError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !value.as_bytes()[0].is_ascii_lowercase()
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(Self(value))
    }
}

/// Opaque UTF-8 ID: never parse as a number, split, trim or case-fold in the host.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct OpaqueId(String);

impl OpaqueId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OpaqueId {
    type Error = MusicError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    Track,
    Playlist,
    Album,
    Artist,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityRef {
    pub source: SourceId,
    pub kind: EntityKind,
    pub id: OpaqueId,
}

impl EntityRef {
    /// JSON tuple encoding avoids delimiter collisions in opaque IDs.
    pub fn key(&self) -> String {
        serde_json::to_string(&(&self.source, self.kind, &self.id))
            .expect("string tuple serializes")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicCredit {
    pub name: String,
    pub reference: Option<EntityRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrack {
    pub reference: EntityRef,
    pub title: String,
    pub aliases: Vec<String>,
    pub artist: String,
    pub artists: Vec<MusicCredit>,
    pub album: String,
    pub album_reference: Option<EntityRef>,
    pub duration_ms: u64,
    pub cover: crate::cover::CoverImage,
    /// Matching/lyric links do not replace reference, including for local tracks.
    pub associations: Vec<EntityRef>,
}

impl MusicTrack {
    pub fn validate(&self, requested: &EntityRef) -> super::MusicResult<()> {
        let same_source_kind = |reference: &EntityRef, kind| {
            reference.source == requested.source && reference.kind == kind
        };
        if &self.reference != requested
            || requested.kind != EntityKind::Track
            || self.artists.iter().any(|credit| {
                credit
                    .reference
                    .as_ref()
                    .is_some_and(|reference| !same_source_kind(reference, EntityKind::Artist))
            })
            || self
                .album_reference
                .as_ref()
                .is_some_and(|reference| !same_source_kind(reference, EntityKind::Album))
            || self
                .associations
                .iter()
                .any(|reference| reference.kind != EntityKind::Track)
            || serde_json::to_vec(self)
                .map_err(|_| MusicError::from(ErrorCode::InvalidData))?
                .len()
                > 64 * 1024
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}
