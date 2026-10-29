//! Token-bucket rate limiter.
//!
//! Optional (off by default). Compared to the reference implementation the
//! Rust port waits for a token instead of exposing a bounded queue: callers are
//! already bounded by their request timeout, which plays the role of the
//! reference's `queueTimeoutMs`.

use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Token-bucket rate limiter configuration.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitOptions {
    /// Sustained refill rate in tokens (requests) per second.
    pub requests_per_second: f64,
    /// Bucket capacity (burst size).
    pub burst: u32,
}

/// A simple token bucket.
#[derive(Debug)]
pub struct RateLimiter {
    options: RateLimitOptions,
    state: Mutex<BucketState>,
}

#[derive(Debug)]
struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a limiter.
    #[must_use]
    pub fn new(options: RateLimitOptions) -> Self {
        Self {
            options,
            state: Mutex::new(BucketState {
                tokens: f64::from(options.burst),
                last_refill: Instant::now(),
            }),
        }
    }

    /// Wait until a token is available.
    ///
    /// Returns an error when waiting would exceed `max_wait`.
    pub async fn acquire(&self, max_wait: Duration) -> crate::error::Result<()> {
        let deadline = Instant::now() + max_wait;
        loop {
            let wait = {
                let mut state = self.state.lock().await;
                let now = Instant::now();
                let elapsed = now.duration_since(state.last_refill).as_secs_f64();
                state.tokens = (state.tokens + elapsed * self.options.requests_per_second)
                    .min(f64::from(self.options.burst));
                state.last_refill = now;

                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    return Ok(());
                }

                // Time until one token refills.
                let deficit = 1.0 - state.tokens;
                let secs = deficit / self.options.requests_per_second.max(f64::MIN_POSITIVE);
                Duration::from_secs_f64(secs.min(0.25))
            };

            if Instant::now() + wait > deadline {
                // Wait out the remaining budget (mirroring the reference
                // queue timeout), then fail.
                let remaining = deadline.saturating_duration_since(Instant::now());
                tokio::time::sleep(remaining).await;
                return Err(crate::error::Error::RateLimitTimeout {
                    timeout_ms: max_wait.as_millis() as u64,
                });
            }
            tokio::time::sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn burst_then_throttle() {
        let limiter = RateLimiter::new(RateLimitOptions {
            requests_per_second: 50.0,
            burst: 2,
        });
        // Burst of 2 passes instantly.
        limiter.acquire(Duration::from_secs(1)).await.unwrap();
        limiter.acquire(Duration::from_secs(1)).await.unwrap();
        // The third needs a refill at 50 rps → ~20 ms.
        limiter.acquire(Duration::from_secs(1)).await.unwrap();
    }

    #[tokio::test]
    async fn times_out_when_starved() {
        let limiter = RateLimiter::new(RateLimitOptions {
            requests_per_second: 1.0,
            burst: 1,
        });
        limiter.acquire(Duration::from_secs(1)).await.unwrap();
        let started = Instant::now();
        let result = limiter.acquire(Duration::from_millis(50)).await;
        assert!(result.is_err());
        assert!(started.elapsed() >= Duration::from_millis(40));
    }
}
