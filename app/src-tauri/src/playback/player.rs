use super::history::PlaybackHistory;
use super::publication::{PlayerBootstrap, Publication};
#[cfg(test)]
use crate::model::RepeatMode;
use crate::{
    media::MediaControls,
    model::{
        AppResult, OutputDevice, PlaybackEngine, PlaybackOptions, PlaybackStatus, PlayerSnapshot,
        ResolvedTrack, Track,
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
    Engine(PlaybackEngine, mpsc::Sender<AppResult<()>>),
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
    Devices(mpsc::Sender<AppResult<Vec<OutputDevice>>>),
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
    // Jump goes through load(), invalidating prepared/armed gapless work.
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
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
            })
            .map(restore_player)
            .unwrap_or_default();
        restored.playback_options = stored_options(&store, restored.playback_options)?;
        restored.playback_engine = if cfg!(windows) {
            store
                .setting("playbackEngine")?
                .map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
                .transpose()?
                .unwrap_or_default()
        } else {
            PlaybackEngine::Gstreamer
        };
        let device_key = if restored.playback_engine == PlaybackEngine::WebAudio {
            "webAudioDevice"
        } else {
            "gstreamerDevice"
        };
        if let Some(value) = store.setting(device_key)? {
            restored.device_id = serde_json::from_str(&value).map_err(|e| e.to_string())?;
        } else if restored.playback_engine == PlaybackEngine::WebAudio {
            restored.device_id = None;
        }
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
                            crate::infrastructure::diagnostics::warn("optional_success", error);
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
                    crate::infrastructure::diagnostics::warn("optional_result", error);
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
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "播放状态不可用".into())
    }

    pub fn bootstrap(&self) -> AppResult<PlayerBootstrap> {
        self.snapshot
            .read()
            .map(|s| s.bootstrap())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "播放状态不可用".into())
    }

    pub(crate) fn needs_playback_frontend(&self) -> AppResult<bool> {
        self.snapshot
            .read()
            .map(|state| {
                let state = &state.snapshot;
                state.playback_engine == PlaybackEngine::WebAudio
                    || state.private_fm_session.is_some()
                        && matches!(
                            state.status,
                            PlaybackStatus::Loading
                                | PlaybackStatus::Playing
                                | PlaybackStatus::Buffering
                        )
            })
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "播放状态不可用".into())
    }

    pub fn devices(&self) -> AppResult<Vec<OutputDevice>> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::Devices(sender))?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "查询音频设备超时".to_string())?
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
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "保存播放设置超时".to_string())?
    }

    pub fn set_engine(&self, engine: PlaybackEngine) -> AppResult<()> {
        let (sender, receiver) = mpsc::channel();
        self.send(Command::Engine(engine, sender))?;
        receiver
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "切换播放引擎超时".to_string())?
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
            crate::infrastructure::diagnostics::warn("self.sender.send", error);
        });
        if let Ok(mut thread) = (self.thread.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_success", error);
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
    engine_switch: Option<EngineSwitch>,
    selections: std::collections::BTreeMap<String, crate::music::service::PlaybackSelection>,
    app: tauri::AppHandle,
    store: Arc<Store>,
    history: Arc<PlaybackHistory>,
    music: Arc<MusicService>,
    shared: Arc<RwLock<Publication>>,
    sender: SyncSender<Command>,
    state: PlayerSnapshot,
    native: Option<GstreamerEngine>,
    media: Option<MediaControls>,
    desired_playing: bool,
    failed_keys: std::collections::HashSet<String>,
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
    start_gate: crate::audio::StreamStartGate,
    gapless_enabled: Arc<AtomicBool>,
    deferred: Option<(Command, u64, Instant)>,
    play_instance_id: String,
    recorded_instance_id: Option<String>,
}

struct EngineSwitch {
    target: PlaybackEngine,
    reply: mpsc::Sender<AppResult<()>>,
    started: Instant,
    sample_position: bool,
    native: Option<GstreamerEngine>,
    device_id: Option<String>,
    preloaded: Option<(String, Prepared)>,
}

struct GstreamerEngine {
    playbin: gst::Element,
    bus: gst::Bus,
    monitor: DeviceMonitor,
    fade: super::transitions::PlaybackFade,
}

struct DeviceMonitor(Option<gst::DeviceMonitor>);
impl std::ops::Deref for DeviceMonitor {
    type Target = Option<gst::DeviceMonitor>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for DeviceMonitor {
    fn drop(&mut self) {
        if let Some(monitor) = &self.0 {
            monitor.stop();
        }
    }
}

fn runtime_cache(app: &tauri::AppHandle) -> AppResult<std::path::PathBuf> {
    use tauri::Manager;
    match crate::infrastructure::directories::portable() {
        Some(data) => Ok(data.join("cache")),
        None => app.path().app_cache_dir().map_err(|e| e.to_string()),
    }
}

impl GstreamerEngine {
    fn new(
        app: &tauri::AppHandle,
        state: &mut PlayerSnapshot,
        prepared: &Arc<Mutex<Option<Prepared>>>,
        armed: &Arc<Mutex<Option<Prepared>>>,
        generation: &Arc<AtomicU64>,
        gapless_enabled: &Arc<AtomicBool>,
    ) -> AppResult<Self> {
        super::audio_runtime::configure(&runtime_cache(app)?)?;
        gst::init().map_err(|e| format!("GStreamer 初始化失败：{e}"))?;
        super::diagnostics::initialize();
        crate::network::prefer_http_source();
        let playbin = gst::ElementFactory::make("playbin3")
            .build()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "GStreamer 缺少 playbin3 插件，请检查运行时")?;
        let bus = playbin.bus().ok_or("音频消息总线不可用")?;
        playbin.set_property("buffer-size", 4_194_304_i32);
        playbin.set_property("buffer-duration", 10_000_000_000_i64);
        playbin.set_property_from_str("flags", "audio+soft-volume+buffering");
        playbin.connect("source-setup", false, |values| {
            if let Ok(source) = (values[1].get::<gst::Element>()).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_success", error);
            }) {
                crate::network::configure_source(&source);
            }
            None
        });
        let monitor = DeviceMonitor(super::output::start_device_monitor()?);
        if let Some(id) = &state.device_id {
            let device = monitor.iter().flat_map(|m| m.devices()).find(|device| {
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
                if let Ok(sink) = (super::output::create_sink(Some(&device))).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_success", error);
                }) {
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
            let sink = super::output::create_sink(None)?;
            playbin.set_property("audio-sink", sink);
        }
        let pending = prepared.clone();
        let advancing = armed.clone();
        let current_generation = generation.clone();
        let gapless = gapless_enabled.clone();
        let fade = super::transitions::PlaybackFade::new()?;
        fade.attach(&playbin)?;
        fade.set_enabled(
            state.playback_options.fades && state.playback_engine == PlaybackEngine::Gstreamer,
        );
        playbin.connect("about-to-finish", false, move |values| {
            // Only consume a prepared URI here. This runs on a streaming thread:
            // no HTTP, database work, events, or channel waits are allowed.
            if let Ok(mut slot) = (pending.lock()).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_success", error);
            }) {
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
                    if let Ok(mut slot) = (advancing.lock()).inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_success", error);
                    }) {
                        element.set_property("uri", &next.resolved.uri);
                        *slot = Some(next);
                    }
                }
            }
            None
        });
        playbin.set_property("volume", state.volume.clamp(0.0, 1.0));
        Ok(Self {
            playbin,
            bus,
            monitor,
            fade,
        })
    }
}

impl Drop for GstreamerEngine {
    fn drop(&mut self) {
        let _ = self.playbin.set_state(gst::State::Null);
    }
}

impl Actor {
    fn native(&self) -> &GstreamerEngine {
        self.native
            .as_ref()
            .expect("native commands require the selected GStreamer engine")
    }

    fn is_web(&self) -> bool {
        self.state.playback_engine == PlaybackEngine::WebAudio
    }

    fn prepare_engine(
        &self,
        engine: PlaybackEngine,
    ) -> AppResult<(Option<GstreamerEngine>, Option<String>)> {
        let mut target_state = self.state.clone();
        target_state.playback_engine = engine;
        let current_key = if self.is_web() {
            "webAudioDevice"
        } else {
            "gstreamerDevice"
        };
        self.store.set_setting(
            current_key,
            &serde_json::to_string(&self.state.device_id).map_err(|e| e.to_string())?,
        )?;
        // Device identifiers belong to their engine; each keeps its own preference.
        target_state.device_id = if engine == PlaybackEngine::WebAudio {
            self.store
                .setting("webAudioDevice")?
                .and_then(|s| serde_json::from_str(&s).ok())
                .flatten()
        } else {
            self.store
                .setting("gstreamerDevice")?
                .and_then(|s| serde_json::from_str(&s).ok())
                .flatten()
        };
        let native = if engine == PlaybackEngine::Gstreamer {
            Some(GstreamerEngine::new(
                &self.app,
                &mut target_state,
                &self.prepared,
                &self.armed,
                &self.generation,
                &self.gapless_enabled,
            )?)
        } else {
            None
        };
        Ok((native, target_state.device_id))
    }

    fn switch_engine(
        &mut self,
        engine: PlaybackEngine,
        native: Option<GstreamerEngine>,
        device_id: Option<String>,
    ) -> AppResult<()> {
        // Browser -> native reaches here only after the browser has acknowledged pause.
        let position = if self.is_web() {
            self.state.position_ms
        } else {
            self.pending_seek.unwrap_or_else(|| {
                self.native()
                    .playbin
                    .query_position::<gst::ClockTime>()
                    .map_or(self.state.position_ms, |p| {
                        crate::audio::track_position(self.state.current(), p.mseconds())
                    })
            })
        };
        // Persist first so a failed write cannot silently stop the current pipeline.
        self.store.set_setting(
            "playbackEngine",
            &serde_json::to_string(&engine).map_err(|e| e.to_string())?,
        )?;
        if let Some(current) = &self.native {
            if let Err(error) = current.playbin.set_state(gst::State::Null) {
                let _ = self.store.set_setting(
                    "playbackEngine",
                    &serde_json::to_string(&self.state.playback_engine)
                        .map_err(|e| e.to_string())?,
                );
                return Err(error.to_string());
            }
        }
        let active = !matches!(
            self.state.status,
            PlaybackStatus::Stopped | PlaybackStatus::Error
        );
        self.cancel_jobs();
        self.native = native;
        self.state.playback_engine = engine;
        self.state.device_id = device_id;
        self.gapless_enabled.store(
            !self.is_web() && self.state.playback_options.gapless,
            Ordering::Release,
        );
        self.web_audio.configure(&self.state);
        let instance = self.play_instance_id.clone();
        let recorded = self.recorded_instance_id.clone();
        if let Some(index) = self.state.index.filter(|_| active) {
            self.load_at(index, self.desired_playing, position)?;
            self.play_instance_id = instance;
            self.recorded_instance_id = recorded;
        } else {
            self.publish();
        }
        Ok(())
    }

    fn web_feedback(&mut self, feedback: super::web_audio::Feedback) -> AppResult<()> {
        use super::web_audio::FeedbackKind;
        if !self.is_web() || !self.web_audio.accepts(&feedback) {
            return Ok(());
        }
        if feedback.kind == FeedbackKind::Stopped {
            if let Some(switch) = self.engine_switch.take() {
                // The browser may start the successor while native initialization runs.
                // Its stop acknowledgement identifies which voice actually owned the timeline.
                if let Some((_, next)) = switch
                    .preloaded
                    .filter(|(source, _)| Some(source.as_str()) == feedback.next_source.as_deref())
                {
                    self.state.index = Some(next.index);
                    self.state.revision += 1;
                    self.state.actual_quality = next.resolved.quality;
                    self.state.playback_source = next.source;
                    self.state.duration_ms = crate::audio::track_duration(
                        self.state.current(),
                        if feedback.duration_ms > 0 {
                            feedback.duration_ms
                        } else {
                            self.state.current().map_or(0, |t| t.duration_ms)
                        },
                    );
                    self.play_instance_id = uuid::Uuid::new_v4().to_string();
                    self.recorded_instance_id = None;
                    self.record_playback_history();
                }
                if switch.sample_position {
                    self.state.position_ms =
                        crate::audio::track_position(self.state.current(), feedback.position_ms);
                }
                let result = self.switch_engine(switch.target, switch.native, switch.device_id);
                if let Err(error) = &result {
                    self.desired_playing = false;
                    self.state.status = PlaybackStatus::Error;
                    self.state.error = Some(error.clone());
                    self.publish();
                }
                let _ = switch.reply.send(result.clone());
                result?;
            }
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
                        .unwrap_or_else(|| "浏览器无法播放此音频，可尝试切换 GStreamer".into()),
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
                crate::infrastructure::diagnostics::warn("translate_error", error);
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
        let armed = Arc::new(Mutex::new(None::<Prepared>));
        let generation = Arc::new(AtomicU64::new(0));
        let gapless_enabled = Arc::new(AtomicBool::new(
            state.playback_options.gapless && state.playback_engine == PlaybackEngine::Gstreamer,
        ));
        let native = if state.playback_engine == PlaybackEngine::Gstreamer {
            Some(GstreamerEngine::new(
                &app,
                &mut state,
                &prepared,
                &armed,
                &generation,
                &gapless_enabled,
            )?)
        } else {
            None
        };
        let mut web_audio = super::web_audio::WebAudio::new()?;
        web_audio.configure(&state);
        Ok(Self {
            web_audio,
            engine_switch: None,
            selections: Default::default(),
            app,
            store,
            history,
            music,
            shared,
            sender,
            state,
            native,
            media,
            desired_playing: false,
            failed_keys: Default::default(),
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
            start_gate: Default::default(),
            gapless_enabled,
            deferred: None,
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
            // Drain a bounded number of messages so control commands stay responsive.
            for _ in 0..32 {
                let Some(message) = self.native.as_ref().and_then(|n| n.bus.pop()) else {
                    break;
                };
                if !self.is_web() {
                    self.message(&message);
                }
            }
            self.finish_deferred();
            for _ in 0..8 {
                let Some(message) = self
                    .native
                    .as_ref()
                    .and_then(|n| n.monitor.as_ref())
                    .and_then(|m| m.bus().pop())
                else {
                    break;
                };
                if let gst::MessageView::DeviceRemoved(_) = message.view() {
                    if !self.is_web()
                        && self.state.device_id.as_ref().is_some_and(|id| {
                            !self.devices().iter().any(|(info, _)| &info.id == id)
                        })
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
        let _ = (self.store.save_player(&self.state)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("self.store.save_player", error);
        });
        self.native.take();
    }

    fn command(&mut self, command: Command) -> AppResult<()> {
        if self.engine_switch.is_some()
            && !matches!(
                &command,
                Command::WebFeedback(_) | Command::WebReady(_) | Command::Shutdown
            )
        {
            let error = "正在切换播放引擎，请稍后重试".to_string();
            match command {
                Command::Engine(_, reply)
                | Command::Options(_, reply)
                | Command::Device(_, reply) => {
                    let _ = reply.send(Err(error.clone()));
                }
                Command::Devices(reply) => {
                    let _ = reply.send(Err(error.clone()));
                }
                _ => {}
            }
            return Err(error);
        }
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
        if matches!(&command, Command::Pause) {
            self.cancel_collection();
            // A repeated pause must not extend a running fade-out.
            if self.deferred.is_some() {
                return Ok(());
            }
            if !self.is_web()
                && self.state.playback_options.fades
                && self.native().playbin.current_state() == gst::State::Playing
            {
                if let Some(time) = self.native().playbin.current_running_time() {
                    self.desired_playing = false;
                    let end = self.native().fade.fade_out(time);
                    self.deferred = Some((command, end.mseconds(), Instant::now()));
                    return Ok(());
                }
            }
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
            Command::Stop
                | Command::Clear
                | Command::Jump(_)
                | Command::PlayNextNow(_)
                | Command::Next
                | Command::Queue(_, _)
                | Command::Collection { .. }
                | Command::FmQueue(_)
        ) && self.deferred.take().is_some()
        {
            self.native().fade.reset();
        } else if matches!(
            &command,
            Command::PlayNext(_)
                | Command::Previous
                | Command::Remove(_, _)
                | Command::Repeat
                | Command::Shuffle
                | Command::Device(_, _)
                | Command::Seek(_)
        ) && self.deferred.take().is_some()
        {
            // These operations do not request playback. Honour a pending pause before
            // they change the timeline, rather than restoring sound under a paused intent.
            self.command_now(Command::Pause)?;
        }
        if self.is_web()
            && matches!(
                &command,
                Command::PlayNext(_)
                    | Command::PlayNextNow(_)
                    | Command::Remove(_, _)
                    | Command::Repeat
                    | Command::Shuffle
                    | Command::Seek(_)
                    | Command::AppendFm { .. }
                    | Command::SelectSource { .. }
            )
        {
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

    fn finish_deferred(&mut self) {
        let ready = self.deferred.as_ref().is_some_and(|(_, end, started)| {
            self.native()
                .playbin
                .current_running_time()
                .is_some_and(|t| t.mseconds() >= *end + super::transitions::PAUSE_GRACE_MS)
                || started.elapsed()
                    >= Duration::from_millis(super::transitions::PLAYBACK_FADE_MS + 100)
        });
        if ready {
            let (command, _, _) = self.deferred.take().unwrap();
            if let Err(error) = self.command_now(command) {
                self.native().fade.reset();
                self.state.error = Some(error);
                self.publish();
            }
        }
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
            Command::Engine(engine, reply) => {
                if engine == self.state.playback_engine {
                    let _ = reply.send(Ok(()));
                } else if engine == PlaybackEngine::WebAudio && !cfg!(windows) {
                    let _ = reply.send(Err("WebAudio 暂仅支持 Windows".into()));
                } else if self.is_web() {
                    let (native, device_id) = match self.prepare_engine(engine) {
                        Ok(prepared) => prepared,
                        Err(error) => {
                            let _ = reply.send(Err(error));
                            return Ok(());
                        }
                    };
                    let control = self.web_audio.control();
                    let sample_position = control.source.is_some();
                    let preloaded = if let Some(next) = control.next {
                        self.prepared
                            .lock()
                            .map_err(|_| "预加载状态不可用")?
                            .take()
                            .map(|prepared| (next.source, prepared))
                    } else {
                        None
                    };
                    self.cancel_jobs();
                    self.web_audio.deactivate(&self.app)?;
                    self.engine_switch = Some(EngineSwitch {
                        target: engine,
                        reply,
                        started: Instant::now(),
                        sample_position,
                        native,
                        device_id,
                        preloaded,
                    });
                } else {
                    let result = self.prepare_engine(engine).and_then(|(native, device_id)| {
                        self.switch_engine(engine, native, device_id)
                    });
                    let _ = reply.send(result.clone());
                    result?;
                }
            }
            Command::WebReady(reply) => {
                if self.is_web() {
                    self.web_audio.intent(
                        &self.app,
                        self.desired_playing,
                        self.state.volume,
                        Some(self.state.position_ms),
                    )?;
                }
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
                        crate::infrastructure::diagnostics::warn("optional_result", error);
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
                // Lock in the streaming callback's order while shifting history.
                let (trim, displaced) = {
                    let mut prepared = self
                        .prepared
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "预加载状态不可用")?;
                    let mut armed = self
                        .armed
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "预加载状态不可用")?;
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
                // Hold the prepared lock while clearing armed state so the streaming
                // callback cannot arm the old next track between these operations.
                let transitioning = {
                    let mut prepared = self
                        .prepared
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "预加载状态不可用")?;
                    *prepared = None;
                    self.armed
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
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
                self.selections.remove(&key);
                let loading = self.state.status == PlaybackStatus::Loading;
                // Use the same lock order as the streaming callback, so it cannot
                // arm a removed next track between checking and invalidating it.
                let transitioning = {
                    let mut prepared = self
                        .prepared
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "预加载状态不可用")?;
                    *prepared = None;
                    self.armed
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
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
                // Match the streaming callback's lock order. If it has already
                // handed over a URI, reload that entry rather than losing the
                // StreamStart metadata while changing the future play order.
                let transitioning = {
                    let mut prepared = self
                        .prepared
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "预加载状态不可用")?;
                    *prepared = None;
                    self.armed
                        .lock()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "预加载状态不可用")?
                        .take()
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
                if self.is_web() {
                    self.web_audio
                        .intent(&self.app, false, self.state.volume, None)?;
                } else {
                    self.native()
                        .playbin
                        .set_state(gst::State::Paused)
                        .map_err(|e| e.to_string())?;
                }
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
                        if self.is_web() {
                            self.web_audio
                                .intent(&self.app, true, self.state.volume, None)?;
                            return Ok(());
                        }
                        if reversing {
                            if let Some(time) = self.native().playbin.current_running_time() {
                                self.native().fade.reverse_pause(time);
                            } else {
                                self.native().fade.reset();
                            }
                        } else if self.native().playbin.current_state() != gst::State::Playing {
                            self.native().fade.prepare_start();
                        }
                        if self.pending_seek.is_none()
                            && (!cfg!(target_os = "android") || self.start_gate.ready())
                        {
                            self.native()
                                .playbin
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
                if self.is_web() {
                    self.web_audio.stop(&self.app)?;
                }
                if let Some(native) = &self.native {
                    native
                        .playbin
                        .set_state(gst::State::Null)
                        .map_err(|e| e.to_string())?;
                    native.fade.reset();
                }
                self.decoded_audio = None;
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
                if let Some(native) = &self.native {
                    native.playbin.set_property("volume", value);
                }
                self.state.volume = value;
                if self.is_web() {
                    self.web_audio
                        .intent(&self.app, self.desired_playing, value, None)?;
                }
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
                    if !(if self.is_web() {
                        self.state.playback_options.crossfade
                    } else {
                        self.state.playback_options.gapless
                    }) || self.state.following(false) != Some(index)
                        || self
                            .state
                            .queue
                            .get(index)
                            .is_none_or(|track| track.key != key)
                    {
                        return Ok(());
                    }
                    self.next_job = None;
                    if let Ok((resolved, resource, source)) = (*result).inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_success", error);
                    }) {
                        if resource
                            .as_ref()
                            .is_some_and(|r| r.check(generation).is_err())
                        {
                            return Ok(());
                        }
                        if self.is_web() {
                            self.web_audio.prepare(&self.app, resolved.clone())?;
                        }
                        *self
                            .prepared
                            .lock()
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("translate_error", error);
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
                            if self.is_web() {
                                self.pending_seek = None;
                                self.web_audio.load(
                                    &self.app,
                                    resolved,
                                    self.state.position_ms,
                                    self.desired_playing,
                                    self.state.volume,
                                )?;
                                return Ok(());
                            }
                            crate::audio::start_stream(
                                &self.native().playbin,
                                &resolved.uri,
                                self.state.volume,
                                if self.desired_playing
                                    && self.pending_seek.is_none()
                                    && (!cfg!(target_os = "android") || self.start_gate.ready())
                                {
                                    gst::State::Playing
                                } else {
                                    gst::State::Paused
                                },
                            )
                            .map_err(|e| e.to_string())?;
                            self.decoded_audio = resolved.decoded_audio;
                        }
                        Err(error) => {
                            self.skip_failed(error)?;
                        }
                    }
                }
            }
            Command::Devices(reply) => {
                let _ = (reply.send(Ok(self
                    .devices()
                    .into_iter()
                    .map(|(info, _)| info)
                    .collect())))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("reply.send", error);
                });
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
        if options.crossfade && options.gapless {
            return Err("交叉淡入淡出与无缝播放不能同时开启".into());
        }
        // Persist before changing live behavior; a failed write preserves both switches.
        self.store.set_setting(
            "playbackOptions",
            &serde_json::to_string(&options).map_err(|e| e.to_string())?,
        )?;
        self.state.playback_options = options;
        if self.is_web() {
            self.web_audio.configure(&self.state);
            if !options.crossfade {
                if let Some(job) = self.next_job.take() {
                    job.abort();
                }
                self.prepared.lock().map_err(|_| "预加载状态不可用")?.take();
                self.web_audio.cancel_preload(&self.app)?;
            } else {
                self.web_audio.emit(&self.app)?;
            }
            self.publish();
            return Ok(());
        }
        if !options.fades {
            if let Some((command, _, _)) = self.deferred.take() {
                self.command_now(command)?;
            }
        }
        let transitioning = {
            let mut prepared = self
                .prepared
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "预加载状态不可用")?;
            self.gapless_enabled
                .store(options.gapless, Ordering::Release);
            if !options.gapless {
                *prepared = None;
                self.armed
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "预加载状态不可用")?
                    .take()
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
        self.native().fade.set_enabled(options.fades);
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
        if self.is_web() {
            self.web_audio.stop(&self.app)?;
        }
        if let Some(native) = &self.native {
            native
                .playbin
                .set_state(gst::State::Null)
                .map_err(|e| e.to_string())?;
            native.fade.reset();
            if playing {
                native.fade.prepare_start();
            }
        }
        self.decoded_audio = None;
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
        self.pending_seek =
            (self.state.position_ms > 0 || track.cue().is_some()).then_some(self.state.position_ms);
        self.awaiting_first_stream = true;
        self.start_gate.prepare(cfg!(target_os = "android"));
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
        let sender = self.sender.clone();
        let quality = self
            .store
            .setting("quality")
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_result", error);
            })
            .ok()
            .flatten()
            .unwrap_or_else(|| "exhigh".into());
        let downgrade = self
            .store
            .setting("allowDowngrade")
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_result", error);
            })
            .ok()
            .flatten()
            .as_deref()
            != Some("false");
        let task = tauri::async_runtime::spawn(async move {
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
        self.deferred = None;
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Some(job) = self.current_job.take() {
            job.abort();
        }
        if let Some(job) = self.next_job.take() {
            job.abort();
        }
        if let Ok(mut slot) = (self.prepared.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_success", error);
        }) {
            *slot = None;
        }
        if let Ok(mut slot) = (self.armed.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_success", error);
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
        self.deferred = None;
        if let Some(native) = &self.native {
            native.fade.reset();
        }
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
        if self.is_web() {
            self.state.position_ms = ms;
            if self.current_job.is_none() {
                self.web_audio.intent(
                    &self.app,
                    self.desired_playing,
                    self.state.volume,
                    Some(ms),
                )?;
            }
            self.publish();
            return Ok(());
        }
        if self.state.status == PlaybackStatus::Loading || self.pending_seek.is_some() {
            self.pending_seek = Some(ms);
            self.state.position_ms = ms;
            self.publish();
            return Ok(());
        }
        crate::audio::seek_track(
            &self.native().playbin,
            self.state.current().ok_or("没有当前歌曲")?,
            ms,
        )
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "当前资源不支持定位")?;
        self.state.position_ms = ms;
        self.publish();
        Ok(())
    }

    fn message(&mut self, message: &gst::MessageRef) {
        use gst::MessageView;
        super::diagnostics::message(message);
        match message.view() {
            MessageView::AsyncDone(_) => {
                super::diagnostics::output(&self.native().playbin);
                let preroll = self.start_gate.complete();
                let mut resume = cfg!(target_os = "android") && preroll;
                if let Some(position) = self.pending_seek.take() {
                    // The resource has finished preroll; seek the pipeline directly.
                    let Some(track) = self.state.current() else {
                        return;
                    };
                    if let Err(error) =
                        crate::audio::seek_track(&self.native().playbin, track, position)
                    {
                        self.state.error = Some(format!("恢复播放位置失败：{error}"));
                        self.state.status = PlaybackStatus::Error;
                        self.desired_playing = false;
                        let _ = self.native().playbin.set_state(gst::State::Null);
                        self.publish();
                        return;
                    }
                    resume = true;
                }
                if resume
                    && self.desired_playing
                    && (!cfg!(target_os = "android") || self.start_gate.ready())
                {
                    if let Err(error) = self.native().playbin.set_state(gst::State::Playing) {
                        self.state.error = Some(error.to_string());
                        self.publish();
                    }
                }
            }
            MessageView::StreamStart(_) => {
                if self.awaiting_first_stream {
                    self.awaiting_first_stream = false;
                    self.publish();
                    return;
                }
                let armed = self
                    .armed
                    .lock()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
                    .and_then(|mut slot| slot.take());
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
                    self.state.playback_source = next.source;
                    self.decoded_audio = next.resolved.decoded_audio;
                    self.play_instance_id = uuid::Uuid::new_v4().to_string();
                    self.recorded_instance_id = None;
                }
                self.publish();
            }
            MessageView::StateChanged(change)
                if message.src().is_some_and(|s| s == &self.native().playbin)
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
                        if status == PlaybackStatus::Playing {
                            self.record_playback_history();
                        }
                        self.publish();
                    }
                }
            }
            MessageView::Buffering(value) => {
                self.start_gate.buffering(value.percent());
                if !self.desired_playing || self.pending_seek.is_some() {
                    return;
                }
                if cfg!(target_os = "android") && !self.start_gate.ready() {
                    self.state.status = PlaybackStatus::Buffering;
                    if let Err(error) = self.native().playbin.set_state(gst::State::Paused) {
                        self.state.error = Some(error.to_string());
                    }
                    self.publish();
                    return;
                }
                match crate::audio::update_buffering(&self.native().playbin, value.percent()) {
                    Ok(status) => self.state.status = status,
                    Err(error) => self.state.error = Some(error.to_string()),
                }
                self.publish();
            }
            MessageView::DurationChanged(_) => {
                self.state.duration_ms = crate::audio::track_duration(
                    self.state.current(),
                    self.native()
                        .playbin
                        .query_duration::<gst::ClockTime>()
                        .map_or(self.state.duration_ms, |t| t.mseconds()),
                );
            }
            MessageView::Eos(_) => {
                if self.deferred.take().is_some() || !self.desired_playing {
                    // EOS can beat the last few milliseconds of a pause. Leave a resumable
                    // stopped track, not a PAUSED pipeline that has already exhausted its stream.
                    self.desired_playing = false;
                    self.state.status = PlaybackStatus::Stopped;
                    self.state.position_ms = self.state.duration_ms;
                    let _ =
                        (self.native().playbin.set_state(gst::State::Null)).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "self.native().playbin.set_state",
                                error,
                            );
                        });
                    self.native().fade.reset();
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
                    self.native().fade.reset();
                } else {
                    self.desired_playing = false;
                    self.state.status = PlaybackStatus::Stopped;
                    self.state.position_ms = self.state.duration_ms;
                    let _ =
                        (self.native().playbin.set_state(gst::State::Null)).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "self.native().playbin.set_state",
                                error,
                            );
                        });
                    self.decoded_audio = None;
                    self.publish();
                }
            }
            MessageView::Error(error) => {
                if self.desired_playing {
                    self.cancel_jobs();
                    let _ = self.native().playbin.set_state(gst::State::Null);
                    self.native().fade.reset();
                    if let Err(cause) = self.skip_failed(format!("音频播放失败：{}", error.error()))
                    {
                        self.state.error = Some(cause);
                        self.state.status = PlaybackStatus::Error;
                        self.desired_playing = false;
                        self.publish();
                    }
                    return;
                }
                self.cancel_jobs();
                self.desired_playing = false;
                self.state.status = PlaybackStatus::Error;
                self.state.error = Some(format!("音频播放失败：{}", error.error()));
                let _ = (self.native().playbin.set_state(gst::State::Null)).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "self.native().playbin.set_state",
                        error,
                    );
                });
                self.native().fade.reset();
                self.publish();
            }
            _ => {}
        }
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
        if self
            .engine_switch
            .as_ref()
            .is_some_and(|switch| switch.started.elapsed() > Duration::from_secs(3))
        {
            let switch = self.engine_switch.take().unwrap();
            self.desired_playing = false;
            self.state.status = PlaybackStatus::Error;
            self.state.error = Some("浏览器音频未确认停止，未启动另一个引擎，请重试".into());
            let _ = switch.reply.send(Err(self.state.error.clone().unwrap()));
            self.publish();
        }
        if self.is_web() {
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
            return;
        }
        if self.state.status == PlaybackStatus::Playing && self.state.position_ms >= 1000 {
            self.failed_keys.clear();
        }

        if let Ok(mut prepared) = (self.prepared.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_success", error);
        }) {
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
                    .native()
                    .playbin
                    .query_position::<gst::ClockTime>()
                    .map_or(self.state.position_ms, |t| {
                        crate::audio::track_position(self.state.current(), t.mseconds())
                    });
            }
            self.state.duration_ms = crate::audio::track_duration(
                self.state.current(),
                self.native()
                    .playbin
                    .query_duration::<gst::ClockTime>()
                    .map_or(self.state.duration_ms, |t| t.mseconds()),
            );
            self.update_media();
            if let Ok(mut shared) = (self.shared.write()).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_success", error);
            }) {
                let _ = (self.app.emit("player-progress", shared.tick(&self.state))).inspect_err(
                    |error| {
                        crate::infrastructure::diagnostics::warn("self.app.emit", error);
                    },
                );
            }
            if self.state.playback_options.gapless
                && self
                    .state
                    .current()
                    .is_none_or(|track| track.cue().is_none())
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
                        if self.state.queue[index].cue().is_some() {
                            return;
                        }
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
        if let Ok(mut shared) = (self.shared.write()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_success", error);
        }) {
            let (state, queue, progress) = shared.update(&self.state);
            if let Some(queue) = queue {
                let _ = (self.app.emit("player-queue", queue)).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("self.app.emit", error);
                });
            }
            let _ = (self.app.emit("player-state", state)).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("self.app.emit", error);
            });
            if let Some(progress) = progress {
                let _ = (self.app.emit("player-progress", progress)).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("self.app.emit", error);
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

    fn devices(&self) -> Vec<(OutputDevice, gst::Device)> {
        self.native
            .iter()
            .flat_map(|n| n.monitor.iter())
            .flat_map(|monitor| monitor.devices())
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

    fn output_sink(&self, id: &Option<String>) -> AppResult<gst::Element> {
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
        super::output::create_sink(device.as_ref())
    }

    fn select_device(&mut self, id: Option<String>) -> AppResult<()> {
        if self.is_web() {
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
        let sink = self.output_sink(&id)?;
        self.store.set_setting(
            "gstreamerDevice",
            &serde_json::to_string(&id).map_err(|e| e.to_string())?,
        )?;
        self.replace_output(sink, id)
    }

    fn replace_output(&mut self, sink: gst::Element, id: Option<String>) -> AppResult<()> {
        let playing = self.desired_playing;
        let index = self.state.index;
        let active = !matches!(
            self.state.status,
            PlaybackStatus::Stopped | PlaybackStatus::Error
        );
        let position = self.pending_seek.unwrap_or_else(|| {
            self.native()
                .playbin
                .query_position::<gst::ClockTime>()
                .map_or(self.state.position_ms, |time| {
                    crate::audio::track_position(self.state.current(), time.mseconds())
                })
        });
        self.cancel_jobs();
        self.native()
            .playbin
            .set_state(gst::State::Null)
            .map_err(|e| e.to_string())?;
        self.native()
            .fade
            .set_output(&self.native().playbin, sink)?;
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
                gapless: true,
                ..PlaybackOptions::default()
            }
        );
        assert_eq!(
            serde_json::from_str::<PlaybackOptions>("{}").unwrap(),
            PlaybackOptions::default()
        );
        for fades in [false, true] {
            for gapless in [false, true] {
                let options = PlaybackOptions {
                    fades,
                    gapless,
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
                .map(|id| Track {
                    key: id.to_string(),
                    title: id.to_string(),
                    aliases: vec![],
                    artist: String::new(),
                    artists: vec![],
                    album_id: None,
                    album: String::new(),
                    duration_ms: 180_000,
                    cover: Default::default(),
                    source: TrackSource::Netease { id },
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
        state.queue = Arc::default();
        assert!(previous_command(&state).is_none());
        state = snapshot(3, 179_000, RepeatMode::All);
        assert!(previous_command(&state).is_none());
    }
}
