//! Bounded local diagnostics. Never log request/response bodies or account credentials.
use crate::music::{ErrorCode, MusicResult};
use regex::Regex;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    LazyLock, Mutex,
};
use std::time::{Duration, Instant};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

pub(crate) fn redact(value: &str) -> String {
    static URL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)https?://[^\s"'<>]+"#).unwrap());
    static PAYLOAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\[{].*$").unwrap());
    static SECRET: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)\b(cookie|authorization|bearer|credential|password|token|secret|music_u|uin|skey|qqmusic_key)\b[\s"':=]+.*$"#).unwrap()
    });
    // Bound work before regex processing and keep every record on a single physical line.
    let value: String = value
        .chars()
        .take(4096)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let value = URL.replace_all(&value, "<url>");
    let value = PAYLOAD.replace_all(&value, "<payload>");
    SECRET
        .replace_all(&value, "$1=<redacted>")
        .chars()
        .take(2048)
        .collect()
}

pub(crate) fn initialize<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let file = match super::directories::portable() {
        Some(data) => TargetKind::Folder {
            path: data.join("logs"),
            file_name: Some("nons".into()),
        },
        None => TargetKind::LogDir {
            file_name: Some("nons".into()),
        },
    };
    if let Err(error) = app.plugin(plugin(Some(file))) {
        // File logging must not prevent music playback or application startup.
        if app.plugin(plugin(None)).is_err() {
            eprintln!("日志初始化失败：{}", redact(&error.to_string()));
        } else {
            log::error!(target: "nons::diagnostics", "file_logging_unavailable reason={}", error);
        }
    }
}
fn plugin<R: tauri::Runtime>(file: Option<TargetKind>) -> tauri::plugin::TauriPlugin<R> {
    let mut targets = vec![
        Target::new(TargetKind::Stdout),
        Target::new(TargetKind::Webview),
    ];
    if let Some(file) = file {
        targets.push(Target::new(file));
    }
    tauri_plugin_log::Builder::new()
        .targets(targets)
        .max_file_size(2 * 1024 * 1024)
        .rotation_strategy(RotationStrategy::KeepSome(4))
        .level(log::LevelFilter::Info)
        // Dependency HTTP debug output may contain headers. Only application diagnostics enter this sink.
        .filter(|metadata| metadata.target().starts_with("nons"))
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} {} {}",
                tauri_plugin_log::TimezoneStrategy::UseUtc.get_now(),
                record.level(),
                record.target(),
                redact(&message.to_string())
            ))
        })
        .build()
}

static NEXT: AtomicU64 = AtomicU64::new(1);

/// Host IPC diagnostics are separate from the stable public adapter error contract.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    #[serde(flatten)]
    error: crate::music::MusicError,
    message: String,
    stage: &'static str,
}
impl From<crate::music::MusicError> for CommandError {
    fn from(error: crate::music::MusicError) -> Self {
        Self {
            message: error.to_string(),
            error,
            stage: "music_request",
        }
    }
}
impl From<ErrorCode> for CommandError {
    fn from(code: ErrorCode) -> Self {
        crate::music::MusicError::from(code).into()
    }
}
pub(crate) fn internal_error(stage: &'static str, reason: &str) -> CommandError {
    let message = redact(reason);
    log::error!(target: "nons::storage", "stage={stage} reason={message}");
    CommandError {
        error: ErrorCode::Internal.into(),
        message,
        stage,
    }
}
pub(crate) async fn observe<T>(
    source: &str,
    operation: &str,
    future: impl std::future::Future<Output = MusicResult<T>>,
) -> MusicResult<T> {
    struct Request<'a> {
        id: u64,
        source: &'a str,
        operation: &'a str,
        start: Instant,
        finished: bool,
    }
    impl Drop for Request<'_> {
        fn drop(&mut self) {
            if !self.finished {
                log::info!(target: "nons::music", "request={} source={} operation={} elapsed_ms={} code=cancelled", self.id, self.source, self.operation, self.start.elapsed().as_millis());
            }
        }
    }
    let mut request = Request {
        id: NEXT.fetch_add(1, Ordering::Relaxed),
        source,
        operation,
        start: Instant::now(),
        finished: false,
    };
    let result = future.await;
    let level = match &result {
        Ok(_) => log::Level::Info,
        Err(error) if matches!(error.code, ErrorCode::Cancelled | ErrorCode::StaleContext) => {
            log::Level::Info
        }
        Err(_) => log::Level::Error,
    };
    let code = result
        .as_ref()
        .err()
        .map(|error| format!("{:?}", error.code))
        .unwrap_or_else(|| "ok".into());
    log::log!(target: "nons::music", level, "request={} source={} operation={} elapsed_ms={} code={}", request.id, source, operation, request.start.elapsed().as_millis(), code);
    request.finished = true;
    result
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FrontendEntry {
    level: String,
    command: String,
    operation: Option<String>,
    source: Option<String>,
    code: Option<String>,
    stage: Option<String>,
    elapsed_ms: Option<u64>,
    message: Option<String>,
}
#[tauri::command]
pub(crate) fn diagnostic_report(entry: FrontendEntry) -> Result<(), String> {
    static BUDGET: Mutex<Option<(Instant, u32)>> = Mutex::new(None);
    let level = match entry.level.as_str() {
        "info" => log::Level::Info,
        "warn" => log::Level::Warn,
        "error" => log::Level::Error,
        _ => return Err("invalidData".into()),
    };
    for value in [
        Some(&entry.command),
        entry.operation.as_ref(),
        entry.source.as_ref(),
        entry.code.as_ref(),
        entry.stage.as_ref(),
        entry.message.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.len() > 8192 {
            return Err("invalidData".into());
        }
    }
    {
        let mut budget = BUDGET.lock().map_err(|_| "internal")?;
        let (start, count) = budget.get_or_insert((Instant::now(), 0));
        if start.elapsed() >= Duration::from_secs(1) {
            *start = Instant::now();
            *count = 0;
        }
        if *count >= 64 {
            return Err("rateLimited".into());
        }
        *count += 1;
    }
    log::log!(target: "nons::webview", level, "command={} source={} operation={} elapsed_ms={} code={} stage={} reason={}", redact(&entry.command), redact(entry.source.as_deref().unwrap_or("-")), redact(entry.operation.as_deref().unwrap_or("-")), entry.elapsed_ms.unwrap_or(0), redact(entry.code.as_deref().unwrap_or("-")), redact(entry.stage.as_deref().unwrap_or("-")), redact(entry.message.as_deref().unwrap_or("-")));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_errors_preserve_database_reason_and_code_for_the_webview() {
        let json = serde_json::to_value(internal_error(
            "save_queue_tracks",
            "no such column: source",
        ))
        .unwrap();
        assert_eq!(json["code"], "internal");
        assert_eq!(json["message"], "no such column: source");
        assert_eq!(json["stage"], "save_queue_tracks");
        assert!(!serde_json::to_string(&internal_error(
            "save_queue_tracks",
            "failed token=SECRET"
        ))
        .unwrap()
        .contains("SECRET"));
    }
    #[test]
    fn errors_are_bounded_and_do_not_expose_urls_credentials_or_payloads() {
        for text in [
            "failed https://u:SECRET@example.com/?token=SECRET",
            "failed Cookie: a=SECRET; b=SECRET",
            "Authorization: Bearer SECRET",
            "failed token=SECRET",
            "response {\"value\":\"SECRET\"}",
        ] {
            assert!(!redact(text).contains("SECRET"));
        }
        assert!(!redact("failed\nforged").contains('\n'));
        assert!(redact(&"a".repeat(10000)).len() <= 2048);
        assert_eq!(
            redact("数据库查询失败: no such column: source"),
            "数据库查询失败: no such column: source"
        );
    }
}
