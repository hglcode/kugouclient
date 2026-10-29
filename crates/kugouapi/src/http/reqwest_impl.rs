//! reqwest-backed [`Transport`] implementation (rustls TLS, gzip, no cookie
//! store).
//!
//! Two `reqwest::Client`s are kept: one follows redirects (the default) and one
//! uses `Policy::none()` for endpoints that must inspect 30x responses and
//! `set-cookie` headers manually (the QR login flows). This mirrors the TS
//! layer's per-request `redirect: 'manual'` option.

use futures::future::BoxFuture;

use super::transport::{Body as HttpBody, HttpRequest, HttpResponse, Redirect, Transport};
use crate::error::{Error, Result};

/// reqwest-based transport.
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    follow: reqwest::Client,
    manual: reqwest::Client,
}

impl ReqwestTransport {
    /// Create the transport with default client options.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying reqwest clients fail to build
    /// (e.g. TLS backend initialization failure).
    pub fn new() -> Result<Self> {
        let follow = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::default())
            .build()
            .map_err(|e| Error::HttpClientInit(e.to_string()))?;
        let manual = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| Error::HttpClientInit(e.to_string()))?;
        Ok(Self { follow, manual })
    }
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::new().expect("reqwest client init failed")
    }
}

impl Transport for ReqwestTransport {
    fn execute(&self, request: HttpRequest) -> BoxFuture<'static, Result<HttpResponse>> {
        let client = match request.redirect {
            Redirect::Follow => self.follow.clone(),
            Redirect::Manual => self.manual.clone(),
        };

        Box::pin(async move {
            let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
                .map_err(|e| Error::HttpClientInit(e.to_string()))?;

            let mut builder = client.request(method, &request.url);
            if !request.query.is_empty() {
                let pairs: Vec<(String, String)> = request
                    .query
                    .iter()
                    .map(|(k, v)| (k.to_owned(), v.to_wire_string()))
                    .collect();
                builder = builder.query(&pairs);
            }

            for (name, value) in &request.headers {
                builder = builder.header(name, value);
            }

            match &request.body {
                Some(HttpBody::Text(text)) => builder = builder.body(text.clone()),
                Some(HttpBody::Bytes(bytes)) => builder = builder.body(bytes.clone()),
                None => {}
            }

            if let Some(timeout) = request.timeout
                && timeout > std::time::Duration::ZERO
            {
                builder = builder.timeout(timeout);
            }

            let response = builder.send().await.map_err(|e| {
                Error::network(
                    format!("{} {}", request.method.as_str(), request.url),
                    Box::new(e),
                )
            })?;

            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_owned(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect::<Vec<_>>();
            let bytes = response.bytes().await.map_err(|e| {
                Error::network(
                    format!("{} {}", request.method.as_str(), request.url),
                    Box::new(e),
                )
            })?;

            Ok(HttpResponse {
                status,
                headers,
                bytes: bytes.to_vec(),
            })
        })
    }
}
