use std::io::{BufRead, BufReader};
use std::process::{Child, Command};

const DEFAULT_PORT: u16 = 37562;

/// Attempt to start the Netease API Express server as a child process.
/// Returns the child handle and the actual port on success.
pub fn start_api_server() -> Option<(Child, u16)> {
    let server_dir = if cfg!(debug_assertions) {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .parent()?
            .join("server")
            .join("netease-api")
    } else {
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("server")
            .join("netease-api")
    };

    if !server_dir.exists() {
        log::warn!("[server] directory not found at {:?}, skipping", server_dir);
        return None;
    }

    let mut child = Command::new("node")
        .args([
            "--import",
            "tsx",
            "src/index.ts",
            &format!("--port={}", DEFAULT_PORT),
        ])
        .current_dir(&server_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| log::warn!("[server] failed to start: {}", e))
        .ok()?;

    // Read the first line from stdout to get the actual port
    let stdout = child.stdout.take()?;
    let mut reader = BufReader::new(stdout);
    let mut first_line = String::new();

    match reader.read_line(&mut first_line) {
        Ok(_) => {
            let port = first_line
                .trim()
                .strip_prefix("NETEASE_API_PORT=")
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(DEFAULT_PORT);

            log::info!("[server] Netease API server ready on port {}", port);

            // Drain remaining stdout in background to prevent buffer blocking
            std::thread::spawn(move || {
                for line in reader.lines() {
                    if let Ok(l) = line {
                        if !l.is_empty() {
                            log::debug!("[server] {}", l);
                        }
                    }
                }
            });

            Some((child, port))
        }
        Err(e) => {
            log::warn!("[server] failed to read port from stdout: {}", e);
            Some((child, DEFAULT_PORT))
        }
    }
}
