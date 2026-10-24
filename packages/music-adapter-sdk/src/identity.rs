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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricIdentifier {
    /// Platform-local ID namespace, e.g. song-id, song-mid or track-id.
    pub kind: String,
    pub id: OpaqueId,
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
    /// IDs belong to reference.source; never cross-platform associations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lyric_identifiers: Vec<LyricIdentifier>,
    /// Retained catalog entries that currently cannot be played; absent means unknown/available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<ErrorCode>,
}

impl MusicTrack {
    pub fn validate(&self, requested: &EntityRef) -> super::MusicResult<()> {
        self.validate_with_reason(requested)
            .map_err(|_| ErrorCode::InvalidData.into())
    }
    /// Static field/rule diagnostics; never includes metadata, URLs or credentials.
    pub fn validate_with_reason(&self, requested: &EntityRef) -> Result<(), &'static str> {
        let same_source_kind = |reference: &EntityRef, kind| {
            reference.source == requested.source && reference.kind == kind
        };
        if &self.reference != requested {
            return Err("track.reference: must equal the requested reference");
        }
        if requested.kind != EntityKind::Track {
            return Err("track.reference.kind: must be track");
        }
        if self.artists.iter().any(|credit| {
            credit
                .reference
                .as_ref()
                .is_some_and(|reference| !same_source_kind(reference, EntityKind::Artist))
        }) {
            return Err("track.artists.reference: must use the track source and artist kind");
        }
        if self
            .album_reference
            .as_ref()
            .is_some_and(|reference| !same_source_kind(reference, EntityKind::Album))
        {
            return Err("track.albumReference: must use the track source and album kind");
        }
        if self
            .associations
            .iter()
            .any(|reference| reference.kind != EntityKind::Track)
        {
            return Err("track.associations.kind: must be track");
        }
        if self.lyric_identifiers.len() > 8
            || self
                .lyric_identifiers
                .iter()
                .any(|value| SourceId::try_from(value.kind.clone()).is_err())
        {
            return Err("track.lyricIdentifiers: at most 8 IDs with valid namespaces");
        }
        if serde_json::to_vec(self)
            .map_err(|_| "track: serialization failed")?
            .len()
            > 64 * 1024
        {
            return Err("track: serialized metadata exceeds 64 KiB");
        }
        Ok(())
    }
}

#[cfg(test)]
mod lyric_identifier_tests {
    use super::*;

    fn track() -> MusicTrack {
        serde_json::from_value(serde_json::json!({
            "reference":{"source":"qqmusic","kind":"track","id":"001mid"},
            "title":"Song","aliases":[],"artist":"Artist","artists":[],"album":"Album",
            "albumReference":null,"durationMs":1000,"cover":crate::cover::CoverImage::default(),"associations":[]
        }))
        .unwrap()
    }

    #[test]
    fn optional_lyric_ids_and_typed_ids_round_trip() {
        let mut value = track();
        assert!(value.lyric_identifiers.is_empty());
        value.lyric_identifiers.push(LyricIdentifier {
            kind: "song-id".into(),
            id: "123".to_string().try_into().unwrap(),
        });
        value.validate(&value.reference).unwrap();
        let decoded: MusicTrack =
            serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn lyric_id_count_and_namespace_are_bounded() {
        let mut value = track();
        value.lyric_identifiers = vec![
            LyricIdentifier {
                kind: "song-id".into(),
                id: "123".to_string().try_into().unwrap()
            };
            9
        ];
        assert!(value.validate(&value.reference).is_err());
        value.lyric_identifiers.truncate(1);
        value.lyric_identifiers[0].kind = "another/platform".into();
        assert!(value.validate(&value.reference).is_err());
    }
}
