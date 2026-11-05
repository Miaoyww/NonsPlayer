//! Native Windows song downloads and their progress state.
use crate::{
    application::Backend,
    infrastructure::{covers::CoverRuntime, http::Http},
    local::{local_folders::LocalFolders, tag_editor},
    model::{AppResult, Track, TrackSource},
    music::{identity::SourceId, service::MusicService},
    storage::{now_millis, Store},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MAX_TASKS: usize = 32;
const MAX_BYTES: u64 = 512 * 1024 * 1024;
const MAX_NETWORK_WAIT: Duration = Duration::from_secs(10 * 60);
const QUALITIES: &[&str] = &["standard", "higher", "exhigh", "lossless", "hires"];
const DEFAULT_QUALITY: &str = "lossless";
const TASKS_KEY: &str = "downloadTasksV1";
const QUALITY_KEY: &str = "downloadQuality";
const FILENAME_FORMAT_KEY: &str = "downloadFilenameFormat";
const DEFAULT_FILENAME_FORMAT: &str = "title-artist";
const FILENAME_FORMATS: &[&str] = &["title-artist", "title", "artist-title"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadState {
    Waiting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub id: u64,
    pub track: Track,
    pub state: DownloadState,
    pub quality: String,
    #[serde(default = "default_filename_format")]
    pub filename_format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_quality: Option<String>,
    pub bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSettings {
    pub directory: String,
    pub quality: String,
    pub filename_format: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    id: u64,
    bytes: u64,
    total: Option<u64>,
}

struct StateData {
    tasks: Vec<DownloadTask>,
    worker: bool,
    cancels: HashMap<u64, Arc<AtomicBool>>,
}

struct DownloadOutcome {
    bytes: u64,
    total: Option<u64>,
    actual_quality: String,
    path: String,
    warning: Option<String>,
}

fn default_filename_format() -> String {
    DEFAULT_FILENAME_FORMAT.into()
}

pub struct Downloads {
    app: tauri::AppHandle,
    store: Arc<Store>,
    music: Arc<MusicService>,
    http: Arc<Http>,
    covers: CoverRuntime,
    folders: Arc<LocalFolders>,
    directory: PathBuf,
    temporary: PathBuf,
    origin: String,
    generation: AtomicU64,
    state: Mutex<StateData>,
}

impl Downloads {
    pub fn new(
        app: tauri::AppHandle,
        store: Arc<Store>,
        music: Arc<MusicService>,
        http: Arc<Http>,
        covers: CoverRuntime,
        folders: Arc<LocalFolders>,
        directory: PathBuf,
        temporary: PathBuf,
    ) -> AppResult<Arc<Self>> {
        if cfg!(windows) {
            std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&temporary).map_err(|error| error.to_string())?;
            cleanup_temporary_files(&temporary);
        }
        let origin = directory
            .canonicalize()
            .unwrap_or_else(|_| directory.clone())
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        let this = Arc::new(Self {
            app,
            store,
            music,
            http,
            covers,
            folders,
            directory,
            temporary,
            origin: origin.clone(),
            generation: AtomicU64::new(1),
            state: Mutex::new(StateData {
                tasks: Vec::new(),
                worker: false,
                cancels: HashMap::new(),
            }),
        });
        this.restore()?;
        if cfg!(windows) {
            if let Err(error) = this.folders.add(origin) {
                crate::infrastructure::diagnostics::warn("downloads.local_folder.add", &error);
            }
        }
        Ok(this)
    }

    pub fn start(self: &Arc<Self>) {
        self.pump();
    }

    fn restore(&self) -> AppResult<()> {
        let mut tasks = self
            .store
            .setting(TASKS_KEY)?
            .and_then(|value| {
                serde_json::from_str::<Vec<DownloadTask>>(&value)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("downloads.restore.decode", error);
                    })
                    .ok()
            })
            .unwrap_or_default();
        tasks.truncate(MAX_TASKS);
        for task in &mut tasks {
            if !QUALITIES.contains(&task.quality.as_str()) {
                task.quality = DEFAULT_QUALITY.into();
            }
            if !FILENAME_FORMATS.contains(&task.filename_format.as_str()) {
                task.filename_format = DEFAULT_FILENAME_FORMAT.into();
            }
            if task.state == DownloadState::Running {
                task.state = DownloadState::Failed;
                task.error = Some("上次下载已中断，请重试".into());
            }
            if task.state == DownloadState::Waiting {
                task.bytes = 0;
                task.total = None;
            }
        }
        for task in &tasks {
            if task.state == DownloadState::Completed {
                if let Err(error) = self.link_task(task) {
                    crate::infrastructure::diagnostics::warn("downloads.restore.link", error);
                }
            }
        }
        self.state.lock().map_err(|_| "下载任务锁不可用")?.tasks = tasks;
        self.persist()
    }

    fn link_task(&self, task: &DownloadTask) -> AppResult<()> {
        let name = task.path.as_deref().ok_or("下载记录缺少本地文件")?;
        let path = self.directory.join(name);
        if !path.is_file() {
            return Err("本地下载文件不存在".into());
        }
        let path = path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        self.store
            .link_download(&task.track.key, &path, task.actual_quality.as_deref())
    }

    pub fn enqueue(self: &Arc<Self>, track: Track) -> AppResult<DownloadTask> {
        if !cfg!(windows) {
            return Err("下载功能当前仅支持 Windows".into());
        }
        online_source(&track)?;
        if serde_json::to_vec(&track)
            .map_err(|error| error.to_string())?
            .len()
            > 64 * 1024
        {
            return Err("歌曲信息超过 64KiB".into());
        }
        let quality = self.quality()?;
        let filename_format = self.filename_format()?;
        self.ensure_folder()?;
        let mut state = self.state.lock().map_err(|_| "下载任务锁不可用")?;
        if let Some(existing) = state.tasks.iter().rev().find(|task| {
            task.track.key == track.key
                && matches!(task.state, DownloadState::Waiting | DownloadState::Running)
        }) {
            return Ok(existing.clone());
        }
        if let Some(existing) = state.tasks.iter().rev().find(|task| {
            task.track.key == track.key
                && task.state == DownloadState::Completed
                && task
                    .path
                    .as_deref()
                    .is_some_and(|path| self.directory.join(path).is_file())
        }) {
            return Ok(existing.clone());
        }
        while state.tasks.len() >= MAX_TASKS {
            let Some(index) = state.tasks.iter().position(|task| {
                matches!(
                    task.state,
                    DownloadState::Completed | DownloadState::Failed | DownloadState::Cancelled
                )
            }) else {
                return Err(format!("下载队列已满（最多 {MAX_TASKS} 首）"));
            };
            state.tasks.remove(index);
        }
        let id = state
            .tasks
            .iter()
            .map(|task| task.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let task = DownloadTask {
            id,
            track,
            state: DownloadState::Waiting,
            quality,
            filename_format,
            actual_quality: None,
            bytes: 0,
            total: None,
            path: None,
            error: None,
            warning: None,
            completed_at: None,
        };
        state.tasks.push(task.clone());
        drop(state);
        if let Err(error) = self.persist() {
            if let Ok(mut state) = self.state.lock() {
                state.tasks.retain(|task| task.id != id);
            }
            return Err(error);
        }
        self.changed();
        self.pump();
        Ok(task)
    }

    pub fn list(&self) -> AppResult<Vec<DownloadTask>> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "下载任务锁不可用")?
            .tasks
            .clone())
    }

    pub fn cancel(&self, id: u64) -> AppResult<()> {
        let mut state = self.state.lock().map_err(|_| "下载任务锁不可用")?;
        let Some(task) = state.tasks.iter_mut().find(|task| task.id == id) else {
            return Err("下载任务不存在".into());
        };
        if matches!(
            task.state,
            DownloadState::Completed | DownloadState::Failed | DownloadState::Cancelled
        ) {
            return Ok(());
        }
        task.state = DownloadState::Cancelled;
        task.error = None;
        if let Some(cancel) = state.cancels.get(&id) {
            cancel.store(true, Ordering::Release);
        }
        drop(state);
        self.persist()?;
        self.changed();
        Ok(())
    }

    pub fn retry(self: &Arc<Self>, id: u64) -> AppResult<()> {
        let mut state = self.state.lock().map_err(|_| "下载任务锁不可用")?;
        let Some(task) = state.tasks.iter_mut().find(|task| task.id == id) else {
            return Err("下载任务不存在".into());
        };
        if !matches!(task.state, DownloadState::Failed | DownloadState::Cancelled) {
            return Ok(());
        }
        task.state = DownloadState::Waiting;
        task.bytes = 0;
        task.total = None;
        task.path = None;
        task.error = None;
        task.warning = None;
        task.actual_quality = None;
        task.completed_at = None;
        drop(state);
        self.persist()?;
        self.changed();
        self.pump();
        Ok(())
    }

    pub fn remove(&self, id: u64) -> AppResult<()> {
        let mut state = self.state.lock().map_err(|_| "下载任务锁不可用")?;
        let Some(task) = state.tasks.iter().find(|task| task.id == id) else {
            return Ok(());
        };
        if matches!(task.state, DownloadState::Waiting | DownloadState::Running) {
            return Err("请先取消正在进行的下载".into());
        }
        state.tasks.retain(|task| task.id != id);
        drop(state);
        self.persist()?;
        self.changed();
        Ok(())
    }

    pub fn clear_finished(&self) -> AppResult<()> {
        let mut state = self.state.lock().map_err(|_| "下载任务锁不可用")?;
        state
            .tasks
            .retain(|task| matches!(task.state, DownloadState::Waiting | DownloadState::Running));
        drop(state);
        self.persist()?;
        self.changed();
        Ok(())
    }

    pub fn settings(&self) -> AppResult<DownloadSettings> {
        Ok(DownloadSettings {
            directory: self.origin.clone(),
            quality: self.quality()?,
            filename_format: self.filename_format()?,
        })
    }

    pub fn set_quality(&self, quality: String) -> AppResult<DownloadSettings> {
        if !QUALITIES.contains(&quality.as_str()) {
            return Err("下载音质无效".into());
        }
        self.store.set_setting(QUALITY_KEY, &quality)?;
        self.settings()
    }

    pub fn set_filename_format(&self, format: String) -> AppResult<DownloadSettings> {
        if !FILENAME_FORMATS.contains(&format.as_str()) {
            return Err("文件名格式无效".into());
        }
        self.store.set_setting(FILENAME_FORMAT_KEY, &format)?;
        self.settings()
    }

    pub fn open_directory(&self, app: &tauri::AppHandle) -> AppResult<()> {
        if !cfg!(windows) {
            return Err("下载功能当前仅支持 Windows".into());
        }
        app.opener()
            .open_path(self.directory.to_string_lossy(), None::<&str>)
            .map_err(|error| error.to_string())
    }

    fn quality(&self) -> AppResult<String> {
        let value = self
            .store
            .setting(QUALITY_KEY)?
            .unwrap_or_else(|| DEFAULT_QUALITY.into());
        Ok(if QUALITIES.contains(&value.as_str()) {
            value
        } else {
            DEFAULT_QUALITY.into()
        })
    }

    fn filename_format(&self) -> AppResult<String> {
        let value = self
            .store
            .setting(FILENAME_FORMAT_KEY)?
            .unwrap_or_else(default_filename_format);
        Ok(if FILENAME_FORMATS.contains(&value.as_str()) {
            value
        } else {
            DEFAULT_FILENAME_FORMAT.into()
        })
    }

    fn ensure_folder(&self) -> AppResult<()> {
        if cfg!(windows) {
            std::fs::create_dir_all(&self.directory).map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&self.temporary).map_err(|error| error.to_string())?;
            if let Err(error) = self.folders.add(self.origin.clone()) {
                crate::infrastructure::diagnostics::warn("downloads.local_folder.ensure", &error);
            }
        }
        Ok(())
    }

    fn persist(&self) -> AppResult<()> {
        let value = {
            let state = self.state.lock().map_err(|_| "下载任务锁不可用")?;
            serde_json::to_string(&state.tasks).map_err(|error| error.to_string())?
        };
        self.store.set_setting(TASKS_KEY, &value)
    }

    fn changed(&self) {
        let tasks = self.list().unwrap_or_default();
        let _ = self
            .app
            .emit("downloads-changed", tasks)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("downloads.changed.emit", error);
            });
    }

    fn progress(&self, id: u64, bytes: u64, total: Option<u64>) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(task) = state.tasks.iter_mut().find(|task| task.id == id) {
                task.bytes = bytes;
                task.total = total;
            }
        }
        let _ = self
            .app
            .emit("downloads-progress", DownloadProgress { id, bytes, total })
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("downloads.progress.emit", error);
            });
    }

    fn pump(self: &Arc<Self>) {
        if !cfg!(windows) {
            return;
        }
        let id = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.worker {
                return;
            }
            let Some(index) = state
                .tasks
                .iter()
                .position(|task| task.state == DownloadState::Waiting)
            else {
                return;
            };
            state.worker = true;
            let id = state.tasks[index].id;
            let task = &mut state.tasks[index];
            task.state = DownloadState::Running;
            task.actual_quality = None;
            task.bytes = 0;
            task.total = None;
            task.path = None;
            task.error = None;
            task.warning = None;
            task.completed_at = None;
            state.cancels.insert(id, Arc::new(AtomicBool::new(false)));
            id
        };
        self.changed();
        if let Err(error) = self.persist() {
            crate::infrastructure::diagnostics::warn("downloads.persist.running", &error);
        }
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            this.run_task(id).await;
            if let Ok(mut state) = this.state.lock() {
                state.worker = false;
                state.cancels.remove(&id);
            }
            this.pump();
        });
    }

    async fn run_task(self: &Arc<Self>, id: u64) {
        let (track, quality, filename_format, cancel) = {
            let Ok(state) = self.state.lock() else {
                return;
            };
            let Some(task) = state.tasks.iter().find(|task| task.id == id) else {
                return;
            };
            let Some(cancel) = state.cancels.get(&id).cloned() else {
                return;
            };
            (
                task.track.clone(),
                task.quality.clone(),
                task.filename_format.clone(),
                cancel,
            )
        };
        if cancel.load(Ordering::Acquire) {
            return;
        }
        let result = self
            .download(id, &track, &quality, &filename_format, &cancel)
            .await;
        let cancelled = cancel.load(Ordering::Acquire);
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        let Some(task) = state.tasks.iter_mut().find(|task| task.id == id) else {
            return;
        };
        if cancelled || task.state == DownloadState::Cancelled {
            task.state = DownloadState::Cancelled;
            task.error = None;
        } else {
            match result {
                Ok(outcome) => {
                    task.state = DownloadState::Completed;
                    task.bytes = outcome.bytes;
                    task.total = outcome.total;
                    task.actual_quality = Some(outcome.actual_quality);
                    task.path = Some(outcome.path);
                    task.warning = outcome.warning;
                    task.error = None;
                    task.completed_at = Some(now_millis());
                }
                Err(error) => {
                    task.state = DownloadState::Failed;
                    task.error = Some(error);
                }
            }
        }
        drop(state);
        if let Err(error) = self.persist() {
            crate::infrastructure::diagnostics::warn("downloads.persist", &error);
        }
        self.changed();
    }

    async fn download(
        &self,
        id: u64,
        track: &Track,
        quality: &str,
        filename_format: &str,
        cancel: &AtomicBool,
    ) -> AppResult<DownloadOutcome> {
        let source = online_source(track)?;
        let identity = self
            .music
            .adapters
            .cache_identity(&source)
            .map_err(|error| error.to_string())?;
        let generation = self.generation.fetch_add(1, Ordering::Relaxed);
        let (_, ticket, _) = self
            .music
            .resolve(track.clone(), quality, true, generation)
            .await?;
        let ticket = ticket.ok_or("无法解析下载资源")?;
        let actual_quality = ticket.quality().to_owned();
        let url = ticket.uri().to_owned();
        let headers = ticket.headers().to_vec();
        drop(ticket);
        self.check_identity(&source, &identity)?;

        let parsed = download_url(&url)?;
        let temporary = self.temporary.join(format!(
            ".nons-download-{id}-{:016x}.part",
            fastrand::u64(..)
        ));
        let downloaded = self
            .download_to_temp(id, &temporary, &parsed, &headers, cancel)
            .await;
        let (bytes, total) = match downloaded {
            Ok(value) => value,
            Err(error) => {
                let _ = tokio::fs::remove_file(&temporary).await;
                return Err(error);
            }
        };
        let format = match audio_format(&temporary).await {
            Ok(format) => format,
            Err(error) => {
                let _ = tokio::fs::remove_file(&temporary).await;
                return Err(error);
            }
        };
        let cover = self.cover_bytes(track).await;
        let cover_warning = match self.write_tags(&temporary, track, cover.clone()).await {
            Ok(warning) => warning,
            Err(error) => {
                let _ = tokio::fs::remove_file(&temporary).await;
                return Err(error);
            }
        };
        if let Err(error) = self.check_identity(&source, &identity) {
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(error);
        }
        let path = match self
            .publish(&temporary, track, filename_format, format)
            .await
        {
            Ok(path) => path,
            Err(error) => {
                let _ = tokio::fs::remove_file(&temporary).await;
                return Err(error);
            }
        };
        let mut warnings = Vec::new();
        if let Some(cover) = cover_warning {
            warnings.push(cover);
        }
        let path_text = path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        if let Err(error) = self
            .store
            .link_download(&track.key, &path_text, Some(&actual_quality))
        {
            warnings.push(format!("本地播放关联保存失败：{error}"));
        }
        if let Err(error) = self.index_file(&path).await {
            warnings.push(error);
        }
        Ok(DownloadOutcome {
            bytes,
            total,
            actual_quality,
            path: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            warning: (!warnings.is_empty()).then(|| warnings.join("；")),
        })
    }

    fn check_identity(&self, source: &SourceId, expected: &str) -> AppResult<()> {
        if self
            .music
            .adapters
            .cache_identity(source)
            .map_err(|error| error.to_string())?
            != expected
        {
            return Err("下载期间账号或来源已变化，请重试".into());
        }
        Ok(())
    }

    async fn download_to_temp(
        &self,
        id: u64,
        path: &Path,
        url: &url::Url,
        headers: &[(String, String)],
        cancel: &AtomicBool,
    ) -> AppResult<(u64, Option<u64>)> {
        if headers.len() > 32 {
            return Err("下载请求头过多".into());
        }
        let mut request = self
            .http
            .client(url)
            .get(url.clone())
            .timeout(Duration::from_secs(24 * 60 * 60));
        for (name, value) in headers {
            let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| "下载请求头无效")?;
            if matches!(
                name.as_str(),
                "host" | "content-length" | "transfer-encoding" | "connection"
            ) {
                continue;
            }
            let value =
                reqwest::header::HeaderValue::from_str(value).map_err(|_| "下载请求头无效")?;
            request = request.header(name, value);
        }
        let mut response = tokio::time::timeout(Duration::from_secs(30), request.send())
            .await
            .map_err(|_| "连接下载资源超时")?
            .map_err(|error| {
                crate::infrastructure::diagnostics::warn("downloads.http.send", &error);
                "连接下载资源失败".to_owned()
            })?;
        if !response.status().is_success() {
            return Err(format!(
                "下载服务器返回 HTTP {}",
                response.status().as_u16()
            ));
        }
        let total = response.content_length();
        if total.is_some_and(|total| total > MAX_BYTES) {
            return Err("下载资源超过 512MiB 上限".into());
        }
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await
            .map_err(|error| format!("无法创建下载文件：{error}"))?;
        let mut bytes = 0u64;
        let mut network_wait = Duration::ZERO;
        let mut last_progress = Instant::now();
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err("下载已取消".into());
            }
            let started = Instant::now();
            let chunk = tokio::time::timeout(Duration::from_secs(30), response.chunk())
                .await
                .map_err(|_| "读取下载内容超时")?
                .map_err(|error| {
                    crate::infrastructure::diagnostics::warn("downloads.http.chunk", &error);
                    "读取下载内容失败".to_owned()
                })?;
            network_wait += started.elapsed();
            if network_wait > MAX_NETWORK_WAIT {
                return Err("下载网络等待超过 10 分钟".into());
            }
            let Some(chunk) = chunk else {
                break;
            };
            bytes = bytes.saturating_add(chunk.len() as u64);
            if bytes > MAX_BYTES {
                return Err("下载资源超过 512MiB 上限".into());
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| format!("写入下载文件失败：{error}"))?;
            if last_progress.elapsed() >= Duration::from_millis(250) {
                last_progress = Instant::now();
                self.progress(id, bytes, total);
            }
        }
        file.flush()
            .await
            .map_err(|error| format!("保存下载文件失败：{error}"))?;
        file.sync_all()
            .await
            .map_err(|error| format!("保存下载文件失败：{error}"))?;
        drop(file);
        if bytes == 0 || total.is_some_and(|total| total != bytes) {
            return Err("下载内容不完整".into());
        }
        self.progress(id, bytes, total);
        Ok((bytes, total))
    }

    async fn cover_bytes(&self, track: &Track) -> Option<Vec<u8>> {
        let url = track.cover.to_string();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return None;
        }
        match tokio::time::timeout(Duration::from_secs(8), self.covers.read(url, &self.app)).await {
            Ok(Ok(bytes)) if !bytes.is_empty() => Some(bytes),
            Ok(Ok(_)) => None,
            Ok(Err(error)) => {
                crate::infrastructure::diagnostics::warn("downloads.cover.read", &error);
                None
            }
            Err(_) => {
                crate::infrastructure::diagnostics::warn(
                    "downloads.cover.timeout",
                    "cover download timed out",
                );
                None
            }
        }
    }

    async fn write_tags(
        &self,
        path: &Path,
        track: &Track,
        cover: Option<Vec<u8>>,
    ) -> AppResult<Option<String>> {
        match write_tags_file(
            path,
            &track.title,
            &track.artist,
            &track.album,
            cover.clone(),
        )
        .await
        {
            Ok(()) => Ok(None),
            Err(error) if cover.is_some() => {
                crate::infrastructure::diagnostics::warn("downloads.tags.cover_retry", &error);
                if let Some(cover) = cover.clone() {
                    if let Some(shrunk) = shrink_cover(cover).await {
                        if write_tags_file(
                            path,
                            &track.title,
                            &track.artist,
                            &track.album,
                            Some(shrunk),
                        )
                        .await
                        .is_ok()
                        {
                            return Ok(Some("封面过大，已压缩后写入".into()));
                        }
                    }
                }
                write_tags_file(path, &track.title, &track.artist, &track.album, None).await?;
                Ok(Some(format!("封面写入失败，已保留其它标签：{error}")))
            }
            Err(error) => Err(error),
        }
    }

    async fn publish(
        &self,
        temporary: &Path,
        track: &Track,
        filename_format: &str,
        format: &str,
    ) -> AppResult<PathBuf> {
        let base = filename(track, filename_format);
        for index in 1..=1000 {
            let name = if index == 1 {
                format!("{base}.{format}")
            } else {
                format!("{base} ({index}).{format}")
            };
            let target = self.directory.join(name);
            if tokio::fs::try_exists(&target).await.unwrap_or(false) {
                continue;
            }
            match tokio::fs::rename(temporary, &target).await {
                Ok(()) => return Ok(target),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(format!("保存下载文件失败：{error}")),
            }
        }
        Err("无法为下载文件生成可用文件名".into())
    }

    async fn index_file(&self, path: &Path) -> AppResult<()> {
        let folders = self.folders.clone();
        let origin = self.origin.clone();
        let path = path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        tauri::async_runtime::spawn_blocking(move || folders.index_file(path, origin))
            .await
            .map_err(|error| error.to_string())??;
        Ok(())
    }
}

async fn shrink_cover(bytes: Vec<u8>) -> Option<Vec<u8>> {
    tauri::async_runtime::spawn_blocking(move || {
        let image = image::load_from_memory(&bytes).ok()?;
        let resized = image.thumbnail(4096, 4096);
        let mut output = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 90)
            .encode_image(&resized)
            .ok()?;
        Some(output)
    })
    .await
    .ok()?
}

async fn write_tags_file(
    path: &Path,
    title: &str,
    artist: &str,
    album: &str,
    cover: Option<Vec<u8>>,
) -> AppResult<()> {
    let path = path.to_path_buf();
    let title = title.to_owned();
    let artist = artist.to_owned();
    let album = album.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("无法打开下载文件：{error}"))?;
        tag_editor::write_download_tags(&mut file, &title, &artist, &album, cover.as_deref())
    })
    .await
    .map_err(|error| error.to_string())?
}

fn online_source(track: &Track) -> AppResult<SourceId> {
    match &track.source {
        TrackSource::Online { track } => Ok(track.reference.source.clone()),
        TrackSource::Local { .. } => Err("只能下载在线歌曲".into()),
    }
}

fn download_url(input: &str) -> AppResult<url::Url> {
    if input.len() > 2048 {
        return Err("下载地址超过 2048 字节".into());
    }
    let url = url::Url::parse(input).map_err(|_| "下载地址无效")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("下载地址无效".into());
    }
    Ok(url)
}

async fn audio_format(path: &Path) -> AppResult<&'static str> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| format!("无法读取下载文件：{error}"))?;
    let mut head = [0u8; 16];
    let count = file
        .read(&mut head)
        .await
        .map_err(|error| format!("无法读取下载文件：{error}"))?;
    let head = &head[..count];
    if head.starts_with(b"fLaC") {
        return Ok("flac");
    }
    if head.starts_with(b"ID3")
        || (head.len() >= 2 && head[0] == 0xff && head[1] & 0xe0 == 0xe0 && head[1] & 6 != 0)
    {
        return Ok("mp3");
    }
    if head.len() >= 8 && &head[4..8] == b"ftyp" {
        return Ok("m4a");
    }
    if head.starts_with(b"OggS") {
        return Ok("ogg");
    }
    if head.starts_with(b"RIFF") && head.len() >= 12 && &head[8..12] == b"WAVE" {
        return Ok("wav");
    }
    Err("下载资源不是受支持的音频文件".into())
}

fn filename(track: &Track, format: &str) -> String {
    let title = if track.title.trim().is_empty() {
        "歌曲"
    } else {
        track.title.as_str()
    };
    let artist = artist_filename(track);
    let name = match format {
        "title" => title.to_owned(),
        "artist-title" if !artist.is_empty() => format!("{artist} - {title}"),
        _ if !artist.is_empty() => format!("{title} - {artist}"),
        _ => title.to_owned(),
    };
    sanitize(&name)
}

fn artist_filename(track: &Track) -> String {
    let names: Vec<&str> = track
        .artists
        .iter()
        .map(|artist| artist.name.trim())
        .filter(|name| !name.is_empty())
        .collect();
    if !names.is_empty() {
        return names.join(", ");
    }
    track
        .artist
        .split(['/', '、', ';'])
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn sanitize(value: &str) -> String {
    let mut value: String = value
        .chars()
        .map(|character| {
            if character.is_control() || r#"<>:"/\|?*"#.contains(character) {
                '_'
            } else {
                character
            }
        })
        .take(80)
        .collect();
    value = value.trim_end_matches(['.', ' ']).to_owned();
    if value.is_empty() {
        return "歌曲".into();
    }
    let upper = value.to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"]
        .into_iter()
        .chain((1..=9).map(|_| "COM"))
        .chain((1..=9).map(|_| "LPT"))
        .any(|prefix| {
            upper == prefix
                || upper
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.len() == 1 && rest.as_bytes()[0].is_ascii_digit())
        });
    if reserved {
        format!("_{value}")
    } else {
        value
    }
}

fn cleanup_temporary_files(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(".nons-download-") {
            let _ = std::fs::remove_file(entry.path()).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("downloads.cleanup", error);
            });
        }
    }
}

#[tauri::command]
pub fn downloads_add(track: Track, backend: State<'_, Backend>) -> AppResult<DownloadTask> {
    backend.downloads.enqueue(track)
}

#[tauri::command]
pub fn downloads_list(backend: State<'_, Backend>) -> AppResult<Vec<DownloadTask>> {
    backend.downloads.list()
}

#[tauri::command]
pub fn downloads_cancel(id: u64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.downloads.cancel(id)
}

#[tauri::command]
pub fn downloads_retry(id: u64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.downloads.retry(id)
}

#[tauri::command]
pub fn downloads_remove(id: u64, backend: State<'_, Backend>) -> AppResult<()> {
    backend.downloads.remove(id)
}

#[tauri::command]
pub fn downloads_clear_finished(backend: State<'_, Backend>) -> AppResult<()> {
    backend.downloads.clear_finished()
}

#[tauri::command]
pub fn download_settings(backend: State<'_, Backend>) -> AppResult<DownloadSettings> {
    backend.downloads.settings()
}

#[tauri::command]
pub fn set_download_quality(
    quality: String,
    backend: State<'_, Backend>,
) -> AppResult<DownloadSettings> {
    backend.downloads.set_quality(quality)
}

#[tauri::command]
pub fn set_download_filename_format(
    format: String,
    backend: State<'_, Backend>,
) -> AppResult<DownloadSettings> {
    backend.downloads.set_filename_format(format)
}

#[tauri::command]
pub fn open_download_directory(
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.downloads.open_directory(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str, artist: &str, title: &str) -> Track {
        serde_json::from_value(serde_json::json!({
            "key": format!("netease/track/{id}"),
            "title": title,
            "artist": artist,
            "album": "",
            "durationMs": 1000,
            "cover": {
                "original": "",
                "small": "",
                "normal": "",
                "large": "",
                "exlarge": ""
            },
            "source": {
                "kind": "online",
                "track": {
                    "reference": {"source": "netease", "kind": "track", "id": id},
                    "title": title,
                    "aliases": [],
                    "artist": artist,
                    "artists": [],
                    "album": "",
                    "albumReference": null,
                    "durationMs": 1000,
                    "cover": {
                        "original": "",
                        "small": "",
                        "normal": "",
                        "large": "",
                        "exlarge": ""
                    },
                    "associations": []
                }
            }
        }))
        .unwrap()
    }

    #[test]
    fn download_urls_allow_credentials_ports_and_private_hosts() {
        for input in [
            "https://user:secret@example.com/a",
            "http://example.com:8080/a",
            "https://127.0.0.1/a",
            "http://localhost/a",
        ] {
            assert!(download_url(input).is_ok(), "{input}");
        }
        assert!(download_url("file:///tmp/a").is_err());
    }

    #[test]
    fn download_names_follow_the_selected_format_and_separate_artists_with_commas() {
        let value = track("12", "Singer / Other", "Title:*?");
        assert_eq!(
            filename(&value, DEFAULT_FILENAME_FORMAT),
            "Title___ - Singer, Other"
        );
        assert_eq!(filename(&value, "title"), "Title___");
        assert_eq!(filename(&value, "artist-title"), "Singer, Other - Title___");
        assert_eq!(sanitize("CON"), "_CON");
        assert_eq!(sanitize("LPT1"), "_LPT1");
    }

    #[tokio::test]
    async fn audio_headers_reject_non_audio_and_detect_containers() {
        let directory = tempfile::tempdir().unwrap();
        let flac = directory.path().join("song.part");
        tokio::fs::write(&flac, b"fLaC\0\0\0\0").await.unwrap();
        assert_eq!(audio_format(&flac).await.unwrap(), "flac");
        let html = directory.path().join("bad.part");
        tokio::fs::write(&html, b"<html>login</html>")
            .await
            .unwrap();
        assert!(audio_format(&html).await.is_err());
    }
}
