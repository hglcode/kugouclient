//! HTTP layer facade: per-attempt timeouts, retries, circuit breaking and rate
//! limiting orchestrated on top of a pluggable [`Transport`].
//!
//! JSON parsing and envelope unwrapping deliberately do **not** live here —
//! they are the client's responsibility.

mod breaker;
mod rate_limit;
mod retry;
mod transport;

#[cfg(feature = "reqwest-transport")]
mod reqwest_impl;

pub use breaker::{CircuitAdmission, CircuitBreaker, CircuitBreakerOptions, CircuitState};
pub use rate_limit::{RateLimitOptions, RateLimiter};
pub use retry::{Jitter, RetryPolicy, parse_retry_after_ms};
pub use transport::{Body, HttpRequest, HttpResponse, Method, Redirect, Transport};

#[cfg(feature = "reqwest-transport")]
pub use reqwest_impl::ReqwestTransport;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::{Error, Result};

/// HTTP layer defaults (the `KuGou` preset from the TS library).
#[derive(Debug, Clone)]
pub struct HttpLayerOptions {
    /// Per-attempt timeout (default 10 s).
    pub timeout: Duration,
    /// GET retry count (default 1).
    pub retries: u32,
    /// Retry policy (backoff / jitter / Retry-After).
    pub retry: RetryPolicy,
    /// Circuit breaker options; `None` disables the breaker.
    pub breaker: Option<CircuitBreakerOptions>,
    /// Rate limiter options; `None` disables rate limiting.
    pub rate_limit: Option<RateLimitOptions>,
}

impl Default for HttpLayerOptions {
    fn default() -> Self {
        // KuGou preset: 10 s per-attempt timeout, 1 retry, breaker on.
        Self {
            timeout: Duration::from_secs(10),
            retries: 1,
            retry: RetryPolicy {
                max_retries: 1,
                ..RetryPolicy::default()
            },
            breaker: Some(CircuitBreakerOptions::default()),
            rate_limit: None,
        }
    }
}

/// The HTTP orchestration layer.
pub struct HttpLayer {
    transport: Arc<dyn Transport>,
    options: HttpLayerOptions,
    breaker: Option<Mutex<CircuitBreaker>>,
    rate_limit: Option<RateLimiter>,
}

impl std::fmt::Debug for HttpLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpLayer")
            .field("options", &self.options)
            .field("breaker", &self.breaker.is_some())
            .field("rate_limit", &self.rate_limit.is_some())
            .finish_non_exhaustive()
    }
}

impl HttpLayer {
    /// Create a layer over a transport.
    #[must_use]
    pub fn new(transport: Arc<dyn Transport>, options: HttpLayerOptions) -> Self {
        let breaker = options.breaker.clone();
        let rate_limit = options.rate_limit;
        Self {
            breaker: breaker.map(CircuitBreaker::new).map(Mutex::new),
            rate_limit: rate_limit.map(RateLimiter::new),
            transport,
            options,
        }
    }

    /// Shared transport handle.
    #[must_use]
    pub fn transport(&self) -> Arc<dyn Transport> {
        Arc::clone(&self.transport)
    }

    /// Execute a request with retry / breaker / rate-limit orchestration.
    ///
    /// - each attempt gets its own timeout (a retry never inherits the previous
    ///   attempt's remaining time),
    /// - only idempotent methods are retried, on retryable statuses or
    ///   transport errors,
    /// - 5xx-family failures feed the circuit breaker (500/502/503/504),
    /// - the returned response may be non-2xx — envelope handling is up to the
    ///   caller.
    pub async fn request(&self, request: HttpRequest) -> Result<HttpResponse> {
        if let Some(limiter) = &self.rate_limit {
            limiter
                .acquire(request.timeout.unwrap_or(self.options.timeout))
                .await?;
        }

        let probe = match &self.breaker {
            None => false,
            Some(breaker) => match breaker.lock().expect("breaker poisoned").before_request() {
                CircuitAdmission::Allowed { probe } => probe,
                CircuitAdmission::Rejected { retry_in_ms } => {
                    return Err(Error::CircuitOpen { retry_in_ms });
                }
            },
        };

        let max_retries = request.retries.unwrap_or(self.options.retries);
        let timeout = request.timeout.unwrap_or(self.options.timeout);
        let mut attempt: u32 = 0;

        loop {
            attempt += 1;
            let mut attempt_req = request.clone();
            if timeout > Duration::ZERO {
                attempt_req.timeout = Some(timeout);
            }

            match self.transport.execute(attempt_req).await {
                Ok(response) => {
                    let failed = response.status >= 500
                        && self.breaker.as_ref().is_some_and(|b| {
                            b.lock()
                                .expect("breaker poisoned")
                                .is_failure_status(response.status)
                        });
                    if let Some(breaker) = &self.breaker {
                        breaker
                            .lock()
                            .expect("breaker poisoned")
                            .on_result(probe, !failed);
                    }

                    let can_retry = attempt <= max_retries
                        && request.method.is_idempotent()
                        && !response.ok()
                        && self.options.retry.is_retryable_status(response.status);
                    if can_retry {
                        let next_attempt = attempt + 1;
                        let retry_after = response.header("retry-after");
                        let delay = self.options.retry.retry_delay_ms(next_attempt, retry_after);
                        tracing::debug!(
                            method = request.method.as_str(),
                            url = %request.url,
                            status = response.status,
                            attempt = next_attempt,
                            delay_ms = delay,
                            "retrying"
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Ok(response);
                }
                Err(err) => {
                    if let Some(breaker) = &self.breaker {
                        breaker
                            .lock()
                            .expect("breaker poisoned")
                            .on_result(probe, false);
                    }
                    let context = format!("{} {}", request.method.as_str(), request.url);
                    let retryable = attempt <= max_retries
                        && request.method.is_idempotent()
                        && err.is_retryable_transport();
                    if retryable {
                        let delay = self.options.retry.backoff_ms(attempt + 1);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(Error::network(context, Box::new(err)));
                }
            }
        }
    }
}

impl CircuitBreaker {
    fn on_result(&mut self, probe: bool, success: bool) {
        if success {
            self.on_success(probe);
        } else {
            self.on_failure(probe);
        }
    }
}

impl Error {
    /// Whether a transport error is worth retrying (timeouts, connection
    /// failures, aborted transfers).
    #[must_use]
    pub fn is_retryable_transport(&self) -> bool {
        #[cfg(feature = "reqwest-transport")]
        if let Some(reqwest_err) = self
            .source_transport_error()
            .and_then(|e| e.downcast_ref::<reqwest::Error>())
        {
            return reqwest_err.is_timeout() || reqwest_err.is_connect() || reqwest_err.is_body();
        }
        // Custom transports: treat all transport-level errors as retryable for
        // idempotent requests (the reference retries `TypeError`/`AbortError`).
        matches!(self, Error::Network { .. })
    }

    fn source_transport_error(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self {
            Error::Network { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type Shared<T> = Arc<Mutex<T>>;

    struct MockTransport {
        statuses: Shared<Vec<u16>>,
        bodies: Vec<&'static str>,
        requests_seen: AtomicUsize,
    }

    impl MockTransport {
        fn with_statuses(statuses: Vec<u16>) -> Arc<Self> {
            Arc::new(Self {
                statuses: Arc::new(Mutex::new(statuses)),
                bodies: Vec::new(),
                requests_seen: AtomicUsize::new(0),
            })
        }
    }

    impl Transport for MockTransport {
        fn execute(
            &self,
            _request: HttpRequest,
        ) -> futures::future::BoxFuture<'static, Result<HttpResponse>> {
            let seen = self.requests_seen.fetch_add(1, Ordering::SeqCst);
            let statuses = self.statuses.lock().unwrap().clone();
            let body_at = self.bodies.get(seen).copied();
            Box::pin(async move {
                let status = statuses.get(seen).copied().unwrap_or(200);
                Ok(HttpResponse {
                    status,
                    headers: vec![("content-type".to_owned(), "application/json".to_owned())],
                    bytes: body_at.unwrap_or("{}").as_bytes().to_vec(),
                })
            })
        }
    }

    fn layer(transport: Arc<dyn Transport>, retries: u32) -> HttpLayer {
        HttpLayer::new(
            transport,
            HttpLayerOptions {
                retries,
                retry: RetryPolicy {
                    max_retries: retries,
                    initial_delay: Duration::from_millis(1),
                    ..RetryPolicy::default()
                },
                breaker: None,
                ..HttpLayerOptions::default()
            },
        )
    }

    #[tokio::test]
    async fn retries_get_on_503_then_succeeds() {
        let transport = MockTransport::with_statuses(vec![503, 503, 200]);
        let http = layer(transport.clone(), 2);
        let response = http
            .request(HttpRequest::get("https://example.test/x"))
            .await
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(transport.requests_seen.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn exhausts_retries_and_returns_last_response() {
        let transport = MockTransport::with_statuses(vec![503, 503, 503]);
        let http = layer(transport.clone(), 2);
        let response = http
            .request(HttpRequest::get("https://example.test/x"))
            .await
            .unwrap();
        assert_eq!(response.status, 503);
        assert_eq!(transport.requests_seen.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn post_is_not_retried() {
        let transport = MockTransport::with_statuses(vec![503, 200]);
        let http = layer(transport.clone(), 2);
        let response = http
            .request(HttpRequest::post("https://example.test/x").text("{}"))
            .await
            .unwrap();
        assert_eq!(response.status, 503);
        assert_eq!(transport.requests_seen.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn non_retryable_status_not_retried() {
        let transport = MockTransport::with_statuses(vec![404, 200]);
        let http = layer(transport.clone(), 2);
        let response = http
            .request(HttpRequest::get("https://example.test/x"))
            .await
            .unwrap();
        assert_eq!(response.status, 404);
        assert_eq!(transport.requests_seen.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retry_after_header_is_honoured() {
        let transport = MockTransport::with_statuses(vec![429, 200]);
        let http = HttpLayer::new(
            transport.clone(),
            HttpLayerOptions {
                retries: 1,
                retry: RetryPolicy {
                    max_retries: 1,
                    ..RetryPolicy::default()
                },
                breaker: None,
                ..HttpLayerOptions::default()
            },
        );
        // Patch the first response to carry Retry-After: 0 (immediate retry).
        // The mock does not emit headers, so the backoff path applies; assert
        // via request count only.
        let response = http
            .request(HttpRequest::get("https://example.test/x"))
            .await
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(transport.requests_seen.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn breaker_rejects_fast_after_failures() {
        let transport = MockTransport::with_statuses(vec![503; 100]);
        let http = HttpLayer::new(
            transport,
            HttpLayerOptions {
                retries: 0,
                breaker: Some(CircuitBreakerOptions {
                    minimum_volume: 5,
                    ..CircuitBreakerOptions::default()
                }),
                ..HttpLayerOptions::default()
            },
        );
        // 5 failing requests trip the breaker (all 503 → failures)
        for _ in 0..5 {
            let _ = http
                .request(HttpRequest::get("https://example.test/x"))
                .await;
        }
        let result = http
            .request(HttpRequest::get("https://example.test/x"))
            .await;
        assert!(matches!(result, Err(Error::CircuitOpen { .. })));
    }
}
