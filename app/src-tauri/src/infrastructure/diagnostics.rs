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
#[path = "diagnostics/tracing.rs"]
mod trace_transport;

const MAX_RECORD_BYTES: usize = 64 * 1024;

/// Capture anyhow errors before the runtime creates worker threads.
pub(crate) fn enable_backtraces() {
    std::env::set_var("RUST_LIB_BACKTRACE", "1");
}

fn format_record(record: &log::Record<'_>) -> String {
    use std::fmt::Write;
    let legacy = record.level() <= log::Level::Warn
        && record.target() != "nons::trace"
        && record.target() != "nons::webview";
    let raw = bounded_format(
        *record.args(),
        if legacy { 16 * 1024 } else { MAX_RECORD_BYTES },
    );
    let mut output = raw.lines().map(redact).collect::<Vec<_>>().join("\n");
    // Legacy log callers have no captured error object. Keep their physical reporting
    // stack explicitly separate from an anyhow error's creation stack.
    if legacy {
        let stack = std::backtrace::Backtrace::force_capture();
        let stack = bounded_format(format_args!("{stack}"), 32 * 1024);
        let _ = write!(output, "\nbacktrace_kind=observation\n{stack}");
    }
    output
}

fn bounded_format(args: std::fmt::Arguments<'_>, limit: usize) -> String {
    use std::fmt::Write;
    struct Bounded(String, usize);
    impl std::fmt::Write for Bounded {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let mut length = value.len().min(self.1.saturating_sub(self.0.len()));
            while !value.is_char_boundary(length) {
                length -= 1;
            }
            self.0.push_str(&value[..length]);
            if length < value.len() {
                Err(std::fmt::Error)
            } else {
                Ok(())
            }
        }
    }
    let mut raw = Bounded(String::new(), limit);
    if raw.write_fmt(args).is_err() {
        raw.0.push_str("\n[truncated]");
    }
    raw.0
}

// Stop formatting when the bound is reached, before a Debug representation or a
// long error can allocate a second copy of an entire response in the log sink.
fn bounded_message(args: std::fmt::Arguments<'_>) -> String {
    redact(&bounded_format(args, 4096 - 16))
}

/// Record a recovered or translated failure before its original cause is lost.
/// Display avoids serializing error internals such as credential bytes.
#[track_caller]
pub(crate) fn warn(operation: &str, error: impl std::fmt::Display) {
    let error = anyhow::Error::msg(bounded_message(format_args!("{error}")));
    report(operation, &error, true);
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
    let stack = std::backtrace::Backtrace::force_capture();
    report_parts(operation, error, &stack, "observation");
}

#[track_caller]
pub(crate) fn report_error(operation: &str, error: &anyhow::Error) {
    report(operation, error, false);
}

#[track_caller]
fn report(operation: &str, error: &anyhow::Error, observation: bool) {
    let fallback;
    let (stack, kind) = if error.backtrace().status() == std::backtrace::BacktraceStatus::Captured {
        (
            error.backtrace(),
            if observation { "observation" } else { "origin" },
        )
    } else {
        fallback = std::backtrace::Backtrace::force_capture();
        (&fallback, "observation")
    };
    report_parts(operation, error.as_ref(), stack, kind);
}

#[track_caller]
fn report_parts(
    operation: &str,
    error: &(dyn std::error::Error + 'static),
    stack: &std::backtrace::Backtrace,
    kind: &str,
) {
    let at = std::panic::Location::caller();
    let stage = operation.rsplit('.').next().unwrap_or(operation);
    let reason = bounded_message(format_args!("{error}"));
    let root = anyhow::Chain::new(error).last().unwrap_or(error);
    let root = bounded_message(format_args!("{root}"));
    let mut causes = String::new();
    for cause in anyhow::Chain::new(error).skip(1).take(32) {
        let line = format!("caused_by={}\n", bounded_message(format_args!("{cause}")));
        if causes.len() + line.len() > 8192 {
            causes.push_str("[causes truncated; see root_cause]\n");
            break;
        }
        causes.push_str(&line);
    }
    let stack = bounded_format(format_args!("{stack}"), 32 * 1024);
    let spans = bounded_format(
        format_args!("{}", tracing_error::SpanTrace::capture()),
        8192,
    );
    ::tracing::warn!(target: "nons::recovery", operation=%operation, stage=%stage, at=%format_args!("{}:{}", at.file(), at.line()), backtrace_kind=%kind, reason=%format_args!("\n{reason}\n"), root_cause=%format_args!("\n{root}\n"), backtrace=%format_args!("\n{stack}\n"), span_trace=%format_args!("\n{spans}\n"), causes=%format_args!("\n{causes}"));
}

pub(crate) fn panic(info: &std::panic::PanicHookInfo<'_>) {
    let stack = std::backtrace::Backtrace::force_capture();
    ::tracing::error!(target: "nons::panic", reason=%info, backtrace_kind="panic_hook", backtrace=%format_args!("\n{stack}"));
    log::logger().flush();
}

pub(crate) fn redact(value: &str) -> String {
    static URL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)https?://[^\s"'<>]+"#).unwrap());
    // JSON payloads are distinct from tracing's `span{request=...}` context and
    // Rust generic/array type names in symbolized stack frames.
    static PAYLOAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?:\{\s*"|\[\s*(?:"|-?\d|true\b|false\b|null\b|\{|\[)).*$"#).unwrap()
    });
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
    trace_transport::install();
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
        .format(|out, _message, record| {
            out.finish(format_args!(
                "{} {} {} {}",
                files::timestamp(tauri_plugin_log::TimezoneStrategy::UseLocal.get_now()),
                record.level(),
                record.target(),
                format_record(record)
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
    let error = anyhow::anyhow!("{message}").context(stage);
    let stack = std::backtrace::Backtrace::force_capture();
    ::tracing::error!(target: "nons::storage", stage, reason=%format_args!("{error:#}"), backtrace_kind="observation", backtrace=%format_args!("\n{stack}"));
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
    use tracing::Instrument;
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
    let span = ::tracing::info_span!(target: "nons::music", "music_request", request=request.id, source=%field(source), operation=%field(operation));
    let result = future.instrument(span).await;
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
    stack: Option<String>,
    stack_kind: Option<String>,
    location: Option<String>,
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
        entry.stack_kind.as_ref(),
        entry.location.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.len() > 8192 {
            return Err("invalidData".into());
        }
    }
    if entry
        .stack
        .as_ref()
        .is_some_and(|stack| stack.len() > 32 * 1024)
    {
        return Err("invalidData".into());
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
    let stack = entry
        .stack
        .as_deref()
        .unwrap_or("original stack unavailable")
        .lines()
        .map(redact)
        .collect::<Vec<_>>()
        .join("\n");
    log::log!(target: "nons::webview", level, "command={} source={} operation={} elapsed_ms={} code={} stage={} reason={} location={}\nstack_kind={}\n{}", redact(&entry.command), redact(entry.source.as_deref().unwrap_or("-")), redact(entry.operation.as_deref().unwrap_or("-")), entry.elapsed_ms.unwrap_or(0), redact(entry.code.as_deref().unwrap_or("-")), redact(entry.stage.as_deref().unwrap_or("-")), redact(entry.message.as_deref().unwrap_or("-")), redact(entry.location.as_deref().unwrap_or("-")), redact(entry.stack_kind.as_deref().unwrap_or("unavailable")), stack);
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
                    records.push(super::format_record(record));
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
        super::trace_transport::install();
    }
    pub(crate) fn take() -> Vec<String> {
        RECORDS.with(|records| records.borrow_mut().take().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inline(never)]
    fn contextual_failure() -> anyhow::Result<()> {
        use anyhow::Context;
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "operation timed out",
        ))
        .context("covers.http.send")
    }

    #[test]
    fn contextual_errors_write_causes_physical_frames_and_async_spans_to_the_sink() {
        test_log::start();
        let span = ::tracing::info_span!(target: "nons::covers", "runtime_cover", request = 73);
        let _entered = span.enter();
        let error = contextual_failure().unwrap_err();
        report_error("covers.load", &error);
        let records = test_log::take();
        let output = records.join("\n");
        assert!(output.contains("covers.http.send"), "{output}");
        assert!(output.contains("operation timed out"), "{output}");
        assert!(output.contains("backtrace_kind="), "{output}");
        assert!(output.contains("diagnostics.rs:"), "{output}");
        assert!(output.contains("runtime_cover"), "{output}");
        assert!(output.contains("request=73"), "{output}");
        assert!(
            output
                .lines()
                .filter(|line| line.trim_start().starts_with("at "))
                .count()
                >= 2,
            "{output}"
        );
        if let Ok(path) = std::env::var("NONS_DIAGNOSTIC_TEST_OUTPUT") {
            std::fs::write(path, &output).unwrap();
        }
    }

    #[test]
    fn frontend_stack_survives_native_serialization_without_losing_frames_or_location() {
        test_log::start();
        let entry: FrontendEntry = serde_json::from_value(serde_json::json!({
            "level":"error", "command":"window.error", "message":"ResizeObserver loop completed with undelivered notifications.",
            "stackKind":"observation", "location":"source:/assets/index.js:10:20",
            "stack":"Original stack unavailable\n    at observe (source:/assets/index.js:10:20)\n    at dispatch (source:/assets/index.js:30:40)"
        })).unwrap();
        diagnostic_report(entry).unwrap();
        let output = test_log::take().join("\n");
        assert!(output.contains("stack_kind=observation"), "{output}");
        assert!(
            output.contains("location=source:/assets/index.js:10:20"),
            "{output}"
        );
        assert!(
            output.contains("\n    at observe (source:/assets/index.js:10:20)"),
            "{output}"
        );
        assert!(
            output.contains("\n    at dispatch (source:/assets/index.js:30:40)"),
            "{output}"
        );
        assert!(!output.contains("backtrace_kind="), "{output}");
    }

    #[test]
    fn legacy_warn_and_error_records_capture_the_reporting_stack() {
        test_log::start();
        log::error!(target: "nons::test", "legacy failure");
        let output = test_log::take().join("\n");
        assert!(output.contains("backtrace_kind=observation"), "{output}");
        assert!(output.contains("diagnostics.rs:"), "{output}");
    }

    #[test]
    fn long_context_cannot_hide_the_root_cause_or_physical_stack() {
        test_log::start();
        let error = contextual_failure()
            .unwrap_err()
            .context("outer context ".repeat(1000));
        report_error("covers.load", &error);
        let output = test_log::take().join("\n");
        assert!(output.contains("root_cause="), "{output}");
        assert!(output.contains("operation timed out"), "{output}");
        assert!(output.contains("backtrace_kind="), "{output}");
        assert!(output.contains("diagnostics.rs:"), "{output}");
        assert!(output.len() < MAX_RECORD_BYTES, "{}", output.len());
    }
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
