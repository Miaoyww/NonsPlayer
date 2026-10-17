use crate::{model::AppResult, storage::MAX_LYRIC_BYTES};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

// Only CUE tracks use a directory image; keep the original audio's cached tags unchanged.
pub(super) fn find(sheet: &Path, covers: &Path, used: &mut u64) -> AppResult<Option<String>> {
    let Some(parent) = sheet.parent() else {
        return Ok(None);
    };
    const NAMES: &[&str] = &[
        "cover.jpg",
        "cover.png",
        "folder.jpg",
        "folder.png",
        "front.jpg",
        "front.png",
        "album.jpg",
    ];
    let files = std::fs::read_dir(parent)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().to_ascii_lowercase(),
                entry.path(),
            )
        })
        .filter(|(name, _)| NAMES.contains(&name.as_str()))
        .collect::<Vec<_>>();
    for &name in NAMES {
        for (_, path) in files.iter().filter(|(file, _)| file == name) {
            let result = (|| -> AppResult<Option<String>> {
                let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
                if file.metadata().map_err(|e| e.to_string())?.len() > MAX_LYRIC_BYTES as u64 {
                    return Ok(None);
                }
                let mut bytes = Vec::new();
                file.take(MAX_LYRIC_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > MAX_LYRIC_BYTES
                    || !matches!(
                        image::guess_format(&bytes),
                        Ok(image::ImageFormat::Jpeg | image::ImageFormat::Png)
                    )
                {
                    return Ok(None);
                }
                let dest = covers.join(format!("{:x}.img", Sha256::digest(&bytes)));
                if !dest.exists() {
                    if *used + bytes.len() as u64 > 32 * 1024 * 1024 {
                        return Ok(None);
                    }
                    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
                    *used += bytes.len() as u64;
                }
                Ok(Some(dest.to_string_lossy().into_owned()))
            })();
            match result {
                Ok(Some(cover)) => return Ok(Some(cover)),
                Ok(None) => {}
                Err(error) => crate::infrastructure::diagnostics::warn("local.cue_cover", error),
            }
        }
    }
    Ok(None)
}
