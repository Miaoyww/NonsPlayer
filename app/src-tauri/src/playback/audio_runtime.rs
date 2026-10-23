//! Configure the private runtime before GStreamer initializes or starts its scanner.
use crate::model::AppResult;

pub fn configure(cache: &std::path::Path) -> AppResult<()> {
    static CONFIGURED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
    let mut configured = CONFIGURED.lock().map_err(|_| "音频运行时配置不可用")?;
    if *configured {
        return Ok(());
    }
    configure_once(cache)?;
    *configured = true;
    Ok(())
}

fn configure_once(cache: &std::path::Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let directory = executable.parent().ok_or("程序目录不可用")?;
        let runtime = directory.join("runtime");
        if runtime.is_dir() {
            let plugins = runtime.join("gstreamer-1.0");
            let scanner = runtime.join("gst-plugin-scanner.exe");
            if !plugins.is_dir() || !scanner.is_file() {
                return Err("安装包的音频运行时不完整，请重新安装".into());
            }
            let mut paths = vec![directory.to_path_buf()];
            paths.extend(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ));
            std::env::set_var(
                "PATH",
                std::env::join_paths(paths).map_err(|e| e.to_string())?,
            );
            // Override inherited development/system paths, including versioned aliases.
            for name in ["GST_PLUGIN_PATH", "GST_PLUGIN_PATH_1_0"] {
                std::env::set_var(name, &plugins);
            }
            for name in ["GST_PLUGIN_SYSTEM_PATH", "GST_PLUGIN_SYSTEM_PATH_1_0"] {
                std::env::set_var(name, "");
            }
            for name in ["GST_PLUGIN_SCANNER", "GST_PLUGIN_SCANNER_1_0"] {
                std::env::set_var(name, &scanner);
            }
            std::env::set_var("GST_REGISTRY_1_0", cache.join("gstreamer-registry.bin"));
        }
    }
    #[cfg(not(windows))]
    let _ = cache;
    Ok(())
}
