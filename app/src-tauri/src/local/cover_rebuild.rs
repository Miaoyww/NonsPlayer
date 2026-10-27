use crate::{
    application::Backend,
    model::{AppResult, Track, TrackSource},
    storage::Store,
};
use lofty::file::TaggedFileExt;
use rusqlite::params;
use serde::Serialize;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, State};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Progress {
    processed: u64,
    total: u64,
    failed: u64,
    running: bool,
    error: Option<String>,
}

#[derive(Default)]
pub(crate) struct RebuildJob(Mutex<Progress>);

impl RebuildJob {
    fn status(&self) -> AppResult<Progress> {
        self.0
            .lock()
            .map(|status| status.clone())
            .map_err(|_| "封面重建状态不可用".into())
    }

    fn begin(&self) -> AppResult<Progress> {
        let mut status = self.0.lock().map_err(|_| "封面重建状态不可用")?;
        if !status.running {
            *status = Progress {
                running: true,
                ..Default::default()
            };
            Ok(status.clone())
        } else {
            Err("本地封面正在重建".into())
        }
    }

    fn publish(&self, app: &tauri::AppHandle, progress: &Progress) {
        if let Ok(mut status) = self.0.lock() {
            *status = progress.clone();
        }
        if let Err(error) = app.emit("local-cover-rebuild-progress", progress) {
            crate::infrastructure::diagnostics::warn("local.cover_rebuild.progress", error);
        }
    }
}

#[tauri::command]
pub(crate) fn local_cover_rebuild_status(backend: State<'_, Backend>) -> AppResult<Progress> {
    backend.cover_rebuild.status()
}

#[tauri::command]
pub(crate) async fn local_cover_rebuild(
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Progress> {
    let initial = backend.cover_rebuild.begin()?;
    let job = backend.cover_rebuild.clone();
    let store = backend.store.clone();
    let covers = backend.covers.clone();
    let fallback = backend.fallback_cover.clone();
    tauri::async_runtime::spawn(async move {
        let worker_app = app.clone();
        let worker_job = job.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            rebuild(&store, &covers, &fallback, |progress| {
                worker_job.publish(&worker_app, progress)
            })
        })
        .await;
        let mut progress = job.status().unwrap_or_default();
        let error = match result {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error),
            Err(error) => Some(error.to_string()),
        };
        if let Some(error) = &error {
            crate::infrastructure::diagnostics::warn("local.cover_rebuild", error);
        }
        // Publish changed records even when a later item or album refresh failed.
        if let Err(error) = app.emit(
            "local-library-updated",
            serde_json::json!({"scanning":false}),
        ) {
            crate::infrastructure::diagnostics::warn("local.cover_rebuild.updated", error);
        }
        progress.running = false;
        progress.error = error;
        job.publish(&app, &progress);
    });
    Ok(initial)
}

pub(super) fn rebuild(
    store: &Arc<Store>,
    covers: &Path,
    fallback: &str,
    mut publish: impl FnMut(&Progress),
) -> AppResult<()> {
    let _writes = super::library::COVER_WRITES
        .lock()
        .map_err(|_| "封面写入锁不可用")?;
    super::library::prepare_covers(covers, fallback)?;
    let _validation = super::cover_art::ValidationSession::new();
    let total = store
        .0
        .lock()
        .map_err(|_| "曲库锁不可用")?
        .query_row(
            "SELECT COUNT(*) FROM tracks WHERE source='local'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?;
    let mut progress = Progress {
        total: total as u64,
        running: true,
        ..Default::default()
    };
    publish(&progress);
    let mut after = String::new();
    let mut last_update = Instant::now();
    loop {
        let rows = {
            let db = store.0.lock().map_err(|_| "曲库锁不可用")?;
            let mut statement = db.prepare("SELECT key,value FROM tracks WHERE source='local' AND key>?1 ORDER BY key LIMIT 64").map_err(|e| e.to_string())?;
            let rows = statement
                .query_map([&after], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };
        if rows.is_empty() {
            break;
        }
        for (key, value) in rows {
            after = key;
            let result = serde_json::from_str::<Track>(&value)
                .map_err(|e| e.to_string())
                .and_then(|track| rebuild_track(store, &track, covers, fallback));
            if let Err(error) = result {
                progress.failed += 1;
                crate::infrastructure::diagnostics::warn("local.cover_rebuild.track", error);
            }
            progress.processed += 1;
            if last_update.elapsed() >= Duration::from_millis(200) {
                publish(&progress);
                last_update = Instant::now();
            }
        }
    }
    publish(&progress);
    store.refresh_local_albums()
}

fn rebuild_track(store: &Store, track: &Track, covers: &Path, fallback: &str) -> AppResult<()> {
    let TrackSource::Local { path, .. } = &track.source else {
        return Ok(());
    };
    let directory = super::cover_art::directory(track).ok_or("音乐目录不可用")?;
    let (tagged, _, ncm_image) = super::encoded_audio::read_tags(path)?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let mut embedded = Vec::new();
    for bytes in tag
        .into_iter()
        .flat_map(|tag| tag.pictures())
        .map(|picture| picture.data())
        .chain(std::iter::once(ncm_image.as_slice()))
    {
        if let Some(cover) = super::cover_art::asset(bytes, covers) {
            if !embedded.contains(&cover) {
                embedded.push(cover);
            }
        }
    }
    let cover = embedded
        .first()
        .cloned()
        .or_else(|| super::cover_art::external(directory, covers))
        .unwrap_or_else(|| fallback.to_owned());
    let mut updated = track.clone();
    updated.cover = cover.into();
    let cover = serde_json::to_string(&updated.cover).map_err(|e| e.to_string())?;
    let embedded = serde_json::to_string(&embedded).map_err(|e| e.to_string())?;
    let signature = super::cover_art::signature(directory);
    let mut db = store.0.lock().map_err(|_| "曲库锁不可用")?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    // Only replace cover fields on the current row, preserving concurrent lyric bindings.
    let changed = tx.execute("UPDATE tracks SET value=json_set(value,'$.cover',json(?2)) WHERE key=?1 AND source='local'", params![track.key, cover]).map_err(|e| e.to_string())?;
    if changed > 0 {
        tx.execute("INSERT INTO local_cover_state(key,signature,embedded) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET signature=excluded.signature,embedded=excluded.embedded", params![track.key, signature, embedded]).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_rejects_duplicate_start_and_allows_restart_after_completion() {
        let job = RebuildJob::default();
        assert!(!job.status().unwrap().running);
        assert!(job.begin().unwrap().running);
        assert!(job.begin().is_err());
        job.0.lock().unwrap().running = false;
        assert!(job.begin().unwrap().running);
    }
}
