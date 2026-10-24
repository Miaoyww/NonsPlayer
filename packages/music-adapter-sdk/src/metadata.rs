//! Queue metadata projection for online and host-owned local tracks.
use crate::queue::{Track, TrackSource};
use crate::{
    identity::{EntityKind, EntityRef, MusicCredit, MusicTrack, OpaqueId, SourceId},
    ErrorCode, MusicResult,
};
/// Local path stays outside the serializable public track, in host-owned storage.
pub struct LocalBinding {
    pub reference: EntityRef,
    pub path: String,
    pub cover: String,
}

pub struct QueueMetadata {
    pub track: MusicTrack,
    pub local_binding: Option<LocalBinding>,
}

fn reference(source: &str, kind: EntityKind, id: String) -> MusicResult<EntityRef> {
    Ok(EntityRef {
        source: SourceId::try_from(source.to_owned())?,
        kind,
        id: OpaqueId::try_from(id)?,
    })
}

fn netease_reference(kind: EntityKind, id: u64) -> MusicResult<EntityRef> {
    if id == 0 {
        return Err(ErrorCode::InvalidData.into());
    }
    reference("netease", kind, id.to_string())
}

pub fn queue_metadata(track: Track) -> MusicResult<QueueMetadata> {
    if let TrackSource::Online { track: music } = &track.source {
        validate_online_queue_track(&track).map_err(|_| ErrorCode::InvalidData)?;
        return Ok(QueueMetadata {
            track: *music.clone(),
            local_binding: None,
        });
    }
    let (identity, local_binding, associations) = match &track.source {
        TrackSource::Online { .. } => unreachable!("handled above"),
        TrackSource::Local {
            path, netease_id, ..
        } => {
            if path.is_empty() {
                return Err(ErrorCode::InvalidData.into());
            }
            let id = track
                .key
                .strip_prefix("local:")
                .ok_or(ErrorCode::InvalidData)?;
            let identity = reference("local", EntityKind::Track, id.into())?;
            let association = netease_id
                .map(|id| netease_reference(EntityKind::Track, id))
                .transpose()?;
            (
                identity.clone(),
                Some(LocalBinding {
                    reference: identity,
                    path: path.clone(),
                    cover: track.cover.to_string(),
                }),
                association.into_iter().collect(),
            )
        }
    };
    let artists = track
        .artists
        .into_iter()
        .map(|credit| MusicCredit {
            name: credit.name,
            reference: None,
        })
        .collect();
    let result = QueueMetadata {
        track: MusicTrack {
            reference: identity.clone(),
            title: track.title,
            aliases: track.aliases,
            artist: track.artist,
            artists,
            album: track.album,
            album_reference: None,
            duration_ms: track.duration_ms,
            // Host resolves local artwork using the binding, never a public file path.
            cover: Default::default(),
            associations,
            lyric_identifiers: vec![],
            unavailable_reason: None,
        },
        local_binding,
    };
    result.track.validate(&result.track.reference)?;
    Ok(result)
}

/// Validate the online queue identity before storage or playback.
pub fn validate_online_queue_track(track: &Track) -> Result<(), &'static str> {
    let TrackSource::Online { track: music } = &track.source else {
        return Err("queue.source.kind: must be online");
    };
    music.validate_with_reason(&music.reference)?;
    if music.reference.source.as_str() == "local" {
        return Err("queue.source: local references cannot be online tracks");
    }
    if track.key != music.reference.key() {
        return Err("queue.key: must match the canonical reference key");
    }
    Ok(())
}
