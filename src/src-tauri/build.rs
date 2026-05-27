use std::env;
use std::path::PathBuf;

fn main() {
    // Copy BASS shared library next to the binary during development.
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let (src_name, _src_dir) = if target.contains("windows") {
        ("bass.dll", "windows")
    } else if target.contains("apple") {
        ("libbass.dylib", "macos")
    } else {
        ("libbass.so", "linux")
    };

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let out_dir = manifest_dir.join("target").join(&profile);

    // Try platform-specific subdirectory first, then root bass/ dir
    let platform_dir = manifest_dir.join("bass").join(_src_dir);
    let dll_src = if platform_dir.join(src_name).exists() {
        platform_dir.join(src_name)
    } else {
        manifest_dir.join("bass").join(src_name)
    };

    if dll_src.exists() {
        std::fs::copy(&dll_src, out_dir.join(src_name)).ok();
    }

    println!("cargo:rerun-if-changed=bass/");
    tauri_build::build()
}
