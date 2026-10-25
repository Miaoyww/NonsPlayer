pub fn track_position(track: Option<&crate::model::Track>, position_ms: u64) -> u64 {
    track
        .and_then(|track| track.cue())
        .map_or(position_ms, |cue| {
            position_ms
                .saturating_sub(cue.start_ms)
                .min(cue.end_ms.saturating_sub(cue.start_ms))
        })
}

pub fn track_duration(track: Option<&crate::model::Track>, duration_ms: u64) -> u64 {
    track
        .and_then(|track| track.cue())
        .map_or(duration_ms, |cue| cue.end_ms.saturating_sub(cue.start_ms))
}
