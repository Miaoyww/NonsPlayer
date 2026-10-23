//! Bounded audio diagnostics. Streaming threads never write application logs.
use gstreamer::{self as gst, prelude::*};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{self, SyncSender},
    LazyLock, Once, OnceLock,
};
use std::time::{Duration, Instant};

struct Record {
    info: bool,
    text: String,
}

struct Gate(AtomicU64);
impl Gate {
    const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    fn admit(&self, now: u64) -> bool {
        let next = self.0.load(Ordering::Relaxed);
        now >= next
            && self
                .0
                .compare_exchange(
                    next,
                    now.saturating_add(1_000_000_000),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
    }
}

static SENDER: OnceLock<SyncSender<Record>> = OnceLock::new();
static START: LazyLock<Instant> = LazyLock::new(Instant::now);
// Separate admission for native warnings, bus warnings, QoS and output facts.
static GATES: [Gate; 4] = [const { Gate::new() }; 4];
static SUPPRESSED: AtomicU64 = AtomicU64::new(0);

fn submit(kind: usize, info: bool, text: impl FnOnce() -> String) {
    let Some(sender) = SENDER.get() else {
        return;
    };
    if !GATES[kind].admit(START.elapsed().as_nanos().min(u64::MAX as u128) as u64) {
        SUPPRESSED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let record = Record { info, text: text() };
    if sender.try_send(record).is_err() {
        SUPPRESSED.fetch_add(1, Ordering::Relaxed);
    }
}

fn bounded(text: &str) -> String {
    text.chars().take(1024).collect()
}

pub fn initialize() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let (sender, receiver) = mpsc::sync_channel::<Record>(32);
        if let Err(error) = std::thread::Builder::new()
            .name("audio-diagnostics".into())
            .spawn(move || loop {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(record) => {
                        let text = crate::infrastructure::diagnostics::redact(&record.text);
                        if record.info {
                            log::info!(target: "nons::audio", "{text}");
                        } else {
                            log::warn!(target: "nons::audio", "{text}");
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let suppressed = SUPPRESSED.swap(0, Ordering::Relaxed);
                if suppressed > 0 {
                    log::info!(target: "nons::audio", "operation=diagnostics suppressed={suppressed}");
                }
            })
        {
            crate::infrastructure::diagnostics::warn("audio.diagnostics", error);
            return;
        }
        let _ = SENDER.set(sender);
        // Keep this process-wide hook for the GStreamer runtime lifetime. Only
        // audio categories at warning/error level are admitted; never HTTP logs.
        gst::log::add_log_function(|category, level, _, _, _, _, message| {
            let name = category.name();
            if !matches!(level, gst::DebugLevel::Error | gst::DebugLevel::Warning)
                || !(name == "audiobasesink"
                    || name == "audioresample"
                    || name.starts_with("wasapi2"))
            {
                return;
            }
            submit(0, false, || {
                let text = message.get().map(|m| bounded(m.as_str())).unwrap_or_default();
                format!("operation=native_warning category={name} reason={text}")
            });
        });
        // Preserve explicit GST_DEBUG settings, including more detailed tracing.
        if std::env::var_os("GST_DEBUG").is_none() {
            // Otherwise enabling warnings would also make GStreamer's default
            // callback synchronously write unredacted stderr on streaming threads.
            gst::log::remove_default_log_function();
            for category in ["audiobasesink", "audioresample", "wasapi2*"] {
                gst::log::set_threshold_for_name(category, gst::DebugLevel::Warning);
            }
        }
    });
}

pub fn message(message: &gst::MessageRef) {
    match message.view() {
        gst::MessageView::Warning(warning) => submit(1, false, || {
            format!(
                "operation=bus_warning reason={}",
                bounded(warning.error().message())
            )
        }),
        gst::MessageView::Qos(qos) => submit(2, false, || {
            let (jitter, proportion, quality) = qos.values();
            let (processed, dropped) = qos.stats();
            format!("operation=qos jitter_ns={jitter} proportion={proportion} quality={quality} processed={processed:?} dropped={dropped:?}")
        }),
        gst::MessageView::NewClock(clock) => {
            if let Some(clock) = clock.clock() {
                // Clock selection is infrequent; do not let it consume the
                // negotiated output-format admission slot.
                log::info!(target: "nons::audio", "operation=clock_selected type={}", clock.type_().name());
            }
        }
        _ => {}
    }
}

pub fn output(playbin: &gst::Element) {
    let Some(bin) = playbin.downcast_ref::<gst::Bin>() else {
        return;
    };
    for element in bin.iterate_recurse().into_iter().flatten() {
        let Some(factory) = element.factory() else {
            continue;
        };
        if !factory
            .metadata("klass")
            .is_some_and(|klass| klass.contains("Sink/Audio"))
        {
            continue;
        }
        let Some(caps) = element
            .static_pad("sink")
            .and_then(|pad| pad.current_caps())
        else {
            continue;
        };
        let Some(format) = caps.structure(0) else {
            continue;
        };
        // Record only format facts, never track URI, tags or arbitrary caps.
        submit(3, true, || {
            format!(
                "operation=output_format backend={} format={} rate={} channels={}",
                factory.name(),
                format.get::<&str>("format").unwrap_or("unknown"),
                format.get::<i32>("rate").unwrap_or(0),
                format.get::<i32>("channels").unwrap_or(0),
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_storm_has_one_admission_per_second() {
        let gate = Gate::new();
        assert!(gate.admit(0));
        for _ in 0..10_000 {
            assert!(!gate.admit(999_999_999));
        }
        assert!(gate.admit(1_000_000_000));
    }

    #[test]
    fn simultaneous_native_warnings_share_the_budget() {
        let gate = std::sync::Arc::new(Gate::new());
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let gate = gate.clone();
                std::thread::spawn(move || gate.admit(0))
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .map(|t| usize::from(t.join().unwrap()))
                .sum::<usize>(),
            1
        );
    }
}
