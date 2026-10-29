//! Sliding-window circuit breaker.
//!
//! Faithful port of the reference `@hglcode/httpclient` breaker:
//!
//! - ring of `buckets` time buckets covering `window_duration`
//! - trips open when `failures / total >= failure_rate_threshold` with at least
//!   `minimum_volume` samples
//! - stays open for `open_duration`, then transitions to half-open
//! - half-open admits at most `half_open_max_probes` concurrent probes and
//!   closes again after `half_open_success_threshold` probe successes

use std::time::{Duration, Instant};

/// Breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Requests flow normally.
    Closed,
    /// Requests are rejected fast.
    Open,
    /// Limited probes are admitted to test recovery.
    HalfOpen,
}

/// Admission decision returned by [`CircuitBreaker::before_request`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitAdmission {
    /// Allowed; `probe` marks half-open probe requests.
    Allowed {
        /// Whether this request is a half-open probe.
        probe: bool,
    },
    /// Rejected; retry after this many milliseconds.
    Rejected {
        /// Milliseconds until the circuit allows requests again.
        retry_in_ms: u64,
    },
}

/// Circuit breaker options.
#[derive(Debug, Clone)]
pub struct CircuitBreakerOptions {
    /// Sliding window length.
    pub window_duration: Duration,
    /// Number of window buckets.
    pub buckets: usize,
    /// Minimum samples before the failure rate is evaluated.
    pub minimum_volume: u64,
    /// Failure rate (0.0 - 1.0) that trips the breaker.
    pub failure_rate_threshold: f64,
    /// How long the breaker stays open.
    pub open_duration: Duration,
    /// Concurrent probes admitted in half-open.
    pub half_open_max_probes: u32,
    /// Probe successes required to close again.
    pub half_open_success_threshold: u32,
    /// Statuses counted as failures.
    pub failure_status_codes: Vec<u16>,
}

impl Default for CircuitBreakerOptions {
    fn default() -> Self {
        Self {
            window_duration: Duration::from_secs(10),
            buckets: 10,
            minimum_volume: 10,
            failure_rate_threshold: 0.5,
            open_duration: Duration::from_secs(30),
            half_open_max_probes: 5,
            half_open_success_threshold: 3,
            failure_status_codes: vec![500, 502, 503, 504],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    slot: i64,
    total: u64,
    failures: u64,
}

/// The breaker state machine; transport-agnostic and independently testable.
#[derive(Debug)]
pub struct CircuitBreaker {
    options: CircuitBreakerOptions,
    bucket_data: Vec<Bucket>,
    bucket_ms: u64,
    state: CircuitState,
    opened_at: Instant,
    half_open_active: u32,
    half_open_successes: u32,
    half_open_completed: u32,
}

impl CircuitBreaker {
    /// Create a breaker with options.
    #[must_use]
    pub fn new(options: CircuitBreakerOptions) -> Self {
        let bucket_ms = options.window_duration.as_millis() as u64 / options.buckets.max(1) as u64;
        Self {
            bucket_data: vec![
                Bucket {
                    slot: -1,
                    total: 0,
                    failures: 0
                };
                options.buckets
            ],
            bucket_ms: bucket_ms.max(1),
            state: CircuitState::Closed,
            opened_at: Instant::now(),
            half_open_active: 0,
            half_open_successes: 0,
            half_open_completed: 0,
            options,
        }
    }

    /// Current state (with the lazy open → half-open transition applied).
    #[must_use]
    pub fn state(&self) -> CircuitState {
        if self.state == CircuitState::Open
            && self.opened_at.elapsed() >= self.options.open_duration
        {
            CircuitState::HalfOpen
        } else {
            self.state
        }
    }

    /// Whether `status` counts as a failure for this breaker.
    #[must_use]
    pub fn is_failure_status(&self, status: u16) -> bool {
        self.options.failure_status_codes.contains(&status)
    }

    /// Request admission. Open circuits fail fast; expired ones move to
    /// half-open and admit limited probes.
    pub fn before_request(&mut self) -> CircuitAdmission {
        if self.state == CircuitState::Open
            && self.opened_at.elapsed() >= self.options.open_duration
        {
            self.transition(CircuitState::HalfOpen);
        }

        match self.state {
            CircuitState::Closed => CircuitAdmission::Allowed { probe: false },
            CircuitState::Open => CircuitAdmission::Rejected {
                retry_in_ms: self
                    .options
                    .open_duration
                    .saturating_sub(self.opened_at.elapsed())
                    .as_millis() as u64,
            },
            CircuitState::HalfOpen => {
                if self.half_open_active < self.options.half_open_max_probes {
                    self.half_open_active += 1;
                    CircuitAdmission::Allowed { probe: true }
                } else {
                    // Probe slots exhausted: in-flight probes will settle soon;
                    // suggest a gentle re-check interval.
                    CircuitAdmission::Rejected {
                        retry_in_ms: 5_000
                            .min((self.options.open_duration.as_millis() as u64 / 4).max(100)),
                    }
                }
            }
        }
    }

    /// Record a completed request (success side).
    pub fn on_success(&mut self, probe: bool) {
        if probe {
            self.half_open_active = self.half_open_active.saturating_sub(1);
            self.half_open_successes += 1;
            self.half_open_completed += 1;
            self.evaluate_half_open();
        } else {
            self.record(false);
            self.maybe_trip();
        }
    }

    /// Record a completed request (failure side).
    pub fn on_failure(&mut self, probe: bool) {
        if probe {
            self.half_open_active = self.half_open_active.saturating_sub(1);
            self.half_open_completed += 1;
            let remaining = self
                .options
                .half_open_max_probes
                .saturating_sub(self.half_open_completed);
            // Remaining probes cannot reach the success threshold → re-open
            // immediately instead of wasting probe slots.
            if self.half_open_successes + remaining < self.options.half_open_success_threshold {
                self.transition(CircuitState::Open);
            } else {
                self.evaluate_half_open();
            }
        } else {
            self.record(true);
            self.maybe_trip();
        }
    }

    fn evaluate_half_open(&mut self) {
        if self.half_open_successes >= self.options.half_open_success_threshold
            || self.half_open_completed >= self.options.half_open_max_probes
        {
            if self.half_open_successes >= self.options.half_open_success_threshold {
                self.transition(CircuitState::Closed);
            } else {
                self.transition(CircuitState::Open);
            }
        }
    }

    fn maybe_trip(&mut self) {
        if self.state != CircuitState::Closed {
            return;
        }
        let (total, failures) = self.window_stats();
        if total >= self.options.minimum_volume
            && failures as f64 / total as f64 >= self.options.failure_rate_threshold
        {
            self.transition(CircuitState::Open);
        }
    }

    fn record(&mut self, failed: bool) {
        let now_ms = now_ms();
        let slot = (now_ms / self.bucket_ms) as i64;
        let len = self.bucket_data.len();
        let bucket = &mut self.bucket_data[(slot as usize) % len];
        if bucket.slot != slot {
            bucket.slot = slot;
            bucket.total = 0;
            bucket.failures = 0;
        }
        bucket.total += 1;
        if failed {
            bucket.failures += 1;
        }
    }

    fn window_stats(&self) -> (u64, u64) {
        let now_ms = now_ms();
        let min_slot = now_ms.saturating_sub(self.options.window_duration.as_millis() as u64)
            as i64
            / self.bucket_ms as i64;
        let mut total = 0;
        let mut failures = 0;
        for bucket in &self.bucket_data {
            if bucket.slot >= min_slot {
                total += bucket.total;
                failures += bucket.failures;
            }
        }
        (total, failures)
    }

    fn transition(&mut self, to: CircuitState) {
        if self.state == to {
            return;
        }
        self.state = to;
        if to == CircuitState::Open || to == CircuitState::HalfOpen {
            self.opened_at = Instant::now();
            self.half_open_active = 0;
            self.half_open_successes = 0;
            self.half_open_completed = 0;
        }
        if to == CircuitState::Closed {
            // Recovering clears the window: historic failures must not
            // immediately re-trip the freshly recovered breaker.
            for bucket in &mut self.bucket_data {
                bucket.slot = -1;
                bucket.total = 0;
                bucket.failures = 0;
            }
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_breaker() -> CircuitBreaker {
        CircuitBreaker::new(CircuitBreakerOptions {
            minimum_volume: 4,
            failure_rate_threshold: 0.5,
            open_duration: Duration::from_millis(50),
            half_open_max_probes: 2,
            half_open_success_threshold: 1,
            ..CircuitBreakerOptions::default()
        })
    }

    #[test]
    fn closed_allows_everything() {
        let mut breaker = fast_breaker();
        assert_eq!(
            breaker.before_request(),
            CircuitAdmission::Allowed { probe: false }
        );
    }

    #[test]
    fn trips_after_failure_threshold() {
        let mut breaker = fast_breaker();
        for _ in 0..4 {
            assert_eq!(
                breaker.before_request(),
                CircuitAdmission::Allowed { probe: false }
            );
            breaker.on_failure(false);
        }
        assert!(matches!(
            breaker.before_request(),
            CircuitAdmission::Rejected { .. }
        ));
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn opens_then_half_opens_then_closes() {
        let mut breaker = fast_breaker();
        for _ in 0..4 {
            let _ = breaker.before_request();
            breaker.on_failure(false);
        }
        assert!(matches!(
            breaker.before_request(),
            CircuitAdmission::Rejected { .. }
        ));

        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(
            breaker.before_request(),
            CircuitAdmission::Allowed { probe: true }
        );
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // One successful probe closes the breaker (threshold = 1)
        breaker.on_success(true);
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(
            breaker.before_request(),
            CircuitAdmission::Allowed { probe: false }
        );
    }

    #[test]
    fn failure_status_detection() {
        let breaker = CircuitBreaker::new(CircuitBreakerOptions::default());
        assert!(breaker.is_failure_status(503));
        assert!(!breaker.is_failure_status(429)); // 429 retries but is not a breaker failure
        assert!(!breaker.is_failure_status(200));
    }

    #[test]
    fn success_reduces_failure_rate() {
        // 2 failures of 4 samples: 0.5 >= 0.5 → trips (evaluated per sample)
        let mut breaker = fast_breaker();
        for _ in 0..2 {
            let _ = breaker.before_request();
            breaker.on_failure(false);
        }
        for _ in 0..2 {
            let _ = breaker.before_request();
            breaker.on_success(false);
        }
        assert_eq!(breaker.state(), CircuitState::Open);

        // 1 failure of 4 samples: 0.25 → stays closed
        let mut breaker = fast_breaker();
        let _ = breaker.before_request();
        breaker.on_failure(false);
        for _ in 0..3 {
            let _ = breaker.before_request();
            breaker.on_success(false);
        }
        assert_eq!(breaker.state(), CircuitState::Closed);
    }
}
