//! Retry policy: backoff computation, `Retry-After` parsing, status sets.
//!
//! Port of the reference `@hglcode/httpclient` retry semantics:
//!
//! - exponential backoff: `initial * factor^max(0, next_attempt - 2)`
//! - half jitter by default: `[delay / 2, delay)`
//! - `Retry-After` (seconds or HTTP-date) takes precedence, clamped to
//!   `max_delay`
//! - retryable statuses: 408 / 425 / 429 / 500 / 502 / 503 / 504
//! - only idempotent methods are retried

use std::time::Duration;

/// Jitter strategy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Jitter {
    /// No jitter.
    None,
    /// Uniform in `[0, delay)`.
    Full,
    /// Uniform in `[delay / 2, delay)` — default, keeps the backoff trend
    /// while spreading concurrent retries.
    #[default]
    Half,
}

/// Retry policy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retries after the initial attempt.
    pub max_retries: u32,
    /// Backoff base for the first retry.
    pub initial_delay: Duration,
    /// Backoff ceiling.
    pub max_delay: Duration,
    /// Backoff multiplier.
    pub backoff_factor: f64,
    /// Jitter strategy.
    pub jitter: Jitter,
    /// Whether `Retry-After` takes precedence over backoff.
    pub respect_retry_after: bool,
    /// Status codes that trigger a retry (when the method is idempotent).
    pub retry_statuses: Vec<u16>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
            jitter: Jitter::Half,
            respect_retry_after: true,
            retry_statuses: vec![408, 425, 429, 500, 502, 503, 504],
        }
    }
}

impl RetryPolicy {
    /// Whether a status is retryable.
    #[must_use]
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retry_statuses.contains(&status)
    }

    /// Exponential backoff with jitter.
    ///
    /// `next_attempt` is the ordinal of the attempt about to start (2 = first
    /// retry), matching the reference implementation.
    #[must_use]
    pub fn backoff_ms(&self, next_attempt: u32) -> u64 {
        let exponent = next_attempt.saturating_sub(2);
        let mut delay =
            self.initial_delay.as_millis() as f64 * self.backoff_factor.powi(exponent as i32);
        if !delay.is_finite() || delay <= 0.0 {
            delay = self.initial_delay.as_millis() as f64;
        }
        delay = delay.min(self.max_delay.as_millis() as f64);

        use rand::RngExt;
        let mut rng = rand::rng();
        match self.jitter {
            Jitter::None => delay.round() as u64,
            Jitter::Full => (rng.random::<f64>() * delay).round() as u64,
            Jitter::Half => (delay / 2.0 + (rng.random::<f64>() * delay) / 2.0).round() as u64,
        }
    }

    /// Final retry wait: `Retry-After` when present and parseable (clamped to
    /// `max(max_delay, initial_delay)`), otherwise the jittered backoff.
    #[must_use]
    pub fn retry_delay_ms(&self, next_attempt: u32, retry_after: Option<&str>) -> u64 {
        if self.respect_retry_after
            && let Some(header) = retry_after
            && let Some(ms) = parse_retry_after_ms(header)
        {
            let cap = self
                .max_delay
                .as_millis()
                .max(self.initial_delay.as_millis()) as u64;
            return ms.min(cap);
        }
        self.backoff_ms(next_attempt)
    }
}

/// Parse a `Retry-After` header (delta-seconds or HTTP-date) into milliseconds.
/// Negative values clamp to 0; unparseable values return `None`.
#[must_use]
pub fn parse_retry_after_ms(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return trimmed
            .parse::<u64>()
            .ok()
            .and_then(|s| s.checked_mul(1000));
    }

    if let Ok(date) = httpdate::parse_http_date(trimmed) {
        let now = std::time::SystemTime::now();
        let delta = match date.duration_since(now) {
            Ok(d) => d.as_millis() as i64,
            Err(_) => -now.duration_since(date).map_or(0, |d| d.as_millis() as i64),
        };
        return Some(delta.max(0) as u64);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_first_retry_is_initial_delayish() {
        let policy = RetryPolicy::default();
        for _ in 0..50 {
            let ms = policy.backoff_ms(2);
            // half jitter: [100, 200]
            assert!((100..=200).contains(&ms), "out of range: {ms}");
        }
    }

    #[test]
    fn backoff_grows_and_caps() {
        let policy = RetryPolicy {
            jitter: Jitter::None,
            ..RetryPolicy::default()
        };
        assert_eq!(policy.backoff_ms(2), 200);
        assert_eq!(policy.backoff_ms(3), 400);
        assert_eq!(policy.backoff_ms(4), 800);
        assert_eq!(policy.backoff_ms(9), 25_600); // 200 * 2^7
        assert_eq!(policy.backoff_ms(10), 30_000); // capped
    }

    #[test]
    fn retry_after_seconds() {
        assert_eq!(parse_retry_after_ms("2"), Some(2000));
        assert_eq!(parse_retry_after_ms(" 2 "), Some(2000));
        assert_eq!(parse_retry_after_ms(""), None);
        assert_eq!(parse_retry_after_ms("soon"), None);
    }

    #[test]
    fn retry_after_takes_precedence_when_capped() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.retry_delay_ms(2, Some("1")), 1000);
        // clamped to max_delay (30s)
        assert_eq!(policy.retry_delay_ms(2, Some("600")), 30_000);
    }

    #[test]
    fn retryable_statuses() {
        let policy = RetryPolicy::default();
        assert!(policy.is_retryable_status(429));
        assert!(policy.is_retryable_status(503));
        assert!(!policy.is_retryable_status(404));
        assert!(!policy.is_retryable_status(200));
    }
}
