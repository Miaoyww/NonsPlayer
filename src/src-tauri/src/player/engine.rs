use std::ffi::{c_void, CString, OsStr};
use std::sync::{Arc, Mutex};

use super::ffi::{BassLib, BASS_ACTIVE_PLAYING, BASS_POS_BYTE, BASS_STREAM_AUTOFREE, BASS_STREAM_STATUS, BASS_SYNC_END, BASS_UNICODE};
use crate::error::{Error, Result};

/// Convert a file path to a null-terminated wide-string (UTF-16LE) for BASS on Windows.
#[cfg(windows)]
fn path_to_bass_ptr(path: &str) -> Result<(Vec<u16>, *const c_void)> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let ptr = wide.as_ptr() as *const c_void;
    Ok((wide, ptr))
}

#[cfg(not(windows))]
fn path_to_bass_ptr(path: &str) -> Result<(Vec<u8>, *const c_void)> {
    let cstr = CString::new(path).map_err(|e| Error::Other(e.to_string()))?;
    let ptr = cstr.as_ptr() as *const c_void;
    Ok((cstr.into_bytes_with_nul(), ptr))
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerState {
    Idle,
    Playing,
    Paused,
    Stopped,
}

/// Thread-safe wrapper around the BASS audio engine.
/// All `unsafe` FFI calls are encapsulated here.
pub struct BassEngine {
    bass: Arc<BassLib>,
    current_stream: Mutex<Option<u32>>,
    volume: Mutex<f32>,
    state: Mutex<PlayerState>,
}

impl BassEngine {
    pub fn new(bass: Arc<BassLib>) -> Result<Self> {
        let result = unsafe { (bass.BASS_Init)(-1, 44100, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        if result == 0 {
            return Err(Error::BassError(unsafe { (bass.BASS_ErrorGetCode)() }));
        }

        Ok(Self {
            bass,
            current_stream: Mutex::new(None),
            volume: Mutex::new(0.8),
            state: Mutex::new(PlayerState::Idle),
        })
    }

    /// Play audio from a URL.
    pub fn play_url(&self, url: &str) -> Result<()> {
        self.stop_current();

        let url_c = CString::new(url).map_err(|e| Error::Other(e.to_string()))?;

        let stream = unsafe {
            (self.bass.BASS_StreamCreateURL)(
                url_c.as_ptr(),
                0,
                BASS_STREAM_STATUS | BASS_STREAM_AUTOFREE | BASS_UNICODE,
                None,
                std::ptr::null_mut(),
            )
        };

        if stream == 0 {
            return Err(Error::BassError(unsafe { (self.bass.BASS_ErrorGetCode)() }));
        }

        unsafe { (self.bass.BASS_ChannelPlay)(stream, 0); }
        *self.current_stream.lock().unwrap() = Some(stream);
        *self.state.lock().unwrap() = PlayerState::Playing;
        Ok(())
    }

    /// Play audio from a local file path.
    pub fn play_file(&self, path: &str) -> Result<()> {
        self.stop_current();

        let (_buf, ptr) = path_to_bass_ptr(path)?;

        let stream = unsafe {
            (self.bass.BASS_StreamCreateFile)(
                0,
                ptr,
                0,
                0,
                BASS_STREAM_AUTOFREE | BASS_UNICODE,
            )
        };

        if stream == 0 {
            return Err(Error::BassError(unsafe { (self.bass.BASS_ErrorGetCode)() }));
        }

        unsafe { (self.bass.BASS_ChannelPlay)(stream, 0); }
        *self.current_stream.lock().unwrap() = Some(stream);
        *self.state.lock().unwrap() = PlayerState::Playing;
        Ok(())
    }

    /// Pause the current stream.
    pub fn pause(&self) {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe { (self.bass.BASS_ChannelPause)(stream); }
            *self.state.lock().unwrap() = PlayerState::Paused;
        }
    }

    /// Resume the paused stream.
    pub fn resume(&self) {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe { (self.bass.BASS_ChannelPlay)(stream, 0); }
            *self.state.lock().unwrap() = PlayerState::Playing;
        }
    }

    /// Toggle between play and pause.
    pub fn toggle_playback(&self) {
        match *self.state.lock().unwrap() {
            PlayerState::Playing => self.pause(),
            PlayerState::Paused => self.resume(),
            _ => {}
        }
    }

    /// Seek to a position in seconds.
    pub fn seek(&self, seconds: f64) {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe {
                let bytes = (self.bass.BASS_ChannelSeconds2Bytes)(stream, seconds);
                (self.bass.BASS_ChannelSetPosition)(stream, bytes, BASS_POS_BYTE);
            }
        }
    }

    /// Get the current playback position in seconds.
    pub fn get_position(&self) -> f64 {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe {
                let bytes = (self.bass.BASS_ChannelGetPosition)(stream, BASS_POS_BYTE);
                (self.bass.BASS_ChannelBytes2Seconds)(stream, bytes)
            }
        } else {
            0.0
        }
    }

    /// Get the total duration of the current stream in seconds.
    pub fn get_duration(&self) -> f64 {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe {
                let bytes = (self.bass.BASS_ChannelGetLength)(stream, BASS_POS_BYTE);
                (self.bass.BASS_ChannelBytes2Seconds)(stream, bytes)
            }
        } else {
            0.0
        }
    }

    /// Set the master volume (0.0 ~ 1.0).
    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        unsafe { (self.bass.BASS_SetVolume)(clamped); }
        *self.volume.lock().unwrap() = clamped;
    }

    /// Get the current volume.
    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    /// Check if the current stream is still playing.
    pub fn is_playing(&self) -> bool {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe { (self.bass.BASS_ChannelIsActive)(stream) == BASS_ACTIVE_PLAYING }
        } else {
            false
        }
    }

    /// Get the current player state.
    pub fn get_state(&self) -> PlayerState {
        self.state.lock().unwrap().clone()
    }

    /// Register an end-of-stream sync callback.
    pub fn set_end_sync<F>(&self, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe extern "C" fn sync_proc(_handle: u32, _channel: u32, _data: u32, user: *mut c_void) {
                if !user.is_null() {
                    let cb: Box<Box<dyn FnOnce() + Send>> = Box::from_raw(user as *mut _);
                    cb();
                }
            }

            let cb: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(callback));
            let user_ptr = Box::into_raw(cb) as *mut c_void;

            unsafe {
                (self.bass.BASS_ChannelSetSync)(stream, BASS_SYNC_END, 0, Some(sync_proc), user_ptr);
            }
        }
    }

    /// Stop and free the current stream.
    fn stop_current(&self) {
        if let Some(stream) = *self.current_stream.lock().unwrap() {
            unsafe {
                (self.bass.BASS_ChannelStop)(stream);
                (self.bass.BASS_StreamFree)(stream);
            }
            *self.current_stream.lock().unwrap() = None;
        }
        *self.state.lock().unwrap() = PlayerState::Stopped;
    }

    /// Stop playback and reset state to Idle.
    pub fn stop(&self) {
        self.stop_current();
        *self.state.lock().unwrap() = PlayerState::Idle;
    }
}

impl Drop for BassEngine {
    fn drop(&mut self) {
        self.stop_current();
        unsafe { (self.bass.BASS_Free)(); }
    }
}
