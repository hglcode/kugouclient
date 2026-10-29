//! Transport abstraction.
//!
//! The HTTP layer is decoupled from reqwest through the [`Transport`] trait so
//! tests can run fully offline and alternative runtimes can plug in their own
//! implementation.

use futures::future::BoxFuture;

use crate::error::Result;
use crate::query::QueryRecord;

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// GET
    Get,
    /// POST
    Post,
    /// DELETE
    Delete,
    /// PUT
    Put,
    /// HEAD
    Head,
    /// OPTIONS
    Options,
    /// TRACE
    Trace,
}

impl Method {
    /// Uppercase wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Delete => "DELETE",
            Self::Put => "PUT",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
        }
    }

    /// Whether the method is idempotent (eligible for retries).
    #[must_use]
    pub fn is_idempotent(self) -> bool {
        matches!(
            self,
            Self::Get | Self::Head | Self::Options | Self::Trace | Self::Put | Self::Delete
        )
    }
}

/// Request body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// UTF-8 text body (JSON, form text, raw strings).
    Text(String),
    /// Binary body (image uploads, fingerprints …).
    Bytes(Vec<u8>),
}

impl Body {
    /// Raw bytes view.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(s) => s.as_bytes(),
            Self::Bytes(b) => b,
        }
    }
}

/// Redirect handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Redirect {
    /// Follow redirects (default).
    #[default]
    Follow,
    /// Do not follow redirects (QR login flows need to inspect 30x + cookies).
    Manual,
}

/// An outgoing HTTP request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method.
    pub method: Method,
    /// Full URL **without** the query string (it is carried separately so the
    /// layer can replay it across retry attempts).
    pub url: String,
    /// Query parameters (insertion ordered).
    pub query: QueryRecord,
    /// Headers (duplicates allowed, later values appended).
    pub headers: Vec<(String, String)>,
    /// Request body.
    pub body: Option<Body>,
    /// Per-attempt timeout override.
    pub timeout: Option<std::time::Duration>,
    /// Retry-count override for this request.
    pub retries: Option<u32>,
    /// Redirect policy for this request.
    pub redirect: Redirect,
}

impl HttpRequest {
    /// Build a GET request.
    #[must_use]
    pub fn get(url: impl Into<String>) -> Self {
        Self::new(Method::Get, url)
    }

    /// Build a POST request.
    #[must_use]
    pub fn post(url: impl Into<String>) -> Self {
        Self::new(Method::Post, url)
    }

    /// Build a request for a method.
    #[must_use]
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            query: QueryRecord::new(),
            headers: Vec::new(),
            body: None,
            timeout: None,
            retries: None,
            redirect: Redirect::Follow,
        }
    }

    /// Append a header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the body (text).
    #[must_use]
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.body = Some(Body::Text(body.into()));
        self
    }

    /// Set the body (bytes).
    #[must_use]
    pub fn bytes(mut self, body: Vec<u8>) -> Self {
        self.body = Some(Body::Bytes(body));
        self
    }

    /// Set the per-attempt timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the redirect policy.
    #[must_use]
    pub fn redirect(mut self, redirect: Redirect) -> Self {
        self.redirect = redirect;
        self
    }
}

/// An HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Headers with duplicates preserved (`set-cookie` repeats).
    pub headers: Vec<(String, String)>,
    /// Raw response bytes.
    pub bytes: Vec<u8>,
}

impl HttpResponse {
    /// `response.ok` semantics: 2xx.
    #[must_use]
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// First header value (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// All header values (case-insensitive), in order.
    #[must_use]
    pub fn headers_all(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// `content-type` header (empty string when absent).
    #[must_use]
    pub fn content_type(&self) -> &str {
        self.header("content-type").unwrap_or("")
    }

    /// Lossy UTF-8 view of the body.
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    /// Parse the body as JSON, `None` when invalid / empty.
    #[must_use]
    pub fn json(&self) -> Option<serde_json::Value> {
        if self.bytes.is_empty() {
            return None;
        }
        serde_json::from_slice(&self.bytes).ok()
    }
}

/// Pluggable HTTP transport.
///
/// The default implementation ([`crate::http::ReqwestTransport`]) is backed by
/// reqwest + rustls. Implement this trait to inject a mock (tests) or a custom
/// stack.
pub trait Transport: Send + Sync + 'static {
    /// Execute a single attempt. Retries / timeouts beyond the per-attempt
    /// timeout are orchestrated by [`crate::http::HttpLayer`], not here.
    fn execute(&self, request: HttpRequest) -> BoxFuture<'static, Result<HttpResponse>>;
}
