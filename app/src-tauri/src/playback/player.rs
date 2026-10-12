#[cfg(test)]
use crate::model::RepeatMode;
use crate::{
    media::MediaControls,
    model::{
        AppResult, AudioOutputMode, OutputDevice, PlaybackOptions, PlaybackStatus, PlayerSnapshot,
        Progress, ResolvedTrack, Track,
    },
    music::{manager::ResourceTicket, service::MusicService},
    storage::Store,
};
use gstreamer::{self as gst, prelude::*};
use sha2::{Digest, Sha256};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};
use tauri::Emitter;

pub enum Command {
    Collection {
        kind: String,
        id: u64,
        track: Option<Track>,
        fallback_cover: String,
        reply: tokio::sync::oneshot::Sender<AppResult<bool>>,
    },
    CollectionReady {
        serial: u64,
        scope: (crate::music::account::SessionContext, u64),
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
    Jump(usize),
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
    Device(Option<String>),
    OutputMode(AudioOutputMode, mpsc::Sender<AppResult<()>>),
    Options(PlaybackOptions, mpsc::Sender<AppResult<()>>),
    Resolved {
        generation: u64,
        index: usize,
        key: String,
        result: Box<AppResult<crate::music::service::ResolvedPlayback>>,
        next: bool,
    },
    Devices(mpsc::Sender<AppResult<Vec<OutputDevice>>>),
    Shutdown,
}

fn previous_command(state: &PlayerSnapshot) -> Option<Command> {
    state.current()?;
    // Manual navigation always selects a queue entry, regardless of position.
    // Jump goes through load(), invalidating prepared/armed gapless work.
    state.previous().map(Command::Jump)
}

fn restore_player(mut state: PlayerSnapshot) -> PlayerSnapshot {
    state.deduplicate_queue();
    state.status = PlaybackStatus::Stopped;
    state.error = None;
    state.media_error = None;
    state.revision = 0;
    state.actual_quality = None;
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

fn stored_output_mode(store: &Store, fallback: AudioOutputMode) -> AppResult<AudioOutputMode> {
    if !cfg!(windows) {
        return Ok(AudioOutputMode::Shared);
    }
    store
        .setting("audioOutputMode")?
        .map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
        .unwrap_or(Ok(fallback))
}

pub struct Player {
    sender: SyncSender<Command>,
    snapshot: Arc<RwLock<PlayerSnapshot>>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

struct Prepared {
    generation: u64,
    index: usize,
    resolved: ResolvedTrack,
    resource: Option<Arc<ResourceTicket>>,
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
            .and_then(|s| serde_json::from_str::<PlayerSnapshot>(&s).ok())
            .map(restore_player)
            .unwrap_or_default();
        restored.playback_options = stored_options(&store, restored.playback_options)?;
        restored.audio_output_mode = stored_output_mode(&store, restored.audio_output_mode)?;
        let snapshot = Arc::new(RwLock::new(restored));
        let shared = Arc::clone(&snapshot);
        let (sender, receiver) = mpsc::sync_channel(64);
        let actor_sender = sender.clone();
        let thread = std::thread::Builder::new()
            .name("nons-audio".into())
            .spawn(move || {
                let result = Actor::new(
                    app.clone(),
                    store,
                    music,
                    shared.clone(),
                    actor_sender,
                    hwnd,
                );
                match result {
                    Ok(mut actor) => actor.run(receiver),
                    Err(error) => {
                        if let Ok(mut state) = shared.write() {
                            state.status = PlaybackStatus::Error;
                            state.error = Some(error);
                            let _ = app.emit("player-state", state.clone());
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            sender,
            snapshot,
            thread: Mutex::new(Some(thread)),
        })
    }

    pub fn send(&self, command: Command) -> AppResult<()> {
        self.sender
            .try_send(command)
            .map_err(|_| "播放核心忙碌或不可用，请稍后重试".into())
    }

    pub fn snapshot(&self) -> AppResult<PlayerSnapshot> {
        self.snapshot
            .read()
            .map(|s| s.clone())
            .map_err(|_| "播放状态不可用".into())
    }

    pub(crate) fn needs_private_fm_frontend(&self) -> AppResult<bool> {
        self.snapshot
            .read()
            .map(|state| {
                state.private_fm_session.is_some()
                    && matches!(
                        state.status,
                        PlaybackStatus::Loading
                            | PlaybackStatus::Playing
                            | PlaybackStatus::Buffering
                    )
            })
            .map_err(|_| "播放状态不可用".into())
    }

    pub fn devices(&self) -> AppResult<Vec<OutputDevice>> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::Devices(sender))?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "查询音频设备超时".to_string())?
    }

    pub fn set_options(&self, options: PlaybackOptions) -> AppResult<()> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::Options(options, sender))?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "保存播放设置超时".to_string())?
    }

    pub fn set_output_mode(&self, mode: AudioOutputMode) -> AppResult<()> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::OutputMode(mode, sender))?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "切换音频输出模式超时".to_string())?
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(Command::Shutdown);
        if let Ok(mut thread) = self.thread.lock() {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
    }
}

struct Actor {
    app: tauri::AppHandle,
    store: Arc<Store>,
    music: Arc<MusicService>,
    shared: Arc<RwLock<PlayerSnapshot>>,
    sender: SyncSender<Command>,
    state: PlayerSnapshot,
    playbin: gst::Element,
    bus: gst::Bus,
    monitor: gst::DeviceMonitor,
    media: Option<MediaControls>,
    desired_playing: bool,
    generation: Arc<AtomicU64>,
    prepared: Arc<Mutex<Option<Prepared>>>,
    armed: Arc<Mutex<Option<Prepared>>>,
    decoded_audio: Option<Arc<tempfile::NamedTempFile>>,
    current_job: Option<tauri::async_runtime::JoinHandle<()>>,
    collection_job: Option<tauri::async_runtime::JoinHandle<()>>,
    collection_request: super::collection::CollectionRequest,
    next_job: Option<tauri::async_runtime::JoinHandle<()>>,
    next_attempt: Option<Instant>,
    next_attempts: u8,
    last_tick: Instant,
    last_save: Instant,
    pending_seek: Option<u64>,
    awaiting_first_stream: bool,
    gapless_enabled: Arc<AtomicBool>,
    fade: super::transitions::PlaybackFade,
    deferred: Option<(Command, u64, Instant)>,
}

impl Actor {
    fn new(
        app: tauri::AppHandle,
        store: Arc<Store>,
        music: Arc<MusicService>,
        shared: Arc<RwLock<PlayerSnapshot>>,
        sender: SyncSender<Command>,
        hwnd: isize,
    ) -> AppResult<Self> {
        gst::init().map_err(|e| format!("GStreamer 初始化失败：{e}"))?;
        crate::network::prefer_http_source();
        let playbin = gst::ElementFactory::make("playbin3")
            .build()
            .map_err(|_| "GStreamer 缺少 playbin3 插件，请检查运行时")?;
        let bus = playbin.bus().ok_or("音频消息总线不可用")?;
        playbin.set_property("buffer-size", 4_194_304_i32);
        playbin.set_property("buffer-duration", 10_000_000_000_i64);
        playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
        playbin.connect("source-setup", false, |values| {
            if let Ok(source) = values[1].get::<gst::Element>() {
                crate::network::configure_source(&source);
            }
            None
        });
        let monitor = gst::DeviceMonitor::new();
        monitor.add_filter(Some("Audio/Sink"), None);
        monitor
            .start()
            .map_err(|e| format!("无法监视音频设备：{e}"))?;
        let mut state = shared.read().map_err(|_| "播放状态不可用")?.clone();
        if let Some(id) = &state.device_id {
            let device = monitor.devices().into_iter().find(|device| {
                let identity = format!(
                    "{}:{}",
                    device.device_class(),
                    device
                        .properties()
                        .map_or_else(|| device.display_name().to_string(), |s| s.to_string())
                );
                format!("{:x}", Sha256::digest(identity.as_bytes())) == *id
            });
            if let Some(device) = device {
                if let Ok(sink) = super::output::create_sink(Some(&device), state.audio_output_mode)
                {
                    playbin.set_property("audio-sink", sink);
                } else {
                    state.device_id = None;
                }
            } else {
                state.device_id = None;
            }
        }
        if playbin
            .property::<Option<gst::Element>>("audio-sink")
            .is_none()
        {
            let sink = super::output::create_sink(None, state.audio_output_mode)?;
            playbin.set_property("audio-sink", sink);
        }
        let media = match MediaControls::new(hwnd, sender.clone()) {
            Ok(m) => Some(m),
            Err(e) => {
                state.media_error = Some(format!("系统媒体控制初始化失败：{e}"));
                None
            }
        };
        let prepared = Arc::new(Mutex::new(None::<Prepared>));
        let armed = Arc::new(Mutex::new(None::<Prepared>));
        let generation = Arc::new(AtomicU64::new(0));
        let pending = prepared.clone();
        let advancing = armed.clone();
        let current_generation = generation.clone();
        let gapless_enabled = Arc::new(AtomicBool::new(state.playback_options.gapless));
        let gapless = gapless_enabled.clone();
        let fade = super::transitions::PlaybackFade::new()?;
        fade.attach(&playbin)?;
        fade.set_enabled(state.playback_options.fades);
        playbin.connect("about-to-finish", false, move |values| {
            // Only consume a prepared URI here. This runs on a streaming thread:
            // no HTTP, database work, events, or channel waits are allowed.
            if let Ok(mut slot) = pending.lock() {
                if !super::transitions::gapless_allowed(&gapless) {
                    return None;
                }
                if let Some(next) = slot.take().filter(|p| {
                    p.generation == current_generation.load(Ordering::Acquire)
                        && p.resource
                            .as_ref()
                            .is_none_or(|r| r.check(p.generation).is_ok())
                }) {
                    let element = values[0]
                        .get::<gst::Element>()
                        .expect("GStreamer signal instance");
                    if let Ok(mut slot) = advancing.lock() {
                        element.set_property("uri", &next.resolved.uri);
                        *slot = Some(next);
                    }
                }
            }
            None
        });
        playbin.set_property("volume", state.volume.clamp(0.0, 1.0));
        Ok(Self {
            app,
            store,
            music,
            shared,
            sender,
            state,
            playbin,
            bus,
            monitor,
            media,
            desired_playing: false,
            generation,
            prepared,
            armed,
            decoded_audio: None,
            current_job: None,
            collection_job: None,
            collection_request: Default::default(),
            next_job: None,
            next_attempt: None,
            next_attempts: 0,
            last_tick: Instant::now(),
            last_save: Instant::now(),
            pending_seek: None,
            awaiting_first_stream: true,
            gapless_enabled,
            fade,
            deferred: None,
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
            // Drain a bounded number of messages so control commands stay responsive.
            for _ in 0..32 {
                let Some(message) = self.bus.pop() else {
                    break;
                };
                self.message(&message);
            }
            self.finish_deferred();
            for _ in 0..8 {
                let Some(message) = self.monitor.bus().pop() else {
                    break;
                };
                if let gst::MessageView::DeviceRemoved(_) = message.view() {
                    if self
                        .state
                        .device_id
                        .as_ref()
                        .is_some_and(|id| !self.devices().iter().any(|(info, _)| &info.id == id))
                    {
                        if let Err(error) = self.select_device(None) {
                            self.state.error = Some(error);
                            self.publish();
                        }
                    }
                }
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
        let _ = self.store.save_player(&self.state);
        let _ = self.playbin.set_state(gst::State::Null);
        self.monitor.stop();
    }

    fn command(&mut self, command: Command) -> AppResult<()> {
        if matches!(&command, Command::Toggle) {
            return self.command(if self.desired_playing {
                Command::Pause
            } else {
                Command::Resume
            });
        }
        if matches!(&command, Command::Pause) {
            self.cancel_collection();
            // A repeated pause must not extend a running fade-out.
            if self.deferred.is_some() {
                return Ok(());
            }
            if self.state.playback_options.fades
                && self.playbin.current_state() == gst::State::Playing
            {
                if let Some(time) = self.playbin.current_running_time() {
                    self.desired_playing = false;
                    let end = self.fade.fade_out(time);
                    self.deferred = Some((command, end.mseconds(), Instant::now()));
                    return Ok(());
                }
            }
        }
        match &command {
            Command::Queue(queue, index)
                if queue.is_empty() || queue.len() > 1000 || *index >= queue.len() =>
            {
                return Err("播放队列无效，最多支持 1000 首歌曲".into());
            }
            Command::Jump(index) if *index >= self.state.queue.len() => {
                return Err("歌曲不在播放队列中".into());
            }
            Command::Next if self.state.following(true).is_none() => return Ok(()),
            _ => {}
        }
        if matches!(
            &command,
            Command::Stop
                | Command::Clear
                | Command::Jump(_)
                | Command::Next
                | Command::Queue(_, _)
                | Command::Collection { .. }
                | Command::FmQueue(_)
        ) && self.deferred.take().is_some()
        {
            self.fade.reset();
        } else if matches!(
            &command,
            Command::PlayNext(_)
                | Command::Previous
                | Command::Remove(_, _)
                | Command::Repeat
                | Command::Shuffle
                | Command::Device(_)
                | Command::OutputMode(_, _)
                | Command::Seek(_)
        ) && self.deferred.take().is_some()
        {
            // These operations do not request playback. Honour a pending pause before
            // they change the timeline, rather than restoring sound under a paused intent.
            self.command_now(Command::Pause)?;
        }
        self.command_now(command)
    }

    fn finish_deferred(&mut self) {
        let ready = self.deferred.as_ref().is_some_and(|(_, end, started)| {
            self.playbin
                .current_running_time()
                .is_some_and(|t| t.mseconds() >= *end + super::transitions::PAUSE_GRACE_MS)
                || started.elapsed()
                    >= Duration::from_millis(super::transitions::PLAYBACK_FADE_MS + 100)
        });
        if ready {
            let (command, _, _) = self.deferred.take().unwrap();
            if let Err(error) = self.command_now(command) {
                self.fade.reset();
                self.state.error = Some(error);
                self.publish();
            }
        }
    }

    fn command_now(&mut self, command: Command) -> AppResult<()> {
        if matches!(
            &command,
            Command::PlayNext(_) | Command::Pause | Command::Next | Command::Previous
        ) {
            self.cancel_collection();
        }
        match command {
            Command::Collection {
                kind,
                id,
                track,
                fallback_cover,
                reply,
            } => {
                let scope = self.music.legacy_stamp()?;
                let key = track.as_ref().map(|track| track.key.clone());
                if let Some(track) = track {
                    self.command_now(Command::Queue(vec![track], 0))?;
                } else {
                    self.command_now(Command::Clear)?;
                    self.state.status = PlaybackStatus::Loading;
                    self.desired_playing = true;
                    self.publish();
                }
                let serial = self.collection_request.begin(key);
                let music = self.music.clone();
                let store = self.store.clone();
                let sender = self.sender.clone();
                self.collection_job = Some(tauri::async_runtime::spawn(async move {
                    let result = async {
                        let mut page = music.library_queue(&kind, id).await?;
                        for track in &mut page.tracks {
                            if track.cover.is_empty() {
                                track.cover = fallback_cover.clone();
                            }
                        }
                        // Metadata persistence must not block the audio actor.
                        let tracks = page.tracks.clone();
                        tauri::async_runtime::spawn_blocking(move || store.save_tracks(&tracks))
                            .await
                            .map_err(|error| error.to_string())??;
                        Ok((page.tracks, page.more))
                    }
                    .await;
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        let _ = sender.send(Command::CollectionReady {
                            serial,
                            scope,
                            result,
                            reply,
                        });
                    })
                    .await;
                }));
            }
            Command::CollectionReady {
                serial,
                scope,
                result,
                reply,
            } => {
                let Some(key) = self.collection_request.take(serial) else {
                    let _ = reply.send(Ok(false));
                    return Ok(());
                };
                self.collection_job = None;
                if self.music.legacy_stamp().ok().as_ref() != Some(&scope) {
                    if self.state.current().is_none() {
                        self.desired_playing = false;
                        self.state.status = PlaybackStatus::Stopped;
                        self.publish();
                    }
                    let _ = reply.send(Ok(false));
                    return Ok(());
                }
                let result = result.and_then(|(tracks, more)| {
                    if tracks.is_empty() {
                        return Err("这个收藏还没有可播放的歌曲".into());
                    }
                    if let Some(key) = key {
                        super::collection::hydrate(&mut self.state, &key, tracks)?;
                        self.publish();
                    } else {
                        self.command_now(Command::Queue(tracks, 0))?;
                    }
                    Ok(more)
                });
                if let Err(error) = &result {
                    // A known song remains playable even if its collection cannot be expanded.
                    self.state.error = Some(format!("歌单准备失败：{error}"));
                    if self.state.current().is_none() {
                        self.state.status = PlaybackStatus::Error;
                        self.desired_playing = false;
                    }
                    self.publish();
                }
                let _ = reply.send(result);
            }
            Command::Queue(queue, index) => {
                if queue.is_empty() || queue.len() > 1000 || index >= queue.len() {
                    return Err("播放队列无效，最多支持 1000 首歌曲".into());
                }
                self.state.private_fm_session = None;
                self.state.queue = queue;
                self.state.index = Some(index);
                self.state.deduplicate_queue();
                let index = self.state.index.unwrap_or(0);
                self.state.reset_shuffle_order(Some(index));
                self.load(index, true)?;
            }
            Command::FmQueue(queue) => {
                self.command_now(Command::Queue(queue, 0))?;
                self.state.shuffle = false;
                self.state.shuffle_order.clear();
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
                // Lock in the streaming callback's order while shifting history.
                let (trim, displaced) = {
                    let mut prepared = self.prepared.lock().map_err(|_| "预加载状态不可用")?;
                    let mut armed = self.armed.lock().map_err(|_| "预加载状态不可用")?;
                    let Some(trim) = self.state.append_private_fm(session, queue_len, tracks)?
                    else {
                        return Ok(());
                    };
                    let mut displaced = false;
                    if trim > 0 {
                        for slot in [&mut *prepared, &mut *armed] {
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
            Command::PlayNext(tracks) => {
                if tracks.iter().all(|t| {
                    self.state
                        .current()
                        .is_some_and(|current| current.key == t.key)
                }) {
                    return Ok(());
                }
                let position = self.state.insert_next(tracks)?;
                if let Some(job) = self.next_job.take() {
                    job.abort();
                }
                // Hold the prepared lock while clearing armed state so the streaming
                // callback cannot arm the old next track between these operations.
                let transitioning = {
                    let mut prepared = self.prepared.lock().map_err(|_| "预加载状态不可用")?;
                    *prepared = None;
                    self.armed
                        .lock()
                        .map_err(|_| "预加载状态不可用")?
                        .take()
                        .is_some()
                };
                self.next_attempt = None;
                self.next_attempts = 0;
                if transitioning {
                    // The old URI was already handed to GStreamer at the boundary.
                    // Start the newly requested next track through the normal loader.
                    self.load(position, self.desired_playing)?;
                }
                self.publish();
            }
            Command::Remove(index, key) => {
                let removed_current = self.state.remove_track(index, &key)?;
                let loading = self.state.status == PlaybackStatus::Loading;
                // Use the same lock order as the streaming callback, so it cannot
                // arm a removed next track between checking and invalidating it.
                let transitioning = {
                    let mut prepared = self.prepared.lock().map_err(|_| "预加载状态不可用")?;
                    *prepared = None;
                    self.armed
                        .lock()
                        .map_err(|_| "预加载状态不可用")?
                        .take()
                        .is_some()
                };
                self.cancel_jobs();
                if self.state.queue.is_empty() {
                    self.command(Command::Clear)?;
                } else if let Some(current) = self.state.index {
                    if removed_current || loading || transitioning {
                        self.load(current, self.desired_playing)?;
                    } else {
                        self.publish();
                    }
                } else {
                    self.publish();
                }
            }
            Command::Jump(index) => self.load(index, true)?,
            Command::Repeat | Command::Shuffle => {
                // Match the streaming callback's lock order. If it has already
                // handed over a URI, reload that entry rather than losing the
                // StreamStart metadata while changing the future play order.
                let transitioning = {
                    let mut prepared = self.prepared.lock().map_err(|_| "预加载状态不可用")?;
                    *prepared = None;
                    self.armed.lock().map_err(|_| "预加载状态不可用")?.take()
                };
                if matches!(command, Command::Shuffle) {
                    self.state.toggle_shuffle(
                        transitioning
                            .as_ref()
                            .map(|next| next.index)
                            .or(self.state.index),
                    );
                } else {
                    self.state.cycle_repeat();
                }
                if let Some(job) = self.next_job.take() {
                    job.abort();
                }
                self.next_attempt = None;
                self.next_attempts = 0;
                if let Some(next) = transitioning {
                    self.load(next.index, self.desired_playing)?;
                }
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
                self.playbin
                    .set_state(gst::State::Paused)
                    .map_err(|e| e.to_string())?;
                self.state.status = PlaybackStatus::Paused;
                self.publish();
            }
            Command::Resume => {
                let reversing = self.deferred.take().is_some();
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
                        if reversing {
                            if let Some(time) = self.playbin.current_running_time() {
                                self.fade.reverse_pause(time);
                            } else {
                                self.fade.reset();
                            }
                        } else if self.playbin.current_state() != gst::State::Playing {
                            self.fade.prepare_start();
                        }
                        if self.pending_seek.is_none() {
                            self.playbin
                                .set_state(gst::State::Playing)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
            Command::Stop | Command::Clear => {
                let clear = matches!(command, Command::Clear);
                self.cancel_jobs();
                self.desired_playing = false;
                self.playbin
                    .set_state(gst::State::Null)
                    .map_err(|e| e.to_string())?;
                self.fade.reset();
                self.decoded_audio = None;
                if clear {
                    self.state.private_fm_session = None;
                    self.state.queue.clear();
                    self.state.shuffle_order.clear();
                    self.state.index = None;
                    self.state.duration_ms = 0;
                    self.state.actual_quality = None;
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
                self.playbin.set_property("volume", value);
                self.state.volume = value;
                self.publish();
            }
            Command::Device(id) => self.select_device(id)?,
            Command::OutputMode(mode, reply) => {
                let result = self.select_output_mode(mode);
                let _ = reply.send(result.clone());
                result?;
            }
            Command::Options(options, reply) => {
                let result = self.apply_options(options);
                let _ = reply.send(result.clone());
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
                    if !self.state.playback_options.gapless
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
                    if let Ok((resolved, resource)) = *result {
                        if resource
                            .as_ref()
                            .is_some_and(|r| r.check(generation).is_err())
                        {
                            return Ok(());
                        }
                        *self.prepared.lock().map_err(|_| "预加载状态不可用")? = Some(Prepared {
                            generation,
                            index,
                            resolved,
                            resource,
                        });
                    }
                } else {
                    self.current_job = None;
                    let result = (*result).and_then(|(resolved, resource)| {
                        if let Some(resource) = &resource {
                            resource.check(generation).map_err(|e| e.to_string())?;
                        }
                        Ok((resolved, resource))
                    });
                    match result {
                        Ok((resolved, resource)) => {
                            let _resource = resource;
                            self.state.actual_quality = resolved.quality;
                            crate::audio::start_stream(
                                &self.playbin,
                                &resolved.uri,
                                self.state.volume,
                                if self.desired_playing && self.pending_seek.is_none() {
                                    gst::State::Playing
                                } else {
                                    gst::State::Paused
                                },
                            )
                            .map_err(|e| e.to_string())?;
                            self.decoded_audio = resolved.decoded_audio;
                        }
                        Err(error) => {
                            self.state.status = PlaybackStatus::Error;
                            self.state.error = Some(error);
                            self.desired_playing = false;
                            self.publish();
                        }
                    }
                }
            }
            Command::Devices(reply) => {
                let _ = reply.send(Ok(self
                    .devices()
                    .into_iter()
                    .map(|(info, _)| info)
                    .collect()));
            }
            Command::Toggle => unreachable!("toggle is resolved before command dispatch"),
            Command::Shutdown => {}
        }
        Ok(())
    }

    fn apply_options(&mut self, options: PlaybackOptions) -> AppResult<()> {
        // Persist before changing live behavior; a failed write preserves both switches.
        self.store.set_setting(
            "playbackOptions",
            &serde_json::to_string(&options).map_err(|e| e.to_string())?,
        )?;
        self.state.playback_options = options;
        if !options.fades {
            if let Some((command, _, _)) = self.deferred.take() {
                self.command_now(command)?;
            }
        }
        let transitioning = {
            let mut prepared = self.prepared.lock().map_err(|_| "预加载状态不可用")?;
            self.gapless_enabled
                .store(options.gapless, Ordering::Release);
            if !options.gapless {
                *prepared = None;
                self.armed.lock().map_err(|_| "预加载状态不可用")?.take()
            } else {
                None
            }
        };
        if !options.gapless {
            if let Some(job) = self.next_job.take() {
                job.abort();
            }
            self.next_attempt = None;
            self.next_attempts = 0;
        }
        if let Some(next) = transitioning {
            self.load(next.index, self.desired_playing)?;
        }
        self.fade.set_enabled(options.fades);
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
        self.playbin
            .set_state(gst::State::Null)
            .map_err(|e| e.to_string())?;
        self.fade.reset();
        if playing {
            self.fade.prepare_start();
        }
        self.decoded_audio = None;
        self.state.revision += 1;
        self.state.index = Some(index);
        self.state.position_ms = if track.duration_ms > 0 {
            position.min(track.duration_ms)
        } else {
            position
        };
        self.state.duration_ms = track.duration_ms;
        self.state.actual_quality = None;
        self.state.error = None;
        self.pending_seek = (self.state.position_ms > 0).then_some(self.state.position_ms);
        self.awaiting_first_stream = true;
        self.state.status = PlaybackStatus::Loading;
        self.desired_playing = playing;
        self.resolve(index, track, false);
        self.publish();
        Ok(())
    }

    fn resolve(&mut self, index: usize, track: Track, next: bool) {
        let generation = self.generation.load(Ordering::Acquire);
        let music = self.music.clone();
        let sender = self.sender.clone();
        let quality = self
            .store
            .setting("quality")
            .ok()
            .flatten()
            .unwrap_or_else(|| "exhigh".into());
        let downgrade = self
            .store
            .setting("allowDowngrade")
            .ok()
            .flatten()
            .as_deref()
            != Some("false");
        let task = tauri::async_runtime::spawn(async move {
            let key = track.key.clone();
            let result = music.resolve(track, &quality, downgrade, generation).await;
            // Bounded channel delivery is performed off the audio thread.
            let _ = tauri::async_runtime::spawn_blocking(move || {
                let _ = sender.send(Command::Resolved {
                    generation,
                    index,
                    key,
                    result: Box::new(result),
                    next,
                });
            })
            .await;
        });
        if next {
            self.next_job = Some(task);
        } else {
            self.current_job = Some(task);
        }
    }

    fn cancel_jobs(&mut self) {
        self.cancel_collection();
        self.deferred = None;
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Some(job) = self.current_job.take() {
            job.abort();
        }
        if let Some(job) = self.next_job.take() {
            job.abort();
        }
        if let Ok(mut slot) = self.prepared.lock() {
            *slot = None;
        }
        if let Ok(mut slot) = self.armed.lock() {
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
        self.deferred = None;
        self.fade.reset();
        if self.state.current().is_none() || self.state.duration_ms == 0 {
            return Err("当前歌曲尚不可定位".into());
        }
        let ms = ms.min(self.state.duration_ms);
        if matches!(
            self.state.status,
            PlaybackStatus::Stopped | PlaybackStatus::Error
        ) {
            self.state.position_ms = ms;
            self.publish();
            return Ok(());
        }
        if self.state.status == PlaybackStatus::Loading || self.pending_seek.is_some() {
            self.pending_seek = Some(ms);
            self.state.position_ms = ms;
            self.publish();
            return Ok(());
        }
        crate::audio::seek_stream(&self.playbin, ms).map_err(|_| "当前资源不支持定位")?;
        self.state.position_ms = ms;
        self.publish();
        Ok(())
    }

    fn message(&mut self, message: &gst::MessageRef) {
        use gst::MessageView;
        match message.view() {
            MessageView::AsyncDone(_) => {
                if let Some(position) = self.pending_seek.take() {
                    // The resource has finished preroll; seek the pipeline directly.
                    if let Err(error) = crate::audio::seek_stream(&self.playbin, position) {
                        self.state.error = Some(format!("恢复播放位置失败：{error}"));
                    }
                    if self.desired_playing {
                        let _ = self.playbin.set_state(gst::State::Playing);
                    }
                }
            }
            MessageView::StreamStart(_) => {
                if self.awaiting_first_stream {
                    self.awaiting_first_stream = false;
                    self.publish();
                    return;
                }
                let armed = self.armed.lock().ok().and_then(|mut slot| slot.take());
                if let Some(next) =
                    armed.filter(|p| p.generation == self.generation.load(Ordering::Acquire))
                {
                    let pause = self.deferred.take();
                    self.cancel_jobs();
                    self.deferred = pause;
                    self.state.revision += 1;
                    self.state.index = Some(next.index);
                    self.state.position_ms = 0;
                    self.state.duration_ms = next.resolved.track.duration_ms;
                    self.state.actual_quality = next.resolved.quality;
                    self.decoded_audio = next.resolved.decoded_audio;
                }
                self.publish();
            }
            MessageView::StateChanged(change)
                if message.src().is_some_and(|s| s == &self.playbin)
                    && self.current_job.is_none() =>
            {
                let status = match change.current() {
                    gst::State::Playing => Some(PlaybackStatus::Playing),
                    gst::State::Paused if !self.desired_playing => Some(PlaybackStatus::Paused),
                    _ => None,
                };
                if let Some(status) = status {
                    if self.state.status != status {
                        self.state.status = status;
                        self.publish();
                    }
                }
            }
            MessageView::Buffering(value)
                if self.desired_playing && self.pending_seek.is_none() =>
            {
                match crate::audio::update_buffering(&self.playbin, value.percent()) {
                    Ok(status) => self.state.status = status,
                    Err(error) => self.state.error = Some(error.to_string()),
                }
                self.publish();
            }
            MessageView::DurationChanged(_) => {
                self.state.duration_ms = self
                    .playbin
                    .query_duration::<gst::ClockTime>()
                    .map_or(self.state.duration_ms, |t| t.mseconds());
            }
            MessageView::Eos(_) => {
                if self.deferred.take().is_some() || !self.desired_playing {
                    // EOS can beat the last few milliseconds of a pause. Leave a resumable
                    // stopped track, not a PAUSED pipeline that has already exhausted its stream.
                    self.desired_playing = false;
                    self.state.status = PlaybackStatus::Stopped;
                    self.state.position_ms = self.state.duration_ms;
                    let _ = self.playbin.set_state(gst::State::Null);
                    self.fade.reset();
                    self.decoded_audio = None;
                    self.publish();
                    return;
                }
                if let Some(index) = self.state.following(false) {
                    if let Err(error) = self.load(index, true) {
                        self.state.error = Some(error);
                        self.state.status = PlaybackStatus::Error;
                        self.publish();
                    }
                    self.fade.reset();
                } else {
                    self.desired_playing = false;
                    self.state.status = PlaybackStatus::Stopped;
                    self.state.position_ms = self.state.duration_ms;
                    let _ = self.playbin.set_state(gst::State::Null);
                    self.decoded_audio = None;
                    self.publish();
                }
            }
            MessageView::Error(error) => {
                self.cancel_jobs();
                self.desired_playing = false;
                self.state.status = PlaybackStatus::Error;
                self.state.error = Some(
                    if self.state.audio_output_mode == AudioOutputMode::Exclusive {
                        format!(
                            "独占模式播放失败，请检查设备独占权限或切回共享模式：{}",
                            error.error()
                        )
                    } else {
                        format!("音频播放失败：{}", error.error())
                    },
                );
                let _ = self.playbin.set_state(gst::State::Null);
                self.fade.reset();
                self.publish();
            }
            _ => {}
        }
    }

    fn tick(&mut self) {
        if let Ok(mut prepared) = self.prepared.lock() {
            if prepared.as_ref().is_some_and(|p| {
                p.resource
                    .as_ref()
                    .is_some_and(|r| r.check(p.generation).is_err())
            }) {
                *prepared = None;
                self.next_attempt = None;
                self.next_attempts = 0;
            }
        }
        if matches!(
            self.state.status,
            PlaybackStatus::Playing
                | PlaybackStatus::Paused
                | PlaybackStatus::Buffering
                | PlaybackStatus::Loading
        ) {
            if self.pending_seek.is_none() {
                self.state.position_ms = self
                    .playbin
                    .query_position::<gst::ClockTime>()
                    .map_or(self.state.position_ms, |t| t.mseconds());
            }
            self.state.duration_ms = self
                .playbin
                .query_duration::<gst::ClockTime>()
                .map_or(self.state.duration_ms, |t| t.mseconds());
            if let Ok(mut shared) = self.shared.write() {
                shared.position_ms = self.state.position_ms;
                shared.duration_ms = self.state.duration_ms;
            }
            self.update_media();
            let _ = self.app.emit(
                "player-progress",
                Progress {
                    revision: self.state.revision,
                    position_ms: self.state.position_ms,
                    duration_ms: self.state.duration_ms,
                    status: self.state.status,
                },
            );
            if self.state.playback_options.gapless
                && !self.collection_request.is_pending()
                && self.deferred.is_none()
                && self.state.status == PlaybackStatus::Playing
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
            {
                let ready = self.prepared.lock().map(|p| p.is_some()).unwrap_or(false)
                    || self.armed.lock().map(|p| p.is_some()).unwrap_or(false);
                if !ready {
                    if let Some(index) = self.state.following(false) {
                        self.next_attempt = Some(Instant::now());
                        self.next_attempts += 1;
                        self.resolve(index, self.state.queue[index].clone(), true);
                    }
                }
            }
        }
    }

    fn publish(&mut self) {
        self.update_media();
        if let Ok(mut shared) = self.shared.write() {
            *shared = self.state.clone();
        }
        let _ = self.app.emit("player-state", self.state.clone());
    }

    fn update_media(&mut self) {
        if let Some(media) = &mut self.media {
            if let Err(error) = media.update(&self.state) {
                self.state.media_error = Some(format!("系统媒体信息同步失败：{error}"));
            }
        }
    }

    fn devices(&self) -> Vec<(OutputDevice, gst::Device)> {
        self.monitor
            .devices()
            .into_iter()
            .map(|device| {
                let identity = format!(
                    "{}:{}",
                    device.device_class(),
                    device
                        .properties()
                        .map_or_else(|| device.display_name().to_string(), |s| s.to_string())
                );
                (
                    OutputDevice {
                        id: format!("{:x}", Sha256::digest(identity.as_bytes())),
                        name: device.display_name().to_string(),
                    },
                    device,
                )
            })
            .collect()
    }

    fn output_sink(&self, id: &Option<String>, mode: AudioOutputMode) -> AppResult<gst::Element> {
        let device = match id {
            Some(id) => Some(
                self.devices()
                    .into_iter()
                    .find(|(info, _)| &info.id == id)
                    .ok_or("该设备已断开，请选择其他设备")?
                    .1,
            ),
            None => None,
        };
        super::output::create_sink(device.as_ref(), mode)
    }

    fn select_device(&mut self, id: Option<String>) -> AppResult<()> {
        let sink = self.output_sink(&id, self.state.audio_output_mode)?;
        self.replace_output(sink, id)
    }

    fn select_output_mode(&mut self, mode: AudioOutputMode) -> AppResult<()> {
        if !super::output::available_modes().contains(&mode) {
            return Err("WASAPI 输出模式仅支持 Windows".into());
        }
        if mode == self.state.audio_output_mode {
            return Ok(());
        }
        let id = self.state.device_id.clone();
        let sink = self.output_sink(&id, mode)?;
        let previous = self.state.audio_output_mode;
        // A failed settings write must leave the running output untouched.
        self.store.set_setting(
            "audioOutputMode",
            &serde_json::to_string(&mode).map_err(|e| e.to_string())?,
        )?;
        self.state.audio_output_mode = mode;
        if let Err(error) = self.replace_output(sink, id.clone()) {
            self.state.audio_output_mode = previous;
            let rollback = self.store.set_setting(
                "audioOutputMode",
                &serde_json::to_string(&previous).map_err(|e| e.to_string())?,
            );
            if let Ok(sink) = self.output_sink(&id, previous) {
                let _ = self.replace_output(sink, id);
            }
            self.publish();
            rollback.map_err(|e| format!("{error}；恢复输出设置失败：{e}"))?;
            return Err(error);
        }
        Ok(())
    }

    fn replace_output(&mut self, sink: gst::Element, id: Option<String>) -> AppResult<()> {
        let playing = self.desired_playing;
        let index = self.state.index;
        let active = !matches!(
            self.state.status,
            PlaybackStatus::Stopped | PlaybackStatus::Error
        );
        let position = self.pending_seek.unwrap_or_else(|| {
            self.playbin
                .query_position::<gst::ClockTime>()
                .map_or(self.state.position_ms, |time| time.mseconds())
        });
        self.cancel_jobs();
        self.playbin
            .set_state(gst::State::Null)
            .map_err(|e| e.to_string())?;
        self.fade.set_output(&self.playbin, sink)?;
        self.state.device_id = id;
        // Device changes restart the pipeline. The normal resolver and state
        // messages remain the sole source of truth; never promise gapless here.
        if let Some(index) = index.filter(|_| active) {
            self.load_at(index, playing, position)?;
        } else {
            self.state.error = None;
            self.state.status = PlaybackStatus::Stopped;
            self.publish();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TrackSource;

    #[test]
    #[cfg(windows)]
    fn output_mode_setting_overrides_snapshot_and_survives_restart() {
        let directory = crate::test_support::TestDir::new();
        let path = directory.0.join("output.sqlite3");
        let store = Store::open(&path).unwrap();
        assert_eq!(
            stored_output_mode(&store, AudioOutputMode::Shared).unwrap(),
            AudioOutputMode::Shared
        );
        for mode in [AudioOutputMode::Exclusive, AudioOutputMode::Shared] {
            store
                .set_setting("audioOutputMode", &serde_json::to_string(&mode).unwrap())
                .unwrap();
            let reopened = Store::open(&path).unwrap();
            assert_eq!(
                stored_output_mode(&reopened, AudioOutputMode::Shared).unwrap(),
                mode
            );
        }
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
                gapless: true
            }
        );
        assert_eq!(
            serde_json::from_str::<PlaybackOptions>("{}").unwrap(),
            PlaybackOptions::default()
        );
        for fades in [false, true] {
            for gapless in [false, true] {
                let options = PlaybackOptions { fades, gapless };
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
                .map(|id| Track {
                    key: id.to_string(),
                    title: id.to_string(),
                    aliases: vec![],
                    artist: String::new(),
                    artists: vec![],
                    album_id: None,
                    album: String::new(),
                    duration_ms: 180_000,
                    cover: String::new(),
                    source: TrackSource::Netease { id },
                })
                .collect(),
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
        saved.queue[1].duration_ms = 0;
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
                state.remove_track(0, "0").unwrap();
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
        state.queue.clear();
        assert!(previous_command(&state).is_none());
        state = snapshot(3, 179_000, RepeatMode::All);
        assert!(previous_command(&state).is_none());
    }
}
