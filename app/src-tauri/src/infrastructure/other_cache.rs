//! Independent, rebuildable resources. Library assets and user data are never traversed.
use crate::model::AppResult;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Usage {
    pub(crate) used_bytes: u64,
    pub(crate) entries: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    covers: Usage,
    collections: Usage,
    metadata: Usage,
    compiled: Usage,
    logs: Usage,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Kind {
    Covers,
    Collections,
    Metadata,
    Compiled,
    Logs,
}

fn logs_directory(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    match super::directories::portable() {
        Some(data) => Ok(data.join("logs")),
        None => app.path().app_log_dir().map_err(|error| error.to_string()),
    }
}

#[tauri::command]
pub(crate) async fn other_cache_status(
    app: tauri::AppHandle,
    backend: State<'_, crate::Backend>,
    plugins: State<'_, std::sync::Arc<crate::plugins::PluginManager>>,
) -> AppResult<Status> {
    let covers = super::covers::cache_status().await;
    let collections = match &backend.music.collections {
        Some(collections) => collections
            .cache_status()
            .await
            .map_err(|error| error.to_string())?,
        None => Usage::default(),
    };
    let metadata = plugins.music_cache_status().await?;
    let directory = logs_directory(&app)?;
    let (compiled, logs) = tauri::async_runtime::spawn_blocking(move || -> AppResult<_> {
        Ok((
            super::wasm_runtime::compilation_cache_status()?,
            super::diagnostics::cache_status(&directory)?,
        ))
    })
    .await
    .map_err(|error| error.to_string())??;
    Ok(Status {
        covers,
        collections,
        metadata,
        compiled,
        logs,
    })
}

#[tauri::command]
pub(crate) async fn clear_other_cache(
    kind: Kind,
    app: tauri::AppHandle,
    backend: State<'_, crate::Backend>,
    plugins: State<'_, std::sync::Arc<crate::plugins::PluginManager>>,
) -> AppResult<()> {
    match kind {
        Kind::Covers => super::covers::clear_cache().await,
        Kind::Collections => {
            if let Some(collections) = &backend.music.collections {
                collections
                    .clear_cache()
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        Kind::Metadata => plugins.clear_music_cache().await,
        Kind::Compiled => {
            tauri::async_runtime::spawn_blocking(super::wasm_runtime::clear_compilation_cache)
                .await
                .map_err(|error| error.to_string())??;
        }
        Kind::Logs => {
            let directory = logs_directory(&app)?;
            tauri::async_runtime::spawn_blocking(move || {
                super::diagnostics::clear_cache(&directory)
            })
            .await
            .map_err(|error| error.to_string())??;
        }
    }
    Ok(())
}

fn is_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

// Walk only a host-selected resource directory, never links or reparse points.
pub(crate) fn resource_files(directory: &Path, recursive: bool) -> AppResult<Vec<PathBuf>> {
    let metadata = match std::fs::symlink_metadata(directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error.to_string()),
    };
    if is_link(&metadata) || !metadata.is_dir() {
        return Err("缓存目录不可用或包含目录链接".into());
    }
    let root = directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let mut files = Vec::new();
    let walk = walkdir::WalkDir::new(&root)
        .min_depth(1)
        .max_depth(if recursive { usize::MAX } else { 1 })
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            std::fs::symlink_metadata(entry.path())
                .map(|metadata| !is_link(&metadata))
                .unwrap_or(true)
        });
    for entry in walk {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error)
                if error
                    .io_error()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
            {
                continue
            }
            Err(error) => return Err(error.to_string()),
        };
        if !entry.file_type().is_file() {
            continue;
        }
        checked_resource(&root, entry.path())?;
        files.push(entry.into_path());
    }
    Ok(files)
}

pub(crate) fn checked_resource(directory: &Path, file: &Path) -> AppResult<()> {
    let metadata = match std::fs::symlink_metadata(file) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    if is_link(&metadata) || !metadata.is_file() {
        return Err("缓存文件不是普通文件".into());
    }
    let root = directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let target = match file.canonicalize() {
        Ok(target) => target,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    if !target.starts_with(root) {
        return Err("缓存文件超出专属目录".into());
    }
    Ok(())
}

pub(crate) fn file_usage(files: &[PathBuf]) -> AppResult<Usage> {
    let mut usage = Usage::default();
    for file in files {
        match std::fs::symlink_metadata(file) {
            Ok(metadata) if metadata.is_file() && !is_link(&metadata) => {
                usage.used_bytes = usage.used_bytes.saturating_add(metadata.len());
                usage.entries += 1;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(usage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_scan_keeps_other_directories_and_rejects_outside_files() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        std::fs::create_dir(&cache).unwrap();
        std::fs::write(cache.join("compiled"), [1, 2, 3]).unwrap();
        let outside = root.path().join("music.flac");
        std::fs::write(&outside, [4, 5]).unwrap();
        let files = resource_files(&cache, true).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(file_usage(&files).unwrap().used_bytes, 3);
        assert!(checked_resource(&cache, &outside).is_err());
        assert!(outside.exists());
    }

    #[cfg(unix)]
    #[test]
    fn resource_scan_never_follows_links() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        std::fs::create_dir(&cache).unwrap();
        let outside = root.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("music"), [1]).unwrap();
        std::os::unix::fs::symlink(&outside, cache.join("link")).unwrap();
        assert!(resource_files(&cache, true).unwrap().is_empty());
        assert!(resource_files(&cache.join("link"), true).is_err());
    }
}
