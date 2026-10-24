//! Bounded local diagnostics. Never log request/response bodies or account credentials.
use crate::music::{ErrorCode, MusicResult};
use regex::Regex;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    LazyLock, Mutex,
};
use std::time::{Duration, Instant};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
mod files;

// Stop formatting when the bound is reached, before a Debug representation or a
// long error can allocate a second copy of an entire response in the log sink.
fn bounded_message(args: std::fmt::Arguments<'_>) -> String {
    struct Bounded(String);
    impl std::fmt::Write for Bounded {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            let remaining = 4096usize.saturating_sub(self.0.len());
            let mut end = remaining.min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            self.0.push_str(&text[..end]);
            if end < text.len() {
                Err(std::fmt::Error)
            } else {
                Ok(())
            }
        }
    }
    let mut output = Bounded(String::with_capacity(256));
    // A formatting error here signals intentional truncation.
    let _ = std::fmt::write(&mut output, args);
    redact(&output.0)
}

/// Record a recovered or translated failure before its original cause is lost.
/// Display avoids serializing error internals such as credential bytes.
#[track_caller]
pub(crate) fn warn(operation: &str, error: impl std::fmt::Display) {
    let at = std::panic::Location::caller();
    let stage = operation.rsplit('.').next().unwrap_or(operation);
    log::warn!(target: "nons::recovery", "operation={operation} stage={stage} at={}:{} error_type={} reason={error}", at.file(), at.line(), std::any::type_name_of_val(&error));
}

/// A bounded scalar that cannot introduce fields or consume the rest of a record.
pub(crate) fn field(value: &str) -> String {
    let clean = redact(value);
    let mut end = clean.len().min(128);
    while !clean.is_char_boundary(end) {
        end -= 1;
    }
    url::form_urlencoded::byte_serialize(clean[..end].as_bytes()).collect()
}

#[track_caller]
pub(crate) fn warn_error(operation: &str, error: &(dyn std::error::Error + 'static)) {
    struct Chain<'a>(&'a (dyn std::error::Error + 'static));
    impl std::fmt::Display for Chain<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)?;
            let mut cause = self.0.source();
            for _ in 0..4 {
                let Some(error) = cause else {
                    break;
                };
                write!(f, "; caused_by={error}")?;
                cause = error.source();
            }
            Ok(())
        }
    }
    warn(operation, Chain(error));
}

pub(crate) fn redact(value: &str) -> String {
    static URL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)https?://[^\s"'<>]+"#).unwrap());
    static PAYLOAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\[{].*$").unwrap());
    static SCALAR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"((?:invalid (?:type|value): string|unknown variant|unknown field) )"(?:\\.|[^"\\])*(?:"|$)"#).unwrap()
    });
    static SECRET: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)\b(cookie|authorization|bearer|credential|password|token|secret|music_u|uin|skey|qqmusic_key)\b[\s"']*[:=].*$"#).unwrap()
    });
    static AUTH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\b(cookie|authorization|bearer)\b\s+.*$").unwrap());
    // Bound work before regex processing and keep every record on a single physical line.
    let value: String = value
        .chars()
        .take(4096)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let value = URL.replace_all(&value, "<url>");
    let value = PAYLOAD.replace_all(&value, "<payload>");
    let value = SCALAR.replace_all(&value, "$1<redacted>");
    let value = AUTH.replace_all(&value, "$1=<redacted>");
    SECRET
        .replace_all(&value, "$1=<redacted>")
        .chars()
        .take(2048)
        .collect()
}

pub(crate) fn initialize<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager;
    let destination = (|| -> Result<_, Box<dyn std::error::Error>> {
        let directory = match super::directories::portable() {
            Some(data) => data.join("logs"),
            None => app.path().app_log_dir()?,
        };
        let name = files::reserve(
            &directory,
            tauri_plugin_log::TimezoneStrategy::UseLocal.get_now(),
            std::process::id(),
        )?;
        Ok((directory, name))
    })();
    let file = destination
        .as_ref()
        .ok()
        .map(|(directory, name)| TargetKind::Folder {
            path: directory.clone(),
            file_name: Some(name.clone()),
        });
    if let Err(error) = app.plugin(plugin(file)) {
        // File logging must not prevent music playback or application startup.
        if app.plugin(plugin(None)).is_err() {
            eprintln!("日志初始化失败：{}", redact(&error.to_string()));
        } else {
            log::error!(target: "nons::diagnostics", "file_logging_unavailable reason={}", error);
        }
    }
    match destination {
        Ok((directory, name)) => {
            log::info!(target: "nons::diagnostics", "session_log={}", directory.join(format!("{name}.log")).display());
            if let Err(error) = files::prune(&directory, std::time::SystemTime::now()) {
                warn("diagnostics.prune", error);
            }
        }
        Err(error) => {
            log::error!(target: "nons::diagnostics", "file_logging_unavailable reason={error}")
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
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
        .level(log::LevelFilter::Info)
        // Dependency HTTP debug output may contain headers. Only application diagnostics enter this sink.
        .filter(|metadata| metadata.target().starts_with("nons"))
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} {} {}",
                files::timestamp(tauri_plugin_log::TimezoneStrategy::UseLocal.get_now()),
                record.level(),
                record.target(),
                bounded_message(*message)
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
pub(crate) mod test_log {
    struct Capture;
    thread_local! {
        static RECORDS: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
    }
    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, record: &log::Record<'_>) {
            RECORDS.with(|records| {
                if let Some(records) = records.borrow_mut().as_mut() {
                    records.push(super::bounded_message(*record.args()));
                }
            });
        }
        fn flush(&self) {}
    }
    pub(crate) fn start() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            static LOGGER: Capture = Capture;
            log::set_logger(&LOGGER).unwrap();
            log::set_max_level(log::LevelFilter::Info);
        });
        RECORDS.with(|records| *records.borrow_mut() = Some(Vec::new()));
    }
    pub(crate) fn take() -> Vec<String> {
        RECORDS.with(|records| records.borrow_mut().take().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovered_errors_keep_display_causes_without_debug_payloads() {
        test_log::start();

        // These are the same values passed by credential and file IO recovery sites.
        warn(
            "credentials.read",
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "credential store denied access",
            ),
        );
        let invalid = String::from_utf8(vec![255, 254, 253]).unwrap_err();
        warn("credentials.decode", invalid);
        #[derive(Debug)]
        struct Transport(std::io::Error);
        impl std::fmt::Display for Transport {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("sending request to https://user:SECRET@example.com")
            }
        }
        impl std::error::Error for Transport {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        warn_error(
            "http.send",
            &Transport(std::io::Error::other("connection refused")),
        );
        {
            let records = test_log::take();
            assert!(records[0].contains("operation=credentials.read"));
            assert!(records[0].contains("credential store denied access"));
            assert!(records[0].contains("diagnostics.rs:"));
            assert!(records[1].contains("invalid utf-8"));
            assert!(!records[1].contains("255"));
            assert!(records[2].contains("caused_by=connection refused"));
            assert!(!records[2].contains("SECRET"));
        }
    }

    #[test]
    fn formatting_stops_before_evaluating_later_payloads_and_preserves_unicode() {
        struct Never;
        impl std::fmt::Display for Never {
            fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                panic!("payload beyond the bound must not be formatted");
            }
        }
        let long = "错".repeat(2000);
        let message = bounded_message(format_args!("reason={long} {}", Never));
        assert!(message.starts_with("reason=错"));
        assert!(message.len() <= 4096);
        assert!(message.chars().count() <= 2048);
    }

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
            "invalid type: string \"SECRET\", expected a boolean at line 1 column 8",
            "invalid value: string \"SECRET\\\"more\", expected a number",
            "unknown variant \"SECRET\", expected `ok` or `error`",
            "invalid type: string \"SECRET",
        ] {
            assert!(!redact(text).contains("SECRET"));
        }
        assert!(!redact("failed\nforged").contains('\n'));
        assert!(redact(&"a".repeat(10000)).len() <= 2048);
        assert_eq!(
            redact("数据库查询失败: no such column: source"),
            "数据库查询失败: no such column: source"
        );
        let error = serde_json::from_str::<bool>("\"SECRET\"").unwrap_err();
        let reason = bounded_message(format_args!("reason={error}"));
        assert!(!reason.contains("SECRET"));
        assert!(reason.contains("expected a boolean"));
    }

    #[test]
    fn diagnostic_fields_cannot_inject_or_truncate_later_context() {
        let id = field("track[opaque]\ncode=ok token=SECRET");
        assert!(!id.contains("SECRET"));
        let record = redact(&format!("track_id={id} stage=metadata code=InvalidData"));
        assert!(record.contains("stage=metadata code=InvalidData"));
        assert!(!id.contains([' ', '\n', '[', '{']));
        assert!(field(&"字".repeat(10000)).len() <= 384);
    }
}
