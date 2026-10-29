//! SAF grants belong to local music, independently of plugin directory grants.
use super::{document_parts, Entry, Metadata};
use crate::{model::AppResult, storage::Store};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs::File,
    io,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, RwLock},
};
use tauri_plugin_android_fs::{AndroidFsExt, FsUri, UriPermission};

struct Roots {
    app: tauri::AppHandle,
    store: Arc<Store>,
    active: RwLock<HashMap<String, (PathBuf, FsUri)>>,
    owned: Mutex<HashSet<String>>,
}
static ROOTS: OnceLock<Roots> = OnceLock::new();
static PICKER: Mutex<()> = Mutex::new(());
const OWNED: &str = "localMusicOwnedGrants";

pub(crate) fn mutation_guard() -> AppResult<std::sync::MutexGuard<'static, ()>> {
    PICKER
        .try_lock()
        .map_err(|_| "目录选择或修改正在进行".into())
}

fn error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}
fn roots() -> io::Result<&'static Roots> {
    ROOTS.get().ok_or_else(|| error("本地目录服务尚未初始化"))
}

pub(crate) fn initialize(
    app: tauri::AppHandle,
    store: Arc<Store>,
    paths: &[String],
) -> AppResult<()> {
    let mut active = HashMap::new();
    for path in paths.iter().filter(|p| super::is_document(p)) {
        let (token, root, relative) = document_parts(Path::new(path)).map_err(|e| e.to_string())?;
        if !relative.as_os_str().is_empty() {
            return Err("音乐根目录标识无效".into());
        }
        let bytes = URL_SAFE_NO_PAD.decode(&token).map_err(|e| e.to_string())?;
        let uri: FsUri = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if !uri.is_content_scheme() || uri.document_top_tree_uri.is_none() {
            return Err("音乐目录授权标识无效".into());
        }
        active.insert(token, (root, uri));
    }
    let owned = store
        .setting(OWNED)?
        .map(|v| serde_json::from_str(&v))
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    ROOTS
        .set(Roots {
            app,
            store,
            active: RwLock::new(active),
            owned: Mutex::new(owned),
        })
        .map_err(|_| "本地目录服务已初始化".to_string())
}

pub(crate) fn authorized(path: &Path) -> io::Result<()> {
    let (token, root, _) = document_parts(path)?;
    if roots()?
        .active
        .read()
        .map_err(error)?
        .get(&token)
        .is_some_and(|(saved, _)| saved == &root)
    {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "音乐目录已移除，请重新添加",
        ))
    }
}

#[derive(Default)]
struct Cache {
    depth: usize,
    entries: HashMap<PathBuf, (FsUri, Metadata)>,
    directories: HashMap<PathBuf, Vec<(Entry, FsUri)>>,
}
thread_local! { static CACHE: RefCell<Cache> = RefCell::new(Cache::default()); }
pub(crate) struct ReadSession(std::marker::PhantomData<std::rc::Rc<()>>);
impl ReadSession {
    pub(crate) fn new() -> Self {
        CACHE.with_borrow_mut(|c| {
            c.depth += 1;
        });
        Self(std::marker::PhantomData)
    }
}
impl Drop for ReadSession {
    fn drop(&mut self) {
        CACHE.with_borrow_mut(|c| {
            c.depth -= 1;
            if c.depth == 0 {
                *c = Cache::default();
            }
        });
    }
}
fn remember(path: PathBuf, uri: FsUri, metadata: Metadata) {
    CACHE.with_borrow_mut(|c| {
        if c.depth > 0 {
            if c.entries.len() >= 4096 {
                c.entries.clear();
            }
            c.entries.insert(path, (uri, metadata));
        }
    });
}
fn stat(entry: &tauri_plugin_android_fs::Entry) -> Metadata {
    Metadata {
        len: entry.file_len().unwrap_or(0),
        modified: Some(entry.last_modified()),
        file: entry.is_file(),
        directory: entry.is_dir(),
    }
}
fn safe_name(name: &str) -> io::Result<()> {
    let mut parts = Path::new(name).components();
    if name.contains(['/', '\\'])
        || !matches!(parts.next(), Some(Component::Normal(_)))
        || parts.next().is_some()
    {
        return Err(error("文件提供器返回了无效名称"));
    }
    Ok(())
}
fn resolve(path: &Path) -> io::Result<(FsUri, Metadata)> {
    authorized(path)?;
    let (_, root, relative) = document_parts(path)?;
    let path = root.join(&relative);
    if let Some(entry) = CACHE.with_borrow(|c| c.entries.get(&path).cloned()) {
        return Ok(entry);
    }
    if relative.as_os_str().is_empty() {
        let (token, _, _) = document_parts(&path)?;
        let uri = roots()?
            .active
            .read()
            .map_err(error)?
            .get(&token)
            .ok_or_else(|| error("音乐目录已移除"))?
            .1
            .clone();
        let entry = roots()?
            .app
            .android_fs()
            .get_metadata(&uri)
            .map_err(error)?;
        let metadata = stat(&entry);
        remember(path, uri.clone(), metadata.clone());
        return Ok((uri, metadata));
    }
    let parent = path.parent().ok_or_else(|| error("文档目录无效"))?;
    // Query the parent to distinguish missing siblings from revoked/unreadable trees.
    let (entry, uri) = directory(parent)?
        .into_iter()
        .find(|(entry, _)| entry.path == path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "音乐文件不存在"))?;
    Ok((uri, entry.metadata))
}
pub(crate) fn metadata(path: &Path) -> io::Result<Metadata> {
    let _session = ReadSession::new();
    resolve(path).map(|(_, stat)| stat)
}
pub(crate) fn open(path: &Path) -> io::Result<File> {
    let _session = ReadSession::new();
    let (uri, metadata) = resolve(path)?;
    if !metadata.file {
        return Err(error("请选择音乐文件"));
    }
    authorized(path)?;
    roots()?
        .app
        .android_fs()
        .open_file_readable(&uri)
        .map_err(error)
}
pub(crate) fn read_dir(path: &Path) -> io::Result<Vec<Entry>> {
    let _session = ReadSession::new();
    directory(path).map(|entries| entries.into_iter().map(|(entry, _)| entry).collect())
}
fn directory(path: &Path) -> io::Result<Vec<(Entry, FsUri)>> {
    authorized(path)?;
    let (_, root, relative) = document_parts(path)?;
    let path = root.join(relative);
    if let Some(entries) = CACHE.with_borrow(|c| c.directories.get(&path).cloned()) {
        return Ok(entries);
    }
    let (uri, _) = resolve(&path)?;
    let entries = roots()?.app.android_fs().read_dir(&uri).map_err(error)?;
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries {
        safe_name(entry.name())?;
        let child = path.join(entry.name());
        let metadata = stat(&entry);
        remember(child.clone(), entry.uri().clone(), metadata.clone());
        result.push((
            Entry {
                path: child,
                metadata,
            },
            entry.uri().clone(),
        ));
    }
    CACHE.with_borrow_mut(|c| {
        if c.depth > 0 && result.len() <= 4096 {
            if c.directories.len() >= 16
                || c.directories.values().map(Vec::len).sum::<usize>() + result.len() > 4096
            {
                c.directories.clear();
            }
            c.directories.insert(path, result.clone());
        }
    });
    Ok(result)
}

pub(crate) fn pick(add: impl FnOnce(String) -> AppResult<()>) -> AppResult<()> {
    let _picker = mutation_guard()?;
    let roots = roots().map_err(|e| e.to_string())?;
    let picker = roots.app.android_fs().picker();
    let Some(uri) = picker.pick_dir(None, true).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let name = roots
        .app
        .android_fs()
        .get_name(&uri)
        .map_err(|e| e.to_string())?;
    safe_name(&name).map_err(|e| e.to_string())?;
    let token = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&uri).map_err(|e| e.to_string())?);
    let existing = roots
        .active
        .read()
        .map_err(|e| e.to_string())?
        .get(&token)
        .cloned();
    let path = existing
        .as_ref()
        .map(|(path, _)| path.clone())
        .unwrap_or_else(|| Path::new(super::DOCUMENT_PREFIX).join(&token).join(name));
    // Never release a grant that another feature already held.
    let shared = picker
        .check_persisted_uri_permission(&uri, UriPermission::ReadOrWrite)
        .map_err(|e| e.to_string())?;
    picker
        .persist_uri_permission(&uri)
        .map_err(|e| format!("无法保留音乐目录授权：{e}"))?;
    if !shared {
        let mut owned = roots.owned.lock().map_err(|e| e.to_string())?;
        owned.insert(token.clone());
        if let Err(error) = roots.store.set_setting(
            OWNED,
            &serde_json::to_string(&*owned).map_err(|e| e.to_string())?,
        ) {
            owned.remove(&token);
            let _ = picker.release_persisted_uri_permission(&uri);
            return Err(error);
        }
    }
    roots
        .active
        .write()
        .map_err(|e| e.to_string())?
        .insert(token.clone(), (path.clone(), uri.clone()));
    if let Err(error) = add(path.to_string_lossy().into_owned()) {
        if existing.is_none() {
            revoke(&path).map_err(|e| e.to_string())?;
        }
        return Err(error);
    }
    Ok(())
}
pub(crate) fn revoke(path: &Path) -> io::Result<()> {
    let (token, _, _) = document_parts(path)?;
    let roots = roots()?;
    let removed = roots.active.write().map_err(error)?.remove(&token);
    let mut owned = roots.owned.lock().map_err(error)?;
    if owned.contains(&token) {
        if let Some((_, uri)) = removed {
            roots
                .app
                .android_fs()
                .picker()
                .release_persisted_uri_permission(&uri)
                .map_err(error)?;
        }
        owned.remove(&token);
        roots
            .store
            .set_setting(OWNED, &serde_json::to_string(&*owned).map_err(error)?)
            .map_err(error)?;
    }
    Ok(())
}
