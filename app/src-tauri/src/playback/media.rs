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
                .map_err(|e| format!("CoInitializeEx: {e}"))?;
        }
        let interop: ISystemMediaTransportControlsInterop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()
                .map_err(|e| format!("factory: {e}"))?;
        let controls: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(HWND(hwnd as *mut _)) }
                .map_err(|e| format!("GetForWindow: {e}"))?;
        controls
            .SetIsEnabled(true)
            .map_err(|e| format!("SetIsEnabled: {e}"))?;
        controls
            .SetIsPlayEnabled(true)
            .map_err(|e| format!("SetIsPlayEnabled: {e}"))?;
        controls
            .SetIsPauseEnabled(true)
            .map_err(|e| format!("SetIsPauseEnabled: {e}"))?;
        controls
            .SetIsStopEnabled(true)
            .map_err(|e| format!("SetIsStopEnabled: {e}"))?;
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
            .map_err(|e| format!("ButtonPressed: {e}"))?;
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
            .map_err(|e| format!("PlaybackPositionChangeRequested: {e}"))?;
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
            Foundation::TimeSpan,
            Media::{
                MediaPlaybackStatus, MediaPlaybackType,
                SystemMediaTransportControlsTimelineProperties,
            },
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
            .map_err(|e| format!("SetPlaybackStatus: {e}"))?;
        controls
            .SetIsNextEnabled(state.following(true).is_some())
            .map_err(|e| format!("SetIsNextEnabled: {e}"))?;
        controls
            .SetIsPreviousEnabled(state.previous().is_some())
            .map_err(|e| format!("SetIsPreviousEnabled: {e}"))?;
        if let Some(track) = state.current() {
            if self.last_key.as_deref() != Some(&track.key) {
                let updater = controls
                    .DisplayUpdater()
                    .map_err(|e| format!("DisplayUpdater: {e}"))?;
                updater.ClearAll().map_err(|e| format!("ClearAll: {e}"))?;
                updater
                    .SetType(MediaPlaybackType::Music)
                    .map_err(|e| format!("SetType: {e}"))?;
                let music = updater
                    .MusicProperties()
                    .map_err(|e| format!("MusicProperties: {e}"))?;
                music
                    .SetTitle(&HSTRING::from(&track.title))
                    .map_err(|e| format!("SetTitle: {e}"))?;
                music
                    .SetArtist(&HSTRING::from(&track.artist))
                    .map_err(|e| format!("SetArtist: {e}"))?;
                music
                    .SetAlbumTitle(&HSTRING::from(&track.album))
                    .map_err(|e| format!("SetAlbumTitle: {e}"))?;
                let cover = track
                    .cover
                    .url(nons_music_adapter_sdk::cover::CoverSize::Exlarge);
                // A missing cover is valid metadata, not a file with an empty path.
                if let Some(thumbnail) = thumbnail(cover)? {
                    updater
                        .SetThumbnail(&thumbnail)
                        .map_err(|e| format!("SetThumbnail: {e}"))?;
                } else {
                    log::info!(target: "nons::media", "operation=thumbnail outcome=absent track_key={}", track.key);
                }
                updater.Update().map_err(|e| format!("Update: {e}"))?;
                self.last_key = Some(track.key.clone());
            }
        } else if self.last_key.take().is_some() {
            let updater = controls
                .DisplayUpdater()
                .map_err(|e| format!("DisplayUpdater: {e}"))?;
            updater.ClearAll().map_err(|e| format!("ClearAll: {e}"))?;
            updater.Update().map_err(|e| format!("Update: {e}"))?;
        }
        let timeline = SystemMediaTransportControlsTimelineProperties::new()
            .map_err(|e| format!("SystemMediaTransportControlsTimelineProperties::new: {e}"))?;
        let span = |ms: u64| TimeSpan {
            Duration: ms.min(i64::MAX as u64 / 10_000) as i64 * 10_000,
        };
        timeline
            .SetStartTime(span(0))
            .map_err(|e| format!("SetStartTime: {e}"))?;
        timeline
            .SetEndTime(span(state.duration_ms))
            .map_err(|e| format!("SetEndTime: {e}"))?;
        timeline
            .SetMinSeekTime(span(0))
            .map_err(|e| format!("SetMinSeekTime: {e}"))?;
        timeline
            .SetMaxSeekTime(span(state.duration_ms))
            .map_err(|e| format!("SetMaxSeekTime: {e}"))?;
        timeline
            .SetPosition(span(state.position_ms.min(state.duration_ms)))
            .map_err(|e| format!("SetPosition: {e}"))?;
        controls
            .UpdateTimelineProperties(&timeline)
            .map_err(|e| format!("UpdateTimelineProperties: {e}"))
    }
}

#[cfg(windows)]
fn thumbnail(
    cover: &str,
) -> AppResult<Option<windows::Storage::Streams::RandomAccessStreamReference>> {
    use windows::{core::HSTRING, Foundation::Uri, Storage::Streams::RandomAccessStreamReference};
    if cover.is_empty() {
        return Ok(None);
    }
    let thumbnail = if cover.starts_with("https://") || cover.starts_with("http://") {
        let uri = Uri::CreateUri(&HSTRING::from(cover)).map_err(|e| format!("CreateUri: {e}"))?;
        RandomAccessStreamReference::CreateFromUri(&uri)
            .map_err(|e| format!("CreateFromUri: {e}"))?
    } else {
        let file = windows::Storage::StorageFile::GetFileFromPathAsync(&HSTRING::from(cover))
            .map_err(|e| format!("GetFileFromPathAsync: {e}"))?
            .join()
            .map_err(|e| format!("GetFileFromPathAsync.await: {e}"))?;
        RandomAccessStreamReference::CreateFromFile(&file)
            .map_err(|e| format!("CreateFromFile: {e}"))?
    };
    Ok(Some(thumbnail))
}

#[cfg(all(test, windows))]
mod tests {
    use super::thumbnail;

    #[test]
    fn missing_cover_is_valid_metadata() {
        assert!(thumbnail("").unwrap().is_none());
    }

    #[test]
    fn invalid_cover_preserves_windows_api_and_hresult() {
        use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap() };
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing-cover.png");
        let error = thumbnail(missing.to_str().unwrap()).unwrap_err();
        assert!(error.starts_with("GetFileFromPathAsync"), "{error}");
        assert!(error.contains("0x80070002"), "{error}");
        unsafe { CoUninitialize() };
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
