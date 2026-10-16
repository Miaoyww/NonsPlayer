use super::manifest::{is_link, safe_relative, Manifest};
use crate::model::AppResult;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

const MAX_FILES: usize = 1024;
const MAX_BYTES: u64 = 64 * 1024 * 1024;

pub fn install(source: &Path, staging: &Path) -> AppResult<Manifest> {
    fs::create_dir(staging).map_err(|e| e.to_string())?;
    let result = (|| {
        let meta = fs::symlink_metadata(source).map_err(|e| e.to_string())?;
        if is_link(&meta) {
            return Err("安装来源不允许链接".into());
        }
        if meta.is_dir() {
            copy_directory(source, staging)?;
        } else {
            extract_zip(source, staging)?;
        }
        Manifest::read(staging)
    })();
    if result.is_err() {
        let _ = (fs::remove_dir_all(staging)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("fs::remove_dir_all", error);
        });
    }
    result
}

fn copy_directory(source: &Path, target: &Path) -> AppResult<()> {
    let mut files = 0;
    let mut bytes = 0;
    for item in walkdir::WalkDir::new(source)
        .follow_links(false)
        .max_open(8)
    {
        let item = item.map_err(|e| e.to_string())?;
        if item.path() == source {
            continue;
        }
        let meta = fs::symlink_metadata(item.path()).map_err(|e| e.to_string())?;
        if is_link(&meta) {
            return Err("插件包不允许链接".into());
        }
        files += 1;
        bytes += meta.len();
        if files > MAX_FILES || bytes > MAX_BYTES {
            return Err("插件包超过文件数或容量上限".into());
        }
        let relative = item
            .path()
            .strip_prefix(source)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let destination = target.join(safe_relative(&relative)?);
        if meta.is_dir() {
            fs::create_dir(&destination).map_err(|e| e.to_string())?;
        } else if meta.is_file() {
            let mut input = fs::File::open(item.path())
                .map_err(|e| e.to_string())?
                .take(meta.len() + 1);
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(|e| e.to_string())?;
            if std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())? != meta.len() {
                return Err("安装时源文件发生变化".into());
            }
        } else {
            return Err("插件包包含非常规文件".into());
        }
    }
    Ok(())
}
fn extract_zip(source: &Path, target: &Path) -> AppResult<()> {
    let file = fs::File::open(source).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("压缩包过大".into());
    }
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    if zip.len() > MAX_FILES {
        return Err("压缩包文件数过多".into());
    }
    let mut total = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("压缩包不允许链接".into());
        }
        let name = entry.name().trim_end_matches('/');
        let destination = target.join(safe_relative(name)?);
        total += entry.size();
        if total > MAX_BYTES {
            return Err("压缩包解压容量过大".into());
        }
        if entry.is_dir() {
            fs::create_dir_all(destination).map_err(|e| e.to_string())?;
        } else {
            fs::create_dir_all(destination.parent().ok_or("文件路径无效")?)
                .map_err(|e| e.to_string())?;
            let expected = entry.size();
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(|e| e.to_string())?;
            let actual = std::io::copy(&mut (&mut entry).take(expected + 1), &mut output)
                .map_err(|e| e.to_string())?;
            if actual != expected {
                return Err("压缩包大小无效".into());
            }
            output.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::{write::SimpleFileOptions, ZipWriter};
    fn archive(path: &Path, files: &[(&str, &[u8])]) {
        let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
        for (name, value) in files {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(value).unwrap();
        }
        zip.finish().unwrap();
    }
    #[test]
    fn directory_and_zip_install_validate_the_same_package() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let manifest = br#"{"id":"sample","name":"Sample","version":"1.0.0","frontend":"ui.mjs","permissions":["ui"],"engines":{"app":"^1.0.0","pluginApi":"^2.0.0","uiApi":"^2.0.0"}}"#;
        fs::write(source.join("manifest.json"), manifest).unwrap();
        fs::write(source.join("ui.mjs"), b"export const View=()=>null;").unwrap();
        assert_eq!(
            install(&source, &temp.path().join("copy")).unwrap().id,
            "sample"
        );
        let zip = temp.path().join("plugin.zip");
        archive(
            &zip,
            &[
                ("manifest.json", manifest),
                ("ui.mjs", b"export const View=()=>null;"),
            ],
        );
        assert_eq!(
            install(&zip, &temp.path().join("unzip")).unwrap().id,
            "sample"
        );
    }
    #[test]
    fn zip_traversal_missing_entries_and_bombs_leave_no_staging_files() {
        let temp = tempfile::tempdir().unwrap();
        let zip = temp.path().join("plugin.zip");
        let staging = temp.path().join("staging");
        archive(&zip, &[("../escape", b"bad")]);
        assert!(install(&zip, &staging).is_err());
        assert!(!temp.path().join("escape").exists());
        assert!(!staging.exists());
        archive(&zip, &[("ui.mjs", b"bad")]);
        assert!(install(&zip, &staging).is_err());
        assert!(!staging.exists());
        let oversized = vec![0; (MAX_BYTES + 1) as usize];
        archive(&zip, &[("assets/bomb", &oversized)]);
        assert!(install(&zip, &staging).is_err());
        assert!(!staging.exists());
    }
}
