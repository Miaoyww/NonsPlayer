//! Scoped audio operations; the plugin owns selection/batch policy and presentation.
use super::files::Files;
use crate::{local::encoded_audio::Decoder, model::AppResult};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub async fn pick(context: &super::capability::Context, args: &Value) -> AppResult<Value> {
    use tauri_plugin_dialog::DialogExt;
    context.check(Some("files:selected"))?;
    let unrestricted = context.file_roots.iter().any(|r| r == "*");
    let extensions: Vec<String> = if let Some(value) = args.get("extensions") {
        serde_json::from_value(value.clone())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.audio.pick.parse_extensions",
                    error,
                );
            })
            .map_err(|_| "音频筛选扩展名无效")?
    } else {
        crate::library::AUDIO_EXTENSIONS
            .iter()
            .map(|s| s.to_string())
            .collect()
    };
    if extensions.is_empty()
        || extensions.len() > crate::library::AUDIO_EXTENSIONS.len()
        || extensions
            .iter()
            .any(|s| !crate::library::AUDIO_EXTENSIONS.contains(&s.as_str()))
    {
        return Err("不支持的音频筛选扩展名".into());
    }
    let app = context.app.clone();
    let selection = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(if unrestricted {
                "选择音频文件（插件已有任意文件读写权限）"
            } else {
                "选择音频文件并授权所在目录读写（转换结果保存在原目录）"
            })
            .add_filter(
                "音频文件",
                &extensions.iter().map(String::as_str).collect::<Vec<_>>(),
            )
            .blocking_pick_files()
    })
    .await
    .map_err(|e| e.to_string())?;
    context.check(Some("files:selected"))?;
    let paths = selection.unwrap_or_default();
    if paths.len() > 128 {
        return Err("单次最多选择 128 个文件".into());
    }
    let context = context.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut items = Vec::new();
        for path in paths {
            context.check(Some("files:selected"))?;
            let path = path.into_path().map_err(|e| e.to_string())?.canonicalize().map_err(|e| e.to_string())?;
            if unrestricted {
                items.push(json!({"root":"*","path":path.to_string_lossy().trim_start_matches(r"\\?\"),"name":path.file_name().unwrap_or_default().to_string_lossy()}));
                continue;
            }
            let parent = path.parent().ok_or("文件目录无效")?;
            let name = path.file_name().and_then(|s| s.to_str()).ok_or("文件名无效")?;
            let grants = context.files.grants(&context.id)?;
            let grant = match grants.into_iter().find(|g| g.writable && Path::new(&g.path) == parent) {
                Some(grant) => grant,
                None => context.files.grant(&context.id, parent.to_path_buf(), true)?,
            };
            context.check(Some("files:selected"))?;
            items.push(json!({"root":grant.id,"path":name,"name":name}));
        }
        Ok(Value::Array(items))
    }).await.map_err(|e| e.to_string())?
}

pub fn decode(
    files: &Files,
    plugin: &str,
    generation: u64,
    args: &Value,
    active: &AtomicBool,
) -> AppResult<Value> {
    let root = args
        .get("root")
        .and_then(Value::as_str)
        .ok_or("缺少授权目录")?;
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("缺少音频路径")?;
    if !crate::local::encoded_audio::is_encoded(Path::new(path)) {
        return Err("此容器格式不支持解码".into());
    }
    // Open using the same capability directory, never a frontend absolute path.
    let (dir, input, base) = files.audio_input(plugin, generation, root, path, active)?;
    let mut decoder = Decoder::new(input)?;
    let to = Path::new(path)
        .with_extension(decoder.format)
        .to_string_lossy()
        .replace('\\', "/");
    let part = Path::new(path)
        .with_file_name(format!(".nons-convert-{:016x}.part", fastrand::u64(..)))
        .to_string_lossy()
        .replace('\\', "/");
    let handle = files.call(
        plugin,
        generation,
        "files.open",
        &json!({"root":root,"path":part,"mode":"create"}),
        active,
    )?["handle"]
        .as_u64()
        .ok_or("无法创建转换文件")?;
    let result = (|| {
        let mut output = ScopedWriter {
            files,
            plugin,
            generation,
            handle,
            active,
        };
        let bytes = decoder.copy(&mut output, || {
            if !active.load(Ordering::SeqCst) {
                return Err("插件已停用".into());
            }
            Ok(())
        })?;
        files.sync_transfer(plugin, generation, handle, active)?;
        files.call(
            plugin,
            generation,
            "files.close",
            &json!({"handle":handle}),
            active,
        )?;
        // Preserve embedded metadata and cover using the host's existing tag library.
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).write(true);
        let mut file = dir
            .open_with(&part, &options)
            .map_err(|e| e.to_string())?
            .into_std();
        crate::local::encoded_audio::write_tags(&mut file, &decoder)?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        files.call(
            plugin,
            generation,
            "files.publish",
            &json!({"root":root,"path":part,"to":to}),
            active,
        )?;
        Ok(
            json!({"path":if root.starts_with("absolute-") { base.join(&to).to_string_lossy().trim_start_matches(r"\\?\").to_string() } else { to },"format":decoder.format,"bytes":bytes}),
        )
    })();
    if result.is_err() {
        let _ = (files.call(
            plugin,
            generation,
            "files.close",
            &json!({"handle":handle}),
            active,
        ))
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("files.call", error);
        });
        // Clean only our unpublished file through the original directory handle.
        let _ = (dir.remove_file(&part)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("dir.remove_file", error);
        });
    }
    result
}
struct ScopedWriter<'a> {
    files: &'a Files,
    plugin: &'a str,
    generation: u64,
    handle: u64,
    active: &'a AtomicBool,
}
impl std::io::Write for ScopedWriter<'_> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.files
            .write_transfer(self.plugin, self.generation, self.handle, data, self.active)
            .map_err(std::io::Error::other)?;
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
