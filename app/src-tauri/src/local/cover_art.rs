use crate::{
    model::{AppResult, Track, TrackSource},
    storage::MAX_LYRIC_BYTES,
};
use image::ImageDecoder;
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

const BUDGET: u64 = 32 * 1024 * 1024;
const SUBDIRECTORIES: &[&str] = &[
    "art",
    "scans",
    "scan",
    "booklet",
    "images",
    "image",
    "cover",
    "covers",
    "coverart",
    "albumart",
    "gallery",
    "jacket",
    "artwork",
    "bonus",
    "bk",
    "cover artwork",
    "cover art",
];

#[derive(Default)]
struct ValidationCache {
    depth: usize,
    entries: std::collections::HashMap<PathBuf, bool>,
}
thread_local! {
    static VALIDATION: std::cell::RefCell<ValidationCache> = std::cell::RefCell::new(ValidationCache::default());
}

pub(super) struct ValidationSession(std::marker::PhantomData<std::rc::Rc<()>>);
impl ValidationSession {
    pub(super) fn new() -> Self {
        VALIDATION.with_borrow_mut(|cache| {
            if cache.depth == 0 {
                cache.entries.clear();
            }
            cache.depth += 1;
        });
        Self(std::marker::PhantomData)
    }
}
impl Drop for ValidationSession {
    fn drop(&mut self) {
        VALIDATION.with_borrow_mut(|cache| {
            cache.depth -= 1;
            if cache.depth == 0 {
                cache.entries.clear();
            }
        });
    }
}

fn remember(path: &Path, result: bool) {
    VALIDATION.with_borrow_mut(|cache| {
        if cache.depth > 0 {
            if cache.entries.len() >= 512 {
                cache.entries.clear();
            }
            cache.entries.insert(path.to_path_buf(), result);
        }
    });
}

pub(super) fn normalized(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/")
}

pub(super) fn directory(track: &Track) -> Option<&Path> {
    match &track.source {
        TrackSource::Local { path, cue, .. } => Path::new(
            cue.as_ref()
                .map_or(path.as_str(), |c| c.sheet_path.as_str()),
        )
        .parent(),
        _ => None,
    }
}

fn supported(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "bmp" | "gif" | "jxl"
        )
    })
}

pub(super) fn candidates(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_file() && supported(&path) {
            found.push(path);
        } else if kind.is_dir()
            && SUBDIRECTORIES.contains(
                &entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .as_str(),
            )
        {
            if let Ok(children) = std::fs::read_dir(path) {
                found.extend(
                    children
                        .flatten()
                        .filter(|e| {
                            e.file_type().is_ok_and(|t| t.is_file()) && supported(&e.path())
                        })
                        .map(|e| e.path()),
                );
            }
        }
    }
    found.sort_by_cached_key(|path| {
        let relative = path.strip_prefix(directory).unwrap_or(path);
        let priority = if relative.components().count() > 1 {
            3
        } else {
            match path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "folder" => 0,
                "cover" => 1,
                _ => 2,
            }
        };
        (priority, normalized(relative))
    });
    found
}

pub(super) fn signature(directory: &Path) -> String {
    let mut digest = Sha256::new();
    for path in candidates(directory) {
        let relative = normalized(path.strip_prefix(directory).unwrap_or(&path));
        digest.update((relative.len() as u64).to_le_bytes());
        digest.update(relative.as_bytes());
        match path.metadata() {
            Ok(metadata) => {
                digest.update(metadata.len().to_le_bytes());
                digest.update(format!("{:?}", metadata.modified()).as_bytes());
            }
            Err(error) => digest.update(format!("{:?}", error.kind()).as_bytes()),
        }
    }
    format!("{:x}", digest.finalize())
}

fn read(path: &Path) -> Option<Vec<u8>> {
    if !std::fs::symlink_metadata(path).ok()?.file_type().is_file() {
        return None;
    }
    let file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() || file.metadata().ok()?.len() > MAX_LYRIC_BYTES as u64 {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_LYRIC_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= MAX_LYRIC_BYTES).then_some(bytes)
}

fn limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    limits
}

fn is_jxl(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xff, 0x0a]) || bytes.starts_with(b"\0\0\0\x0cJXL \x0d\x0a\x87\x0a")
}

fn decode(bytes: &[u8]) -> Option<image::DynamicImage> {
    if bytes.is_empty() || bytes.len() > MAX_LYRIC_BYTES {
        return None;
    }
    if is_jxl(bytes) {
        let mut decoder = jxl_oxide::integration::JxlDecoder::new(Cursor::new(bytes)).ok()?;
        decoder.set_limits(limits()).ok()?;
        image::DynamicImage::from_decoder(decoder).ok()
    } else {
        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        reader.limits(limits());
        reader.decode().ok()
    }
}

pub(super) fn valid(path: &Path) -> bool {
    if let Some(result) = VALIDATION.with_borrow(|cache| {
        (cache.depth > 0)
            .then(|| cache.entries.get(path).copied())
            .flatten()
    }) {
        return result;
    }
    let result = valid_uncached(path);
    remember(path, result);
    result
}

fn valid_uncached(path: &Path) -> bool {
    let Some(bytes) = read(path) else {
        return false;
    };
    if path.extension().is_some_and(|ext| ext == "img") {
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if path.file_stem().and_then(|s| s.to_str()) != Some(digest.as_str()) {
            return false;
        }
    }
    decode(&bytes).is_some()
}

pub(super) fn usage(covers: &Path) -> u64 {
    std::fs::read_dir(covers)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

pub(super) fn asset(bytes: &[u8], covers: &Path, used: &mut u64) -> Option<String> {
    let decoded = decode(bytes)?;
    let converted;
    let bytes = if is_jxl(bytes) {
        let mut output = Cursor::new(Vec::new());
        decoded
            .write_to(&mut output, image::ImageFormat::Png)
            .ok()?;
        converted = output.into_inner();
        if converted.len() > MAX_LYRIC_BYTES {
            return None;
        }
        converted.as_slice()
    } else {
        bytes
    };
    let dest = covers.join(format!("{:x}.img", Sha256::digest(bytes)));
    if read(&dest).as_deref() == Some(bytes) {
        remember(&dest, true);
        return Some(dest.to_string_lossy().into_owned());
    }
    static WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = WRITE.lock().ok()?;
    if read(&dest).as_deref() == Some(bytes) {
        remember(&dest, true);
        return Some(dest.to_string_lossy().into_owned());
    }
    *used = usage(covers);
    let previous = dest
        .symlink_metadata()
        .ok()
        .filter(|m| m.is_file())
        .map_or(0, |m| m.len());
    let next = used
        .saturating_sub(previous)
        .saturating_add(bytes.len() as u64);
    if next > BUDGET {
        return None;
    }
    let result = (|| -> std::io::Result<()> {
        let mut temporary = tempfile::NamedTempFile::new_in(covers)?;
        temporary.write_all(bytes)?;
        temporary.flush()?;
        temporary.persist(&dest).map_err(|error| error.error)?;
        Ok(())
    })();
    if let Err(error) = result {
        crate::infrastructure::diagnostics::warn("local.cover_asset.write", error);
        return None;
    }
    *used = next;
    remember(&dest, true);
    Some(dest.to_string_lossy().into_owned())
}

pub(super) fn external(directory: &Path, covers: &Path, used: &mut u64) -> Option<String> {
    for path in candidates(directory) {
        if let Some(found) = read(&path).and_then(|bytes| asset(&bytes, covers, used)) {
            return Some(found);
        }
    }
    None
}

pub(super) fn album_cover(db: &Connection, tracks: &[Track]) -> AppResult<Option<String>> {
    let Some(covers) = tracks
        .iter()
        .map(|track| Path::new(&*track.cover))
        .find(|path| path.is_absolute())
        .and_then(Path::parent)
    else {
        return Ok(None);
    };
    let mut embedded = db
        .prepare_cached("SELECT embedded FROM local_cover_state WHERE key=?1")
        .map_err(|e| e.to_string())?;
    for track in tracks {
        let value: Option<String> = embedded
            .query_row([&track.key], |row| row.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(value) = value {
            match serde_json::from_str::<Vec<String>>(&value) {
                Ok(paths) => {
                    for path in paths {
                        if Path::new(&path).parent() == Some(covers) && valid(Path::new(&path)) {
                            return Ok(Some(path));
                        }
                    }
                }
                Err(error) => {
                    crate::infrastructure::diagnostics::warn("local.cover_state.decode", error);
                    db.execute("DELETE FROM local_cover_state WHERE key=?1", [&track.key])
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    let directories: BTreeMap<String, &Path> = tracks
        .iter()
        .filter_map(directory)
        .map(|path| (normalized(path), path))
        .collect();
    let mut used = usage(covers);
    for directory in directories.values() {
        if let Some(cover) = external(directory, covers, &mut used) {
            return Ok(Some(cover));
        }
    }
    Ok(None)
}

#[cfg(test)]
#[path = "cover_art_tests.rs"]
mod tests;
