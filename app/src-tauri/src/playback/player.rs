use super::history::PlaybackHistory;
use super::publication::{PlayerBootstrap, Publication};
#[cfg(test)]
use crate::model::RepeatMode;
use crate::{
    media::MediaControls,
    model::{
        AppResult, PlaybackOptions, PlaybackStatus, PlayerSnapshot, ResolvedTrack, Track,
        TrackSource,
    },
    music::{manager::ResourceTicket, service::MusicService},
    storage::Store,
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};
use tauri::Emitter;

pub enum Command {
    WebReady(mpsc::Sender<AppResult<super::web_audio::Control>>),
    WebFeedback(super::web_audio::Feedback),
    SourceAttempt {
        generation: u64,
        index: usize,
        key: String,
        source: crate::music::manager::PlaybackSource,
    },
    SelectSource {
        revision: u64,
        selection: crate::music::service::PlaybackSelection,
        reply: tokio::sync::oneshot::Sender<AppResult<()>>,
    },
    #[cfg(target_os = "android")]
    Media {
        command: Box<Command>,
        reply:
            tokio::sync::oneshot::Sender<AppResult<crate::platform::android::playback::Snapshot>>,
    },
    Collection {
        entity: crate::music::identity::EntityRef,
        track: Option<Box<Track>>,
        cached: Option<crate::music::collections::CachedCollection>,
        scope: String,
        fallback_cover: String,
        reply: tokio::sync::oneshot::Sender<AppResult<bool>>,
    },
    CollectionReady {
        serial: u64,
        scope: String,
        prefix: Vec<String>,
        entity: crate::music::identity::EntityRef,
        fallback_cover: String,
        result: AppResult<(Vec<Track>, bool)>,
        reply: tokio::sync::oneshot::Sender<AppResult<bool>>,
    },
    Queue(Vec<Track>, usize),
    FmQueue(Vec<Track>),
    AppendFm {
        session: u64,
        queue_len: usize,
        tracks: Vec<Track>,
    },
    PlayNext(Vec<Track>),
    PlayNextNow(Vec<Track>),
    Jump(usize),
    Retry {
        revision: u64,
        key: String,
    },
    Pause,
    Toggle,
    Resume,
    Stop,
    Clear,
    Remove(usize, String),
    Next,
    Previous,
    Repeat,
    Shuffle,
    Seek(u64),
    Volume(f64),
    Device(Option<String>, mpsc::Sender<AppResult<()>>),
    Options(PlaybackOptions, mpsc::Sender<AppResult<()>>),
    Resolved {
        generation: u64,
        index: usize,
        key: String,
        result: Box<AppResult<crate::music::service::ResolvedPlayback>>,
        next: bool,
    },
    Shutdown,
}

fn next_available(
    state: &PlayerSnapshot,
    failed: &std::collections::HashSet<String>,
) -> Option<usize> {
    let current = state.index?;
    let count = state.queue.len();
    let slot = if state.shuffle {
        state.shuffle_order.iter().position(|i| *i == current)?
    } else {
        current
    };
    for step in 1..=count {
        let candidate = slot + step;
        if candidate >= count
            && !state.shuffle
            && state.repeat_mode == crate::model::RepeatMode::Off
        {
            break;
        }
        let index = if state.shuffle {
            *state.shuffle_order.get(candidate % count)?
        } else {
            candidate % count
        };
        if !failed.contains(&state.queue[index].key) {
            return Some(index);
        }
    }
    None
}

fn previous_command(state: &PlayerSnapshot) -> Option<Command> {
    state.current()?;
    // Manual navigation always selects a queue entry, regardless of position.
    // Jump goes through load(), invalidating pending preload work.
    state.previous().map(Command::Jump)
}

fn retry_position(state: &PlayerSnapshot, revision: u64, key: &str) -> Option<(usize, u64)> {
    if state.revision != revision
        || state.current()?.key != key
        || !matches!(
            state.status,
            PlaybackStatus::Stopped | PlaybackStatus::Error
        )
    {
        return None;
    }
    Some((state.index?, state.position_ms))
}

fn restore_player(mut state: PlayerSnapshot) -> PlayerSnapshot {
    // Android volume is controlled by the system, including for older saved sessions.
    if cfg!(target_os = "android") {
        state.volume = 1.0;
    }
    state.deduplicate_queue();
    state.status = PlaybackStatus::Stopped;
    state.error = None;
    state.media_error = None;
    state.revision = 0;
    state.actual_quality = None;
    state.playback_source = None;
    if let Some(track) = state.current() {
        if track.duration_ms > 0 {
            state.duration_ms = track.duration_ms;
        }
        if state.duration_ms > 0 {
            state.position_ms = state.position_ms.min(state.duration_ms);
        }
    } else {
        state.index = None;
        state.position_ms = 0;
        state.duration_ms = 0;
    }
    state.restore_shuffle_order();
    state
}

fn stored_options(store: &Store, fallback: PlaybackOptions) -> AppResult<PlaybackOptions> {
    store
        .setting("playbackOptions")?
        .map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
        .unwrap_or(Ok(fallback))
}

pub struct Player {
    sender: SyncSender<Command>,
    snapshot: Arc<RwLock<Publication>>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    history: Arc<PlaybackHistory>,
}

struct Prepared {
    generation: u64,
    index: usize,
    resolved: ResolvedTrack,
    resource: Option<Arc<ResourceTicket>>,
    source: Option<crate::music::manager::PlaybackSource>,
}

impl Player {
    pub fn start(
        app: tauri::AppHandle,
        store: Arc<Store>,
        music: Arc<MusicService>,
        hwnd: isize,
    ) -> AppResult<Self> {
        let mut restored = store
            .setting("player")?
            .and_then(|s| {
                serde_json::from_str::<PlayerSnapshot>(&s)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "playback.restore.decode_snapshot",
                            error,
                        );
                    })
                    .ok()
            })
            .map(restore_player)
            .unwrap_or_default();
        restored.playback_options = stored_options(&store, restored.playback_options)?;
        restored.device_id = store
            .setting("webAudioDevice")?
            .and_then(|value| serde_json::from_str(&value).ok())
            .flatten();
        let snapshot = Arc::new(RwLock::new(Publication::new(restored)));
        let shared = Arc::clone(&snapshot);
        let history = PlaybackHistory::start(Arc::clone(&store));
        let actor_history = Arc::clone(&history);
        let (sender, receiver) = mpsc::sync_channel(64);
        let actor_sender = sender.clone();
        let thread = std::thread::Builder::new()
            .name("nons-audio".into())
            .spawn(move || {
                let result = Actor::new(
                    app.clone(),
                    store,
                    music,
                    actor_history,
                    shared.clone(),
                    actor_sender,
                    hwnd,
                );
                match result {
                    Ok(mut actor) => actor.run(receiver),
                    Err(error) => {
                        if let Ok(mut state) = (shared.write()).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "playback.start.failure_state_lock",
                                error,
                            );
                        }) {
                            let mut failed = state.snapshot.clone();
                            failed.status = PlaybackStatus::Error;
                            failed.error = Some(error);
                            let (event, _, progress) = state.update(&failed);
                            let _ = (app.emit("player-state", event)).inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("app.emit", error);
                            });
                            if let Some(progress) = progress {
                                let _ =
                                    (app.emit("player-progress", progress)).inspect_err(|error| {
                                        crate::infrastructure::diagnostics::warn("app.emit", error);
                                    });
                            }
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            sender,
            snapshot,
            thread: Mutex::new(Some(thread)),
            history,
        })
    }

    pub fn send(&self, command: Command) -> AppResult<()> {
        self.sender.try_send(command).map_err(|error| match error {
            mpsc::TrySendError::Full(_) => "播放核心忙碌，请稍后重试".into(),
            mpsc::TrySendError::Disconnected(_) => self
                .snapshot
                .read()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "playback.send.failure_state_read",
                        error,
                    );
                })
                .ok()
                .and_then(|state| state.snapshot.error.clone())
                .unwrap_or_else(|| "播放核心不可用，请重启应用".into()),
        })
    }

    pub fn snapshot(&self) -> AppResult<PlayerSnapshot> {
        self.snapshot
            .read()
            .map(|s| s.snapshot.clone())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("playback.snapshot.read", error);
            })
            .map_err(|_| "播放状态不可用".into())
    }

    pub fn bootstrap(&self) -> AppResult<PlayerBootstrap> {
        self.snapshot
            .read()
            .map(|s| s.bootstrap())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("playback.bootstrap.read", error);
            })
            .map_err(|_| "播放状态不可用".into())
    }

    pub fn set_device(&self, id: Option<String>) -> AppResult<()> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::Device(id, sender))?;
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "切换音频设备超时".to_string())?
    }

    pub fn set_options(&self, options: PlaybackOptions) -> AppResult<()> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::Options(options, sender))?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("playback.options.reply_timeout", error);
            })
            .map_err(|_| "保存播放设置超时".to_string())?
    }

    pub fn web_ready(&self) -> AppResult<super::web_audio::Control> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::WebReady(sender))?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "浏览器播放核心未响应".to_string())?
    }

    pub fn shutdown(&self) {
        let _ = (self.sender.send(Command::Shutdown)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("playback.shutdown.enqueue", error);
        });
        if let Ok(mut thread) = (self.thread.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("playback.shutdown.thread_lock", error);
        }) {
            if let Some(thread) = thread.take() {
                let _ = (thread.join()).inspect_err(|error| {
                    let reason = error
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| error.downcast_ref::<&str>().copied())
                        .unwrap_or("audio worker panicked with a non-text payload");
                    crate::infrastructure::diagnostics::warn("audio.join", reason);
                });
            }
        }
        self.history.shutdown();
    }
}

struct Actor {
    web_audio: super::web_audio::WebAudio,
    selections: std::collections::BTreeMap<String, crate::music::service::PlaybackSelection>,
    app: tauri::AppHandle,
    store: Arc<Store>,
    history: Arc<PlaybackHistory>,
    music: Arc<MusicService>,
    shared: Arc<RwLock<Publication>>,
    sender: SyncSender<Command>,
    state: PlayerSnapshot,
    media: Option<MediaControls>,
    desired_playing: bool,
    failed_keys: std::collections::HashSet<String>,
    generation: Arc<AtomicU64>,
    prepared: Arc<Mutex<Option<Prepared>>>,
    current_job: Option<tauri::async_runtime::JoinHandle<()>>,
    collection_job: Option<tauri::async_runtime::JoinHandle<()>>,
    collection_request: super::collection::CollectionRequest,
    next_job: Option<tauri::async_runtime::JoinHandle<()>>,
    next_attempt: Option<Instant>,
    next_attempts: u8,
    last_tick: Instant,
    last_save: Instant,
    play_instance_id: String,
    recorded_instance_id: Option<String>,
}

impl Actor {
    fn web_feedback(&mut self, feedback: super::web_audio::Feedback) -> AppResult<()> {
        use super::web_audio::FeedbackKind;
        if !self.web_audio.accepts(&feedback) {
            return Ok(());
        }
        if feedback.kind == FeedbackKind::Stopped {
            return Ok(());
        }
        if self.web_audio.control().source.is_none() || self.current_job.is_some() {
            return Ok(());
        }
        if feedback.kind == FeedbackKind::DeviceDefault {
            return self.select_device(None);
        }
        if feedback.kind == FeedbackKind::Transition {
            let control = self.web_audio.control();
            if !self.state.playback_options.crossfade
                || control.next.as_ref().map(|n| n.source.as_str())
                    != feedback.next_source.as_deref()
            {
                return Ok(());
            }
            let next = self.prepared.lock().map_err(|_| "预加载状态不可用")?.take();
            if let Some(next) = next.filter(|p| {
                self.state.following(false) == Some(p.index)
                    && p.resource
                        .as_ref()
                        .is_none_or(|r| r.check(p.generation).is_ok())
            }) {
                self.state.index = Some(next.index);
                self.state.revision += 1;
                self.state.position_ms =
                    crate::audio::track_position(self.state.current(), feedback.position_ms);
                self.state.duration_ms =
                    crate::audio::track_duration(self.state.current(), feedback.duration_ms);
                self.state.actual_quality = next.resolved.quality;
                self.state.playback_source = next.source;
                self.state.status = if self.desired_playing {
                    PlaybackStatus::Playing
                } else {
                    PlaybackStatus::Paused
                };
                self.state.error = None;
                self.play_instance_id = uuid::Uuid::new_v4().to_string();
                self.recorded_instance_id = None;
                self.web_audio.promote(&self.app, self.state.position_ms)?;
                if self.desired_playing {
                    self.record_playback_history();
                }
                self.next_attempt = None;
                self.next_attempts = 0;
                self.publish();
            } else {
                self.web_audio.cancel_preload(&self.app)?;
            }
            return Ok(());
        }
        if feedback.kind == FeedbackKind::TransitionEnd {
            self.web_audio.retire()?;
            return Ok(());
        }
        if feedback.kind == FeedbackKind::PreloadError {
            if self
                .web_audio
                .control()
                .next
                .as_ref()
                .map(|n| n.source.as_str())
                == feedback.next_source.as_deref()
            {
                self.prepared.lock().map_err(|_| "预加载状态不可用")?.take();
                self.web_audio.cancel_preload(&self.app)?;
            }
            return Ok(());
        }
        self.state.position_ms =
            crate::audio::track_position(self.state.current(), feedback.position_ms);
        if self.state.position_ms >= 1000 && feedback.kind == FeedbackKind::Playing {
            self.failed_keys.clear();
        }
        if feedback.duration_ms > 0 {
            self.state.duration_ms =
                crate::audio::track_duration(self.state.current(), feedback.duration_ms);
        }
        #[cfg(feature = "performance-probe")]
        if feedback.kind == FeedbackKind::Playing && self.desired_playing {
            crate::performance::browser_playing();
        }
        let status = match feedback.kind {
            FeedbackKind::Playing if self.desired_playing => Some(PlaybackStatus::Playing),
            FeedbackKind::Paused if !self.desired_playing => Some(PlaybackStatus::Paused),
            FeedbackKind::Buffering if self.desired_playing => Some(PlaybackStatus::Buffering),
            FeedbackKind::Ended => {
                if self.desired_playing {
                    if let Some(index) = self.state.following(false) {
                        return self.load(index, true);
                    }
                }
                self.desired_playing = false;
                self.state.status = PlaybackStatus::Stopped;
                self.state.position_ms = self.state.duration_ms;
                self.web_audio.stop(&self.app)?;
                self.publish();
                return Ok(());
            }
            FeedbackKind::Error => {
                self.web_audio.stop(&self.app)?;
                self.skip_failed(
                    feedback
                        .error
                        .filter(|e| e.len() <= 256)
                        .unwrap_or_else(|| "系统 WebView 无法播放此音频".into()),
                )?;
                return Ok(());
            }
            _ => None,
        };
        if let Some(status) = status {
            if self.state.status != status {
                self.state.status = status;
                if status == PlaybackStatus::Playing {
                    self.record_playback_history();
                }
                self.publish();
            }
        }
        Ok(())
    }

    fn new(
        app: tauri::AppHandle,
        store: Arc<Store>,
        music: Arc<MusicService>,
        history: Arc<PlaybackHistory>,
        shared: Arc<RwLock<Publication>>,
        sender: SyncSender<Command>,
        hwnd: isize,
    ) -> AppResult<Self> {
        let mut state = shared
            .read()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("playback.actor.restore_lock", error);
            })
            .map_err(|_| "播放状态不可用")?
            .snapshot
            .clone();
        let media = match MediaControls::new(hwnd, sender.clone()) {
            Ok(m) => Some(m),
            Err(e) => {
                log::error!(target: "nons::media", "operation=initialize reason={e}");
                state.media_error = Some(format!("系统媒体控制初始化失败：{e}"));
                None
            }
        };
        let prepared = Arc::new(Mutex::new(None::<Prepared>));
        let generation = Arc::new(AtomicU64::new(0));
        let mut web_audio = super::web_audio::WebAudio::new()?;
        web_audio.configure(&state);
        Ok(Self {
            web_audio,
            selections: Default::default(),
            app,
            store,
            history,
            music,
            shared,
            sender,
            state,
            media,
            desired_playing: false,
            failed_keys: Default::default(),
            generation,
            prepared,
            current_job: None,
            collection_job: None,
            collection_request: Default::default(),
            next_job: None,
            next_attempt: None,
            next_attempts: 0,
            last_tick: Instant::now(),
            last_save: Instant::now(),
            play_instance_id: uuid::Uuid::new_v4().to_string(),
            recorded_instance_id: None,
        })
    }

    fn run(&mut self, receiver: Receiver<Command>) {
        self.publish();
        loop {
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(command) => {
                    if let Err(error) = self.command(command) {
                        self.state.error = Some(error);
                        self.publish();
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if self.last_tick.elapsed() >= Duration::from_millis(250) {
                self.tick();
                self.last_tick = Instant::now();
            }
            if self.last_save.elapsed() >= Duration::from_secs(5) {
                if let Err(e) = self.store.save_player(&self.state) {
                    self.state.error = Some(format!("播放记录保存失败：{e}"));
                    self.publish();
                }
                self.last_save = Instant::now();
            }
        }
        self.cancel_jobs();
        let _ = (self.store.save_player(&self.state)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("playback.persist.snapshot", error);
        });
    }

    fn command(&mut self, command: Command) -> AppResult<()> {
        if let Command::Retry { revision, key } = &command {
            if retry_position(&self.state, *revision, key).is_none() {
                return Ok(());
            }
        }
        if !matches!(
            &command,
            Command::Resolved { .. }
                | Command::SourceAttempt { .. }
                | Command::CollectionReady { .. }
                | Command::WebFeedback(_)
                | Command::WebReady(_)
        ) {
            self.failed_keys.clear();
        }

        #[cfg(target_os = "android")]
        let command = match command {
            Command::Media { command, reply } => {
                let result = self.command(*command);
                let _ = (reply.send(result.clone().map(|_| {
                    crate::platform::android::playback::Snapshot::from_player(&self.state)
                })))
                .inspect_err(|_| {
                    crate::infrastructure::diagnostics::warn(
                        "media.reply.send",
                        "媒体会话响应接收端已关闭",
                    );
                });
                return result;
            }
            command => command,
        };
        if matches!(&command, Command::Toggle) {
            return self.command(if self.desired_playing {
                Command::Pause
            } else {
                Command::Resume
            });
        }
        match &command {
            Command::Queue(queue, index)
                if queue.is_empty() || queue.len() > 10000 || *index >= queue.len() =>
            {
                return Err("播放队列无效，最多支持 10000 首歌曲".into());
            }
            Command::Jump(index) if *index >= self.state.queue.len() => {
                return Err("歌曲不在播放队列中".into());
            }
            Command::Next if self.state.following(true).is_none() => return Ok(()),
            _ => {}
        }
        if matches!(
            &command,
            Command::PlayNext(_)
                | Command::PlayNextNow(_)
                | Command::Remove(_, _)
                | Command::Repeat
                | Command::Shuffle
                | Command::Seek(_)
                | Command::AppendFm { .. }
                | Command::SelectSource { .. }
        ) {
            if let Some(job) = self.next_job.take() {
                job.abort();
            }
            self.prepared.lock().map_err(|_| "预加载状态不可用")?.take();
            self.web_audio.cancel_preload(&self.app)?;
            self.next_attempt = None;
            self.next_attempts = 0;
        }
        self.command_now(command)
    }

    fn command_now(&mut self, command: Command) -> AppResult<()> {
        if matches!(
            &command,
            Command::PlayNext(_)
                | Command::PlayNextNow(_)
                | Command::Remove(_, _)
                | Command::Pause
                | Command::Next
                | Command::Previous
        ) {
            self.cancel_collection();
        }
        let play_now = matches!(&command, Command::PlayNextNow(_));
        match command {
            Command::WebReady(reply) => {
                self.web_audio.intent(
                    &self.app,
                    self.desired_playing,
                    self.state.volume,
                    Some(self.state.position_ms),
                )?;

                let _ = reply.send(Ok(self.web_audio.control()));
            }
            Command::WebFeedback(feedback) => self.web_feedback(feedback)?,
            Command::Retry { revision, key } => {
                if let Some((index, position)) = retry_position(&self.state, revision, &key) {
                    let play_instance_id = self.play_instance_id.clone();
                    let recorded_instance_id = self.recorded_instance_id.clone();
                    self.load_at(index, true, position)?;
                    self.play_instance_id = play_instance_id;
                    self.recorded_instance_id = recorded_instance_id;
                }
            }
            #[cfg(target_os = "android")]
            Command::Media { .. } => unreachable!("Media commands are unwrapped before dispatch"),
            Command::Collection {
                entity,
                track,
                cached,
                scope,
                fallback_cover,
                reply,
            } => {
                if self
                    .music
                    .adapters
                    .cache_identity(&entity.source)
                    .map_err(|e| e.to_string())?
                    != scope
                {
                    let _ = reply.send(Ok(false));
                    return Ok(());
                }
                let known = track.as_ref().map(|track| track.key.clone());
                let (mut initial, ready, cached_more) = cached
                    .map(|cached| (cached.tracks, cached.ready, cached.more))
                    .unwrap_or_default();
                if known
                    .as_ref()
                    .is_some_and(|key| !initial.iter().any(|track| &track.key == key))
                {
                    initial.clear();
                }
                let cached_ready = ready && !initial.is_empty();
                if initial.is_empty() {
                    if let Some(track) = track {
                        initial.push(*track);
                    }
                }
                let index = known
                    .as_ref()
                    .and_then(|key| initial.iter().position(|track| &track.key == key))
                    .unwrap_or(0);
                let key = initial.get(index).map(|track| track.key.clone());
                let prefix = initial
                    .iter()
                    .map(|track| track.key.clone())
                    .collect::<Vec<_>>();
                if !initial.is_empty() {
                    self.command_now(Command::Queue(initial, index))?;
                    if cached_ready {
                        let _ = reply.send(Ok(cached_more));
                        return Ok(());
                    }
                } else {
                    self.command_now(Command::Clear)?;
                    self.state.status = PlaybackStatus::Loading;
                    self.desired_playing = true;
                    self.publish();
                }
                let first = key.is_none();
                let serial = self.collection_request.begin(key);
                let music = self.music.clone();
                let store = self.store.clone();
                let sender = self.sender.clone();
                self.collection_job = Some(tauri::async_runtime::spawn(async move {
                    let result = async {
                        let mut page = if first {
                            music.collection_first(&entity).await?
                        } else {
                            music.collection_queue(&entity).await?
                        };
                        for track in &mut page.tracks {
                            if track.cover.is_empty() {
                                track.cover = fallback_cover.clone().into();
                            }
                        }
                        // Metadata persistence must not block the audio actor.
                        let tracks = page.tracks.clone();
                        if entity.kind != crate::music::identity::EntityKind::Playlist
                            || music.collections.is_none()
                        {
                            tauri::async_runtime::spawn_blocking(move || {
                                store.save_tracks(&tracks)
                            })
                            .await
                            .map_err(|error| error.to_string())??;
                        }
                        Ok((page.tracks, page.more))
                    }
                    .await;
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        let _ = (sender.send(Command::CollectionReady {
                            serial,
                            scope,
                            prefix,
                            entity,
                            fallback_cover,
                            result,
                            reply,
                        }))
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("sender.send", error);
                        });
                    })
                    .await
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("collection.deliver.join", error);
                    });
                }));
            }
            Command::CollectionReady {
                serial,
                scope,
                prefix,
                entity,
                fallback_cover,
                result,
                reply,
            } => {
                let Some(key) = self.collection_request.take(serial) else {
                    let _ = (reply.send(Ok(false))).inspect_err(|_| {
                        crate::infrastructure::diagnostics::warn(
                            "collection.reply",
                            "reply receiver dropped",
                        );
                    });
                    return Ok(());
                };
                self.collection_job = None;
                if self
                    .music
                    .adapters
                    .cache_identity(&entity.source)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "playback.collection.context",
                            error,
                        );
                    })
                    .ok()
                    .as_ref()
                    != Some(&scope)
                {
                    if self.state.current().is_none() {
                        self.desired_playing = false;
                        self.state.status = PlaybackStatus::Stopped;
                        self.publish();
                    }
                    let _ = (reply.send(Ok(false))).inspect_err(|_| {
                        crate::infrastructure::diagnostics::warn(
                            "collection.reply",
                            "reply receiver dropped",
                        );
                    });
                    return Ok(());
                }
                if key.is_none() {
                    if let Ok((tracks, true)) = &result {
                        if let Some(track) = tracks.first() {
                            // Start the first song now; a new actor-owned request hydrates
                            // the full collection without restarting this song.
                            return self.command_now(Command::Collection {
                                entity,
                                track: Some(Box::new(track.clone())),
                                cached: None,
                                scope: scope.clone(),
                                fallback_cover,
                                reply,
                            });
                        }
                    }
                }
                let result = result.and_then(|(tracks, more)| {
                    if tracks.is_empty() {
                        return Err("这个收藏还没有可播放的歌曲".into());
                    }
                    if let Some(key) = key {
                        if entity.kind == crate::music::identity::EntityKind::Playlist
                            && self.music.collections.is_some()
                        {
                            super::collection::hydrate_prefix(
                                &mut self.state,
                                &key,
                                &prefix,
                                tracks,
                            )?;
                        } else {
                            super::collection::hydrate(&mut self.state, &key, tracks)?;
                        }
                        self.publish();
                    } else {
                        self.command_now(Command::Queue(tracks, 0))?;
                    }
                    Ok(more)
                });
                if let Err(error) = &result {
                    crate::infrastructure::diagnostics::warn("collection.prepare", error);
                    // A known song remains playable even if its collection cannot be expanded.
                    self.state.error = Some(format!("歌单准备失败：{error}"));
                    if self.state.current().is_none() {
                        self.state.status = PlaybackStatus::Error;
                        self.desired_playing = false;
                    }
                    self.publish();
                }
                let _ = (reply.send(result)).inspect_err(|_| {
                    crate::infrastructure::diagnostics::warn(
                        "collection.reply",
                        "reply receiver dropped",
                    );
                });
            }
            Command::Queue(queue, index) => {
                if queue.is_empty() || queue.len() > 10000 || index >= queue.len() {
                    return Err("播放队列无效，最多支持 10000 首歌曲".into());
                }
                self.state.private_fm_session = None;
                self.selections.clear();
                self.state.queue = queue.into();
                self.state.index = Some(index);
                self.state.deduplicate_queue();
                let index = self.state.index.unwrap_or(0);
                self.state.reset_shuffle_order(Some(index));
                self.load(index, true)?;
            }
            Command::FmQueue(queue) => {
                self.command_now(Command::Queue(queue, 0))?;
                self.state.shuffle = false;
                self.state.shuffle_order = Arc::default();
                self.state.repeat_mode = crate::model::RepeatMode::Off;
                self.state.private_fm_session = Some(self.state.revision);
                self.publish();
            }
            Command::AppendFm {
                session,
                queue_len,
                tracks,
            } => {
                let exhausted = self.state.status == PlaybackStatus::Stopped
                    && self.state.duration_ms > 0
                    && self.state.position_ms >= self.state.duration_ms
                    && self.state.index == self.state.queue.len().checked_sub(1);
                // Keep any prepared successor aligned with the trimmed queue.
                let (trim, displaced) = {
                    let mut prepared = self
                        .prepared
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "playback.fm.prepared_lock",
                                error,
                            );
                        })
                        .map_err(|_| "预加载状态不可用")?;
                    let Some(trim) = self.state.append_private_fm(session, queue_len, tracks)?
                    else {
                        return Ok(());
                    };
                    let mut displaced = false;
                    if trim > 0 {
                        for slot in [&mut *prepared] {
                            if let Some(next) = slot {
                                if let Some(index) = next.index.checked_sub(trim) {
                                    next.index = index;
                                } else {
                                    *slot = None;
                                    displaced = true;
                                }
                            }
                        }
                    }
                    (trim, displaced)
                };
                if trim > 0 {
                    let retained: std::collections::HashSet<_> = self
                        .state
                        .queue
                        .iter()
                        .map(|track| track.key.as_str())
                        .collect();
                    self.selections
                        .retain(|key, _| retained.contains(key.as_str()));
                    if let Some(job) = self.next_job.take() {
                        job.abort();
                    }
                    self.next_attempt = None;
                    self.next_attempts = 0;
                    if displaced || self.state.status == PlaybackStatus::Loading {
                        self.load(self.state.index.unwrap_or(0), self.desired_playing)?;
                    }
                }
                if exhausted {
                    if let Some(index) = self.state.index {
                        self.load(index + 1, true)?;
                    }
                }
                self.publish();
            }
            Command::PlayNext(tracks) | Command::PlayNextNow(tracks) => {
                if !play_now
                    && tracks.iter().all(|t| {
                        self.state
                            .current()
                            .is_some_and(|current| current.key == t.key)
                    })
                {
                    return Ok(());
                }
                let position = self.state.insert_next(tracks)?;
                if play_now {
                    self.load(position, true)?;
                    return Ok(());
                }
                if let Some(job) = self.next_job.take() {
                    job.abort();
                }
                self.prepared.lock().map_err(|_| "预加载状态不可用")?.take();
                self.next_attempt = None;
                self.next_attempts = 0;
                self.publish();
            }
            Command::Remove(index, key) => {
                let removed_current = self.state.remove_track(index, &key)?;
                self.selections.remove(&key);
                let loading = self.state.status == PlaybackStatus::Loading;
                self.cancel_jobs();
                if self.state.queue.is_empty() {
                    self.command(Command::Clear)?;
                } else if let Some(current) = self.state.index {
                    if removed_current || loading {
                        self.load(current, self.desired_playing)?;
                    } else {
                        self.publish();
                    }
                } else {
                    self.publish();
                }
            }
            Command::SelectSource {
                revision,
                selection,
                reply,
            } => {
                let result = (|| {
                    if revision != self.state.revision {
                        return Err("播放歌曲已变化，请重新选择音源".into());
                    }
                    let index = self.state.index.ok_or("没有正在播放的歌曲")?;
                    let current = self.state.current().ok_or("没有正在播放的歌曲")?;
                    if matches!(current.source, crate::model::TrackSource::Local { .. }) {
                        return Err("本地歌曲不切换在线音源".into());
                    }
                    self.selections.insert(current.key.clone(), selection);
                    self.load_at(
                        index,
                        self.desired_playing || self.state.status == PlaybackStatus::Error,
                        self.state.position_ms,
                    )
                })();
                let _ = (reply.send(result.clone())).inspect_err(|_| {
                    crate::infrastructure::diagnostics::warn(
                        "source.reply",
                        "reply receiver dropped",
                    );
                });
                result?;
            }
            Command::SourceAttempt {
                generation,
                index,
                key,
                source,
            } => {
                if generation == self.generation.load(Ordering::Acquire)
                    && self.state.index == Some(index)
                    && self.state.current().is_some_and(|track| track.key == key)
                {
                    self.state.playback_source = Some(source);
                    self.publish();
                }
            }
            Command::Jump(index) => self.load(index, true)?,
            Command::Repeat | Command::Shuffle => {
                self.prepared.lock().map_err(|_| "预加载状态不可用")?.take();
                if matches!(command, Command::Shuffle) {
                    self.state.toggle_shuffle(self.state.index);
                } else {
                    self.state.cycle_repeat();
                }
                if let Some(job) = self.next_job.take() {
                    job.abort();
                }
                self.next_attempt = None;
                self.next_attempts = 0;
                self.publish();
            }
            Command::Next => {
                if let Some(index) = self.state.following(true) {
                    self.load(index, true)?;
                }
            }
            Command::Previous => {
                if let Some(command) = previous_command(&self.state) {
                    self.command_now(command)?;
                }
            }
            Command::Pause => {
                self.desired_playing = false;

                self.web_audio
                    .intent(&self.app, false, self.state.volume, None)?;

                self.state.status = PlaybackStatus::Paused;
                self.publish();
            }
            Command::Resume => {
                if self.state.current().is_some() {
                    if matches!(
                        self.state.status,
                        PlaybackStatus::Stopped | PlaybackStatus::Error
                    ) {
                        let position = if self.state.duration_ms > 0
                            && self.state.position_ms >= self.state.duration_ms
                        {
                            0
                        } else {
                            self.state.position_ms
                        };
                        self.load_at(self.state.index.unwrap_or(0), true, position)?;
                    } else {
                        self.desired_playing = true;

                        self.web_audio
                            .intent(&self.app, true, self.state.volume, None)?;
                        return Ok(());
                    }
                }
            }
            Command::Stop | Command::Clear => {
                let clear = matches!(command, Command::Clear);
                self.cancel_jobs();
                self.desired_playing = false;

                self.web_audio.stop(&self.app)?;

                if clear {
                    self.selections.clear();
                    self.state.private_fm_session = None;
                    self.state.queue = Arc::default();
                    self.state.shuffle_order = Arc::default();
                    self.state.index = None;
                    self.state.duration_ms = 0;
                    self.state.actual_quality = None;
                    self.state.playback_source = None;
                    self.state.media_error = None;
                    self.state.revision += 1;
                }
                self.state.status = PlaybackStatus::Stopped;
                self.state.position_ms = 0;
                self.state.error = None;
                self.publish();
            }
            Command::Seek(ms) => self.seek(ms)?,
            Command::Volume(value) => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err("音量必须介于 0 和 1".into());
                }
                let value = if cfg!(target_os = "android") {
                    1.0
                } else {
                    value
                };

                self.state.volume = value;

                self.web_audio
                    .intent(&self.app, self.desired_playing, value, None)?;

                self.publish();
            }
            Command::Device(id, reply) => {
                let result = self.select_device(id);
                let _ = reply.send(result.clone());
                result?;
            }
            Command::Options(options, reply) => {
                let result = self.apply_options(options);
                let _ = (reply.send(result.clone())).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("reply.send", error);
                });
                result?;
            }
            Command::Resolved {
                generation,
                index,
                key,
                result,
                next,
            } => {
                if generation != self.generation.load(Ordering::Acquire) {
                    return Ok(());
                }
                if next {
                    if !self.state.playback_options.crossfade
                        || self.state.following(false) != Some(index)
                        || self
                            .state
                            .queue
                            .get(index)
                            .is_none_or(|track| track.key != key)
                    {
                        return Ok(());
                    }
                    self.next_job = None;
                    if let Ok((resolved, resource, source)) = *result {
                        if resource
                            .as_ref()
                            .is_some_and(|r| r.check(generation).is_err())
                        {
                            return Ok(());
                        }

                        self.web_audio.prepare(&self.app, resolved.clone())?;

                        *self
                            .prepared
                            .lock()
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn(
                                    "playback.preload.publish_lock",
                                    error,
                                );
                            })
                            .map_err(|_| "预加载状态不可用")? = Some(Prepared {
                            generation,
                            index,
                            resolved,
                            resource,
                            source,
                        });
                    }
                } else {
                    self.current_job = None;
                    let result = (*result).and_then(|(resolved, resource, source)| {
                        if let Some(resource) = &resource {
                            resource.check(generation).map_err(|e| e.to_string())?;
                        }
                        Ok((resolved, resource, source))
                    });
                    match result {
                        Ok((resolved, resource, source)) => {
                            let _resource = resource;
                            self.state.actual_quality = resolved.quality.clone();
                            self.state.playback_source = source;
                            self.publish();

                            self.web_audio.load(
                                &self.app,
                                resolved,
                                self.state.position_ms,
                                self.desired_playing,
                                self.state.volume,
                            )?;
                            return Ok(());
                        }
                        Err(error) => {
                            self.skip_failed(error)?;
                        }
                    }
                }
            }
            Command::Toggle => unreachable!("toggle is resolved before command dispatch"),
            Command::Shutdown => {}
        }
        Ok(())
    }

    fn apply_options(&mut self, options: PlaybackOptions) -> AppResult<()> {
        if !(1000..=12000).contains(&options.crossfade_duration_ms) {
            return Err("交叉淡入淡出时长须在 1–12 秒之间".into());
        }
        self.store.set_setting(
            "playbackOptions",
            &serde_json::to_string(&options).map_err(|e| e.to_string())?,
        )?;
        self.state.playback_options = options;
        self.web_audio.configure(&self.state);
        if !options.crossfade {
            if let Some(job) = self.next_job.take() {
                job.abort();
            }
            self.prepared.lock().map_err(|_| "预加载状态不可用")?.take();
            self.next_attempt = None;
            self.next_attempts = 0;
            self.web_audio.cancel_preload(&self.app)?;
        } else {
            self.web_audio.emit(&self.app)?;
        }
        self.publish();
        Ok(())
    }

    fn skip_failed(&mut self, error: String) -> AppResult<()> {
        if let Some(track) = self.state.current() {
            self.failed_keys.insert(track.key.clone());
            let _ = self.app.emit(
                "playback-notice",
                format!("已跳过「{}」：{}", track.title, error),
            );
        }
        if self.desired_playing && self.state.private_fm_session.is_none() {
            if let Some(index) = next_available(&self.state, &self.failed_keys) {
                self.load(index, true)?;
                return Ok(());
            }
        }
        self.desired_playing = false;
        self.state.status = PlaybackStatus::Stopped;
        self.state.error = Some(format!("没有后续可播放的歌曲：{error}"));
        self.publish();
        Ok(())
    }
    fn load(&mut self, index: usize, playing: bool) -> AppResult<()> {
        self.load_at(index, playing, 0)
    }

    fn load_at(&mut self, index: usize, playing: bool, position: u64) -> AppResult<()> {
        let track = self
            .state
            .queue
            .get(index)
            .cloned()
            .ok_or("歌曲不在播放队列中")?;
        self.cancel_jobs();

        self.web_audio.stop(&self.app)?;

        self.state.revision += 1;
        self.play_instance_id = uuid::Uuid::new_v4().to_string();
        self.recorded_instance_id = None;
        self.state.index = Some(index);
        self.state.position_ms = if track.duration_ms > 0 {
            position.min(track.duration_ms)
        } else {
            position
        };
        self.state.duration_ms = track.duration_ms;
        self.state.actual_quality = None;
        self.state.playback_source = None;
        self.state.error = None;
        self.state.status = PlaybackStatus::Loading;
        self.desired_playing = playing;
        self.resolve(index, track, false);
        self.publish();
        Ok(())
    }

    fn resolve(&mut self, index: usize, track: Track, next: bool) {
        let generation = self.generation.load(Ordering::Acquire);
        let music = self.music.clone();
        let selection = self.selections.get(&track.key).cloned();
        let attempt = if next { self.next_attempts } else { 1 };
        let sender = self.sender.clone();
        let quality = self
            .store
            .setting("quality")
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("playback.resolve.quality_setting", error);
            })
            .ok()
            .flatten()
            .unwrap_or_else(|| "exhigh".into());
        let downgrade = self
            .store
            .setting("allowDowngrade")
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "playback.resolve.downgrade_setting",
                    error,
                );
            })
            .ok()
            .flatten()
            .as_deref()
            != Some("false");
        let task = tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let source_id = crate::infrastructure::diagnostics::field(track.source_id());
            let track_id = match &track.source {
                TrackSource::Online { track } => {
                    crate::infrastructure::diagnostics::field(track.reference.id.as_str())
                }
                TrackSource::Local { .. } => "local".into(),
            };
            let provider = crate::infrastructure::diagnostics::field(
                selection
                    .as_ref()
                    .and_then(|s| s.provider.as_deref())
                    .unwrap_or("automatic"),
            );
            log::info!(target: "nons::playback", "operation=playback_resolve stage=start generation={generation} index={index} next={next} attempt={attempt} source={source_id} provider={provider} track_id={track_id} quality={} allow_downgrade={downgrade}", crate::infrastructure::diagnostics::field(&quality));
            let key = track.key.clone();
            let progress_sender = sender.clone();
            let progress_key = key.clone();
            let notify = move |source| {
                if !next {
                    let _ = (progress_sender.try_send(Command::SourceAttempt {
                        generation,
                        index,
                        key: progress_key.clone(),
                        source,
                    }))
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("progress_sender.try_send", error);
                    });
                }
            };
            let result = music
                .resolve_selected_notifying(
                    track, &quality, downgrade, generation, selection, &notify,
                )
                .await;
            match &result {
                Ok(_) => {
                    log::info!(target: "nons::playback", "operation=playback_resolve stage=complete generation={generation} index={index} next={next} attempt={attempt} elapsed_ms={} source={source_id} provider={provider} track_id={track_id} code=ok", started.elapsed().as_millis())
                }
                Err(error) => {
                    log::warn!(target: "nons::playback", "operation=playback_resolve stage=failed generation={generation} index={index} next={next} attempt={attempt} elapsed_ms={} source={source_id} provider={provider} track_id={track_id} reason={error}", started.elapsed().as_millis())
                }
            }
            // Bounded channel delivery is performed off the audio thread.
            let _ = tauri::async_runtime::spawn_blocking(move || {
                let _ = (sender.send(Command::Resolved {
                    generation,
                    index,
                    key,
                    result: Box::new(result),
                    next,
                }))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("sender.send", error);
                });
            })
            .await
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("playback.resolve.deliver.join", error);
            });
        });
        if next {
            self.next_job = Some(task);
        } else {
            self.current_job = Some(task);
        }
    }

    fn cancel_jobs(&mut self) {
        self.cancel_collection();
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Some(job) = self.current_job.take() {
            job.abort();
        }
        if let Some(job) = self.next_job.take() {
            job.abort();
        }
        if let Ok(mut slot) = (self.prepared.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("playback.cancel.prepared_lock", error);
        }) {
            *slot = None;
        }
        self.next_attempt = None;
        self.next_attempts = 0;
    }

    fn cancel_collection(&mut self) {
        self.collection_request.cancel();
        if let Some(job) = self.collection_job.take() {
            job.abort();
        }
    }

    fn seek(&mut self, ms: u64) -> AppResult<()> {
        if self.state.current().is_none() || self.state.duration_ms == 0 {
            return Err("当前歌曲尚不可定位".into());
        }
        let ms = ms.min(self.state.duration_ms);
        self.state.position_ms = ms;
        if !matches!(
            self.state.status,
            PlaybackStatus::Stopped | PlaybackStatus::Error
        ) && self.current_job.is_none()
        {
            self.web_audio
                .intent(&self.app, self.desired_playing, self.state.volume, Some(ms))?;
        }
        self.publish();
        Ok(())
    }

    fn record_playback_history(&mut self) {
        if self.recorded_instance_id.as_deref() == Some(self.play_instance_id.as_str()) {
            return;
        }
        if let Some(track) = self.state.current().cloned() {
            self.history.record(self.play_instance_id.clone(), track);
            self.recorded_instance_id = Some(self.play_instance_id.clone());
        }
    }

    fn tick(&mut self) {
        if self.state.playback_options.crossfade
            && self.desired_playing
            && self.state.status == PlaybackStatus::Playing
            && !self.collection_request.is_pending()
            && self.state.duration_ms > 0
            && self
                .state
                .duration_ms
                .saturating_sub(self.state.position_ms)
                <= 30_000
            && self.next_job.is_none()
            && self.next_attempts < 3
            && self
                .next_attempt
                .is_none_or(|t| t.elapsed() > Duration::from_secs(5))
            && self.web_audio.ready_to_prepare()
            && self.prepared.lock().map(|p| p.is_none()).unwrap_or(false)
        {
            if let Some(index) = self.state.following(false) {
                self.next_attempt = Some(Instant::now());
                self.next_attempts += 1;
                self.resolve(index, self.state.queue[index].clone(), true);
            }
        }
        self.update_media();
        if let Ok(mut shared) = self.shared.write() {
            let _ = self.app.emit("player-progress", shared.tick(&self.state));
        }
    }

    fn publish(&mut self) {
        self.update_media();
        if let Ok(mut shared) = (self.shared.write()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("playback.publish.state_lock", error);
        }) {
            let (state, queue, progress) = shared.update(&self.state);
            if let Some(queue) = queue {
                let _ = (self.app.emit("player-queue", queue)).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("playback.queue.emit", error);
                });
            }
            let _ = (self.app.emit("player-state", state)).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("playback.state.emit", error);
            });
            if let Some(progress) = progress {
                let _ = (self.app.emit("player-progress", progress)).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("playback.progress.emit", error);
                });
            }
        }
    }

    fn update_media(&mut self) {
        if let Some(media) = &mut self.media {
            match media.update(&self.state) {
                Err(error) => {
                    let message = format!("系统媒体信息同步失败：{error}");
                    if self.state.media_error.as_ref() != Some(&message) {
                        log::error!(
                            target: "nons::media",
                            "operation=update revision={} status={:?} track_key={} reason={error}",
                            self.state.revision,
                            self.state.status,
                            self.state.current().map_or("-", |track| track.key.as_str()),
                        );
                    }
                    self.state.media_error = Some(message);
                }
                Ok(()) => {
                    if self.state.media_error.take().is_some() {
                        log::info!(target: "nons::media", "operation=update outcome=recovered");
                    }
                }
            }
        }
    }

    fn select_device(&mut self, id: Option<String>) -> AppResult<()> {
        self.store.set_setting(
            "webAudioDevice",
            &serde_json::to_string(&id).map_err(|e| e.to_string())?,
        )?;
        self.state.device_id = id;
        self.web_audio.configure(&self.state);
        self.web_audio.emit(&self.app)?;
        self.publish();
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_is_bound_to_failed_revision_and_keeps_the_current_position() {
        let mut state = snapshot(0, 850, RepeatMode::Off);
        state.status = PlaybackStatus::Error;
        state.revision = 7;
        let key = state.queue[0].key.clone();
        assert_eq!(retry_position(&state, 7, &key), Some((0, 850)));
        assert_eq!(retry_position(&state, 6, &key), None);
        assert_eq!(retry_position(&state, 7, "another-song"), None);
        state.status = PlaybackStatus::Loading;
        assert_eq!(retry_position(&state, 7, &key), None);
        state.status = PlaybackStatus::Stopped;
        state.index = None;
        assert_eq!(retry_position(&state, 7, &key), None);
    }

    #[test]
    fn unavailable_tracks_stop_boundedly_with_repeat_and_shuffle() {
        let mut state = snapshot(0, 0, RepeatMode::One);
        let mut failed = std::collections::HashSet::new();
        failed.insert(state.queue[0].key.clone());
        assert_eq!(next_available(&state, &failed), Some(1));
        failed.extend(state.queue.iter().map(|t| t.key.clone()));
        assert_eq!(next_available(&state, &failed), None);
        state.shuffle = true;
        state.shuffle_order = vec![2, 0, 1].into();
        assert_eq!(next_available(&state, &failed), None);
        failed.remove(&state.queue[2].key);
        assert_eq!(next_available(&state, &failed), Some(2));
        state.shuffle = false;
        state.index = Some(2);
        state.repeat_mode = RepeatMode::Off;
        assert_eq!(next_available(&state, &failed), None);
    }

    #[test]
    fn command_backpressure_and_initialization_failure_have_distinct_errors() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let directory = tempfile::tempdir().unwrap();
        let player = Player {
            sender,
            snapshot: Arc::new(RwLock::new(Publication::new(PlayerSnapshot::default()))),
            thread: Mutex::new(None),
            history: PlaybackHistory::start(Arc::new(
                Store::open(&directory.path().join("player.sqlite3")).unwrap(),
            )),
        };
        player.send(Command::Pause).unwrap();
        assert_eq!(
            player.send(Command::Pause).unwrap_err(),
            "播放核心忙碌，请稍后重试"
        );
        drop(receiver);
        assert_eq!(
            player.send(Command::Pause).unwrap_err(),
            "播放核心不可用，请重启应用"
        );
        player.snapshot.write().unwrap().snapshot.error =
            Some("无法监视音频设备：Failed to start".into());
        assert_eq!(
            player.send(Command::Pause).unwrap_err(),
            "无法监视音频设备：Failed to start"
        );
        player.shutdown();
    }

    #[test]
    fn playback_options_restore_defaults_and_persist_both_switches_independently() {
        let directory = crate::test_support::TestDir::new();
        let path = directory.0.join("options.sqlite3");
        let store = Store::open(&path).unwrap();
        assert_eq!(
            stored_options(&store, PlaybackOptions::default()).unwrap(),
            PlaybackOptions {
                fades: false,
                crossfade: false,
                ..PlaybackOptions::default()
            }
        );
        assert_eq!(
            serde_json::from_str::<PlaybackOptions>("{}").unwrap(),
            PlaybackOptions::default()
        );
        for fades in [false, true] {
            for crossfade in [false, true] {
                let options = PlaybackOptions {
                    fades,
                    crossfade,
                    ..PlaybackOptions::default()
                };
                store
                    .set_setting("playbackOptions", &serde_json::to_string(&options).unwrap())
                    .unwrap();
                let reopened = Store::open(&path).unwrap();
                assert_eq!(
                    stored_options(&reopened, PlaybackOptions::default()).unwrap(),
                    options
                );
            }
        }
    }

    fn snapshot(index: usize, position_ms: u64, mode: RepeatMode) -> PlayerSnapshot {
        PlayerSnapshot {
            queue: (0..3)
                .map(|id| {
                    Track::from_music(nons_music_adapter_sdk::identity::MusicTrack {
                        title: id.to_string(),
                        aliases: vec![],
                        artist: String::new(),
                        artists: vec![],
                        album_reference: None,
                        album: String::new(),
                        duration_ms: 180_000,
                        cover: Default::default(),
                        reference: nons_music_adapter_sdk::identity::EntityRef {
                            source: "netease".to_string().try_into().unwrap(),
                            kind: nons_music_adapter_sdk::identity::EntityKind::Track,
                            id: (id).to_string().try_into().unwrap(),
                        },
                        associations: vec![],
                        lyric_identifiers: vec![],
                        unavailable_reason: None,
                    })
                    .unwrap()
                })
                .collect::<Vec<_>>()
                .into(),
            index: Some(index),
            position_ms,
            duration_ms: 180_000,
            repeat_mode: mode,
            status: PlaybackStatus::Playing,
            ..PlayerSnapshot::default()
        }
    }

    #[test]
    fn saved_progress_survives_reopening_the_player_store() {
        let directory = crate::test_support::TestDir::new();
        let path = directory.0.join("player.sqlite3");
        let mut saved = snapshot(1, 42_000, RepeatMode::Off);
        saved.error = Some("old playback error".into());
        saved.media_error = Some("old media error".into());
        saved.actual_quality = Some("lossless".into());
        saved.revision = 7;
        let store = Store::open(&path).unwrap();
        store.save_player(&saved).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        let restored = restore_player(
            serde_json::from_str(&store.setting("player").unwrap().unwrap()).unwrap(),
        );
        assert_eq!(restored.index, Some(1));
        assert_eq!(restored.position_ms, 42_000);
        assert_eq!(restored.duration_ms, 180_000);
        assert_eq!(restored.status, PlaybackStatus::Stopped);
        assert_eq!(restored.revision, 0);
        assert!(restored.error.is_none());
        assert!(restored.media_error.is_none());
        assert!(restored.actual_quality.is_none());
    }

    #[test]
    fn restored_progress_is_bounded_and_invalid_selection_is_cleared() {
        let restored = restore_player(snapshot(1, 200_000, RepeatMode::Off));
        assert_eq!(restored.position_ms, 180_000);
        let restored = restore_player(snapshot(3, 42_000, RepeatMode::Off));
        assert_eq!(restored.index, None);
        assert_eq!(restored.position_ms, 0);
        assert_eq!(restored.duration_ms, 0);
        let mut saved = snapshot(1, 42_000, RepeatMode::Off);
        Arc::make_mut(&mut saved.queue)[1].duration_ms = 0;
        let restored = restore_player(saved);
        assert_eq!(restored.duration_ms, 180_000);
        assert_eq!(restored.position_ms, 42_000);
    }

    #[test]
    fn fm_edits_end_session_and_late_batches_are_rejected() {
        for insert in [false, true] {
            let mut state = snapshot(1, 0, RepeatMode::Off);
            state.private_fm_session = Some(7);
            let new_track = state.queue[0].clone();
            if insert {
                state.insert_next(vec![new_track.clone()]).unwrap();
            } else {
                state.remove_track(0, &new_track.key).unwrap();
            }
            assert_eq!(state.private_fm_session, None);
            let original_len = state.queue.len();
            assert_eq!(
                state
                    .append_private_fm(7, original_len, vec![new_track])
                    .unwrap(),
                None
            );
            assert_eq!(state.queue.len(), original_len);
        }
    }

    #[test]
    fn fm_append_preserves_playback_and_rejects_old_sessions() {
        let mut state = snapshot(1, 42_000, RepeatMode::Off);
        state.private_fm_session = Some(7);
        let mut track = state.queue[0].clone();
        track.key = "fresh".into();
        assert_eq!(
            state.append_private_fm(6, 3, vec![track.clone()]).unwrap(),
            None
        );
        assert_eq!(
            state.append_private_fm(7, 2, vec![track.clone()]).unwrap(),
            None
        );
        assert_eq!(
            state
                .append_private_fm(7, 3, vec![track.clone(), track])
                .unwrap(),
            Some(0)
        );
        assert_eq!(state.queue.len(), 4);
        assert_eq!(state.index, Some(1));
        assert_eq!(state.position_ms, 42_000);
        assert_eq!(state.private_fm_session, Some(7));
    }

    #[test]
    fn fm_keeps_a_bounded_queue_across_repeated_refills() {
        let mut state = snapshot(1, 42_000, RepeatMode::Off);
        state.private_fm_session = Some(7);
        for id in 3..2003 {
            state.index = Some(state.queue.len() - 2);
            let current_key = state.current().unwrap().key.clone();
            let mut track = state.queue[0].clone();
            track.key = id.to_string();
            let len = state.queue.len();
            state.append_private_fm(7, len, vec![track]).unwrap();
            assert!(state.queue.len() <= 1000);
            assert_eq!(state.current().unwrap().key, current_key);
            assert_eq!(state.position_ms, 42_000);
        }
    }

    #[test]
    fn previous_near_end_switches_track_instead_of_restarting_current() {
        let state = snapshot(1, 179_000, RepeatMode::Off);
        assert!(
            matches!(previous_command(&state), Some(Command::Jump(0))),
            "上一首必须选择队列中的前一首，不能 seek(0) 重播当前歌曲"
        );
    }

    #[test]
    fn previous_selection_is_independent_of_position_status_and_repeat() {
        for mode in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
            for position_ms in [0, 3000, 3001, 90_000, 179_000, 180_000] {
                for status in [
                    PlaybackStatus::Loading,
                    PlaybackStatus::Playing,
                    PlaybackStatus::Paused,
                    PlaybackStatus::Buffering,
                ] {
                    let mut state = snapshot(2, position_ms, mode);
                    state.status = status;
                    assert!(matches!(previous_command(&state), Some(Command::Jump(1))));
                }
            }
        }
    }

    #[test]
    fn previous_wraps_only_when_repeating_and_requires_current_track() {
        assert!(previous_command(&snapshot(0, 179_000, RepeatMode::Off)).is_none());
        for mode in [RepeatMode::All, RepeatMode::One] {
            assert!(matches!(
                previous_command(&snapshot(0, 179_000, mode)),
                Some(Command::Jump(2))
            ));
        }
        assert!(previous_command(&PlayerSnapshot::default()).is_none());
        let mut state = snapshot(0, 179_000, RepeatMode::All);
        state.queue = Arc::default();
        assert!(previous_command(&state).is_none());
        state = snapshot(3, 179_000, RepeatMode::All);
        assert!(previous_command(&state).is_none());
    }
}
