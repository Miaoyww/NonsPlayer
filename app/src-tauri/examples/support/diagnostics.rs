//! Standalone harness logging without the Tauri diagnostics service.
#[track_caller]
pub(crate) fn warn(operation: &str, error: impl std::fmt::Display) {
    log::warn!("operation={operation} reason={error}");
}

#[track_caller]
pub(crate) fn warn_error(operation: &str, error: &(dyn std::error::Error + 'static)) {
    warn(operation, error);
}
