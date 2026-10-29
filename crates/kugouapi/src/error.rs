//! Unified error type for the whole crate.
//!
//! `code` convention (mirrors the TypeScript `KuGouApiError`):
//!
//! - `> 0` — `KuGou` business error (`error_code` from the response envelope) or an
//!   HTTP status code
//! - `-1` — network error / timeout (the original cause is attached as `source`)
//! - `0` — uncategorised

use serde_json::Value;

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the `KuGou` client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Non-2xx HTTP response (the TS library surfaces these after the retry
    /// policy gives up; the response payload is included when it parsed as JSON).
    #[error("HTTP {status}: {context}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Human-readable context, e.g. `POST /v5/url`.
        context: String,
        /// Parsed JSON payload of the failed response, if any.
        payload: Option<Value>,
    },

    /// `KuGou` business error decoded from the response envelope
    /// (`error_code` / `errcode` families).
    #[error("KuGou API error (code={code}): {message}")]
    Api {
        /// Business error code (`error_code` or `errcode`).
        code: i64,
        /// Human-readable message.
        message: String,
        /// Full response payload for troubleshooting.
        payload: Option<Value>,
    },

    /// Transport-level failure (connection reset, DNS, TLS, timeout …).
    #[error("request failed: {context}")]
    Network {
        /// Human-readable context, e.g. `GET https://gateway.kugou.com/...`.
        context: String,
        /// Underlying transport error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// The response body was not valid JSON.
    #[error("response is not valid JSON: {context}")]
    InvalidResponse {
        /// Human-readable context, e.g. `GET /v2/search/song`.
        context: String,
        /// Raw body text (lossy UTF-8) when available.
        body: Option<String>,
    },

    /// `client.call` was invoked with an id that is not in the catalog.
    #[error("unknown endpoint: {0}")]
    UnknownEndpoint(String),

    /// Endpoint parameters failed to deserialize (dynamic `call` path).
    #[error("invalid params for `{endpoint}`: {message}")]
    InvalidParams {
        /// Endpoint id the params were destined for.
        endpoint: String,
        /// Deserialization error message.
        message: String,
    },

    /// A crypto primitive failed (bad key material, padding, RSA input …).
    #[error("crypto error: {0}")]
    Crypto(String),

    /// The circuit breaker is open and the request was rejected fast.
    #[error("circuit breaker open; retry in {retry_in_ms} ms")]
    CircuitOpen {
        /// Milliseconds until the breaker transitions to half-open.
        retry_in_ms: u64,
    },

    /// Rate limiter queue timed out while waiting for a token.
    #[error("rate limit queue timeout after {timeout_ms} ms")]
    RateLimitTimeout {
        /// Configured queue timeout in milliseconds.
        timeout_ms: u64,
    },

    /// JSON (de)serialization failure outside of a response body.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// reqwest client construction failed.
    #[cfg(feature = "reqwest-transport")]
    #[error("http client init failed: {0}")]
    HttpClientInit(String),
}

impl Error {
    /// Numeric error code following the TS convention
    /// (`>0` business/HTTP, `-1` network, `0` uncategorised).
    #[must_use]
    pub fn code(&self) -> i64 {
        match self {
            Self::Http { status, .. } => i64::from(*status),
            Self::Api { code, .. } => *code,
            Self::Network { .. } => -1,
            Self::InvalidResponse { .. }
            | Self::UnknownEndpoint(_)
            | Self::InvalidParams { .. }
            | Self::Crypto(_)
            | Self::CircuitOpen { .. }
            | Self::RateLimitTimeout { .. }
            | Self::Json(_) => 0,
            #[cfg(feature = "reqwest-transport")]
            Self::HttpClientInit(_) => 0,
        }
    }

    /// Server-returned payload for troubleshooting, when the error originated
    /// from a decoded response.
    #[must_use]
    pub fn payload(&self) -> Option<&Value> {
        match self {
            Self::Http { payload, .. } | Self::Api { payload, .. } => payload.as_ref(),
            _ => None,
        }
    }

    /// Extract the HTTP status code when this error came from a non-2xx response.
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }
}

impl Error {
    /// Build a [`Error::Network`] from any transport error.
    pub(crate) fn network(
        context: impl Into<String>,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::Network {
            context: context.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_convention() {
        let e = Error::Http {
            status: 404,
            context: "GET /x".into(),
            payload: None,
        };
        assert_eq!(e.code(), 404);
        assert_eq!(e.status(), Some(404));

        let e = Error::Api {
            code: 30001,
            message: "boom".into(),
            payload: None,
        };
        assert_eq!(e.code(), 30001);

        let e = Error::network("GET x", "dns".into());
        assert_eq!(e.code(), -1);

        let e = Error::UnknownEndpoint("nope".into());
        assert_eq!(e.code(), 0);
    }
}
