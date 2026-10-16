//! Diagnostics from recovered guest failures share the host's bounded, redacted sink.
use crate::model::AppResult;
use serde::Deserialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    operation: String,
    message: String,
}

pub(super) fn report(
    id: &str,
    budget: &Mutex<(Instant, usize)>,
    args: serde_json::Value,
) -> AppResult<()> {
    let entry: Entry = serde_json::from_value(args).map_err(|_| "invalidData")?;
    if entry.operation.is_empty()
        || entry.operation.len() > 64
        || !entry
            .operation
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        || entry.message.len() > 4096
    {
        return Err("invalidData".into());
    }
    let mut budget = budget.lock().map_err(|_| "internal")?;
    if budget.0.elapsed() >= Duration::from_secs(1) {
        *budget = (Instant::now(), 0);
    }
    if budget.1 >= 16 {
        return Err("rateLimited".into());
    }
    budget.1 += 1;
    log::warn!(target: "nons::plugin", "plugin={id} operation={} reason={}", entry.operation, entry.message);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn guest_diagnostics_enforce_payload_and_rate_limits() {
        let budget = Mutex::new((Instant::now(), 0));
        for args in [
            json!({"operation":"parse", "message":"x".repeat(4097)}),
            json!({"operation":"parse\nforged", "message":"failed"}),
            json!({"operation":"parse", "message":"failed", "payload":"private"}),
        ] {
            assert_eq!(report("fixture", &budget, args).unwrap_err(), "invalidData");
        }
        for _ in 0..16 {
            report(
                "fixture",
                &budget,
                json!({"operation":"parse", "message":"invalid response at line 1"}),
            )
            .unwrap();
        }
        assert_eq!(
            report(
                "fixture",
                &budget,
                json!({"operation":"parse", "message":"failed"})
            )
            .unwrap_err(),
            "rateLimited"
        );
        *budget.lock().unwrap() = (Instant::now() - Duration::from_secs(2), 16);
        report(
            "fixture",
            &budget,
            json!({"operation":"parse", "message":"recovered"}),
        )
        .unwrap();
    }
}
