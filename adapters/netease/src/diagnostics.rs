//! The host log sink bounds and redacts these records before writing them.
use std::error::Error;

/// Preserve the platform cause before converting it into a stable public error.
#[track_caller]
pub(crate) fn warn(operation: &str, error: &(dyn Error + '_)) {
    struct Chain<'a>(&'a (dyn Error + 'a));
    impl std::fmt::Display for Chain<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)?;
            let mut cause = self.0.source();
            for _ in 0..4 {
                let Some(error) = cause else { break };
                write!(f, "; caused_by={error}")?;
                cause = error.source();
            }
            Ok(())
        }
    }
    let at = std::panic::Location::caller();
    log::warn!(target: "nons_adapter_netease::recovery", "operation={operation} at={}:{} reason={}", at.file(), at.line(), Chain(error));
}
