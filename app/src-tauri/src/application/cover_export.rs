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
                    let image = decode(&bytes)?;
                    copy_image(image, &app)?;
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

#[cfg(all(desktop, not(windows)))]
fn copy_image(image: image::DynamicImage, _app: &tauri::AppHandle) -> AppResult<()> {
    use clipboard_rs::{common::RustImage, Clipboard, ClipboardContext};
    ClipboardContext::new()
        .map_err(|e| e.to_string())?
        .set_image(clipboard_rs::common::RustImageData::from_dynamic_image(
            image,
        ))
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn copy_image(image: image::DynamicImage, app: &tauri::AppHandle) -> AppResult<()> {
    use tauri::Manager;
    let window = app.get_window("main").ok_or("主窗口不可用")?;
    let owner = window.hwnd().map_err(|e| e.to_string())?;
    copy_windows_image(image, owner.0)
}

#[cfg(windows)]
fn open_windows_clipboard(
    owner: clipboard_win::types::HWND,
) -> AppResult<clipboard_win::Clipboard> {
    // A null owner lets other null-owner threads open and close this transaction.
    if owner.is_null() {
        return Err("复制封面需要有效窗口".into());
    }
    clipboard_win::Clipboard::new_attempts_for(owner, 10)
        .map_err(|e| format!("打开剪贴板失败：{e}"))
}

#[cfg(windows)]
fn copy_windows_image(
    image: image::DynamicImage,
    owner: clipboard_win::types::HWND,
) -> AppResult<()> {
    use clipboard_rs::common::{RustImage, RustImageData};
    let image = RustImageData::from_dynamic_image(image);
    // Encoding can take seconds; finish it before acquiring or clearing the clipboard.
    let png = image.to_png().map_err(|e| e.to_string())?;
    let bitmap = image.to_bitmap().map_err(|e| e.to_string())?;
    // CF_DIB contains the encoded DIB header and pixels, without the 14-byte BMP file header.
    let dib = bitmap.get_bytes().get(14..).ok_or("封面位图数据无效")?;
    let format = clipboard_win::register_format("PNG").ok_or("注册剪贴板图片格式失败")?;
    let _clipboard = open_windows_clipboard(owner)?;
    clipboard_win::empty().map_err(|e| format!("清空剪贴板失败：{e}"))?;
    clipboard_win::raw::set_without_clear(format.get(), png.get_bytes())
        .map_err(|e| format!("写入剪贴板 PNG 失败：{e}"))?;
    clipboard_win::raw::set_without_clear(clipboard_win::formats::CF_DIB, dib)
        .map_err(|e| format!("写入剪贴板位图失败：{e}"))
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

    #[cfg(windows)]
    mod windows_clipboard {
        use super::*;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
        };

        struct Owner(HWND);

        impl Owner {
            fn new() -> Self {
                Self(unsafe {
                    CreateWindowExW(
                        WINDOW_EX_STYLE::default(),
                        windows::core::w!("STATIC"),
                        windows::core::w!("Nons cover clipboard test"),
                        WINDOW_STYLE::default(),
                        0,
                        0,
                        0,
                        0,
                        Some(HWND_MESSAGE),
                        None,
                        None,
                        None,
                    )
                    .unwrap()
                })
            }
        }

        impl Drop for Owner {
            fn drop(&mut self) {
                unsafe { DestroyWindow(self.0).unwrap() };
            }
        }

        #[test]
        #[ignore = "requires exclusive access to the Windows clipboard"]
        fn clipboard_owner_prevents_another_thread_from_closing_transaction() {
            let owner = Owner::new();
            let _clipboard = open_windows_clipboard(owner.0 .0).unwrap();
            std::thread::spawn(|| {
                // Null-owner readers must not acquire a guard that can close our transaction.
                assert!(clipboard_win::Clipboard::new_attempts(0).is_err());
            })
            .join()
            .unwrap();
            assert!(open_windows_clipboard(std::ptr::null_mut()).is_err());
        }

        #[test]
        #[ignore = "writes images to the Windows clipboard"]
        fn clipboard_image_roundtrip_preserves_png_and_bitmap() {
            let owner = Owner::new();
            for side in [32, 1024, 3000] {
                let image = image::RgbaImage::from_fn(side, side, |x, y| {
                    image::Rgba([(x % 251) as u8, (y % 251) as u8, ((x * y) % 251) as u8, 255])
                });
                copy_windows_image(image.clone().into(), owner.0 .0).unwrap();
                let _clipboard = open_windows_clipboard(owner.0 .0).unwrap();
                let png_format = clipboard_win::register_format("PNG").unwrap();
                let png: Vec<u8> =
                    clipboard_win::get(clipboard_win::formats::RawData(png_format.get())).unwrap();
                assert_eq!(image::load_from_memory(&png).unwrap().into_rgba8(), image);
                let bitmap: Vec<u8> = clipboard_win::get(clipboard_win::formats::Bitmap).unwrap();
                assert_eq!(
                    image::load_from_memory(&bitmap).unwrap().into_rgba8(),
                    image
                );
            }
        }
    }
}
