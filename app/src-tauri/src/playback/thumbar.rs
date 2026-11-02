use crate::{
    model::{AppResult, PlaybackStatus, PlayerSnapshot},
    player::Command,
};
use std::sync::mpsc::SyncSender;
#[cfg(windows)]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
#[cfg(windows)]
use tauri::Manager;

#[cfg(windows)]
const PREVIOUS: u32 = 1;
#[cfg(windows)]
const TOGGLE: u32 = 2;
#[cfg(windows)]
const NEXT: u32 = 3;

#[cfg(windows)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Palette {
    previous: &'static str,
    next: &'static str,
    play: &'static str,
    pause: &'static str,
}

#[cfg(windows)]
impl Palette {
    const DARK: Self = Self {
        previous: "icons/taskbar/previous-white.ico",
        next: "icons/taskbar/next-white.ico",
        play: "icons/taskbar/play-white.ico",
        pause: "icons/taskbar/pause-white.ico",
    };
    const LIGHT: Self = Self {
        previous: "icons/taskbar/previous-black.ico",
        next: "icons/taskbar/next-black.ico",
        play: "icons/taskbar/play-black.ico",
        pause: "icons/taskbar/pause-black.ico",
    };

    const fn for_theme(theme: tauri::Theme) -> Self {
        match theme {
            tauri::Theme::Light => Self::LIGHT,
            _ => Self::DARK,
        }
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct State {
    playing: bool,
    previous: bool,
    next: bool,
    active: bool,
}

#[cfg(windows)]
pub struct ThumbarControls {
    window: tauri::WebviewWindow,
    palette: Palette,
    last: Option<State>,
    cover: Option<(String, String)>,
    cover_generation: Arc<AtomicU64>,
    cover_lock: Arc<Mutex<()>>,
}

#[cfg(windows)]
impl ThumbarControls {
    /// Windows only lets a window add thumbnail toolbar buttons once, so the
    /// complete list is installed here and later changes go through `update`.
    pub fn new(window: &tauri::WebviewWindow, sender: SyncSender<Command>) -> AppResult<Self> {
        use tauri_plugin_thumbar::{Button, ButtonHandler, ThumbarExt};
        let palette = Palette::for_theme(window.theme().unwrap_or(tauri::Theme::Dark));
        let handler: ButtonHandler = Box::new(move |_, id| {
            let command = match id {
                PREVIOUS => Command::Previous,
                TOGGLE => Command::Toggle,
                NEXT => Command::Next,
                _ => return,
            };
            let _ = sender.try_send(command).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("thumbar.command", error);
            });
        });
        window
            .app_handle()
            .thumbar()
            .initialize(
                window,
                vec![
                    Button::new(PREVIOUS, palette.previous)
                        .tooltip("上一首")
                        .enabled(false),
                    Button::new(TOGGLE, palette.play)
                        .tooltip("播放")
                        .enabled(false),
                    Button::new(NEXT, palette.next)
                        .tooltip("下一首")
                        .enabled(false),
                ],
                Some(handler),
            )
            .map_err(|error| format!("thumbar.initialize: {error}"))?;
        Ok(Self {
            window: window.clone(),
            palette,
            last: None,
            cover: None,
            cover_generation: Arc::new(AtomicU64::new(0)),
            cover_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn update(&mut self, state: &PlayerSnapshot) -> AppResult<()> {
        use tauri_plugin_thumbar::ButtonUpdate;
        self.sync_cover(state);
        let next = State {
            playing: state.status == PlaybackStatus::Playing,
            previous: state.previous().is_some(),
            next: state.following(true).is_some(),
            active: state.current().is_some(),
        };
        if self.last.as_ref() == Some(&next) {
            return Ok(());
        }
        self.apply(ButtonUpdate::new(PREVIOUS).enabled(next.previous))?;
        self.apply(ButtonUpdate::new(NEXT).enabled(next.next))?;
        self.apply(self.toggle_update(next.playing).enabled(next.active))?;
        self.last = Some(next);
        Ok(())
    }

    pub fn set_theme(&mut self, theme: tauri::Theme) -> AppResult<()> {
        use tauri_plugin_thumbar::ButtonUpdate;
        let palette = Palette::for_theme(theme);
        if self.palette == palette {
            return Ok(());
        }
        self.palette = palette;
        self.apply(ButtonUpdate::new(PREVIOUS).icon(palette.previous))?;
        self.apply(ButtonUpdate::new(NEXT).icon(palette.next))?;
        self.apply(self.toggle_update(self.last.is_some_and(|state| state.playing)))
    }

    fn sync_cover(&mut self, state: &PlayerSnapshot) {
        let current = state.current().map(|track| {
            (
                track.key.clone(),
                track
                    .cover
                    .url(nons_music_adapter_sdk::cover::CoverSize::Exlarge)
                    .to_owned(),
            )
        });
        if self.cover == current {
            return;
        }
        self.cover = current.clone();
        let generation = self.cover_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let Some((key, source)) = current else {
            self.apply_cover(generation, None);
            return;
        };
        if source.is_empty() {
            self.apply_cover(generation, None);
            return;
        }
        if !source.starts_with("http://") && !source.starts_with("https://") {
            self.apply_cover(
                generation,
                Some(tauri_plugin_thumbar::Cover::from_path(source)),
            );
            return;
        }
        let window = self.window.clone();
        let cover_generation = self.cover_generation.clone();
        let cover_lock = self.cover_lock.clone();
        tauri::async_runtime::spawn(async move {
            let app = window.app_handle().clone();
            let path = match crate::infrastructure::covers::fetch(
                source,
                &app.state::<crate::Backend>(),
                &app,
            )
            .await
            {
                Ok(path) => path,
                Err(error) => {
                    crate::infrastructure::diagnostics::warn(
                        "thumbar.cover.fetch",
                        format!("{key}: {error}"),
                    );
                    apply_cover(&window, &cover_lock, &cover_generation, generation, None);
                    return;
                }
            };
            apply_cover(
                &window,
                &cover_lock,
                &cover_generation,
                generation,
                Some(tauri_plugin_thumbar::Cover::from_path(path)),
            );
        });
    }

    fn apply_cover(&self, generation: u64, cover: Option<tauri_plugin_thumbar::Cover>) {
        apply_cover(
            &self.window,
            &self.cover_lock,
            &self.cover_generation,
            generation,
            cover,
        );
    }

    fn toggle_update(&self, playing: bool) -> tauri_plugin_thumbar::ButtonUpdate {
        use tauri_plugin_thumbar::ButtonUpdate;
        if playing {
            ButtonUpdate::new(TOGGLE)
                .icon(self.palette.pause)
                .tooltip("暂停")
        } else {
            ButtonUpdate::new(TOGGLE)
                .icon(self.palette.play)
                .tooltip("播放")
        }
    }

    fn apply(&self, update: tauri_plugin_thumbar::ButtonUpdate) -> AppResult<()> {
        use tauri_plugin_thumbar::ThumbarExt;
        self.window
            .app_handle()
            .thumbar()
            .update(&self.window, update)
            .map_err(|error| format!("thumbar.update: {error}"))
    }
}

#[cfg(windows)]
fn apply_cover(
    window: &tauri::WebviewWindow,
    lock: &Arc<Mutex<()>>,
    generation: &Arc<AtomicU64>,
    expected: u64,
    cover: Option<tauri_plugin_thumbar::Cover>,
) {
    use tauri_plugin_thumbar::ThumbarExt;
    let window = window.clone();
    let lock = lock.clone();
    let generation = generation.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let Ok(_guard) = lock.lock() else {
            return;
        };
        if generation.load(Ordering::SeqCst) != expected {
            return;
        }
        if let Err(error) = window.app_handle().thumbar().set_cover(&window, cover) {
            crate::infrastructure::diagnostics::warn("thumbar.cover", error);
        }
    });
}

#[cfg(not(windows))]
pub struct ThumbarControls;

#[cfg(not(windows))]
impl ThumbarControls {
    pub fn new(_: &tauri::WebviewWindow, _: SyncSender<Command>) -> AppResult<Self> {
        Ok(Self)
    }

    pub fn update(&mut self, _: &PlayerSnapshot) -> AppResult<()> {
        Ok(())
    }

    pub fn set_theme(&mut self, _: tauri::Theme) -> AppResult<()> {
        Ok(())
    }
}
