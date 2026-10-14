#[cfg(windows)]
use crate::model::PlaybackStatus;
use crate::{
    model::{AppResult, PlayerSnapshot},
    player::Command,
};
use std::sync::mpsc::SyncSender;

#[cfg(windows)]
pub struct MediaControls {
    controls: windows::Media::SystemMediaTransportControls,
    button_token: i64,
    seek_token: i64,
    last_key: Option<String>,
}

#[cfg(windows)]
impl MediaControls {
    pub fn new(hwnd: isize, sender: SyncSender<Command>) -> AppResult<Self> {
        use windows::{
            core::factory,
            Foundation::TypedEventHandler,
            Media::{
                PlaybackPositionChangeRequestedEventArgs, SystemMediaTransportControls,
                SystemMediaTransportControlsButton,
                SystemMediaTransportControlsButtonPressedEventArgs,
            },
            Win32::{
                Foundation::HWND,
                System::{
                    Com::{CoInitializeEx, COINIT_MULTITHREADED},
                    WinRT::ISystemMediaTransportControlsInterop,
                },
            },
        };
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| e.to_string())?;
        }
        let interop: ISystemMediaTransportControlsInterop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()
                .map_err(|e| e.to_string())?;
        let controls: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(HWND(hwnd as *mut _)) }.map_err(|e| e.to_string())?;
        controls.SetIsEnabled(true).map_err(|e| e.to_string())?;
        controls.SetIsPlayEnabled(true).map_err(|e| e.to_string())?;
        controls
            .SetIsPauseEnabled(true)
            .map_err(|e| e.to_string())?;
        controls.SetIsStopEnabled(true).map_err(|e| e.to_string())?;
        let button_sender = sender.clone();
        let button_token = controls
            .ButtonPressed(&TypedEventHandler::<
                SystemMediaTransportControls,
                SystemMediaTransportControlsButtonPressedEventArgs,
            >::new(move |_, args| {
                if let Some(args) = args.as_ref() {
                    let command = match args.Button()? {
                        SystemMediaTransportControlsButton::Play => Some(Command::Resume),
                        SystemMediaTransportControlsButton::Pause => Some(Command::Pause),
                        SystemMediaTransportControlsButton::Stop => Some(Command::Stop),
                        SystemMediaTransportControlsButton::Next => Some(Command::Next),
                        SystemMediaTransportControlsButton::Previous => Some(Command::Previous),
                        _ => None,
                    };
                    if let Some(command) = command {
                        let _ = button_sender.try_send(command);
                    }
                }
                Ok(())
            }))
            .map_err(|e| e.to_string())?;
        let seek_token = controls
            .PlaybackPositionChangeRequested(&TypedEventHandler::<
                SystemMediaTransportControls,
                PlaybackPositionChangeRequestedEventArgs,
            >::new(move |_, args| {
                if let Some(args) = args.as_ref() {
                    let ms = args.RequestedPlaybackPosition()?.Duration.max(0) as u64 / 10_000;
                    let _ = sender.try_send(Command::Seek(ms));
                }
                Ok(())
            }))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            controls,
            button_token,
            seek_token,
            last_key: None,
        })
    }

    pub fn update(&mut self, state: &PlayerSnapshot) -> AppResult<()> {
        use windows::{
            core::HSTRING,
            Foundation::{TimeSpan, Uri},
            Media::{
                MediaPlaybackStatus, MediaPlaybackType,
                SystemMediaTransportControlsTimelineProperties,
            },
            Storage::Streams::RandomAccessStreamReference,
        };
        let controls = &self.controls;
        let status = match state.status {
            PlaybackStatus::Playing => MediaPlaybackStatus::Playing,
            PlaybackStatus::Paused => MediaPlaybackStatus::Paused,
            PlaybackStatus::Loading | PlaybackStatus::Buffering => MediaPlaybackStatus::Changing,
            _ => MediaPlaybackStatus::Stopped,
        };
        controls
            .SetPlaybackStatus(status)
            .map_err(|e| e.to_string())?;
        controls
            .SetIsNextEnabled(state.following(true).is_some())
            .map_err(|e| e.to_string())?;
        controls
            .SetIsPreviousEnabled(state.previous().is_some())
            .map_err(|e| e.to_string())?;
        if let Some(track) = state.current() {
            if self.last_key.as_deref() != Some(&track.key) {
                let updater = controls.DisplayUpdater().map_err(|e| e.to_string())?;
                updater.ClearAll().map_err(|e| e.to_string())?;
                updater
                    .SetType(MediaPlaybackType::Music)
                    .map_err(|e| e.to_string())?;
                let music = updater.MusicProperties().map_err(|e| e.to_string())?;
                music
                    .SetTitle(&HSTRING::from(&track.title))
                    .map_err(|e| e.to_string())?;
                music
                    .SetArtist(&HSTRING::from(&track.artist))
                    .map_err(|e| e.to_string())?;
                music
                    .SetAlbumTitle(&HSTRING::from(&track.album))
                    .map_err(|e| e.to_string())?;
                let thumbnail = if track.cover.starts_with("https://")
                    || track.cover.starts_with("http://")
                {
                    let uri =
                        Uri::CreateUri(&HSTRING::from(&track.cover)).map_err(|e| e.to_string())?;
                    RandomAccessStreamReference::CreateFromUri(&uri).map_err(|e| e.to_string())?
                } else {
                    let file = windows::Storage::StorageFile::GetFileFromPathAsync(&HSTRING::from(
                        &track.cover,
                    ))
                    .map_err(|e| e.to_string())?
                    .join()
                    .map_err(|e| e.to_string())?;
                    RandomAccessStreamReference::CreateFromFile(&file).map_err(|e| e.to_string())?
                };
                updater
                    .SetThumbnail(&thumbnail)
                    .map_err(|e| e.to_string())?;
                updater.Update().map_err(|e| e.to_string())?;
                self.last_key = Some(track.key.clone());
            }
        } else if self.last_key.take().is_some() {
            let updater = controls.DisplayUpdater().map_err(|e| e.to_string())?;
            updater.ClearAll().map_err(|e| e.to_string())?;
            updater.Update().map_err(|e| e.to_string())?;
        }
        let timeline =
            SystemMediaTransportControlsTimelineProperties::new().map_err(|e| e.to_string())?;
        let span = |ms: u64| TimeSpan {
            Duration: ms.min(i64::MAX as u64 / 10_000) as i64 * 10_000,
        };
        timeline.SetStartTime(span(0)).map_err(|e| e.to_string())?;
        timeline
            .SetEndTime(span(state.duration_ms))
            .map_err(|e| e.to_string())?;
        timeline
            .SetMinSeekTime(span(0))
            .map_err(|e| e.to_string())?;
        timeline
            .SetMaxSeekTime(span(state.duration_ms))
            .map_err(|e| e.to_string())?;
        timeline
            .SetPosition(span(state.position_ms.min(state.duration_ms)))
            .map_err(|e| e.to_string())?;
        controls
            .UpdateTimelineProperties(&timeline)
            .map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
impl Drop for MediaControls {
    fn drop(&mut self) {
        let _ = self.controls.RemoveButtonPressed(self.button_token);
        let _ = self
            .controls
            .RemovePlaybackPositionChangeRequested(self.seek_token);
        let _ = self.controls.SetIsEnabled(false);
        // Controls are released after Drop; balancing COM while references remain
        // would be premature. The owning thread exits immediately afterwards.
    }
}

#[cfg(all(not(windows), not(target_os = "android")))]
pub struct MediaControls;
#[cfg(all(not(windows), not(target_os = "android")))]
impl MediaControls {
    pub fn new(_: isize, _: SyncSender<Command>) -> AppResult<Self> {
        Ok(Self)
    }
    pub fn update(&mut self, _: &PlayerSnapshot) -> AppResult<()> {
        Ok(())
    }
}

#[cfg(target_os = "android")]
pub struct MediaControls {
    last: Option<crate::platform::android::playback::Snapshot>,
    last_update: std::time::Instant,
}
#[cfg(target_os = "android")]
impl MediaControls {
    pub fn new(_: isize, sender: SyncSender<Command>) -> AppResult<Self> {
        crate::platform::android::playback::initialize(sender)?;
        Ok(Self {
            last: None,
            last_update: std::time::Instant::now(),
        })
    }
    pub fn update(&mut self, state: &PlayerSnapshot) -> AppResult<()> {
        let next = crate::platform::android::playback::Snapshot::from_player(state);
        let mut same = next.clone();
        if let Some(last) = &self.last {
            same.position_ms = last.position_ms;
            same.sequence = last.sequence;
        }
        let changed = self.last.as_ref() != Some(&same);
        // State changes/seek corrections are immediate; steady progress crosses JNI at most once/sec.
        let seek = self.last.as_ref().is_some_and(|last| {
            let predicted = last.position_ms
                + if last.status == "playing" {
                    self.last_update.elapsed().as_millis() as u64
                } else {
                    0
                };
            predicted.abs_diff(next.position_ms) > 750
        });
        if changed
            || seek
            || (next.status == "playing"
                && self.last_update.elapsed() >= std::time::Duration::from_secs(1))
        {
            crate::platform::android::playback::update(&next)?;
            if !next.cover.is_empty()
                && self
                    .last
                    .as_ref()
                    .is_none_or(|last| last.key != next.key || last.cover != next.cover)
            {
                crate::platform::android::playback::artwork(next.key.clone(), next.cover.clone());
            }
            self.last = Some(next);
            self.last_update = std::time::Instant::now();
        }
        Ok(())
    }
}
#[cfg(target_os = "android")]
impl Drop for MediaControls {
    fn drop(&mut self) {
        crate::platform::android::playback::shutdown();
    }
}
