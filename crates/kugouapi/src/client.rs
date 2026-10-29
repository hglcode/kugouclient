//! The `KuGou` API client.
//!
//! Design (mirrors the TS `KuGouClient`):
//!
//! - a handful of high-frequency endpoints get typed methods
//!   (`client.search().song(..)`, `client.song().url(..)`,
//!   [`KuGouClient::register_device`]),
//! - `KuGouClient::call` dispatches the remaining endpoints through the
//!   271-entry catalog,
//! - `KuGouClient::request` is a generic escape hatch for endpoints the
//!   catalog does not know (yet),
//! - register / login endpoints maintain the session automatically.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde_json::Value;

use crate::config::{self, DEFAULT_USER_AGENT, Platform};
use crate::crypto::{CipherInput, CryptoProvider, DefaultCryptoProvider};
use crate::device::{DEFAULT_DEVICE, Device, derive_mid};
use crate::error::{Error, Result};
use crate::http::{HttpLayer, HttpLayerOptions, HttpRequest, Method, Transport};
use crate::query::QueryRecord;
use crate::session::SessionSnapshot;
use crate::sign::{self, SignKind, SignOptions};

/// Options for [`KuGouClient::request`] (the generic escape hatch).
#[derive(Default)]
pub struct RequestOptions {
    /// HTTP method, GET by default.
    pub method: Option<Method>,
    /// Endpoint-specific query params (merged over the identity params).
    pub query: Option<QueryRecord>,
    /// Extra headers.
    pub headers: Vec<(String, String)>,
    /// Request body.
    pub body: Option<String>,
    /// Base URL override (the gateway by default).
    pub base_url: Option<String>,
    /// `x-router` header.
    pub router: Option<String>,
    /// Signature control, android by default.
    pub sign: Option<crate::catalog::ctx::SignControl>,
    /// Do not inject the default identity params.
    pub clear_params: bool,
    /// Per-request timeout.
    pub timeout: Option<Duration>,
    /// Per-request retry override.
    pub retries: Option<u32>,
}

impl RequestOptions {
    /// Default options (GET, android signature).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

struct ClientInner {
    platform: Platform,
    appid: u64,
    clientver: u64,
    base_url: String,
    ip: Option<String>,
    device: Device,
    crypto: Box<dyn CryptoProvider>,
    http: HttpLayer,
    session: RwLock<Option<SessionSnapshot>>,
}

/// The `KuGou` API client. Cheaply cloneable; the session is shared between
/// clones.
#[derive(Clone)]
pub struct KuGouClient {
    inner: Arc<ClientInner>,
}

impl std::fmt::Debug for KuGouClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KuGouClient")
            .field("platform", &self.inner.platform)
            .field("appid", &self.inner.appid)
            .field("base_url", &self.inner.base_url)
            .field(
                "has_session",
                &self.inner.session.read().is_ok_and(|s| s.is_some()),
            )
            .finish_non_exhaustive()
    }
}

impl KuGouClient {
    /// Start building a client.
    #[must_use]
    pub fn builder() -> KuGouClientBuilder {
        KuGouClientBuilder::default()
    }

    /// Current platform.
    #[must_use]
    pub fn platform_id(&self) -> Platform {
        self.inner.platform
    }

    /// Gateway appid in use.
    #[must_use]
    pub fn appid(&self) -> u64 {
        self.inner.appid
    }

    /// Gateway clientver in use.
    #[must_use]
    pub fn clientver(&self) -> u64 {
        self.inner.clientver
    }

    /// Base URL (gateway) in use.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// The device info (read-only).
    #[must_use]
    pub fn device_info(&self) -> &Device {
        &self.inner.device
    }

    /// Current session credentials, when registered / logged in.
    #[must_use]
    pub fn session(&self) -> Option<SessionSnapshot> {
        self.inner.session.read().expect("session lock").clone()
    }

    /// Manually set session credentials (restore a saved login state).
    pub fn set_session(&self, session: SessionSnapshot) {
        tracing::info!(
            dfid = %session.dfid,
            userid = session.userid,
            "session restored"
        );
        *self.inner.session.write().expect("session lock") = Some(session);
    }

    /// The underlying HTTP layer (transport / breaker introspection).
    #[must_use]
    pub fn http_layer(&self) -> &HttpLayer {
        &self.inner.http
    }

    // ─────────────────────────────────────────────────────────
    // Typed high-frequency endpoints
    // ─────────────────────────────────────────────────────────

    /// Register the device; caches dfid / mid / token / userid automatically.
    ///
    /// The protocol crypto and public keys are built in; replace via
    /// [`KuGouClientBuilder::crypto_provider`] if needed.
    pub async fn register_device(
        &self,
        params: crate::api::RegisterDeviceParams,
    ) -> Result<crate::api::RegisterData> {
        crate::api::AccountApi::new(self)
            .register_device(params)
            .await
    }

    // ─────────────────────────────────────────────────────────
    // Catalog dispatch
    // ─────────────────────────────────────────────────────────

    /// Call any catalog endpoint (271 ids) with dynamic JSON params.
    ///
    /// Params use each endpoint's native naming (same as the reference
    /// implementation); identity params are attached automatically. Returns the
    /// unwrapped `data`.
    ///
    /// ```no_run
    /// # async fn demo(client: &kugouapi::KuGouClient) -> Result<(), kugouapi::Error> {
    /// let rank = client.call("rank.list", serde_json::json!({})).await?;
    /// let hot = client.call("search.hot", serde_json::json!({})).await?;
    /// # let _ = (rank, hot);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call(&self, id: &str, params: Value) -> Result<Value> {
        let endpoint = crate::catalog::REGISTRY
            .get(id)
            .ok_or_else(|| Error::UnknownEndpoint(id.to_owned()))?;
        let ctx = crate::catalog::ctx::call_ctx(self);
        (endpoint.invoke)(&ctx, &params).await
    }

    /// Call a catalog endpoint with typed params and a typed response
    /// (internal workhorse for the domain APIs).
    pub(crate) async fn call_typed<P, R>(&self, id: &str, params: P) -> Result<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let value = serde_json::to_value(params)?;
        let out = self.call(id, value).await?;
        serde_json::from_value(out).map_err(|e| Error::InvalidResponse {
            context: format!("decode response of `{id}`"),
            body: Some(e.to_string()),
        })
    }

    /// Call a catalog endpoint with typed params and a dynamic response.
    pub(crate) async fn call_value<P>(&self, id: &str, params: P) -> Result<Value>
    where
        P: serde::Serialize,
    {
        let value = serde_json::to_value(params)?;
        self.call(id, value).await
    }

    // ─────────────────────────────────────────────────────────
    // Generic escape hatch
    // ─────────────────────────────────────────────────────────

    /// Call any endpoint (auto identity params + signature, each step
    /// individually switchable).
    ///
    /// ```no_run
    /// # async fn demo(client: &kugouapi::KuGouClient) -> Result<(), kugouapi::Error> {
    /// let data = client
    ///     .request(
    ///         "/v1/search/lyric",
    ///         kugouapi::RequestOptions::new()
    ///             .query_one("keyword", "晴天")
    ///             .router("complexsearch.kugou.com"),
    ///     )
    ///     .await?;
    /// # let _ = data;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request(&self, path: &str, options: RequestOptions) -> Result<Value> {
        let method = options.method.unwrap_or(Method::Get);
        self.gateway_request(
            path,
            method,
            options.query,
            None,
            options.body,
            options.router,
            options.headers,
            options.base_url,
            options.sign.unwrap_or_default(),
            options.clear_params,
        )
        .await
    }

    /// Shared implementation of the gateway request pipeline for
    /// [`Self::request`] and [`crate::catalog::ctx::CallCtx::request`].
    pub(crate) async fn gateway_request(
        &self,
        path: &str,
        method: Method,
        query: Option<QueryRecord>,
        json: Option<Value>,
        text: Option<String>,
        router: Option<String>,
        extra_headers: Vec<(String, String)>,
        base_url: Option<String>,
        sign_control: crate::catalog::ctx::SignControl,
        clear_params: bool,
    ) -> Result<Value> {
        let mut merged = if clear_params {
            query.clone().unwrap_or_default()
        } else {
            let mut q = self.base_query();
            if let Some(endpoint_query) = &query {
                q.merge(endpoint_query);
            }
            q
        };

        let body: Option<String> = match (text, json) {
            (Some(text), _) => Some(text),
            (None, Some(json)) => Some(serde_json::to_string(&json)?),
            (None, None) => None,
        };

        let sign_kind = match sign_control {
            crate::catalog::ctx::SignControl::None => None,
            crate::catalog::ctx::SignControl::Android => Some(SignKind::Android),
            crate::catalog::ctx::SignControl::Web => Some(SignKind::Web),
        };
        if let Some(kind) = sign_kind
            && !merged.contains_key("signature")
        {
            let signature = sign::signature(
                &merged,
                SignOptions {
                    kind,
                    platform: self.inner.platform,
                    data: body.as_deref(),
                },
            );
            merged.set("signature", signature);
        }

        let base = base_url.as_deref().unwrap_or(&self.inner.base_url);
        let mut headers = self.common_headers();
        if let Some(router) = router {
            headers.push(("x-router".to_owned(), router));
        }
        headers.extend(extra_headers);
        if body.is_some()
            && !headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            headers.push(("Content-Type".to_owned(), "application/json".to_owned()));
        }

        let request = HttpRequest {
            method,
            url: format!("{base}{path}"),
            query: merged,
            headers,
            body: body.map(crate::http::Body::Text),
            timeout: None,
            retries: None,
            redirect: crate::http::Redirect::Follow,
        };
        let result = self.inner.http.request(request).await?;

        if !result.ok() {
            return Err(Error::Http {
                status: result.status,
                context: format!("{} {}", method.as_str(), path),
                payload: result.json(),
            });
        }
        let parsed = result.json().ok_or_else(|| Error::InvalidResponse {
            context: format!("{} {}", method.as_str(), path),
            body: Some(result.text()),
        })?;
        crate::catalog::def::unwrap_envelope(parsed)
    }

    // ─────────────────────────────────────────────────────────
    // Internals
    // ─────────────────────────────────────────────────────────

    /// Identity query params (appid / clientver / dfid / mid / uuid /
    /// clienttime, plus token / userid when logged in).
    pub(crate) fn base_query(&self) -> QueryRecord {
        let session = self.session();
        let dfid = session
            .as_ref()
            .map_or_else(|| "-".to_owned(), |s| s.dfid.clone());
        let mut q = QueryRecord::with_capacity(8);
        q.set("appid", self.inner.appid);
        q.set("clientver", self.inner.clientver);
        q.set("dfid", dfid.clone());
        q.set(
            "mid",
            session
                .as_ref()
                .map_or_else(|| derive_mid(&dfid), |s| s.mid.clone()),
        );
        q.set(
            "uuid",
            session
                .as_ref()
                .map_or_else(|| "-".to_owned(), |s| s.uuid.clone()),
        );
        q.set("clienttime", crate::catalog::ctx::now_secs_public());
        if let Some(session) = &session {
            if !session.token.is_empty() {
                q.set("token", session.token.clone());
            }
            if session.userid != 0 {
                q.set("userid", session.userid);
            }
        }
        q
    }

    /// Client identification headers (kg-* / dfid / mid / clienttime, plus the
    /// forwarded IP when configured).
    pub(crate) fn common_headers(&self) -> Vec<(String, String)> {
        let session = self.session();
        let dfid = session
            .as_ref()
            .map_or_else(|| "-".to_owned(), |s| s.dfid.clone());
        let mut headers: Vec<(String, String)> = config::CLIENT_HEADERS
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        headers.push(("dfid".to_owned(), dfid.clone()));
        headers.push((
            "mid".to_owned(),
            session
                .as_ref()
                .map_or_else(|| derive_mid(&dfid), |s| s.mid.clone()),
        ));
        headers.push((
            "clienttime".to_owned(),
            crate::catalog::ctx::now_secs_public().to_string(),
        ));
        if let Some(ip) = &self.inner.ip {
            headers.push(("X-Real-IP".to_owned(), ip.clone()));
            headers.push(("X-Forwarded-For".to_owned(), ip.clone()));
        }
        headers
    }

    /// Register-device pipeline (used by the `register.dev` handler).
    pub(crate) async fn register_device_impl(
        &self,
        params: &crate::api::RegisterDeviceParams,
    ) -> Result<Value> {
        let userid = params.userid.unwrap_or(0);
        let token = params.token.clone().unwrap_or_default();

        // payload = device fields overridden by non-empty params
        let mut payload = serde_json::to_value(&self.inner.device)?;
        if let Some(obj) = payload.as_object_mut() {
            let params_value = serde_json::to_value(params)?;
            if let Some(params_obj) = params_value.as_object() {
                for (key, value) in params_obj {
                    if matches!(value, Value::Null) {
                        continue;
                    }
                    if matches!(value, Value::String(s) if s.is_empty()) {
                        continue;
                    }
                    obj.insert(key.clone(), value.clone());
                }
            }
        }

        let encrypted = self
            .inner
            .crypto
            .encrypt_request(&CipherInput::Json(payload));
        let p = self
            .inner
            .crypto
            .encrypt_key(&encrypted.key, userid, &token)?;

        let clienttime = crate::catalog::ctx::now_secs_public();
        let mid = self
            .session()
            .map_or_else(|| config::DEFAULT_MID.to_owned(), |s| s.mid);
        let uuid = if self.inner.device.uuid.is_empty() {
            "-".to_owned()
        } else {
            self.inner.device.uuid.clone()
        };

        let mut query = QueryRecord::with_capacity(10);
        query.set("dfid", "-");
        query.set("mid", mid.clone());
        query.set("uuid", uuid);
        query.set("appid", self.inner.appid);
        query.set("clientver", self.inner.clientver);
        query.set("clienttime", clienttime);
        query.set("part", 1);
        query.set("platid", 1);
        query.set("p", p);
        query.set(
            "signature",
            sign::signature(
                &query,
                SignOptions {
                    kind: SignKind::Android,
                    platform: self.inner.platform,
                    ..SignOptions::default()
                },
            ),
        );

        let mut headers: Vec<(String, String)> = Vec::with_capacity(6);
        headers.push(("User-Agent".to_owned(), DEFAULT_USER_AGENT.to_owned()));
        headers.push(("dfid".to_owned(), "-".to_owned()));
        headers.push(("mid".to_owned(), mid));
        headers.push(("clienttime".to_owned(), clienttime.to_string()));
        if let Some(ip) = &self.inner.ip {
            headers.push(("X-Real-IP".to_owned(), ip.clone()));
            headers.push(("X-Forwarded-For".to_owned(), ip.clone()));
        }

        let request = HttpRequest {
            method: Method::Post,
            url: format!("{}/risk/v3/r_register_dev", config::REGISTER_BASE_URL),
            query,
            headers,
            body: Some(crate::http::Body::Text(encrypted.body.clone())),
            timeout: None,
            retries: None,
            redirect: crate::http::Redirect::Follow,
        };
        let result = self.inner.http.request(request).await?;
        if !result.ok() {
            return Err(Error::Http {
                status: result.status,
                context: "POST /risk/v3/r_register_dev".to_owned(),
                payload: result.json().or_else(|| Some(Value::String(result.text()))),
            });
        }

        // Successful responses carry the base64 of the session-cipher
        // ciphertext; error responses are plain JSON envelopes — surface
        // those before attempting the decryption.
        if let Some(parsed) = result.json()
            && (parsed.get("error_code").is_some() || parsed.get("errcode").is_some())
        {
            let data = crate::catalog::def::unwrap_envelope(parsed)?;
            self.apply_session(&data, &data);
            return Ok(data);
        }
        use base64::Engine as _;
        let payload_b64 = base64::engine::general_purpose::STANDARD.encode(&result.bytes);
        let decrypted = self
            .inner
            .crypto
            .decrypt_response(&payload_b64, &encrypted.key)?;
        let data = crate::catalog::def::unwrap_envelope(decrypted.into_value())?;
        self.apply_session(&data, &data);
        Ok(data)
    }

    /// Extract credentials from a register/login response and merge them into
    /// the session.
    pub(crate) fn apply_session(&self, data: &Value, raw: &Value) {
        self.apply_session_inner(data, raw);
    }

    /// Login-path variant used by [`crate::catalog::ctx::CallCtx::apply_login`].
    pub(crate) fn apply_session_from_login(&self, data: &Value) {
        self.apply_session_inner(data, data);
    }

    fn apply_session_inner(&self, data: &Value, raw: &Value) {
        let source = data.as_object().cloned().unwrap_or_default();
        let fallback = raw.as_object().cloned().unwrap_or_default();
        let pick = |key: &str| source.get(key).or_else(|| fallback.get(key));

        let has_credential = ["dfid", "token", "mid", "userid", "vip_type", "vip_token"]
            .iter()
            .any(|key| pick(key).is_some());
        if !has_credential {
            tracing::warn!("response carries no credential fields; session unchanged");
            return;
        }

        let previous = self.session();
        let dfid = pick("dfid")
            .and_then(Value::as_str)
            .map(std::borrow::ToOwned::to_owned)
            .or_else(|| previous.as_ref().map(|s| s.dfid.clone()))
            .unwrap_or_else(|| "-".to_owned());
        let new_session = SessionSnapshot {
            mid: pick("mid")
                .and_then(Value::as_str)
                .filter(|m| !m.is_empty())
                .map(std::borrow::ToOwned::to_owned)
                .or_else(|| previous.as_ref().map(|s| s.mid.clone()))
                .unwrap_or_else(|| derive_mid(&dfid)),
            uuid: if self.inner.device.uuid.is_empty() {
                "-".to_owned()
            } else {
                self.inner.device.uuid.clone()
            },
            token: pick("token")
                .and_then(Value::as_str)
                .map(std::borrow::ToOwned::to_owned)
                .or_else(|| previous.as_ref().map(|s| s.token.clone()))
                .unwrap_or_default(),
            userid: pick("userid")
                .and_then(Value::as_u64)
                .or_else(|| previous.as_ref().map(|s| s.userid))
                .unwrap_or(0),
            vip_type: pick("vip_type")
                .and_then(Value::as_i64)
                .or_else(|| previous.as_ref().and_then(|s| s.vip_type)),
            vip_token: pick("vip_token")
                .and_then(Value::as_str)
                .map(std::borrow::ToOwned::to_owned)
                .or_else(|| previous.as_ref().and_then(|s| s.vip_token.clone())),
            dfid,
            registered_at: Some(crate::catalog::ctx::now_secs_public()),
        };
        tracing::info!(dfid = %new_session.dfid, "session updated");
        *self.inner.session.write().expect("session lock") = Some(new_session);
    }
}

impl RequestOptions {
    /// Set one query parameter (builder style).
    #[must_use]
    pub fn query_one(
        mut self,
        key: impl Into<String>,
        value: impl Into<crate::query::QueryValue>,
    ) -> Self {
        self.query
            .get_or_insert_with(QueryRecord::new)
            .set(key, value);
        self
    }

    /// Set the `x-router` header (builder style).
    #[must_use]
    pub fn router(mut self, router: impl Into<String>) -> Self {
        self.router = Some(router.into());
        self
    }

    /// Disable the signature (builder style).
    #[must_use]
    pub fn sign_none(mut self) -> Self {
        self.sign = Some(crate::catalog::ctx::SignControl::None);
        self
    }
}

/// Builder for [`KuGouClient`].
pub struct KuGouClientBuilder {
    platform: Platform,
    base_url: Option<String>,
    device: Option<Device>,
    session: Option<SessionSnapshot>,
    ip: Option<String>,
    appid: Option<u64>,
    clientver: Option<u64>,
    timeout: Duration,
    retries: u32,
    retry_policy: Option<crate::http::RetryPolicy>,
    breaker: Option<Option<crate::http::CircuitBreakerOptions>>,
    rate_limit: Option<crate::http::RateLimitOptions>,
    crypto_provider: Option<Box<dyn CryptoProvider>>,
    transport: Option<Arc<dyn Transport>>,
}

impl std::fmt::Debug for KuGouClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KuGouClientBuilder")
            .field("platform", &self.platform)
            .field("base_url", &self.base_url)
            .field("appid", &self.appid)
            .field("clientver", &self.clientver)
            .field("timeout", &self.timeout)
            .field("retries", &self.retries)
            .field("has_custom_transport", &self.transport.is_some())
            .field("has_custom_crypto", &self.crypto_provider.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for KuGouClientBuilder {
    fn default() -> Self {
        Self {
            platform: Platform::Standard,
            base_url: None,
            device: None,
            session: None,
            ip: None,
            appid: None,
            clientver: None,
            timeout: Duration::from_secs(10),
            retries: 1,
            retry_policy: None,
            breaker: None,
            rate_limit: None,
            crypto_provider: None,
            transport: None,
        }
    }
}

impl KuGouClientBuilder {
    /// Select the platform preset (standard / lite).
    #[must_use]
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// Override the gateway base URL.
    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the device info (defaults to the bundled sample; prefer
    /// [`crate::device::random_device`]).
    #[must_use]
    pub fn device(mut self, device: Device) -> Self {
        self.device = Some(device);
        self
    }

    /// Restore a saved session.
    #[must_use]
    pub fn session(mut self, session: SessionSnapshot) -> Self {
        self.session = Some(session);
        self
    }

    /// Forward the client's real IP (`X-Real-IP` / `X-Forwarded-For`).
    #[must_use]
    pub fn ip(mut self, ip: impl Into<String>) -> Self {
        self.ip = Some(ip.into());
        self
    }

    /// Override the platform appid.
    #[must_use]
    pub fn appid(mut self, appid: u64) -> Self {
        self.appid = Some(appid);
        self
    }

    /// Override the platform clientver.
    #[must_use]
    pub fn clientver(mut self, clientver: u64) -> Self {
        self.clientver = Some(clientver);
        self
    }

    /// Per-attempt timeout (default 10 s).
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Idempotent GET retry count (default 1).
    #[must_use]
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Replace the retry policy wholesale.
    #[must_use]
    pub fn retry_policy(mut self, policy: crate::http::RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Configure the circuit breaker (`None` disables it; it is on by
    /// default).
    #[must_use]
    pub fn circuit_breaker(mut self, options: Option<crate::http::CircuitBreakerOptions>) -> Self {
        self.breaker = Some(options);
        self
    }

    /// Enable the token-bucket rate limiter.
    #[must_use]
    pub fn rate_limit(mut self, options: crate::http::RateLimitOptions) -> Self {
        self.rate_limit = Some(options);
        self
    }

    /// Replace the crypto provider wholesale (register-device pipeline).
    #[must_use]
    pub fn crypto_provider(mut self, provider: Box<dyn CryptoProvider>) -> Self {
        self.crypto_provider = Some(provider);
        self
    }

    /// Inject a custom HTTP transport (mocks, proxies, custom stacks).
    #[must_use]
    pub fn transport(mut self, transport: Arc<dyn Transport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Build the client.
    ///
    /// # Errors
    ///
    /// Returns an error when the default reqwest transport fails to
    /// initialize.
    pub fn build(self) -> Result<KuGouClient> {
        let platform_config = self.platform.config();
        let appid = self.appid.unwrap_or(platform_config.appid);
        let clientver = self.clientver.unwrap_or(platform_config.clientver);

        let transport = match self.transport {
            Some(t) => t,
            None => Arc::new(default_transport()?),
        };
        let http = HttpLayer::new(
            transport,
            HttpLayerOptions {
                timeout: self.timeout,
                retries: self.retries,
                retry: self
                    .retry_policy
                    .unwrap_or_else(|| crate::http::RetryPolicy {
                        max_retries: self.retries,
                        ..crate::http::RetryPolicy::default()
                    }),
                breaker: self.breaker.unwrap_or_default(),
                rate_limit: self.rate_limit,
            },
        );

        let crypto: Box<dyn CryptoProvider> = self
            .crypto_provider
            .unwrap_or_else(|| Box::new(DefaultCryptoProvider::new(self.platform, None)));

        Ok(KuGouClient {
            inner: Arc::new(ClientInner {
                platform: self.platform,
                appid,
                clientver,
                base_url: self
                    .base_url
                    .unwrap_or_else(|| config::DEFAULT_BASE_URL.to_owned()),
                ip: self.ip,
                device: self.device.unwrap_or_else(|| DEFAULT_DEVICE.clone()),
                crypto,
                http,
                session: RwLock::new(self.session),
            }),
        })
    }
}

#[cfg(feature = "reqwest-transport")]
fn default_transport() -> Result<crate::http::ReqwestTransport> {
    crate::http::ReqwestTransport::new()
}

#[cfg(not(feature = "reqwest-transport"))]
fn default_transport() -> Result<NoTransport> {
    Err(Error::HttpClientInit(
        "the default reqwest transport is disabled; inject one via `.transport(...)`".to_owned(),
    ))
}

#[cfg(not(feature = "reqwest-transport"))]
#[derive(Debug, Clone)]
struct NoTransport;

#[cfg(not(feature = "reqwest-transport"))]
impl crate::http::Transport for NoTransport {
    fn execute(
        &self,
        _request: crate::http::HttpRequest,
    ) -> futures::future::BoxFuture<'static, Result<crate::http::HttpResponse>> {
        Box::pin(async {
            Err::<crate::http::HttpResponse, _>(Error::HttpClientInit(
                "no transport configured; inject one via `.transport(...)`".to_owned(),
            ))
        })
    }
}
