//! Retry loop shared by the vendor clients.
//!
//! Every client classifies an HTTP outcome into an `Outcome` (its own status/body mapping)
//! and hands the sending to `send_with_retry`, which owns the policy: transient failures and
//! rate limits are retried with [`backoff_delay`] (exponential, jittered, capped) or with the
//! server-mandated `retry-after` wait, fatal errors are returned at once and a `retry-after`
//! above the cap aborts the call instead of blocking a worker for minutes.

use std::time::Duration;

use rand::Rng;
use tracing::warn;
use ubiqx_core::{CoreError, CoreResult};

/// Retry settings of a client (see the `*Config` structs for the documented defaults).
#[derive(Debug, Clone, Copy)]
pub(crate) struct RetryPolicy {
    /// Attempts per call, including the first one. `0` behaves like `1`.
    pub max_attempts: u32,
    /// Wait before the first retry; doubled on every subsequent retry.
    pub backoff_base: Duration,
    /// Upper bound of any single wait. A `retry-after` above it aborts the call instead.
    pub backoff_cap: Duration,
}

/// Wait before retrying after the `attempt`-th failure (1-based): `base * 2^(attempt-1)`, capped
/// at `cap`, then scaled by `0.75 + 0.5 * jitter` (`jitter` in `[0, 1)`) and capped again.
pub fn backoff_delay(attempt: u32, base: Duration, cap: Duration, jitter: f64) -> Duration {
    let exp = attempt.saturating_sub(1).min(16);
    let raw = base.saturating_mul(1u32 << exp).min(cap);
    let factor = 0.75 + 0.5 * jitter.clamp(0.0, 1.0);
    raw.mul_f64(factor).min(cap)
}

/// What to do with an HTTP outcome.
#[derive(Debug)]
pub(crate) enum Outcome {
    Ok(String),
    Fatal(CoreError),
    /// Retry after an optional server-mandated wait (from `retry-after` or the error text).
    Retry {
        error: RetryableError,
        wait: Option<Duration>,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RetryableError {
    #[error("rate limited")]
    RateLimited { retry_after_secs: Option<u64> },
    #[error("transient failure: {0}")]
    Transient(String),
}

impl RetryableError {
    pub(crate) fn into_core(self, fallback_wait: Duration) -> CoreError {
        match self {
            RetryableError::RateLimited { retry_after_secs } => CoreError::RateLimited {
                retry_after_secs: retry_after_secs.unwrap_or(fallback_wait.as_secs().max(1)),
            },
            RetryableError::Transient(msg) => CoreError::Ai(msg),
        }
    }
}

/// Parses a `retry-after` header given in seconds (integer or decimal). HTTP dates are ignored.
pub(crate) fn parse_retry_after(value: Option<&reqwest::header::HeaderValue>) -> Option<u64> {
    let text = value?.to_str().ok()?.trim();
    if let Ok(secs) = text.parse::<u64>() {
        return Some(secs);
    }
    let secs: f64 = text.parse().ok()?;
    (secs.is_finite() && secs >= 0.0).then(|| secs.ceil() as u64)
}

/// Sends the request built by `build` until it succeeds, fails fatally or the policy is
/// exhausted. `classify` maps a response (status, parsed `retry-after` in seconds, body text)
/// onto an [`Outcome`]; network errors and timeouts are always transient. `vendor` and `what`
/// only label the log lines.
pub(crate) async fn send_with_retry<F, C>(
    policy: RetryPolicy,
    vendor: &'static str,
    what: &'static str,
    build: F,
    classify: C,
) -> CoreResult<String>
where
    F: Fn() -> reqwest::RequestBuilder,
    C: Fn(reqwest::StatusCode, Option<u64>, String) -> Outcome,
{
    let max_attempts = policy.max_attempts.max(1);
    let mut attempt: u32 = 1;
    loop {
        let outcome = match build().send().await {
            Ok(resp) => {
                let status = resp.status();
                let retry_after = parse_retry_after(resp.headers().get("retry-after"));
                let body = resp.text().await.unwrap_or_default();
                classify(status, retry_after, body)
            }
            Err(e) => {
                let msg = if e.is_timeout() {
                    format!("request timed out: {e}")
                } else {
                    format!("network error: {e}")
                };
                Outcome::Retry {
                    error: RetryableError::Transient(msg),
                    wait: None,
                }
            }
        };

        match outcome {
            Outcome::Ok(body) => return Ok(body),
            Outcome::Fatal(err) => {
                warn!(vendor, what, attempt, error = %err, "request failed");
                return Err(err);
            }
            Outcome::Retry { error, wait } => {
                let backoff = backoff_delay(
                    attempt,
                    policy.backoff_base,
                    policy.backoff_cap,
                    rand::thread_rng().gen::<f64>(),
                );
                let delay = match wait {
                    Some(w) if w > policy.backoff_cap => {
                        warn!(
                            vendor,
                            what,
                            attempt,
                            wait_secs = w.as_secs(),
                            "retry-after exceeds cap, giving up"
                        );
                        return Err(error.into_core(w));
                    }
                    Some(w) => w,
                    None => backoff,
                };
                if attempt >= max_attempts {
                    warn!(vendor, what, attempt, error = %error, "giving up after retries");
                    return Err(error.into_core(delay));
                }
                warn!(
                    vendor,
                    what,
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %error,
                    "retrying"
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_schedule() {
        let base = Duration::from_secs(2);
        let cap = Duration::from_secs(30);
        // jitter 0.5 → factor 1.0
        assert_eq!(backoff_delay(1, base, cap, 0.5), Duration::from_secs(2));
        assert_eq!(backoff_delay(2, base, cap, 0.5), Duration::from_secs(4));
        assert_eq!(backoff_delay(3, base, cap, 0.5), Duration::from_secs(8));
        assert_eq!(backoff_delay(4, base, cap, 0.5), Duration::from_secs(16));
        assert_eq!(backoff_delay(5, base, cap, 0.5), Duration::from_secs(30));
        assert_eq!(backoff_delay(40, base, cap, 0.5), Duration::from_secs(30));
        // Jitter stays within ±25 % and never exceeds the cap.
        assert_eq!(
            backoff_delay(1, base, cap, 0.0),
            Duration::from_millis(1500)
        );
        assert_eq!(
            backoff_delay(1, base, cap, 1.0),
            Duration::from_millis(2500)
        );
        assert_eq!(backoff_delay(5, base, cap, 1.0), cap);
        assert_eq!(backoff_delay(0, base, cap, 0.5), Duration::from_secs(2));
    }

    #[test]
    fn retry_after_parsing() {
        use reqwest::header::HeaderValue;
        assert_eq!(parse_retry_after(None), None);
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static("12"))),
            Some(12)
        );
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static("1.2"))),
            Some(2)
        );
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static(
                "Wed, 21 Oct 2015 07:28:00 GMT"
            ))),
            None
        );
    }

    #[test]
    fn rate_limit_fallback_wait_is_at_least_one_second() {
        let err = RetryableError::RateLimited {
            retry_after_secs: None,
        }
        .into_core(Duration::from_millis(10));
        assert!(matches!(
            err,
            CoreError::RateLimited {
                retry_after_secs: 1
            }
        ));
        let err = RetryableError::Transient("HTTP 503".into()).into_core(Duration::ZERO);
        assert!(matches!(err, CoreError::Ai(m) if m == "HTTP 503"));
    }
}
