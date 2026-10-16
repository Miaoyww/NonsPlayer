//! Interpret platform-local lyric IDs at the independent lyric service boundary.
use crate::model::{Track, TrackSource};
use nons_music_adapter_sdk::identity::LyricIdentifier;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(super) struct Identity {
    pub platform: String,
    pub identifiers: Vec<LyricIdentifier>,
}

impl Identity {
    pub fn of(track: &Track) -> Option<Self> {
        let mut value = match &track.source {
            TrackSource::Online { track } => Self {
                platform: track.reference.source.as_str().into(),
                identifiers: track.lyric_identifiers.clone(),
            },
            TrackSource::Netease { .. } | TrackSource::Local { .. } => Self {
                platform: "netease".into(),
                identifiers: vec![],
            },
        };
        value
            .identifiers
            .retain(|v| valid_id(&value.platform, &v.kind, v.id.as_str()));
        // Compatibility uses only the primary online identity, never associations.
        let primary = match &track.source {
            TrackSource::Online { track } => Some(track.reference.id.as_str().to_owned()),
            _ => track.netease_id().map(|id| id.to_string()),
        };
        if let Some(id) = primary {
            let fallback = match value.platform.as_str() {
                "netease" | "applemusic" => Some(("song-id", id.as_str())),
                "spotify" => Some(("track-id", id.as_str())),
                "qqmusic" => id
                    .strip_prefix("id:")
                    .map(|id| ("song-id", id))
                    .or_else(|| id.starts_with("00").then_some(("song-mid", id.as_str()))),
                _ => None,
            };
            if let Some((kind, id)) = fallback {
                // Explicit adapter IDs take precedence over a legacy primary ID.
                if !value.identifiers.iter().any(|v| v.kind == kind) {
                    if let Ok(id) = id.to_owned().try_into() {
                        value.identifiers.push(LyricIdentifier {
                            kind: kind.into(),
                            id,
                        });
                    }
                }
            }
        }
        value
            .identifiers
            .retain(|v| valid_id(&value.platform, &v.kind, v.id.as_str()));
        value
            .identifiers
            .sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        value.identifiers.dedup();
        (!value.identifiers.is_empty()).then_some(value)
    }

    pub fn directory(&self) -> Option<&'static str> {
        directory(&self.platform)
    }
}

pub(super) fn directory(platform: &str) -> Option<&'static str> {
    match platform {
        "netease" => Some("ncm-lyrics"),
        "qqmusic" => Some("qq-lyrics"),
        "applemusic" => Some("am-lyrics"),
        "spotify" => Some("spotify-lyrics"),
        _ => None,
    }
}

fn valid_id(platform: &str, kind: &str, id: &str) -> bool {
    match (platform, kind) {
        ("netease" | "applemusic" | "qqmusic", "song-id") => {
            id.bytes().all(|b| b.is_ascii_digit()) && id.parse::<u64>().is_ok_and(|id| id > 0)
        }
        ("qqmusic", "song-mid") | ("spotify", "track-id") => {
            !id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric())
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn online(platform: &str, id: &str, ids: serde_json::Value) -> Track {
        serde_json::from_value(serde_json::json!({
            "key":"test", "title":"Song", "artist":"Artist", "album":"Album", "durationMs":1000,
            "cover":"", "source":{"kind":"online","track":{
                "reference":{"source":platform,"kind":"track","id":id},
                "title":"Song","aliases":[],"artist":"Artist","artists":[],"album":"Album",
                "albumReference":null,"durationMs":1000,"cover":"",
                "associations":[{"source":"netease","kind":"track","id":"999"}],
                "lyricIdentifiers":ids
            }}
        }))
        .unwrap()
    }
    #[test]
    fn primary_identity_and_typed_ids_never_use_cross_platform_associations() {
        let qq = online(
            "qqmusic",
            "001mid",
            serde_json::json!([
                {"kind":"song-id","id":"123"}, {"kind":"song-id","id":"123"}
            ]),
        );
        let identity = Identity::of(&qq).unwrap();
        assert_eq!(identity.platform, "qqmusic");
        assert_eq!(identity.identifiers.len(), 2);
        assert_eq!(identity.directory(), Some("qq-lyrics"));
        assert!(Identity::of(&online("unknown", "123", serde_json::json!([]))).is_none());
        assert!(Identity::of(&online("qqmusic", "opaque", serde_json::json!([]))).is_none());
    }
    #[test]
    fn legacy_ids_and_local_bindings_keep_working() {
        for (platform, id, kind, directory) in [
            ("netease", "123", "song-id", "ncm-lyrics"),
            ("qqmusic", "id:123", "song-id", "qq-lyrics"),
            ("qqmusic", "001mid", "song-mid", "qq-lyrics"),
            ("applemusic", "123", "song-id", "am-lyrics"),
            ("spotify", "abc123", "track-id", "spotify-lyrics"),
        ] {
            let value = Identity::of(&online(platform, id, serde_json::json!([]))).unwrap();
            assert_eq!(value.identifiers[0].kind, kind);
            assert_eq!(value.directory(), Some(directory));
        }
        let track: Track = serde_json::from_value(serde_json::json!({
            "key":"local:x","title":"Song","artist":"Artist","album":"","durationMs":0,"cover":"",
            "source":{"kind":"local","path":"C:/song.flac","neteaseId":42}
        }))
        .unwrap();
        assert_eq!(
            Identity::of(&track).unwrap().identifiers[0].id.as_str(),
            "42"
        );
    }
}
