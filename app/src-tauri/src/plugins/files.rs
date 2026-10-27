use super::{database::Database, manifest::valid_id};
use crate::model::AppResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use cap_std::{
    ambient_authority,
    fs::{Dir, File, OpenOptions},
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

const BLOCK: usize = 32 * 1024;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Grant {
    pub id: String,
    pub path: String,
    pub writable: bool,
}
struct Root {
    dir: Dir,
    writable: bool,
    path: PathBuf,
}
struct Handle {
    plugin: String,
    generation: u64,
    root: String,
    writable: bool,
    file: File,
}
#[derive(Default)]
struct State {
    roots: BTreeMap<(String, String), Root>,
    handles: BTreeMap<u64, Handle>,
    serial: u64,
}
pub struct Files {
    data: PathBuf,
    database: Arc<Database>,
    state: Mutex<State>,
}
impl Files {
    pub fn new(data: PathBuf, database: Arc<Database>) -> Self {
        Self {
            data,
            database,
            state: Default::default(),
        }
    }
    pub fn grants(&self, plugin: &str) -> AppResult<Vec<Grant>> {
        Ok(self
            .database
            .grants(plugin)?
            .into_iter()
            .map(|(id, path, writable)| Grant { id, path, writable })
            .collect())
    }
    pub fn data_directory(&self, plugin: &str) -> AppResult<PathBuf> {
        if !valid_id(plugin) {
            return Err("插件 ID 无效".into());
        }
        let path = self.data.join(plugin);
        std::fs::create_dir_all(&self.data).map_err(|e| e.to_string())?;
        if super::manifest::is_link(
            &std::fs::symlink_metadata(&self.data).map_err(|e| e.to_string())?,
        ) {
            return Err("数据根目录不允许链接".into());
        }
        let root =
            Dir::open_ambient_dir(&self.data, ambient_authority()).map_err(|e| e.to_string())?;
        if let Ok(meta) = (std::fs::symlink_metadata(&path)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn(
                "plugins.files.data_directory.symlink_metadata",
                error,
            );
        }) {
            if super::manifest::is_link(&meta) {
                return Err("专属数据目录不允许链接".into());
            }
        }
        root.create_dir_all(plugin).map_err(|e| e.to_string())?;
        root.open_dir(plugin).map_err(|e| e.to_string())?;
        Ok(path)
    }
    pub fn grant(&self, plugin: &str, path: PathBuf, writable: bool) -> AppResult<Grant> {
        if !valid_id(plugin) {
            return Err("插件 ID 无效".into());
        }
        if self.grants(plugin)?.len() >= 32 {
            return Err("每个插件最多授权 32 个外部目录".into());
        }
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        let dir = Dir::open_ambient_dir(&path, ambient_authority()).map_err(|e| e.to_string())?;
        let id = format!("dir-{:016x}", fastrand::u64(..));
        let grant = Grant {
            id: id.clone(),
            path: path.to_string_lossy().into_owned(),
            writable,
        };
        self.database.grant(plugin, &id, &grant.path, writable)?;
        self.state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.files.grant.lock", error);
            })
            .map_err(|_| "插件文件管理器不可用")?
            .roots
            .insert(
                (plugin.into(), id),
                Root {
                    dir,
                    writable,
                    path,
                },
            );
        Ok(grant)
    }
    pub fn revoke(&self, plugin: &str, root: Option<&str>) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.files.revoke.lock", error);
            })
            .map_err(|_| "插件文件管理器不可用")?;
        self.database.revoke(plugin, root)?;
        state
            .handles
            .retain(|_, h| h.plugin != plugin || root.is_some_and(|r| h.root != r));
        state
            .roots
            .retain(|(p, r), _| p != plugin || root.is_some_and(|id| id != r));
        Ok(())
    }
    pub fn close_instance(self: &Arc<Self>, plugin: &str, generation: u64) {
        if let Ok(mut state) = (self.state.try_lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn(
                "plugins.files.close_instance.try_lock",
                error,
            );
        }) {
            state
                .handles
                .retain(|_, h| h.plugin != plugin || h.generation != generation);
            state.roots.retain(|(p, r), _| {
                p != plugin || !r.starts_with(&format!("absolute-{generation}-"))
            });
        } else {
            let files = self.clone();
            let plugin = plugin.to_string();
            tauri::async_runtime::spawn_blocking(move || {
                if let Ok(mut state) = (files.state.lock()).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.files.close_instance.worker_lock",
                        error,
                    );
                }) {
                    state
                        .handles
                        .retain(|_, h| h.plugin != plugin || h.generation != generation);
                    state.roots.retain(|(p, r), _| {
                        p != &plugin || !r.starts_with(&format!("absolute-{generation}-"))
                    });
                }
            });
        }
    }
    pub fn clear_data(&self, plugin: &str) -> AppResult<()> {
        if !valid_id(plugin) {
            return Err("插件 ID 无效".into());
        }
        let path = self.data.join(plugin);
        if !path.exists() {
            return Ok(());
        }
        let root =
            Dir::open_ambient_dir(&self.data, ambient_authority()).map_err(|e| e.to_string())?;
        root.remove_dir_all(plugin).map_err(|e| e.to_string())
    }
    pub fn write_transfer(
        &self,
        plugin: &str,
        generation: u64,
        handle: u64,
        data: &[u8],
        active: &std::sync::atomic::AtomicBool,
    ) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.files.write_transfer.lock",
                    error,
                );
            })
            .map_err(|_| "插件文件管理器不可用")?;
        if !active.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("插件已停用".into());
        }
        let h = state
            .handles
            .get_mut(&handle)
            .filter(|h| h.plugin == plugin && h.generation == generation && h.writable)
            .ok_or("文件句柄已失效或目录授权已撤销")?;
        h.file
            .write_all(data)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.files.write_transfer.write_all",
                    error,
                );
            })
            .map_err(|_| "写入文件失败：请检查磁盘空间".to_string())
    }
    pub fn sync_transfer(
        &self,
        plugin: &str,
        generation: u64,
        handle: u64,
        active: &std::sync::atomic::AtomicBool,
    ) -> AppResult<()> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.files.sync_transfer.lock", error);
            })
            .map_err(|_| "插件文件管理器不可用")?;
        if !active.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("插件已停用".into());
        }
        state
            .handles
            .get(&handle)
            .filter(|h| h.plugin == plugin && h.generation == generation)
            .ok_or("文件句柄已失效或目录授权已撤销")?
            .file
            .sync_all()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.files.sync_transfer.sync_all",
                    error,
                );
            })
            .map_err(|_| "保存文件失败".to_string())
    }
    /// Absolute paths are accepted only after the caller validates wildcard authorization.
    pub fn call_with_access(
        &self,
        plugin: &str,
        generation: u64,
        operation: &str,
        args: &Value,
        active: &std::sync::atomic::AtomicBool,
        unrestricted: bool,
    ) -> AppResult<Value> {
        if args.get("root").and_then(Value::as_str) != Some("*") {
            if args
                .get("root")
                .and_then(Value::as_str)
                .is_some_and(|r| r.starts_with("absolute-"))
            {
                return Err("不能直接使用内部文件根".into());
            }
            return self.call(plugin, generation, operation, args, active);
        }
        if !unrestricted || !active.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("插件未获任意文件权限或已停用".into());
        }
        let path = Path::new(text(args, "path")?);
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        {
            return Err("任意文件访问须提供完整绝对路径".into());
        }
        let base = path.ancestors().last().ok_or("绝对路径无效")?;
        use sha2::{Digest, Sha256};
        let root_id = format!(
            "absolute-{generation}-{:x}",
            Sha256::digest(base.to_string_lossy().as_bytes())
        );
        let mut normalized = args.clone();
        normalized["root"] = json!(root_id);
        normalized["path"] = json!(path
            .strip_prefix(base)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/"));
        if let Some(to) = args.get("to").and_then(Value::as_str) {
            let to = Path::new(to);
            if to.is_absolute() {
                normalized["to"] = json!(to
                    .strip_prefix(base)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "plugins.files.absolute_access.relative_destination",
                            error,
                        );
                    })
                    .map_err(|_| "目标须在相同文件系统根内")?
                    .to_string_lossy()
                    .replace('\\', "/"));
            }
        }
        {
            let mut state = self
                .state
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.files.absolute_access.lock",
                        error,
                    );
                })
                .map_err(|_| "文件管理器不可用")?;
            if !state.roots.contains_key(&(plugin.into(), root_id.clone())) {
                if state
                    .roots
                    .keys()
                    .filter(|(p, r)| p == plugin && r.starts_with("absolute-"))
                    .count()
                    >= 32
                {
                    return Err("每实例最多打开 32 个文件系统根".into());
                }
                let dir =
                    Dir::open_ambient_dir(base, ambient_authority()).map_err(|e| e.to_string())?;
                state.roots.insert(
                    (plugin.into(), root_id),
                    Root {
                        dir,
                        writable: true,
                        path: base.into(),
                    },
                );
            }
        }
        self.call(plugin, generation, operation, &normalized, active)
    }
    pub(super) fn audio_input(
        &self,
        plugin: &str,
        generation: u64,
        root: &str,
        path: &str,
        active: &std::sync::atomic::AtomicBool,
    ) -> AppResult<(Dir, File, PathBuf)> {
        self.call(
            plugin,
            generation,
            "files.stat",
            &json!({"root":root,"path":path}),
            active,
        )?;
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.files.audio_input.lock", error);
            })
            .map_err(|_| "插件文件管理器不可用")?;
        let root = state
            .roots
            .get(&(plugin.into(), root.into()))
            .filter(|r| r.writable)
            .ok_or("目录未授权读写或授权已撤销")?;
        relative(path)?;
        let file = root.dir.open(path).map_err(|e| e.to_string())?;
        Ok((
            root.dir.try_clone().map_err(|e| e.to_string())?,
            file,
            root.path.clone(),
        ))
    }
    pub fn call(
        &self,
        plugin: &str,
        generation: u64,
        operation: &str,
        args: &Value,
        active: &std::sync::atomic::AtomicBool,
    ) -> AppResult<Value> {
        if operation == "files.decode-audio" {
            static JOBS: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
            let _permit = JOBS
                .get_or_init(|| tokio::sync::Semaphore::new(2))
                .try_acquire()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.files.decode_audio.admission",
                        error,
                    );
                })
                .map_err(|_| "转换任务繁忙，请稍后重试")?;
            return super::audio_files::decode(self, plugin, generation, args, active);
        }
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.files.call.lock", error);
            })
            .map_err(|_| "插件文件管理器不可用")?;
        if !active.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("插件加载代次已失效".into());
        }
        if matches!(
            operation,
            "files.read" | "files.write" | "files.truncate" | "files.close"
        ) {
            let id = args
                .get("handle")
                .and_then(Value::as_u64)
                .ok_or("缺少文件句柄")?;
            let h = state
                .handles
                .get_mut(&id)
                .filter(|h| h.plugin == plugin && h.generation == generation)
                .ok_or("文件句柄已失效")?;
            let value = match operation {
                "files.read" => {
                    let length = args
                        .get("length")
                        .and_then(Value::as_u64)
                        .unwrap_or(BLOCK as u64);
                    if length > BLOCK as u64 {
                        return Err("读取块超过 32KiB".into());
                    }
                    let offset = args
                        .get("offset")
                        .and_then(Value::as_u64)
                        .ok_or("缺少读取偏移")?;
                    h.file
                        .seek(SeekFrom::Start(offset))
                        .map_err(|e| e.to_string())?;
                    let mut data = vec![0; length as usize];
                    let count = h.file.read(&mut data).map_err(|e| e.to_string())?;
                    data.truncate(count);
                    json!({"data":STANDARD.encode(data),"bytes":count})
                }
                "files.write" => {
                    if !h.writable {
                        return Err("目录只读".into());
                    }
                    let data = STANDARD
                        .decode(text(args, "data")?)
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "plugins.files.write.decode_base64",
                                error,
                            );
                        })
                        .map_err(|_| "文件数据必须为 base64")?;
                    if data.len() > BLOCK {
                        return Err("写入块超过 32KiB".into());
                    }
                    let offset = args
                        .get("offset")
                        .and_then(Value::as_u64)
                        .ok_or("缺少写入偏移")?;
                    h.file
                        .seek(SeekFrom::Start(offset))
                        .map_err(|e| e.to_string())?;
                    h.file.write_all(&data).map_err(|e| e.to_string())?;
                    json!({"bytes":data.len()})
                }
                "files.truncate" => {
                    if !h.writable {
                        return Err("目录只读".into());
                    }
                    h.file
                        .set_len(
                            args.get("length")
                                .and_then(Value::as_u64)
                                .ok_or("缺少长度")?,
                        )
                        .map_err(|e| e.to_string())?;
                    Value::Null
                }
                _ => Value::Null,
            };
            if operation == "files.close" {
                state.handles.remove(&id);
            }
            return Ok(value);
        }
        let root_id = text(args, "root")?;
        let key = (plugin.to_string(), root_id.to_string());
        if !state.roots.contains_key(&key) {
            let (path, writable) = if root_id == "data" {
                let path = self.data_directory(plugin)?;
                (path, true)
            } else {
                let grant = self
                    .grants(plugin)?
                    .into_iter()
                    .find(|g| g.id == root_id)
                    .ok_or("目录未授权或授权已撤销")?;
                (PathBuf::from(grant.path), grant.writable)
            };
            let dir =
                Dir::open_ambient_dir(&path, ambient_authority()).map_err(|e| e.to_string())?;
            state.roots.insert(
                key.clone(),
                Root {
                    dir,
                    writable,
                    path,
                },
            );
        }
        let root = &state.roots[&key];
        let path = args.get("path").and_then(Value::as_str).unwrap_or("");
        relative(path)?;
        let file_path = if path.is_empty() { "." } else { path };
        let mutating = matches!(
            operation,
            "files.mkdir" | "files.rename" | "files.publish" | "files.remove"
        ) || (operation == "files.open"
            && args.get("mode").and_then(Value::as_str).unwrap_or("read") != "read");
        if mutating && !root.writable {
            return Err("目录只读".into());
        }
        if matches!(operation, "files.remove" | "files.rename" | "files.open") && path.is_empty() {
            return Err("不允许操作目录根本身".into());
        }
        match operation {
            "files.resolve" => {
                let base = root.path.clone();
                let resolved = if path.is_empty() {
                    let metadata = std::fs::symlink_metadata(&base).map_err(|e| e.to_string())?;
                    if super::manifest::is_link(&metadata) || !metadata.is_dir() {
                        return Err("授权目录不可用或被替换为链接".into());
                    }
                    base
                } else {
                    super::manifest::checked_file(&base, path)?
                };
                // Validate through the granted directory handle as well as its current path.
                root.dir.metadata(file_path).map_err(|e| e.to_string())?;
                Ok(json!({"path":resolved.to_string_lossy().trim_start_matches(r"\\?\")}))
            }
            "files.stat" => {
                let m = root.dir.metadata(file_path).map_err(|e| e.to_string())?;
                Ok(json!({"isDirectory":m.is_dir(),"size":m.len()}))
            }
            "files.list" => {
                let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0);
                if offset > usize::MAX as u64 {
                    return Err("目录偏移无效".into());
                }
                let mut entries = Vec::new();
                let mut more = false;
                let mut bytes = 0;
                for entry in root
                    .dir
                    .read_dir(file_path)
                    .map_err(|e| e.to_string())?
                    .skip(offset as usize)
                    .take(129)
                {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entries.len() == 128 {
                        more = true;
                        break;
                    }
                    let name = entry
                        .file_name()
                        .into_string()
                        .inspect_err(|_| {
                            crate::infrastructure::diagnostics::warn(
                                "files.list",
                                "file name is not valid UTF-8",
                            );
                        })
                        .map_err(|_| "文件名称不是 UTF-8")?;
                    let kind = entry.file_type().map_err(|e| e.to_string())?;
                    let item =
                        json!({"name":name,"isDirectory":kind.is_dir(),"isLink":kind.is_symlink()});
                    bytes += serde_json::to_vec(&item).map_err(|e| e.to_string())?.len();
                    if bytes > 60000 {
                        more = true;
                        break;
                    }
                    entries.push(item);
                }
                Ok(
                    json!({"entries":entries,"nextOffset":if more{Some(offset+entries.len() as u64)}else{None}}),
                )
            }
            "files.mkdir" => {
                if path.is_empty() {
                    return Err("缺少目录路径".into());
                }
                root.dir.create_dir_all(path).map_err(|e| e.to_string())?;
                Ok(Value::Null)
            }
            "files.publish" => {
                let to = text(args, "to")?;
                relative(to)?;
                if path.is_empty() || to.is_empty() {
                    return Err("缺少文件路径".into());
                }
                // Creating a hard link is atomic and fails if the destination exists.
                root.dir
                    .hard_link(path, &root.dir, to)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "plugins.files.publish.hard_link",
                            error,
                        );
                    })
                    .map_err(|_| "保存失败：目标已存在或目录不支持原子发布")?;
                root.dir.remove_file(path).map_err(|e| e.to_string())?;
                Ok(Value::Null)
            }
            "files.rename" => {
                let to = text(args, "to")?;
                relative(to)?;
                if to.is_empty() {
                    return Err("缺少目标路径".into());
                }
                root.dir
                    .rename(path, &root.dir, to)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Null)
            }
            "files.remove" => {
                if root
                    .dir
                    .symlink_metadata(path)
                    .map_err(|e| e.to_string())?
                    .is_dir()
                {
                    root.dir.remove_dir(path).map_err(|e| e.to_string())?;
                } else {
                    root.dir.remove_file(path).map_err(|e| e.to_string())?;
                }
                Ok(Value::Null)
            }
            "files.open" => {
                if state
                    .handles
                    .values()
                    .filter(|h| h.plugin == plugin && h.generation == generation)
                    .count()
                    >= 16
                {
                    return Err("每实例最多打开 16 个文件".into());
                }
                let mode = args.get("mode").and_then(Value::as_str).unwrap_or("read");
                let mut options = OpenOptions::new();
                match mode {
                    "read" => {
                        options.read(true);
                    }
                    "readWrite" => {
                        options.read(true).write(true).create(true);
                    }
                    "create" => {
                        options.read(true).write(true).create_new(true);
                    }
                    _ => return Err("未知文件打开方式".into()),
                };
                let file = state.roots[&key]
                    .dir
                    .open_with(path, &options)
                    .map_err(|e| e.to_string())?;
                state.serial = state.serial.checked_add(1).ok_or("文件句柄耗尽")?;
                let id = state.serial;
                state.handles.insert(
                    id,
                    Handle {
                        plugin: plugin.into(),
                        generation,
                        root: root_id.into(),
                        writable: mode != "read",
                        file,
                    },
                );
                Ok(json!({"handle":id}))
            }
            _ => Err("未知文件操作".into()),
        }
    }
}
fn text<'a>(args: &'a Value, key: &str) -> AppResult<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("缺少 {key}"))
}
fn relative(path: &str) -> AppResult<()> {
    if path.len() > 4096
        || path.contains(['\\', ':', '\0'])
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            && !path.is_empty()
    {
        return Err("文件路径必须是目录内的相对路径".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wildcard_access_requires_authorization_and_expires_with_instance() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let files = Arc::new(Files::new(tmp.path().join("data"), db));
        let active = std::sync::atomic::AtomicBool::new(true);
        let path = tmp
            .path()
            .join("arbitrary.txt")
            .to_string_lossy()
            .into_owned();
        let args = json!({"root":"*","path":path,"mode":"create"});
        assert!(files
            .call_with_access("one", 7, "files.open", &args, &active, false)
            .is_err());
        let handle = files
            .call_with_access("one", 7, "files.open", &args, &active, true)
            .unwrap()["handle"]
            .as_u64()
            .unwrap();
        files
            .write_transfer("one", 7, handle, b"content", &active)
            .unwrap();
        assert!(files
            .write_transfer("other", 7, handle, b"bad", &active)
            .is_err());
        assert!(files
            .call_with_access(
                "one",
                7,
                "files.stat",
                &json!({"root":"*","path":"relative.txt"}),
                &active,
                true
            )
            .is_err());
        files.close_instance("one", 7);
        assert!(files
            .write_transfer("one", 7, handle, b"late", &active)
            .is_err());
        active.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(files
            .call_with_access(
                "one",
                7,
                "files.remove",
                &json!({"root":"*","path":path}),
                &active,
                true
            )
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"content");
    }
    #[test]
    fn conversion_preserves_metadata_and_never_overwrites_or_leaves_part_files() {
        use crate::local::encoded_audio::tests::{flac, write_padded_fixture};
        use lofty::prelude::*;
        let tmp = tempfile::tempdir().unwrap();
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let files = Files::new(tmp.path().join("data"), db);
        let active = std::sync::atomic::AtomicBool::new(true);
        let source = tmp.path().join("测试.ncm");
        write_padded_fixture(&source, &flac(), 300 * 1024 * 1024);
        let args = json!({"root":"*","path":source.to_string_lossy()});
        let value = files
            .call_with_access("one", 7, "files.decode-audio", &args, &active, true)
            .unwrap();
        let destination = tmp.path().join("测试.flac");
        assert_eq!(Path::new(value["path"].as_str().unwrap()), destination);
        let tagged = lofty::probe::Probe::open(&destination)
            .unwrap()
            .read()
            .unwrap();
        assert_eq!(
            tagged.primary_tag().unwrap().title().as_deref(),
            Some("Fixture")
        );
        assert_eq!(
            tagged.primary_tag().unwrap().artist().as_deref(),
            Some("Test")
        );
        let before = std::fs::read(&destination).unwrap();
        assert!(files
            .call_with_access("one", 7, "files.decode-audio", &args, &active, true)
            .is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), before);
        assert!(source.exists());
        assert!(!std::fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .any(|e| e.path().extension().is_some_and(|ext| ext == "part")));
        let grant = files.grant("scoped", tmp.path().into(), false).unwrap();
        assert!(files
            .call(
                "scoped",
                1,
                "files.decode-audio",
                &json!({"root":grant.id,"path":"测试.ncm"}),
                &active
            )
            .is_err());
    }
    #[test]
    fn transfer_writes_and_publication_respect_revocation_and_never_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let files = Arc::new(Files::new(tmp.path().join("data"), db));
        let grant = files.grant("one", tmp.path().into(), true).unwrap();
        let active = std::sync::atomic::AtomicBool::new(true);
        let call = |op, args| files.call("one", 1, op, &args, &active);
        let handle = call(
            "files.open",
            json!({"root":grant.id,"path":"track.part","mode":"create"}),
        )
        .unwrap()["handle"]
            .as_u64()
            .unwrap();
        assert!(files
            .write_transfer("other", 1, handle, b"bad", &active)
            .is_err());
        files
            .write_transfer("one", 1, handle, b"audio", &active)
            .unwrap();
        files.sync_transfer("one", 1, handle, &active).unwrap();
        call("files.close", json!({"handle":handle})).unwrap();
        std::fs::write(tmp.path().join("track.mp3"), b"original").unwrap();
        assert!(call(
            "files.publish",
            json!({"root":grant.id,"path":"track.part","to":"track.mp3"})
        )
        .is_err());
        assert_eq!(
            std::fs::read(tmp.path().join("track.mp3")).unwrap(),
            b"original"
        );
        call(
            "files.publish",
            json!({"root":grant.id,"path":"track.part","to":"new.mp3"}),
        )
        .unwrap();
        assert_eq!(std::fs::read(tmp.path().join("new.mp3")).unwrap(), b"audio");
        assert!(!tmp.path().join("track.part").exists());
        let handle = call(
            "files.open",
            json!({"root":grant.id,"path":"second.part","mode":"create"}),
        )
        .unwrap()["handle"]
            .as_u64()
            .unwrap();
        files.revoke("one", Some(&grant.id)).unwrap();
        assert!(files
            .write_transfer("one", 1, handle, b"late", &active)
            .is_err());
        assert!(call(
            "files.publish",
            json!({"root":grant.id,"path":"second.part","to":"late.mp3"})
        )
        .is_err());
        assert!(!tmp.path().join("late.mp3").exists());
    }
    #[test]
    fn pagination_and_readonly_handle_revocation() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let files = Arc::new(Files::new(tmp.path().join("data"), db));
        let outside = tempfile::tempdir().unwrap();
        for i in 0..140 {
            std::fs::write(outside.path().join(format!("file-{i}")), [1, 2, 3]).unwrap();
        }
        let grant = files.grant("one", outside.path().into(), false).unwrap();
        let active = std::sync::atomic::AtomicBool::new(true);
        let call = |op, args| files.call("one", 1, op, &args, &active);
        assert!(
            call("files.resolve", json!({"root":grant.id,"path":"file-0"})).unwrap()["path"]
                .as_str()
                .unwrap()
                .ends_with("file-0")
        );
        assert!(files
            .call(
                "two",
                1,
                "files.resolve",
                &json!({"root":grant.id,"path":"file-0"}),
                &active
            )
            .is_err());
        let first = call("files.list", json!({"root":grant.id})).unwrap();
        assert_eq!(first["entries"].as_array().unwrap().len(), 128);
        let second = call(
            "files.list",
            json!({"root":grant.id,"offset":first["nextOffset"]}),
        )
        .unwrap();
        assert_eq!(second["entries"].as_array().unwrap().len(), 12);
        assert!(second["nextOffset"].is_null());
        let opened = call("files.open", json!({"root":grant.id,"path":"file-0"})).unwrap();
        let h = opened["handle"].clone();
        assert!(call("files.write", json!({"handle":h,"offset":0,"data":"AA=="})).is_err());
        files.revoke("one", Some(&grant.id)).unwrap();
        assert!(call("files.read", json!({"handle":h,"offset":0})).is_err());
        assert_eq!(
            std::fs::read(outside.path().join("file-0")).unwrap(),
            [1, 2, 3]
        );
    }
    #[cfg(windows)]
    #[test]
    fn junction_cannot_escape_data_root() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), b"private").unwrap();
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let files = Files::new(tmp.path().join("data"), db);
        let root = files.data_directory("one").unwrap();
        let junction = root.join("escape");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let active = std::sync::atomic::AtomicBool::new(true);
        assert!(files
            .call(
                "one",
                1,
                "files.open",
                &json!({"root":"data","path":"escape/secret"}),
                &active
            )
            .is_err());
        assert!(files
            .call(
                "one",
                1,
                "files.open",
                &json!({"root":"data","path":"escape/new","mode":"create"}),
                &active
            )
            .is_err());
        std::fs::remove_dir(junction).unwrap();
        assert!(!outside.path().join("new").exists());
        assert_eq!(
            std::fs::read(outside.path().join("secret")).unwrap(),
            b"private"
        );
    }
    #[test]
    fn binary_handles_grants_and_revocation() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let files = Arc::new(Files::new(tmp.path().join("data"), db.clone()));
        let active = std::sync::atomic::AtomicBool::new(true);
        let call = |op, args| files.call("one", 1, op, &args, &active);
        let h = call(
            "files.open",
            json!({"root":"data","path":"sample.bin","mode":"create"}),
        )
        .unwrap()["handle"]
            .as_u64()
            .unwrap();
        call(
            "files.write",
            json!({"handle":h,"offset":0,"data":STANDARD.encode([0,255,1])}),
        )
        .unwrap();
        assert_eq!(
            call("files.read", json!({"handle":h,"offset":0})).unwrap()["data"],
            STANDARD.encode([0, 255, 1])
        );
        assert!(files
            .call(
                "two",
                1,
                "files.read",
                &json!({"handle":h,"offset":0}),
                &active
            )
            .is_err());
        assert!(call("files.read", json!({"handle":h,"offset":0,"length":32769})).is_err());
        assert!(call("files.open", json!({"root":"data","path":"../outside"})).is_err());
        let grant = files.grant("one", outside.path().into(), false).unwrap();
        assert!(call("files.mkdir", json!({"root":grant.id,"path":"new"})).is_err());
        files.revoke("one", Some(&grant.id)).unwrap();
        assert!(call("files.list", json!({"root":grant.id})).is_err());
        assert!(outside.path().exists());
        files.close_instance("one", 1);
        assert!(call("files.read", json!({"handle":h,"offset":0})).is_err());
        active.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(call("files.list", json!({"root":"data"})).is_err());
        assert!(db.grants("one").unwrap().is_empty());
    }
}
