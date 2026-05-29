// Dynamic loading of BASS shared library via libloading.
// No compile-time .lib required — bass.dll / libbass.dylib / libbass.so is loaded at runtime.

use libloading::{self, Library, Symbol};

pub type DWORD = u32;
pub type QWORD = u64;
pub type BOOL = i32;
pub type HSTREAM = DWORD;
pub type HSYNC = DWORD;
pub type Handle = DWORD;

pub const BASS_SYNC_END: DWORD = 2;
#[allow(dead_code)]
pub const BASS_ACTIVE_STOPPED: DWORD = 0;
pub const BASS_ACTIVE_PLAYING: DWORD = 1;
#[allow(dead_code)]
pub const BASS_ACTIVE_PAUSED: DWORD = 3;
pub const BASS_POS_BYTE: DWORD = 0;
pub const BASS_STREAM_AUTOFREE: DWORD = 0x40000;
pub const BASS_STREAM_STATUS: DWORD = 0x200000;
pub const BASS_UNICODE: DWORD = 0x80000000;
#[allow(dead_code)]
pub const BASS_ERROR_UNKNOWN: i32 = -1;

type DownloadProc = Option<unsafe extern "C" fn(*const std::ffi::c_void, DWORD, *mut std::ffi::c_void)>;
type SyncProc = Option<unsafe extern "C" fn(HSTREAM, DWORD, DWORD, *mut std::ffi::c_void)>;

macro_rules! load_fn {
    ($lib:expr, $name:literal) => {
        unsafe {
            let sym: Symbol<*const ()> = $lib.get($name.as_bytes()).map_err(|e| e.to_string())?;
            std::mem::transmute(sym)
        }
    };
}

/// Loaded BASS function pointers. Created once, shared via Arc.
#[allow(non_snake_case, dead_code)]
pub struct BassLib {
    lib: Library,

    pub BASS_Init: unsafe extern "C" fn(device: i32, freq: DWORD, flags: DWORD, win: *mut std::ffi::c_void, clsid: *mut std::ffi::c_void) -> BOOL,
    pub BASS_Free: unsafe extern "C" fn() -> BOOL,
    pub BASS_ErrorGetCode: unsafe extern "C" fn() -> i32,
    pub BASS_SetVolume: unsafe extern "C" fn(volume: f32) -> BOOL,

    pub BASS_StreamCreateURL: unsafe extern "C" fn(url: *const i8, offset: QWORD, flags: DWORD, proc: DownloadProc, user: *mut std::ffi::c_void) -> HSTREAM,
    pub BASS_StreamCreateFile: unsafe extern "C" fn(mem: BOOL, file: *const std::ffi::c_void, offset: QWORD, length: QWORD, flags: DWORD) -> HSTREAM,
    pub BASS_StreamFree: unsafe extern "C" fn(handle: HSTREAM) -> BOOL,

    pub BASS_ChannelPlay: unsafe extern "C" fn(handle: Handle, restart: BOOL) -> BOOL,
    pub BASS_ChannelPause: unsafe extern "C" fn(handle: Handle) -> BOOL,
    pub BASS_ChannelStop: unsafe extern "C" fn(handle: Handle) -> BOOL,
    pub BASS_ChannelIsActive: unsafe extern "C" fn(handle: Handle) -> DWORD,

    pub BASS_ChannelSetPosition: unsafe extern "C" fn(handle: Handle, pos: QWORD, mode: DWORD) -> BOOL,
    pub BASS_ChannelGetPosition: unsafe extern "C" fn(handle: Handle, mode: DWORD) -> QWORD,
    pub BASS_ChannelGetLength: unsafe extern "C" fn(handle: Handle, mode: DWORD) -> QWORD,

    pub BASS_ChannelBytes2Seconds: unsafe extern "C" fn(handle: Handle, pos: QWORD) -> f64,
    pub BASS_ChannelSeconds2Bytes: unsafe extern "C" fn(handle: Handle, pos: f64) -> QWORD,

    pub BASS_ChannelSetSync: unsafe extern "C" fn(handle: HSTREAM, sync_type: DWORD, param: QWORD, proc: SyncProc, user: *mut std::ffi::c_void) -> HSYNC,
}

impl BassLib {
    /// Load the BASS library for the current platform.
    pub fn load() -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        let lib_name = "bass.dll";

        #[cfg(target_os = "macos")]
        let lib_name = "libbass.dylib";

        #[cfg(target_os = "linux")]
        let lib_name = "libbass.so";

        // Search paths: exe dir, working dir, then just the filename (system search).
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join(lib_name));
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join(lib_name));
        }
        candidates.push(std::path::PathBuf::from(lib_name));

        let mut last_err = String::new();
        for path in &candidates {
            eprintln!("[BassLib] trying: {}", path.display());
            match unsafe { Library::new(path) } {
                Ok(lib) => {
                    eprintln!("[BassLib] loaded successfully from: {}", path.display());
                    return Ok(Self {
                        BASS_Init: load_fn!(lib, "BASS_Init"),
                        BASS_Free: load_fn!(lib, "BASS_Free"),
                        BASS_ErrorGetCode: load_fn!(lib, "BASS_ErrorGetCode"),
                        BASS_SetVolume: load_fn!(lib, "BASS_SetVolume"),
                        BASS_StreamCreateURL: load_fn!(lib, "BASS_StreamCreateURL"),
                        BASS_StreamCreateFile: load_fn!(lib, "BASS_StreamCreateFile"),
                        BASS_StreamFree: load_fn!(lib, "BASS_StreamFree"),
                        BASS_ChannelPlay: load_fn!(lib, "BASS_ChannelPlay"),
                        BASS_ChannelPause: load_fn!(lib, "BASS_ChannelPause"),
                        BASS_ChannelStop: load_fn!(lib, "BASS_ChannelStop"),
                        BASS_ChannelIsActive: load_fn!(lib, "BASS_ChannelIsActive"),
                        BASS_ChannelSetPosition: load_fn!(lib, "BASS_ChannelSetPosition"),
                        BASS_ChannelGetPosition: load_fn!(lib, "BASS_ChannelGetPosition"),
                        BASS_ChannelGetLength: load_fn!(lib, "BASS_ChannelGetLength"),
                        BASS_ChannelBytes2Seconds: load_fn!(lib, "BASS_ChannelBytes2Seconds"),
                        BASS_ChannelSeconds2Bytes: load_fn!(lib, "BASS_ChannelSeconds2Bytes"),
                        BASS_ChannelSetSync: load_fn!(lib, "BASS_ChannelSetSync"),
                        lib,
                    });
                }
                Err(e) => {
                    last_err = format!("{}: {}", path.display(), e);
                    eprintln!("[BassLib] {}", last_err);
                }
            }
        }

        Err(format!("failed to load {}: {}", lib_name, last_err))
    }
}
