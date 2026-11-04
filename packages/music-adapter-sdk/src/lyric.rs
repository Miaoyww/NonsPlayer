//! Platform-derived metadata used only to search lyrics; it does not replace track identity.
use serde::{Deserialize, Serialize};

use crate::identity::MusicTrack;
use crate::ErrorCode;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricMatchRequest {
    pub track: MusicTrack,
}

impl LyricMatchRequest {
    pub fn validate(&self, requested: &crate::identity::EntityRef) -> crate::MusicResult<()> {
        self.track.validate(requested)
    }
}

/// Never an identity: only query/scoring input for the independent lyric service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricMatchSubject {
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl LyricMatchSubject {
    pub fn validate(&self) -> crate::MusicResult<()> {
        if self.title.trim().is_empty()
            || self.title.chars().count() > 256
            || self.artists.len() > 8
            || self
                .artists
                .iter()
                .any(|artist| artist.trim().is_empty() || artist.chars().count() > 128)
            || self
                .album
                .as_ref()
                .is_some_and(|album| album.chars().count() > 256)
            || self.duration_ms == Some(0)
        {
            return Err(ErrorCode::InvalidData.into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject() -> LyricMatchSubject {
        LyricMatchSubject {
            title: "Song".into(),
            artists: vec!["Artist".into()],
            album: Some("Album".into()),
            duration_ms: Some(1000),
        }
    }

    #[test]
    fn subjects_are_bounded_and_round_trip() {
        subject().validate().unwrap();
        let decoded: LyricMatchSubject =
            serde_json::from_slice(&serde_json::to_vec(&subject()).unwrap()).unwrap();
        assert_eq!(decoded, subject());
        for invalid in [
            LyricMatchSubject {
                title: " ".into(),
                ..subject()
            },
            LyricMatchSubject {
                title: "文".repeat(257),
                ..subject()
            },
            LyricMatchSubject {
                artists: vec!["".into()],
                ..subject()
            },
            LyricMatchSubject {
                artists: vec!["文".repeat(129)],
                ..subject()
            },
            LyricMatchSubject {
                duration_ms: Some(0),
                ..subject()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
        subject().validate().unwrap();
        assert_eq!(
            serde_json::to_value(LyricMatchSubject {
                artists: vec![],
                album: None,
                duration_ms: None,
                ..subject()
            })
            .unwrap(),
            serde_json::json!({"title":"Song"})
        );
    }
}
