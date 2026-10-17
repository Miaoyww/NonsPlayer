//! Only read operations opt in. The caller owns the original context/deadline
//! and monitors it while this future waits; no permit is retained between attempts.
use crate::music::{ErrorCode, MusicResult};
use std::{future::Future, time::Duration};

const DELAYS_MS: [u64; 3] = [500, 1000, 2000];

pub(super) async fn rate_limited<T, F, Fut>(operation: F) -> MusicResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = MusicResult<T>>,
{
    rate_limited_or(operation, |_| false).await
}

/// Partial responses (such as a library summary with unavailable favorites)
/// retain their data on exhaustion while receiving the same retry budget.
pub(super) async fn rate_limited_or<T, F, Fut>(
    mut operation: F,
    partially_busy: impl Fn(&T) -> bool,
) -> MusicResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = MusicResult<T>>,
{
    let mut retries = 0;
    loop {
        let result = operation().await;
        let busy = match &result {
            Err(error) => error.code == ErrorCode::RateLimited,
            Ok(value) => partially_busy(value),
        };
        if !busy || retries == DELAYS_MS.len() {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(DELAYS_MS[retries])).await;
        retries += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, time::Instant};

    #[tokio::test]
    async fn busy_reads_recover_on_the_third_retry() {
        let calls = Cell::new(0);
        let start = Instant::now();
        let result = rate_limited(|| {
            calls.set(calls.get() + 1);
            std::future::ready(if calls.get() < 4 {
                Err(ErrorCode::RateLimited.into())
            } else {
                Ok(42)
            })
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.get(), 4);
        assert!(start.elapsed() >= Duration::from_millis(3500));
    }

    #[tokio::test]
    async fn persistent_busy_error_stops_after_four_calls() {
        let calls = Cell::new(0);
        let result: MusicResult<()> = rate_limited(|| {
            calls.set(calls.get() + 1);
            std::future::ready(Err(ErrorCode::RateLimited.into()))
        })
        .await;
        assert_eq!(result.unwrap_err().code, ErrorCode::RateLimited);
        assert_eq!(calls.get(), 4);
    }

    #[tokio::test]
    async fn partial_busy_responses_retry_and_keep_the_final_data() {
        let calls = Cell::new(0);
        let result = rate_limited_or(
            || {
                calls.set(calls.get() + 1);
                std::future::ready(Ok(calls.get()))
            },
            |_| true,
        )
        .await;
        assert_eq!(result.unwrap(), 4);
        assert_eq!(calls.get(), 4);
    }

    #[tokio::test]
    async fn other_errors_are_not_retried_even_after_a_busy_response() {
        for code in [
            ErrorCode::Network,
            ErrorCode::StaleContext,
            ErrorCode::Cancelled,
        ] {
            let calls = Cell::new(0);
            let result: MusicResult<()> = rate_limited(|| {
                calls.set(calls.get() + 1);
                std::future::ready(Err(if calls.get() == 1 {
                    ErrorCode::RateLimited.into()
                } else {
                    code.into()
                }))
            })
            .await;
            assert_eq!(result.unwrap_err().code, code);
            assert_eq!(calls.get(), 2);
        }
    }
}
