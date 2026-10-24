use crate::{
    model::{AppResult, Track, TrackSource},
    storage::Store,
};
use lofty::prelude::*;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub(super) static COVER_WRITES: Mutex<()> = Mutex::new(());

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "m4a", "aac", "ogg", "opus", "aiff", "aif", "ape", "wv", "ncm",
];

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

pub fn import(
    mut paths: Vec<String>,
    store: Arc<Store>,
    cover_dir: PathBuf,
    fallback: String,
    origin: Option<String>,
) -> AppResult<ImportReport> {
    let _writes = COVER_WRITES.lock().map_err(|_| "封面写入锁不可用")?;
    prepare_covers(&cover_dir, &fallback)?;
    let _validation = super::cover_art::ValidationSession::new();
    if paths.len() > 1000 {
        return Err("单次选择的路径过多".into());
    }
    paths.sort_by_cached_key(|path| super::cover_art::normalized(Path::new(path)));
    let mut report = ImportReport {
        imported: 0,
        skipped: 0,
        errors: vec![],
    };
    let mut batch = Vec::with_capacity(64);
    let mut seen = std::collections::HashSet::new();
    let mut complete = true;
    let mut covered = std::collections::HashSet::new();
    let mut cover_bytes = super::cover_art::usage(&cover_dir);
    // Process sheets first so traversal order cannot briefly expose the whole album.
    for root in &paths {
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .max_open(8)
            .sort_by_file_name()
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() || !super::cue::is_cue(entry.path()) {
                continue;
            }
            match import_sheet(
                entry.path(),
                &cover_dir,
                &fallback,
                &store,
                &mut cover_bytes,
            ) {
                Ok((base, tracks)) => {
                    covered.insert(base.key.clone());
                    let mut imported = tracks;
                    report.imported += imported.len();
                    imported.push(base);
                    seen.extend(imported.iter().map(|track| track.key.clone()));
                    let sheet_origin = format!(
                        "cue:{}",
                        entry
                            .path()
                            .canonicalize()
                            .map_err(|e| e.to_string())?
                            .display()
                    );
                    store.mark_origin(&imported, origin.as_deref().unwrap_or(&sheet_origin))?;
                    if origin.is_none() {
                        store.reconcile_origin(
                            &sheet_origin,
                            &imported.iter().map(|track| track.key.clone()).collect(),
                        )?;
                    }
                }
                Err(error) => {
                    report.skipped += 1;
                    if report.errors.len() < 20 {
                        report
                            .errors
                            .push(format!("{}：{error}", entry.file_name().to_string_lossy()));
                    }
                }
            }
        }
    }
    for root in paths {
        // walkdir does not follow symlinks; max_open bounds file handles.
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .max_open(8)
            .sort_by_file_name()
        {
            let entry = match entry {
                Ok(e) => e,
                Err(error) => {
                    crate::infrastructure::diagnostics::warn_error(
                        "local.scan.walk_directory",
                        &error,
                    );
                    report.skipped += 1;
                    complete = false;
                    continue;
                }
            };
            if !entry.file_type().is_file() || !is_audio(entry.path()) {
                continue;
            }
            if let Ok(path) = (entry.path().canonicalize()).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "local.scan.canonicalize_audio",
                    error,
                );
            }) {
                let path = path
                    .to_string_lossy()
                    .trim_start_matches(r"\\?\")
                    .to_string();
                seen.insert(format!("local:{:x}", Sha256::digest(path.as_bytes())));
            }
            match read_track(
                entry.path(),
                &cover_dir,
                &fallback,
                &store,
                &mut cover_bytes,
            ) {
                Ok(track) => {
                    if covered.contains(&track.key) {
                        continue;
                    }
                    batch.push(track);
                    report.imported += 1;
                }
                Err(error) => {
                    crate::infrastructure::diagnostics::warn("local.read_track", &error);
                    report.skipped += 1;
                    if report.errors.len() < 20 {
                        report
                            .errors
                            .push(format!("{}：{error}", entry.file_name().to_string_lossy()));
                    }
                }
            }
            if batch.len() == 64 {
                store.mark_origin(&batch, origin.as_deref().unwrap_or(""))?;
                batch.clear();
            }
        }
    }
    store.mark_origin(&batch, origin.as_deref().unwrap_or(""))?;
    if complete {
        if let Some(origin) = origin {
            store.reconcile_origin(&origin, &seen)?;
        }
    }
    store.refresh_local_albums()?;
    Ok(report)
}

pub(super) fn prepare_covers(covers: &Path, fallback: &str) -> AppResult<()> {
    use std::io::Write;
    std::fs::create_dir_all(covers).map_err(|e| e.to_string())?;
    if !fallback.is_empty() && !super::cover_art::valid(Path::new(fallback)) {
        let mut temporary = tempfile::NamedTempFile::new_in(covers).map_err(|e| e.to_string())?;
        temporary
            .write_all(include_bytes!("../../icons/128x128.png"))
            .map_err(|e| e.to_string())?;
        temporary.flush().map_err(|e| e.to_string())?;
        temporary.persist(fallback).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn import_sheet(
    path: &Path,
    covers: &Path,
    fallback: &str,
    store: &Store,
    cover_bytes: &mut u64,
) -> AppResult<(Track, Vec<Track>)> {
    let sheet = super::cue::read(path)?;
    let audio = path
        .parent()
        .ok_or("CUE 目录不可用")?
        .join(sheet.file.replace('\\', "/"));
    if !is_audio(&audio) {
        return Err("CUE 引用的音频格式不受支持".into());
    }
    let base = read_track(&audio, covers, fallback, store, cover_bytes)?;
    let information = store.local_information(&base.key)?;
    let album_artist = sheet
        .performer
        .clone()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| information.album_artist.clone());
    let mut tracks = super::cue::expand(sheet, path, &base)?;
    let embedded = store
        .local_cover_state(&base.key)?
        .unwrap_or_default()
        .embedded;
    let directory = path.parent().ok_or("CUE 目录不可用")?;
    let cover = embedded
        .first()
        .cloned()
        .or_else(|| super::cover_art::external(directory, covers, cover_bytes))
        .unwrap_or_else(|| fallback.to_owned());
    for track in &mut tracks {
        track.cover = cover.clone().into();
        store.save_local_cover_state(
            &track.key,
            &super::cover_state::CoverState {
                signature: super::cover_art::signature(directory),
                embedded: embedded.clone(),
            },
        )?;
    }
    for track in &mut tracks {
        let mut metadata = information.clone();
        metadata.track_number = track.cue().map(|cue| cue.track_number);
        metadata.album_artist = album_artist.clone();
        // A binding made for a virtual track must survive rescans, unlike the whole-file binding.
        if let TrackSource::Local { netease_id, .. } = &mut track.source {
            *netease_id = store
                .track(&track.key)
                .ok()
                .and_then(|old| old.netease_id());
        }
        store.save_local_information(&track.key, &metadata)?;
    }
    store.save_tracks(&tracks)?;
    Ok((base, tracks))
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| AUDIO_EXTENSIONS.contains(&s.to_ascii_lowercase().as_str()))
}

pub(super) fn read_track(
    path: &Path,
    cover_dir: &Path,
    fallback: &str,
    store: &Store,
    cover_bytes: &mut u64,
) -> AppResult<Track> {
    let path = path
        .canonicalize()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn_error("local.import.canonicalize", error);
        })
        .map_err(|_| "文件路径不可用")?;
    let path = path
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();
    let key = format!("local:{:x}", Sha256::digest(path.as_bytes()));
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let modified = format!("{:?}", metadata.modified().map_err(|e| e.to_string())?);
    let directory = Path::new(&path).parent().ok_or("音乐目录不可用")?;
    let signature = super::cover_art::signature(directory);
    if store.file_unchanged(&key, &modified, metadata.len())?
        && store.has_local_information(&key)?
    {
        if let Ok(track) = (store.track(&key)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("local.import.read_cached_track", error);
        }) {
            if let Some(state) = store.local_cover_state(&key)? {
                if state.signature == signature
                    && (track.cover.is_empty() || super::cover_art::valid(Path::new(&*track.cover)))
                    && state
                        .embedded
                        .iter()
                        .all(|cover| super::cover_art::valid(Path::new(cover)))
                {
                    return Ok(track);
                }
            }
        }
    }
    let (tagged, ncm_info, ncm_image) = super::encoded_audio::read_tags(&path)?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let mut embedded = Vec::new();
    for picture in tag.into_iter().flat_map(|tag| tag.pictures()) {
        if let Some(cover) = super::cover_art::asset(picture.data(), cover_dir, cover_bytes) {
            if !embedded.contains(&cover) {
                embedded.push(cover);
            }
        }
    }
    if let Some(cover) = super::cover_art::asset(&ncm_image, cover_dir, cover_bytes) {
        if !embedded.contains(&cover) {
            embedded.push(cover);
        }
    }
    let cover = embedded
        .first()
        .cloned()
        .or_else(|| super::cover_art::external(directory, cover_dir, cover_bytes))
        .unwrap_or_else(|| fallback.to_owned());
    store.save_local_cover_state(
        &key,
        &super::cover_state::CoverState {
            signature,
            embedded,
        },
    )?;
    // Re-importing a file must preserve a user's explicit lyric binding.
    let netease_id = store
        .track(&key)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("local.import.restore_lyric_binding", error);
        })
        .ok()
        .and_then(|t| t.netease_id())
        .or_else(|| ncm_info.as_ref().map(|i| i.id).filter(|id| *id > 0));
    let information = crate::local_library::information(&tagged, &path);
    let mut track = Track {
        aliases: Vec::new(),
        artists: Vec::new(),
        key: key.clone(),
        title: tag
            .and_then(|t| t.title())
            .filter(|s| !s.is_empty())
            .map(|s| s.into_owned())
            .unwrap_or_else(|| {
                Path::new(&path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            }),
        artist: tag
            .and_then(|t| t.artist())
            .filter(|s| !s.is_empty())
            .map(|s| s.into_owned())
            .unwrap_or_else(|| "未知艺术家".into()),
        album: tag
            .and_then(|t| t.album())
            .filter(|s| !s.is_empty())
            .map(|s| s.into_owned())
            .unwrap_or_else(|| "未知专辑".into()),
        duration_ms: tagged.properties().duration().as_millis() as u64,
        cover: cover.into(),
        source: TrackSource::Local {
            path,
            netease_id,
            cue: None,
        },
    };
    if let Some(info) = ncm_info {
        if !info.name.is_empty() {
            track.title = info.name;
        }
        if !info.album.is_empty() {
            track.album = info.album;
        }
        if !info.artist.is_empty() {
            track.artist = info
                .artist
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join("/");
        }
        if track.duration_ms == 0 {
            track.duration_ms = info.duration;
        }
        track.aliases = info.alias.unwrap_or_default();
    }
    store.save_local_information(&key, &information)?;
    store.save_tracks(std::slice::from_ref(&track))?;
    store.save_file_stat(&key, &modified, metadata.len())?;
    Ok(track)
}
