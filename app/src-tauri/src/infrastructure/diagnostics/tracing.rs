//! Adapt tracing's standard formatter to the existing rotating Tauri log transport.
use std::io::{self, Write};
use tracing_subscriber::{filter::filter_fn, prelude::*};

pub(super) fn install() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let subscriber = tracing_subscriber::registry()
            .with(filter_fn(|metadata| {
                metadata.target().starts_with("nons")
                    && *metadata.level() <= tracing::Level::INFO
            }))
            .with(tracing_error::ErrorLayer::default())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .without_time()
                    .with_file(true)
                    .with_line_number(true)
                    .with_writer(Transport),
            );
        if let Err(error) = tracing::subscriber::set_global_default(subscriber) {
            log::error!(target: "nons::diagnostics", "tracing subscriber initialization failed: {error}");
        }
    });
}

struct Transport;
struct Record {
    level: log::Level,
    bytes: Vec<u8>,
    truncated: bool,
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Transport {
    type Writer = Record;
    fn make_writer(&'a self) -> Record {
        Record {
            level: log::Level::Info,
            bytes: Vec::new(),
            truncated: false,
        }
    }
    fn make_writer_for(&'a self, metadata: &tracing::Metadata<'_>) -> Record {
        let level = match *metadata.level() {
            tracing::Level::ERROR => log::Level::Error,
            tracing::Level::WARN => log::Level::Warn,
            _ => log::Level::Info,
        };
        Record {
            level,
            bytes: Vec::new(),
            truncated: false,
        }
    }
}
impl Write for Record {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = super::MAX_RECORD_BYTES.saturating_sub(self.bytes.len());
        self.bytes
            .extend_from_slice(&bytes[..remaining.min(bytes.len())]);
        self.truncated |= bytes.len() > remaining;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for Record {
    fn drop(&mut self) {
        if self.truncated {
            // Reserve space for the marker so the downstream record bound preserves it.
            self.bytes.truncate(super::MAX_RECORD_BYTES - 32);
            self.bytes
                .extend_from_slice(b"\n[tracing record truncated]\n");
        }
        log::log!(target: "nons::trace", self.level, "{}", String::from_utf8_lossy(&self.bytes).trim_end());
    }
}
