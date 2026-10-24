use crate::{
    application::Backend,
    model::{AppResult, Track, TrackSource},
    storage::Store,
};
use std::{collections::BTreeSet, path::Path};
use tauri::{Emitter, State};

#[tauri::command]
pub(crate) async fn local_cover_repair(
    url: String,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<String> {
    let store = backend.store.clone();
    let covers = backend.covers.clone();
    let fallback = backend.fallback_cover.clone();
    let (cover, changed) =
        tauri::async_runtime::spawn_blocking(move || repair(&url, &store, &covers, &fallback))
            .await
            .map_err(|e| e.to_string())??;
    if changed {
        app.emit(
            "local-library-updated",
            serde_json::json!({"scanning":false}),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(cover)
}

fn repair(url: &str, store: &Store, covers: &Path, fallback: &str) -> AppResult<(String, bool)> {
    let _writes = super::library::COVER_WRITES
        .lock()
        .map_err(|_| "封面写入锁不可用")?;
    super::library::prepare_covers(covers, fallback)?;
    let _validation = super::cover_art::ValidationSession::new();
    let requested = Path::new(url);
    // The requested asset may no longer exist; canonicalize its parent instead.
    if requested.parent().and_then(|p| p.canonicalize().ok())
        != Some(covers.canonicalize().map_err(|e| e.to_string())?)
    {
        return Err("只允许恢复曲库封面".into());
    }
    if super::cover_art::valid(requested) {
        return Ok((url.to_owned(), false));
    }
    let tracks = {
        let db = store.0.lock().map_err(|_| "曲库锁不可用")?;
        let values = db.prepare("SELECT DISTINCT t.value FROM tracks t WHERE t.source='local' AND (json_extract(t.value,'$.cover.original')=?1 OR EXISTS(SELECT 1 FROM local_cover_state s,json_each(s.embedded) e WHERE s.key=t.key AND e.value=?1) OR EXISTS(SELECT 1 FROM local_members m JOIN local_entities a ON a.kind=m.kind AND a.id=m.id WHERE m.key=t.key AND a.kind='album' AND a.cover=?1))").map_err(|e| e.to_string())?
            .query_map([url], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        let tracks = values
            .into_iter()
            .map(|v| serde_json::from_str::<Track>(&v).map_err(|e| e.to_string()))
            .collect::<AppResult<Vec<_>>>()?;
        tracks
    };
    let mut used = super::cover_art::usage(covers);
    let mut sources = BTreeSet::new();
    for track in &tracks {
        if let TrackSource::Local { path, cue, .. } = &track.source {
            sources.insert((
                cue.is_some(),
                cue.as_ref()
                    .map_or(path.clone(), |cue| cue.sheet_path.clone()),
            ));
        }
    }
    let mut changed = false;
    for (cue, path) in sources {
        let result = if cue {
            super::library::import_sheet(Path::new(&path), covers, fallback, store, &mut used)
                .map(|_| ())
        } else {
            super::library::read_track(Path::new(&path), covers, fallback, store, &mut used)
                .map(|_| ())
        };
        if let Err(error) = result {
            crate::infrastructure::diagnostics::warn("local.cover.repair", error);
            // An unavailable original must not leave a persistent dangling reference.
            for old in tracks.iter().filter(|track| match &track.source {
                TrackSource::Local {
                    path: audio,
                    cue: sheet,
                    ..
                } => {
                    if cue {
                        sheet.as_ref().is_some_and(|s| s.sheet_path == path)
                    } else {
                        *audio == path
                    }
                }
                _ => false,
            }) {
                let mut track = old.clone();
                track.cover = fallback.into();
                store.save_tracks(&[track])?;
                store.save_local_cover_state(&old.key, &Default::default())?;
            }
        }
        changed = true;
    }
    store.refresh_local_albums()?;
    // A shared asset can belong to several albums; fresh entity metadata selects their
    // individual replacements. A failed URL must not borrow another album's cover.
    let cover = if super::cover_art::valid(requested) {
        url.to_owned()
    } else {
        fallback.to_owned()
    };
    Ok((cover, changed))
}

#[cfg(test)]
#[path = "cover_repair_tests.rs"]
mod tests;
