pub const MAX_LYRIC_BYTES: usize = 2 * 1024 * 1024;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TrackSource {
    /// Online entries retain the full uniform metadata and opaque identity.
    Online {
        track: Box<crate::identity::MusicTrack>,
    },
    Local {
        path: String,
        netease_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cue: Option<CueTrack>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CueTrack {
    pub sheet_path: String,
    pub track_number: u32,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicCredit {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub artist: String,
    #[serde(default)]
    pub artists: Vec<MusicCredit>,
    pub album: String,
    pub duration_ms: u64,
    pub cover: crate::cover::CoverImage,
    pub source: TrackSource,
}

impl Track {
    pub fn cue(&self) -> Option<&CueTrack> {
        match &self.source {
            TrackSource::Local { cue, .. } => cue.as_ref(),
            _ => None,
        }
    }

    pub fn from_music(track: crate::identity::MusicTrack) -> crate::MusicResult<Self> {
        track.validate(&track.reference)?;
        if track.reference.source.as_str() == "local" {
            return Err(crate::ErrorCode::InvalidData.into());
        }
        Ok(Self {
            key: track.reference.key(),
            title: track.title.clone(),
            aliases: track.aliases.clone(),
            artist: track.artist.clone(),
            artists: track
                .artists
                .iter()
                .map(|credit| MusicCredit {
                    name: credit.name.clone(),
                })
                .collect(),
            album: track.album.clone(),
            duration_ms: track.duration_ms,
            cover: track.cover.clone(),
            source: TrackSource::Online {
                track: Box::new(track),
            },
        })
    }

    pub fn source_id(&self) -> &str {
        match &self.source {
            TrackSource::Online { track } => track.reference.source.as_str(),
            TrackSource::Local { .. } => "local",
        }
    }

    pub fn netease_id(&self) -> Option<u64> {
        match &self.source {
            TrackSource::Online { track } => std::iter::once(&track.reference)
                .chain(track.associations.iter())
                .find(|reference| reference.source.as_str() == "netease")
                .and_then(|reference| reference.id.as_str().parse().ok()),
            TrackSource::Local { netease_id, .. } => *netease_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_score: Option<u32>,
    pub source: String,
    pub format: String,
    pub content: String,
    pub translation: Option<String>,
    pub romanization: Option<String>,
}

pub type AppResult<T> = Result<T, String>;
