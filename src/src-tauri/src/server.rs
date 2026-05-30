use std::process::{Child, Command};

const API_PORT: u16 = 25884;

/// Attempt to start the Netease API Express server as a child process.
/// Returns the child handle on success, or None if Node.js is unavailable
/// or the server directory doesn't exist.
pub fn start_api_server() -> Option<Child> {
    // In dev: CARGO_MANIFEST_DIR = .../src/src-tauri, server is two levels up
    let server_dir = if cfg!(debug_assertions) {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .parent()?
            .join("server")
            .join("netease-api")
    } else {
        // In production: look adjacent to the executable
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("server")
            .join("netease-api")
    };

    // Check that the server directory exists
    if !server_dir.exists() {
        log::warn!(
            "[server] directory not found at {:?}, skipping",
            server_dir
        );
        return None;
    }

    match Command::new("node")
        .args([
            "--import",
            "tsx",
            "src/index.ts",
            &format!("--port={}", API_PORT),
        ])
        .current_dir(&server_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
    {
        Ok(child) => {
            log::info!("[server] Netease API server started on port {}", API_PORT);
            Some(child)
        }
        Err(e) => {
            log::warn!("[server] failed to start (Node.js may not be installed): {}", e);
            None
        }
    }
}
