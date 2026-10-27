//! Persistent public cover assets. Callers perform IO on blocking workers and authorize URLs.
use crate::model::AppResult;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub(crate) struct Asset {
    pub(crate) path: PathBuf,
    pub(crate) fetched_at: u64,
    pub(crate) mime: String,
    pub(crate) bytes: u64,
    filename: String,
}

pub(crate) struct Lookup {
    pub(crate) generation: u64,
    pub(crate) asset: Option<Asset>,
}

struct State {
    db: Connection,
    generation: u64,
    access: i64,
}

pub(crate) struct DiskCache {
    root: PathBuf,
    images: PathBuf,
    budget: u64,
    state: Mutex<State>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn extension(mime: &str) -> AppResult<&'static str> {
    match mime {
        "image/jpeg" => Ok("jpg"),
        "image/png" => Ok("png"),
        "image/webp" => Ok("webp"),
        "image/gif" => Ok("gif"),
        _ => Err("封面格式不支持".into()),
    }
}

fn owned_filename(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(hash, ext)| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
            && matches!(ext, "jpg" | "png" | "webp" | "gif")
    })
}

fn linked(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    false
}

fn safe_directory(path: &Path) -> AppResult<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if linked(&metadata) || !metadata.is_dir() => {
                return Err("在线封面缓存目录包含链接或无效目录".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

fn safe_file(path: &Path) -> AppResult<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if linked(&metadata) || !metadata.is_file() => {
            Err("在线封面缓存文件不是普通文件".into())
        }
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

impl DiskCache {
    pub(crate) fn open(root: PathBuf, budget: u64) -> AppResult<Self> {
        let root = std::path::absolute(root).map_err(|e| e.to_string())?;
        // The host's parent can use platform aliases, while the cache root itself cannot.
        match fs::symlink_metadata(&root) {
            Ok(metadata) if linked(&metadata) || !metadata.is_dir() => {
                return Err("在线封面缓存目录包含链接或无效目录".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        let parent = root.parent().ok_or("在线封面缓存父目录不可用")?;
        let name = root.file_name().ok_or("在线封面缓存目录名称无效")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let root = parent.canonicalize().map_err(|e| e.to_string())?.join(name);
        safe_directory(&root)?;
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        safe_directory(&root)?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let images = root.join("images");
        safe_directory(&images)?;
        fs::create_dir_all(&images).map_err(|e| e.to_string())?;
        safe_directory(&images)?;
        Self::check_index(&root)?;
        let db = match Self::open_index(&root) {
            Ok(db) => db,
            Err(rusqlite::Error::SqliteFailure(error, _))
                if matches!(
                    error.code,
                    rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase
                ) =>
            {
                for suffix in ["", "-journal", "-wal", "-shm"] {
                    let path = root.join(format!("index.sqlite3{suffix}"));
                    if safe_file(&path)?.is_some() {
                        fs::remove_file(path).map_err(|e| e.to_string())?;
                    }
                }
                Self::open_index(&root).map_err(|e| e.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        };
        let access = db
            .query_row("SELECT COALESCE(MAX(accessed),0) FROM covers", [], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        let cache = Self {
            root,
            images,
            budget,
            state: Mutex::new(State {
                db,
                generation: 0,
                access,
            }),
        };
        {
            let mut state = cache.lock()?;
            cache.reconcile(&mut state)?;
            cache.evict(&mut state, None)?;
        }
        Ok(cache)
    }

    fn open_index(root: &Path) -> rusqlite::Result<Connection> {
        let db = Connection::open(root.join("index.sqlite3"))?;
        db.busy_timeout(Duration::from_secs(2))?;
        db.execute_batch(
            "PRAGMA journal_mode=DELETE; PRAGMA synchronous=NORMAL;
            CREATE TABLE IF NOT EXISTS covers (
                url_key TEXT PRIMARY KEY, filename TEXT NOT NULL, bytes INTEGER NOT NULL,
                mime TEXT NOT NULL, fetched INTEGER NOT NULL, accessed INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS covers_accessed ON covers(accessed);
            CREATE INDEX IF NOT EXISTS covers_filename ON covers(filename);",
        )?;
        Ok(db)
    }

    fn check_index(root: &Path) -> AppResult<()> {
        for suffix in ["", "-journal", "-wal", "-shm"] {
            safe_file(&root.join(format!("index.sqlite3{suffix}")))?;
        }
        Ok(())
    }

    fn lock(&self) -> AppResult<MutexGuard<'_, State>> {
        let state = self.state.lock().map_err(|_| "在线封面缓存锁不可用")?;
        safe_directory(&self.root)?;
        safe_directory(&self.images)?;
        Self::check_index(&self.root)?;
        Ok(state)
    }

    pub(crate) fn generation(&self) -> AppResult<u64> {
        Ok(self.lock()?.generation)
    }

    fn asset(&self, filename: String, bytes: i64, mime: String, fetched: i64) -> AppResult<Asset> {
        if !owned_filename(&filename)
            || !filename.ends_with(extension(&mime)?)
            || bytes <= 0
            || fetched < 0
        {
            return Err("在线封面缓存索引无效".into());
        }
        Ok(Asset {
            path: self.images.join(&filename),
            fetched_at: fetched as u64,
            mime,
            bytes: bytes as u64,
            filename,
        })
    }

    fn read_asset(&self, asset: &Asset) -> AppResult<Vec<u8>> {
        if !owned_filename(&asset.filename)
            || !asset.filename.ends_with(extension(&asset.mime)?)
            || asset.path != self.images.join(&asset.filename)
            || asset.bytes > self.budget
        {
            return Err("在线封面缓存路径无效".into());
        }
        let metadata = safe_file(&asset.path)?.ok_or("在线封面缓存已失效")?;
        if metadata.len() != asset.bytes {
            return Err("在线封面缓存内容已损坏".into());
        }
        let file = fs::File::open(&asset.path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(asset.bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != asset.bytes
            || digest(&bytes) != asset.filename.split('.').next().unwrap_or_default()
        {
            return Err("在线封面缓存内容已损坏".into());
        }
        Ok(bytes)
    }

    pub(crate) fn read(&self, asset: &Asset) -> AppResult<Vec<u8>> {
        let _state = self.lock()?;
        self.read_asset(asset)
    }

    pub(crate) fn lookup(&self, url: &str) -> AppResult<Lookup> {
        let mut state = self.lock()?;
        let key = digest(url.as_bytes());
        let row = state
            .db
            .query_row(
                "SELECT filename,bytes,mime,fetched FROM covers WHERE url_key=?1",
                [&key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let mut asset = None;
        if let Some((filename, bytes, mime, fetched)) = row {
            let candidate = self.asset(filename.clone(), bytes, mime, fetched);
            if let Ok(candidate) = candidate {
                if self.read_asset(&candidate).is_ok() {
                    state.access = state
                        .access
                        .checked_add(1)
                        .ok_or("在线封面缓存访问序号已耗尽")?;
                    state
                        .db
                        .execute(
                            "UPDATE covers SET accessed=?2 WHERE url_key=?1",
                            params![key, state.access],
                        )
                        .map_err(|e| e.to_string())?;
                    asset = Some(candidate);
                }
            }
            if asset.is_none() {
                state
                    .db
                    .execute("DELETE FROM covers WHERE url_key=?1", [&key])
                    .map_err(|e| e.to_string())?;
                self.remove_unreferenced(&state.db, &filename)?;
            }
        }
        Ok(Lookup {
            generation: state.generation,
            asset,
        })
    }

    pub(crate) fn publish(
        &self,
        url: &str,
        bytes: &[u8],
        mime: &str,
        generation: u64,
    ) -> AppResult<Asset> {
        let mut state = self.lock()?;
        if state.generation != generation {
            return Err("封面缓存已清理，请重试".into());
        }
        if bytes.is_empty() || bytes.len() as u64 > self.budget || bytes.len() > i64::MAX as usize {
            return Err("封面文件过大或为空".into());
        }
        let key = digest(url.as_bytes());
        let filename = format!("{}.{}", digest(bytes), extension(mime)?);
        let fetched = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let asset = self.asset(
            filename.clone(),
            bytes.len() as i64,
            mime.into(),
            fetched as i64,
        )?;
        safe_file(&asset.path)?;
        if self.read_asset(&asset).is_err() {
            let mut temporary =
                tempfile::NamedTempFile::new_in(&self.images).map_err(|e| e.to_string())?;
            temporary.write_all(bytes).map_err(|e| e.to_string())?;
            temporary.flush().map_err(|e| e.to_string())?;
            temporary
                .persist(&asset.path)
                .map_err(|e| e.error.to_string())?;
        }
        let previous: Option<String> = state
            .db
            .query_row(
                "SELECT filename FROM covers WHERE url_key=?1",
                [&key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        state.access = state
            .access
            .checked_add(1)
            .ok_or("在线封面缓存访问序号已耗尽")?;
        state.db.execute("INSERT INTO covers(url_key,filename,bytes,mime,fetched,accessed) VALUES(?1,?2,?3,?4,?5,?6)
            ON CONFLICT(url_key) DO UPDATE SET filename=excluded.filename,bytes=excluded.bytes,mime=excluded.mime,fetched=excluded.fetched,accessed=excluded.accessed",
            params![key, filename, bytes.len() as i64, mime, fetched as i64, state.access]).map_err(|e| e.to_string())?;
        if let Some(previous) = previous {
            self.remove_unreferenced(&state.db, &previous)?;
        }
        self.evict(&mut state, Some(&key))?;
        Ok(asset)
    }

    fn remove_unreferenced(&self, db: &Connection, filename: &str) -> AppResult<()> {
        if !owned_filename(filename) {
            return Ok(());
        }
        let references: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM covers WHERE filename=?1",
                [filename],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if references == 0 {
            let path = self.images.join(filename);
            if safe_file(&path)?.is_some() {
                fs::remove_file(path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn payload_usage(db: &Connection) -> AppResult<super::other_cache::Usage> {
        db.query_row("SELECT COALESCE(SUM(bytes),0),COUNT(*) FROM (SELECT filename,MAX(bytes) AS bytes FROM covers GROUP BY filename)", [], |row| {
            Ok(super::other_cache::Usage { used_bytes: row.get::<_, i64>(0)?.max(0) as u64, entries: row.get::<_, i64>(1)?.max(0) as u64 })
        }).map_err(|e| e.to_string())
    }

    fn evict(&self, state: &mut State, keep: Option<&str>) -> AppResult<()> {
        while Self::payload_usage(&state.db)?.used_bytes > self.budget {
            let row: Option<(String, String)> = state.db.query_row("SELECT url_key,filename FROM covers WHERE (?1 IS NULL OR url_key<>?1) ORDER BY accessed,url_key LIMIT 1", [keep], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|e| e.to_string())?;
            let Some((key, filename)) = row else {
                return Err("在线封面缓存空间不足".into());
            };
            state
                .db
                .execute("DELETE FROM covers WHERE url_key=?1", [key])
                .map_err(|e| e.to_string())?;
            self.remove_unreferenced(&state.db, &filename)?;
        }
        Ok(())
    }

    fn reconcile(&self, state: &mut State) -> AppResult<()> {
        let mut after: Option<String> = None;
        loop {
            let rows = {
                let query = if after.is_some() {
                    "SELECT url_key,filename,bytes,mime,fetched FROM covers WHERE url_key>?1 ORDER BY url_key LIMIT 128"
                } else {
                    "SELECT url_key,filename,bytes,mime,fetched FROM covers WHERE url_key>=?1 ORDER BY url_key LIMIT 128"
                };
                let mut statement = state.db.prepare(query).map_err(|e| e.to_string())?;
                let rows = statement
                    .query_map([after.as_deref().unwrap_or("")], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, i64>(4)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                rows
            };
            if rows.is_empty() {
                break;
            }
            for (key, filename, bytes, mime, fetched) in rows {
                after = Some(key.clone());
                let valid = self
                    .asset(filename, bytes, mime, fetched)
                    .is_ok_and(|asset| {
                        safe_file(&asset.path)
                            .ok()
                            .flatten()
                            .is_some_and(|metadata| metadata.len() == asset.bytes)
                    });
                if !valid {
                    state
                        .db
                        .execute("DELETE FROM covers WHERE url_key=?1", [key])
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        for entry in fs::read_dir(&self.images).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let Some(name) = name.to_str().filter(|name| owned_filename(name)) else {
                continue;
            };
            if entry.file_type().map_err(|e| e.to_string())?.is_file()
                && !linked(&entry.metadata().map_err(|e| e.to_string())?)
            {
                self.remove_unreferenced(&state.db, name)?;
            }
        }
        Ok(())
    }

    pub(crate) fn clear(&self) -> AppResult<()> {
        let mut state = self.lock()?;
        state.generation = state.generation.wrapping_add(1);
        state.access = 0;
        state
            .db
            .execute("DELETE FROM covers", [])
            .map_err(|e| e.to_string())?;
        self.reconcile(&mut state)
    }

    pub(crate) fn invalidate(&self, url: &str) -> AppResult<()> {
        let mut state = self.lock()?;
        state.generation = state.generation.wrapping_add(1);
        let key = digest(url.as_bytes());
        let filename: Option<String> = state
            .db
            .query_row(
                "SELECT filename FROM covers WHERE url_key=?1",
                [&key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        state
            .db
            .execute("DELETE FROM covers WHERE url_key=?1", [key])
            .map_err(|e| e.to_string())?;
        if let Some(filename) = filename {
            self.remove_unreferenced(&state.db, &filename)?;
        }
        Ok(())
    }

    pub(crate) fn status(&self) -> AppResult<super::other_cache::Usage> {
        let mut state = self.lock()?;
        self.reconcile(&mut state)?;
        Self::payload_usage(&state.db)
    }
}

#[cfg(test)]
#[path = "cover_disk_tests.rs"]
mod tests;
