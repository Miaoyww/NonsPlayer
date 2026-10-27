use crate::model::AppResult;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Action {
    Copy,
    Save,
}

#[tauri::command]
pub(crate) async fn cover_export(
    url: String,
    title: String,
    action: Action,
    app: tauri::AppHandle,
    backend: State<'_, super::Backend>,
) -> AppResult<bool> {
    #[cfg(desktop)]
    {
        use tauri_plugin_dialog::DialogExt;

        // Do not queue multiple full-size decodes or native save dialogs.
        static EXPORT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
        let _permit = EXPORT
            .try_acquire()
            .map_err(|_| "正在处理封面，请稍后重试")?;
        let bytes = if url.starts_with("https://") || url.starts_with("http://") {
            // Same bounded cache, access checks and invalidation as runtime_cover.
            crate::infrastructure::covers::read(url, &backend, &app).await?
        } else {
            let root = backend.covers.clone();
            tauri::async_runtime::spawn_blocking(move || read_local(&root, &url))
                .await
                .map_err(|e| e.to_string())??
        };
        tauri::async_runtime::spawn_blocking(move || {
            let format = image::guess_format(&bytes).map_err(|e| e.to_string())?;
            match action {
                Action::Copy => {
                    use clipboard_rs::{common::RustImage, Clipboard, ClipboardContext};
                    let image = decode(&bytes)?;
                    let context = ClipboardContext::new().map_err(|e| e.to_string())?;
                    context
                        .set_image(clipboard_rs::common::RustImageData::from_dynamic_image(
                            image,
                        ))
                        .map_err(|e| e.to_string())?;
                }
                Action::Save => {
                    let extension = format.extensions_str().first().ok_or("封面格式不支持")?;
                    let Some(path) = app
                        .dialog()
                        .file()
                        .set_title("保存封面原图")
                        .set_file_name(format!("{}.{}", file_stem(&title), extension))
                        .add_filter("封面原图", &[*extension])
                        .blocking_save_file()
                    else {
                        return Ok(false);
                    };
                    let path = path.into_path().map_err(|e| e.to_string())?;
                    // Keep the original encoding and bytes; never save the displayed thumbnail.
                    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
                }
            }
            Ok(true)
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(desktop))]
    {
        let _ = (url, title, action, app, backend);
        Err("当前平台不支持封面导出".into())
    }
}

#[cfg(desktop)]
fn read_local(root: &std::path::Path, url: &str) -> AppResult<Vec<u8>> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let path = std::path::Path::new(url)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.starts_with(root) {
        return Err("只能导出曲库封面".into());
    }
    std::fs::read(path).map_err(|e| e.to_string())
}

#[cfg(desktop)]
fn decode(bytes: &[u8]) -> AppResult<image::DynamicImage> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = None;
    limits.max_image_height = None;
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|e| e.to_string())
}

#[cfg(desktop)]
fn file_stem(title: &str) -> String {
    let name: String = title
        .chars()
        .take(100)
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let name = name.trim().trim_matches('.');
    if name.is_empty() {
        "封面".into()
    } else {
        format!("{name} - 封面")
    }
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;

    #[test]
    fn local_export_is_confined_to_cover_assets() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        assert!(read_local(root.path(), outside.path().to_str().unwrap()).is_err());
        let path = root.path().join("cover.img");
        std::fs::write(&path, b"original image bytes").unwrap();
        assert_eq!(
            read_local(root.path(), path.to_str().unwrap()).unwrap(),
            b"original image bytes"
        );
    }

    #[test]
    fn local_export_reads_assets_larger_than_online_download_limit() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("large.img");
        let bytes = vec![7; 8 * 1024 * 1024 + 1];
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(
            read_local(root.path(), path.to_str().unwrap()).unwrap(),
            bytes
        );
    }

    #[test]
    fn clipboard_decode_accepts_wide_and_tall_images() {
        for (width, height) in [(8193, 1), (1, 8193)] {
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(width, height)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .unwrap();
            let decoded = decode(encoded.get_ref()).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (width, height));
        }
    }

    #[test]
    fn save_name_removes_path_characters() {
        assert_eq!(file_stem("../a:b\\c"), "_a_b_c - 封面");
        assert_eq!(file_stem("..."), "封面");
    }
}
