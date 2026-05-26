// Direct FFI bindings to bass.dll.
// Based on the BASS 2.4 API: https://www.un4seen.com/

use std::ffi::c_void;

type DWORD = u32;
type QWORD = u64;
type BOOL = i32;
type HSTREAM = DWORD;
type HSYNC = DWORD;

pub const BASS_UNICODE: DWORD = 0x80000000;
pub const BASS_SYNC_END: DWORD = 2;
#[allow(dead_code)]
pub const BASS_ACTIVE_STOPPED: DWORD = 0;
pub const BASS_ACTIVE_PLAYING: DWORD = 1;
#[allow(dead_code)]
pub const BASS_ACTIVE_PAUSED: DWORD = 3;
pub const BASS_POS_BYTE: DWORD = 0;
pub const BASS_STREAM_AUTOFREE: DWORD = 0x40000;
pub const BASS_STREAM_STATUS: DWORD = 0x200000;
#[allow(dead_code)]
pub const BASS_ERROR_UNKNOWN: i32 = -1;

pub type DownloadProc = Option<unsafe extern "C" fn(*const c_void, DWORD, *mut c_void)>;
pub type SyncProc = Option<unsafe extern "C" fn(HSTREAM, DWORD, DWORD, *mut c_void)>;

#[link(name = "bass")]
extern "C" {
    pub fn BASS_Init(device: i32, freq: DWORD, flags: DWORD, win: *mut c_void, clsid: *mut c_void) -> BOOL;
    pub fn BASS_Free() -> BOOL;
    pub fn BASS_ErrorGetCode() -> i32;
    pub fn BASS_SetVolume(volume: f32) -> BOOL;

    pub fn BASS_StreamCreateURL(
        url: *const i8,
        offset: QWORD,
        flags: DWORD,
        proc: DownloadProc,
        user: *mut c_void,
    ) -> HSTREAM;

    pub fn BASS_StreamCreateFile(
        mem: BOOL,
        file: *const c_void,
        offset: QWORD,
        length: QWORD,
        flags: DWORD,
    ) -> HSTREAM;

    pub fn BASS_StreamFree(handle: HSTREAM) -> BOOL;

    pub fn BASS_ChannelPlay(handle: DWORD, restart: BOOL) -> BOOL;
    pub fn BASS_ChannelPause(handle: DWORD) -> BOOL;
    pub fn BASS_ChannelStop(handle: DWORD) -> BOOL;
    pub fn BASS_ChannelIsActive(handle: DWORD) -> DWORD;

    pub fn BASS_ChannelSetPosition(handle: DWORD, pos: QWORD, mode: DWORD) -> BOOL;
    pub fn BASS_ChannelGetPosition(handle: DWORD, mode: DWORD) -> QWORD;
    pub fn BASS_ChannelGetLength(handle: DWORD, mode: DWORD) -> QWORD;

    pub fn BASS_ChannelBytes2Seconds(handle: DWORD, pos: QWORD) -> f64;
    pub fn BASS_ChannelSeconds2Bytes(handle: DWORD, pos: f64) -> QWORD;

    pub fn BASS_ChannelSetSync(
        handle: HSTREAM,
        sync_type: DWORD,
        param: QWORD,
        proc: SyncProc,
        user: *mut c_void,
    ) -> HSYNC;
}
