//! Read-only legacy track conversion; persistence remains host owned.
use crate::legacy::{Track, TrackSource};
/// Preserve persisted queue/cache keys while all new tracks use the uniform source shape.
pub(crate) fn historical_key(reference: &crate::identity::EntityRef) -> Option<String> {
    if reference.source.as_str() == "netease" {
        reference
            .id
            .as_str()
            .parse::<u64>()
            .ok()
            .filter(|id| *id > 0)
            .map(|id| format!("netease:{id}"))
    } else {
        None
    }
}
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

pub struct MigratedTrack {
    pub legacy_key: String,
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

pub fn convert_track(track: Track) -> MusicResult<MigratedTrack> {
    if let TrackSource::Online { track: music } = &track.source {
        validate_online_queue_track(&track).map_err(|_| ErrorCode::InvalidData)?;
        return Ok(MigratedTrack {
            legacy_key: track.key,
            track: *music.clone(),
            local_binding: None,
        });
    }
    let (identity, local_binding, associations, online) = match &track.source {
        TrackSource::Online { .. } => unreachable!("handled above"),
        TrackSource::Netease { id } => (
            netease_reference(EntityKind::Track, *id)?,
            None,
            vec![],
            true,
        ),
        TrackSource::Local { path, netease_id } => {
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
                false,
            )
        }
    };
    let artists = track
        .artists
        .into_iter()
        .map(|credit| {
            Ok(MusicCredit {
                name: credit.name,
                // Local credit IDs must not become remote identities through a lyric binding.
                reference: if online {
                    credit
                        .id
                        .map(|id| netease_reference(EntityKind::Artist, id))
                        .transpose()?
                } else {
                    None
                },
            })
        })
        .collect::<MusicResult<Vec<_>>>()?;
    let album_reference = if online {
        track
            .album_id
            .map(|id| netease_reference(EntityKind::Album, id))
            .transpose()?
    } else {
        None
    };
    let result = MigratedTrack {
        legacy_key: track.key,
        track: MusicTrack {
            reference: identity,
            title: track.title,
            aliases: track.aliases,
            artist: track.artist,
            artists,
            album: track.album,
            album_reference,
            duration_ms: track.duration_ms,
            // Host resolves local artwork using the binding, never a public file path.
            cover: if online {
                track.cover
            } else {
                Default::default()
            },
            associations,
        },
        local_binding,
    };
    result.track.validate(&result.track.reference)?;
    Ok(result)
}

/// The same identity validation serves persistence and queue migration, with safe diagnostic rules.
pub fn validate_online_queue_track(track: &Track) -> Result<(), &'static str> {
    let TrackSource::Online { track: music } = &track.source else {
        return Err("queue.source.kind: must be online");
    };
    music.validate_with_reason(&music.reference)?;
    if music.reference.source.as_str() == "local" {
        return Err("queue.source: local references cannot be online tracks");
    }
    if track.key != music.reference.key()
        && historical_key(&music.reference).as_deref() != Some(track.key.as_str())
    {
        return Err("queue.key: must match the canonical reference key or the matching historical Netease key");
    }
    Ok(())
}

pub fn legacy_track(json: &str) -> MusicResult<MigratedTrack> {
    if json.len() > 64 * 1024 {
        return Err(ErrorCode::InvalidData.into());
    }
    convert_track(serde_json::from_str(json).map_err(|_| ErrorCode::InvalidData)?)
}
