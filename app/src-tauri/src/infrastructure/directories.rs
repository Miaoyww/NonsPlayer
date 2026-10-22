//! A portable marker selects executable-relative storage before any WebView is created.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PORTABLE_DATA: OnceLock<Option<PathBuf>> = OnceLock::new();

#[cfg(any(windows, test))]
fn portable_data(executable: &Path) -> std::io::Result<Option<PathBuf>> {
    let directory = executable
        .parent()
        .ok_or_else(|| std::io::Error::other("程序目录不可用"))?;
    match std::fs::metadata(directory.join("portable.flag")) {
        Ok(marker) if marker.is_file() => Ok(Some(directory.join("data"))),
        Ok(_) => Err(std::io::Error::other("portable.flag 必须是文件")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn initialize() -> std::io::Result<()> {
    #[cfg(windows)]
    let data = match crate::performance::root() {
        Some(root) => Some(root.join("data")),
        None => portable_data(&std::env::current_exe()?)?,
    };
    #[cfg(not(windows))]
    let data: Option<PathBuf> = None;
    if let Some(data) = &data {
        for child in ["cache", "temp", "credentials", "webview"] {
            std::fs::create_dir_all(data.join(child))?;
        }
        // Fail explicitly on a read-only program directory, without falling back to AppData.
        tempfile::NamedTempFile::new_in(data)?;
    }
    PORTABLE_DATA
        .set(data)
        .inspect_err(|_| {
            crate::infrastructure::diagnostics::warn(
                "directories.initialize",
                "data directory was already initialized",
            );
        })
        .map_err(|_| std::io::Error::other("数据目录已初始化"))
}

pub(crate) fn portable() -> Option<&'static Path> {
    PORTABLE_DATA.get().and_then(Option::as_deref)
}

pub(crate) fn temporary() -> PathBuf {
    portable()
        .map(|p| p.join("temp"))
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_selects_program_directory_only() {
        let folder = tempfile::tempdir().unwrap();
        let executable = folder.path().join("Nons.exe");
        assert_eq!(portable_data(&executable).unwrap(), None);
        std::fs::write(folder.path().join("portable.flag"), b"").unwrap();
        assert_eq!(
            portable_data(&executable).unwrap(),
            Some(folder.path().join("data"))
        );
    }

    #[test]
    fn invalid_marker_does_not_fall_back() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir(folder.path().join("portable.flag")).unwrap();
        assert!(portable_data(&folder.path().join("Nons.exe")).is_err());
    }
}
