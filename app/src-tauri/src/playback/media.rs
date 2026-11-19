#[cfg(any(windows, target_os = "linux"))]
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
                    let command = match args.Button().inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("smtc.button", error)
                    })? {
                        SystemMediaTransportControlsButton::Play => Some(Command::Resume),
                        SystemMediaTransportControlsButton::Pause => Some(Command::Pause),
                        SystemMediaTransportControlsButton::Stop => Some(Command::Stop),
                        SystemMediaTransportControlsButton::Next => Some(Command::Next),
                        SystemMediaTransportControlsButton::Previous => Some(Command::Previous),
                        _ => None,
                    };
                    if let Some(command) = command {
                        let _ = (button_sender.try_send(command)).inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "button_sender.try_send",
                                error,
                            );
                        });
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
                    let ms = args
                        .RequestedPlaybackPosition()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("smtc.seek", error)
                        })?
                        .Duration
                        .max(0) as u64
                        / 10_000;
                    let _ = (sender.try_send(Command::Seek(ms))).inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("sender.try_send", error);
                    });
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
        let _ = (self.controls.RemoveButtonPressed(self.button_token)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("smtc.shutdown.unsubscribe_buttons", error);
        });
        let _ = (self
            .controls
            .RemovePlaybackPositionChangeRequested(self.seek_token))
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("smtc.shutdown.unsubscribe_seek", error);
        });
        let _ = (self.controls.SetIsEnabled(false)).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("smtc.shutdown.disable", error);
        });
        // Controls are released after Drop; balancing COM while references remain
        // would be premature. The owning thread exits immediately afterwards.
    }
}

#[cfg(target_os = "linux")]
pub use mpris::MediaControls;

/// Linux MPRIS 2 (`org.mpris.MediaPlayer2.nons`). zbus runs the bus on its own executor thread;
/// method calls only `try_send` into the playback actor, and `update` publishes property changes.
#[cfg(target_os = "linux")]
mod mpris {
    use super::{AppResult, Command, PlaybackStatus, PlayerSnapshot, SyncSender};
    use crate::model::RepeatMode;
    use nons_music_adapter_sdk::{cover::CoverSize, queue::TrackSource};
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
    use sha2::{Digest, Sha256};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
        time::Instant,
    };
    use zbus::{
        blocking::{connection, Connection},
        interface,
        object_server::SignalEmitter,
        zvariant::{ObjectPath, OwnedValue, Value},
    };

    const BUS_NAME: &str = "org.mpris.MediaPlayer2.nons";
    const PATH: &str = "/org/mpris/MediaPlayer2";
    const NO_TRACK: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";
    // A position this far from the extrapolated one is a seek, announced with `Seeked`.
    const SEEK_TOLERANCE_MS: u64 = 1500;
    const PATH_SET: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'%').add(b'?');

    #[derive(Clone, Debug, Default, PartialEq)]
    struct View {
        status: &'static str,
        track_id: String,
        title: String,
        artists: Vec<String>,
        album: String,
        length_us: i64,
        art_url: String,
        url: String,
        can_next: bool,
        can_previous: bool,
        volume: f64,
        shuffle: bool,
        repeat: RepeatMode,
    }

    const REPEAT_CYCLE: [(RepeatMode, &str); 3] = [
        (RepeatMode::Off, "None"),
        (RepeatMode::All, "Playlist"),
        (RepeatMode::One, "Track"),
    ];

    // Command::Repeat steps Off -> All -> One -> Off; the setters need no queue revision.
    fn repeat_steps(from: RepeatMode, to: RepeatMode) -> usize {
        let at = |mode| {
            REPEAT_CYCLE
                .iter()
                .position(|(m, _)| *m == mode)
                .unwrap_or(0)
        };
        (at(to) + REPEAT_CYCLE.len() - at(from)) % REPEAT_CYCLE.len()
    }
    // Stopped with a loaded track (restored queue, track ended) is Paused in MPRIS terms;
    // shells such as DMS hide Stopped players from their player picker.
    fn mpris_status(status: PlaybackStatus, has_track: bool) -> &'static str {
        match status {
            PlaybackStatus::Playing | PlaybackStatus::Loading | PlaybackStatus::Buffering => {
                "Playing"
            }
            PlaybackStatus::Paused => "Paused",
            _ if has_track => "Paused",
            _ => "Stopped",
        }
    }

    impl View {
        fn of(state: &PlayerSnapshot) -> Self {
            let mut view = Self {
                status: mpris_status(state.status, state.current().is_some()),
                track_id: NO_TRACK.into(),
                can_next: state.following(true).is_some(),
                can_previous: state.previous().is_some(),
                volume: state.volume,
                shuffle: state.shuffle,
                repeat: state.repeat_mode,
                ..Self::default()
            };
            let Some(track) = state.current() else {
                return view;
            };
            let digest = format!("{:x}", Sha256::digest(track.key.as_bytes()));
            view.track_id = format!("/org/nons/track/t{}", &digest[..32]);
            view.title = track.title.clone();
            view.artists = if track.artists.is_empty() {
                vec![track.artist.clone()]
            } else {
                track
                    .artists
                    .iter()
                    .map(|artist| artist.name.clone())
                    .collect()
            };
            view.album = track.album.clone();
            let length_ms = if state.duration_ms > 0 {
                state.duration_ms
            } else {
                track.duration_ms
            };
            view.length_us = length_ms.min(i64::MAX as u64 / 1000) as i64 * 1000;
            let cover = track.cover.url(CoverSize::Exlarge);
            view.art_url = if cover.starts_with("https://") || cover.starts_with("http://") {
                cover.to_owned()
            } else {
                file_url(cover)
            };
            // A CUE track shares its file with the other tracks, so the file says nothing about this one.
            if let TrackSource::Local {
                path, cue: None, ..
            } = &track.source
            {
                view.url = file_url(path);
            }
            view
        }

        fn metadata(&self) -> HashMap<String, OwnedValue> {
            let mut metadata = HashMap::new();
            let mut put = |key: &str, value: Value<'_>| {
                if let Ok(value) = OwnedValue::try_from(value) {
                    metadata.insert(key.to_owned(), value);
                }
            };
            if let Ok(path) = ObjectPath::try_from(self.track_id.as_str()) {
                put("mpris:trackid", path.into());
            }
            if self.track_id == NO_TRACK {
                return metadata;
            }
            put("xesam:title", self.title.as_str().into());
            put("xesam:artist", self.artists.clone().into());
            put("xesam:album", self.album.as_str().into());
            put("mpris:length", self.length_us.into());
            for (key, value) in [("mpris:artUrl", &self.art_url), ("xesam:url", &self.url)] {
                if !value.is_empty() {
                    put(key, value.as_str().into());
                }
            }
            metadata
        }
    }

    fn file_url(path: &str) -> String {
        if path.starts_with('/') {
            format!("file://{}", utf8_percent_encode(path, PATH_SET))
        } else {
            String::new()
        }
    }

    #[derive(Default)]
    struct Shared {
        view: View,
        position_ms: u64,
        sampled: Option<Instant>,
        advancing: bool,
    }

    impl Shared {
        fn position_ms(&self) -> u64 {
            let elapsed = match (self.advancing, self.sampled) {
                (true, Some(sampled)) => sampled.elapsed().as_millis() as u64,
                _ => 0,
            };
            let position = self.position_ms.saturating_add(elapsed);
            match self.view.length_us {
                0 => position,
                length => position.min(length as u64 / 1000),
            }
        }
    }

    type State = Arc<Mutex<Shared>>;

    fn read<T>(state: &State, f: impl FnOnce(&Shared) -> T) -> T {
        match state.lock() {
            Ok(shared) => f(&shared),
            Err(poisoned) => f(&poisoned.into_inner()),
        }
    }

    struct Root;

    #[interface(name = "org.mpris.MediaPlayer2")]
    impl Root {
        fn raise(&self) {}
        fn quit(&self) {}
        #[zbus(property)]
        fn can_quit(&self) -> bool {
            false
        }
        #[zbus(property)]
        fn can_raise(&self) -> bool {
            false
        }
        #[zbus(property)]
        fn has_track_list(&self) -> bool {
            false
        }
        #[zbus(property)]
        fn identity(&self) -> &str {
            "Nons"
        }
        #[zbus(property)]
        fn desktop_entry(&self) -> &str {
            "Nons"
        }
        #[zbus(property)]
        fn supported_uri_schemes(&self) -> Vec<String> {
            Vec::new()
        }
        #[zbus(property)]
        fn supported_mime_types(&self) -> Vec<String> {
            Vec::new()
        }
    }

    struct Player {
        state: State,
        sender: SyncSender<Command>,
    }

    impl Player {
        fn send(&self, command: Command) {
            let _ = (self.sender.try_send(command)).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("mpris.send", error);
            });
        }

        fn has_track(&self) -> bool {
            read(&self.state, |shared| shared.view.track_id != NO_TRACK)
        }
    }

    #[interface(name = "org.mpris.MediaPlayer2.Player")]
    impl Player {
        fn next(&self) {
            self.send(Command::Next);
        }
        fn previous(&self) {
            self.send(Command::Previous);
        }
        fn pause(&self) {
            self.send(Command::Pause);
        }
        fn play_pause(&self) {
            self.send(Command::Toggle);
        }
        fn stop(&self) {
            self.send(Command::Stop);
        }
        fn play(&self) {
            self.send(Command::Resume);
        }
        fn seek(&self, offset: i64) {
            let position = read(&self.state, Shared::position_ms) as i64;
            let target = position.saturating_add(offset / 1000).max(0);
            self.send(Command::Seek(target as u64));
        }
        fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
            let (current, length) = read(&self.state, |shared| {
                (shared.view.track_id.clone(), shared.view.length_us)
            });
            // The spec ignores stale track ids and positions outside the track.
            if track_id.as_str() == current && (0..=length).contains(&position) {
                self.send(Command::Seek(position as u64 / 1000));
            }
        }
        fn open_uri(&self, _uri: &str) {}

        #[zbus(signal)]
        async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;

        #[zbus(property)]
        fn playback_status(&self) -> &str {
            read(&self.state, |shared| match shared.view.status {
                "" => "Stopped",
                status => status,
            })
        }
        #[zbus(property)]
        fn rate(&self) -> f64 {
            1.0
        }
        #[zbus(property)]
        fn minimum_rate(&self) -> f64 {
            1.0
        }
        #[zbus(property)]
        fn maximum_rate(&self) -> f64 {
            1.0
        }
        #[zbus(property)]
        fn metadata(&self) -> HashMap<String, OwnedValue> {
            read(&self.state, |shared| shared.view.metadata())
        }
        #[zbus(property)]
        fn volume(&self) -> f64 {
            read(&self.state, |shared| shared.view.volume)
        }
        #[zbus(property)]
        fn set_volume(&mut self, volume: f64) {
            if volume.is_finite() {
                self.send(Command::Volume(volume.clamp(0.0, 1.0)));
            }
        }
        #[zbus(property)]
        fn shuffle(&self) -> bool {
            read(&self.state, |shared| shared.view.shuffle)
        }
        #[zbus(property)]
        fn set_shuffle(&mut self, value: bool) {
            if read(&self.state, |shared| shared.view.shuffle) != value {
                self.send(Command::Shuffle);
            }
        }
        #[zbus(property)]
        fn loop_status(&self) -> &str {
            let repeat = read(&self.state, |shared| shared.view.repeat);
            REPEAT_CYCLE
                .iter()
                .find(|(m, _)| *m == repeat)
                .map_or("None", |(_, s)| s)
        }
        #[zbus(property)]
        fn set_loop_status(&mut self, value: &str) {
            let Some((target, _)) = REPEAT_CYCLE.iter().find(|(_, s)| *s == value) else {
                return;
            };
            let current = read(&self.state, |shared| shared.view.repeat);
            for _ in 0..repeat_steps(current, *target) {
                self.send(Command::Repeat);
            }
        }
        #[zbus(property(emits_changed_signal = "false"))]
        fn position(&self) -> i64 {
            read(&self.state, Shared::position_ms) as i64 * 1000
        }
        #[zbus(property)]
        fn can_go_next(&self) -> bool {
            read(&self.state, |shared| shared.view.can_next)
        }
        #[zbus(property)]
        fn can_go_previous(&self) -> bool {
            read(&self.state, |shared| shared.view.can_previous)
        }
        #[zbus(property)]
        fn can_play(&self) -> bool {
            self.has_track()
        }
        #[zbus(property)]
        fn can_pause(&self) -> bool {
            self.has_track()
        }
        #[zbus(property)]
        fn can_seek(&self) -> bool {
            read(&self.state, |shared| shared.view.length_us > 0)
        }
        #[zbus(property)]
        fn can_control(&self) -> bool {
            true
        }
    }

    pub struct MediaControls {
        connection: Connection,
        state: State,
    }

    impl MediaControls {
        pub fn new(_: isize, sender: SyncSender<Command>) -> AppResult<Self> {
            let state = State::default();
            let player = Player {
                state: state.clone(),
                sender,
            };
            let connection = connection::Builder::session()
                .and_then(|builder| builder.name(BUS_NAME))
                .and_then(|builder| builder.serve_at(PATH, Root))
                .and_then(|builder| builder.serve_at(PATH, player))
                .and_then(|builder| builder.build())
                .map_err(|e| format!("MPRIS: {e}"))?;
            Ok(Self { connection, state })
        }

        pub fn update(&mut self, state: &PlayerSnapshot) -> AppResult<()> {
            let view = View::of(state);
            let (previous, seeked) = {
                let mut shared = self.state.lock().map_err(|_| "MPRIS state lock poisoned")?;
                let predicted = shared.position_ms();
                let previous = std::mem::replace(&mut shared.view, view.clone());
                shared.position_ms = state.position_ms;
                shared.sampled = Some(Instant::now());
                shared.advancing = state.status == PlaybackStatus::Playing;
                let seeked = previous.track_id == view.track_id
                    && predicted.abs_diff(state.position_ms) > SEEK_TOLERANCE_MS;
                (previous, seeked)
            };
            if previous == view && !seeked {
                return Ok(());
            }
            let iface = self
                .connection
                .object_server()
                .interface::<_, Player>(PATH)
                .map_err(|e| format!("MPRIS interface: {e}"))?;
            let emitter = iface.signal_emitter();
            let player = iface.get();
            zbus::block_on(async {
                if previous.status != view.status {
                    player.playback_status_changed(emitter).await?;
                }
                if previous.metadata() != view.metadata() {
                    player.metadata_changed(emitter).await?;
                    player.can_play_changed(emitter).await?;
                    player.can_pause_changed(emitter).await?;
                    player.can_seek_changed(emitter).await?;
                }
                if previous.can_next != view.can_next {
                    player.can_go_next_changed(emitter).await?;
                }
                if previous.can_previous != view.can_previous {
                    player.can_go_previous_changed(emitter).await?;
                }
                if previous.volume != view.volume {
                    player.volume_changed(emitter).await?;
                }
                if previous.shuffle != view.shuffle {
                    player.shuffle_changed(emitter).await?;
                }
                if previous.repeat != view.repeat {
                    player.loop_status_changed(emitter).await?;
                }
                if seeked {
                    Player::seeked(emitter, state.position_ms as i64 * 1000).await?;
                }
                Ok::<_, zbus::Error>(())
            })
            .map_err(|e| format!("MPRIS signal: {e}"))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn file_urls_are_absolute_and_escaped() {
            assert_eq!(
                file_url("/music/a b#1.flac"),
                "file:///music/a%20b%231.flac"
            );
            assert_eq!(file_url(""), "");
            assert_eq!(file_url("relative.jpg"), "");
        }

        #[test]
        fn position_stops_at_track_end() {
            let shared = Shared {
                view: View {
                    length_us: 2_000_000,
                    ..View::default()
                },
                position_ms: 5_000,
                sampled: Some(Instant::now()),
                advancing: true,
            };
            assert_eq!(shared.position_ms(), 2_000);
        }

        #[test]
        fn empty_player_reports_no_track() {
            let metadata = View {
                track_id: NO_TRACK.into(),
                ..View::default()
            }
            .metadata();
            assert_eq!(metadata.len(), 1);
            assert!(metadata.contains_key("mpris:trackid"));
        }

        #[test]
        fn loaded_track_is_never_stopped() {
            assert_eq!(mpris_status(PlaybackStatus::Stopped, true), "Paused");
            assert_eq!(mpris_status(PlaybackStatus::Error, true), "Paused");
            assert_eq!(mpris_status(PlaybackStatus::Stopped, false), "Stopped");
            assert_eq!(mpris_status(PlaybackStatus::Buffering, true), "Playing");
        }

        #[test]
        fn repeat_steps_follow_the_cycle() {
            assert_eq!(repeat_steps(RepeatMode::Off, RepeatMode::Off), 0);
            assert_eq!(repeat_steps(RepeatMode::Off, RepeatMode::One), 2);
            assert_eq!(repeat_steps(RepeatMode::One, RepeatMode::Off), 1);
            assert_eq!(repeat_steps(RepeatMode::All, RepeatMode::Off), 2);
        }
    }
}

#[cfg(all(not(windows), not(target_os = "android"), not(target_os = "linux")))]
pub struct MediaControls;
#[cfg(all(not(windows), not(target_os = "android"), not(target_os = "linux")))]
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
