//! The call context handed to endpoint handlers.
//!
//! Identity fields and the session snapshot come from the client's current
//! state; the timestamp refreshes on every call. Crypto primitives and request
//! escape hatches mirror the TS `CallCtx` one-to-one.

use futures::future::BoxFuture;
use serde_json::Value;

use crate::client::KuGouClient;
use crate::config::{Platform, SRCAPPID};
use crate::crypto::{
    AesHexResult, CipherInput, CipherOutput, SessionCipherResult, aes_hex_decrypt, aes_hex_encrypt,
    rsa_encrypt_hex, rsa_encrypt_raw, session_cipher_decrypt, session_cipher_encrypt,
};
use crate::error::Result;
use crate::http::{Body, HttpRequest, HttpResponse, Method, Redirect};
use crate::query::QueryRecord;
use crate::sign;

/// Signature control for [`CallCtx::request`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SignControl {
    /// Android signature (default).
    #[default]
    Android,
    /// Web signature.
    Web,
    /// No signature.
    None,
}

/// Options for [`CallCtx::request`] (the gateway request escape hatch).
#[derive(Default)]
pub struct GatewayRequest {
    /// HTTP method (GET by default).
    pub method: Option<Method>,
    /// Query parameters (merged over the identity params unless
    /// `clear_params`).
    pub query: Option<QueryRecord>,
    /// JSON body (serialized compactly).
    pub json: Option<Value>,
    /// Raw text body (wins over `json`).
    pub text: Option<String>,
    /// `x-router` header.
    pub router: Option<String>,
    /// Extra headers.
    pub headers: Vec<(String, String)>,
    /// Base URL override.
    pub base_url: Option<String>,
    /// Signature control.
    pub sign: SignControl,
    /// Do not inject the default identity params.
    pub clear_params: bool,
}

/// Options for [`CallCtx::raw`] (no identity injection, no envelope unwrap).
pub struct RawRequest {
    /// HTTP method (GET by default).
    pub method: Option<Method>,
    /// Absolute URL.
    pub url: String,
    /// Query parameters.
    pub query: Option<QueryRecord>,
    /// Extra headers.
    pub headers: Vec<(String, String)>,
    /// Request body.
    pub body: Option<Body>,
    /// Per-attempt timeout.
    pub timeout: Option<std::time::Duration>,
    /// Retry override.
    pub retries: Option<u32>,
    /// Redirect policy (QR login flows need `Manual`).
    pub redirect: Redirect,
}

impl RawRequest {
    /// Build a raw request for a URL.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            method: None,
            url: url.into(),
            query: None,
            headers: Vec::new(),
            body: None,
            timeout: None,
            retries: None,
            redirect: Redirect::Follow,
        }
    }
}

/// Shared call context for endpoint handlers.
pub struct CallCtx<'c> {
    /// Current UNIX timestamp in seconds.
    pub clienttime: u64,
    /// Platform (standard / lite).
    pub platform: Platform,
    /// Gateway appid.
    pub appid: u64,
    /// Gateway clientver.
    pub clientver: u64,
    /// Third-party login srcappid.
    pub srcappid: u64,
    /// Session mid (defaults to `derive_mid(dfid)` before registration).
    pub mid: String,
    /// Session dfid (`"-"` before registration).
    pub dfid: String,
    /// Device uuid.
    pub uuid: String,
    /// Session token (empty before login).
    pub token: String,
    /// Session userid (0 before login).
    pub userid: u64,
    /// VIP type (0 when not logged in).
    pub vip_type: i64,
    /// VIP token (empty when not logged in).
    pub vip_token: String,
    /// The owning client (session export & device access).
    pub client: &'c KuGouClient,
}

impl CallCtx<'_> {
    /// `signParamsKey(data)`: `md5(appid + salt + clientver + data)`.
    /// Explicit appid / clientver overrides are supported.
    #[must_use]
    pub fn sign_params_key(&self, data: impl std::fmt::Display) -> String {
        self.sign_params_key_with(data, None, None)
    }

    /// `signParamsKey` with explicit appid / clientver.
    #[must_use]
    pub fn sign_params_key_with(
        &self,
        data: impl std::fmt::Display,
        appid: Option<u64>,
        clientver: Option<u64>,
    ) -> String {
        sign::sign_params_key(
            data,
            sign::ParamsKeyOptions {
                platform: self.platform,
                appid: appid.or(Some(self.appid)),
                clientver: clientver.or(Some(self.clientver)),
            },
        )
    }

    /// trackercdn key: `md5(hash + salt + appid + mid + userid)`.
    #[must_use]
    pub fn tracker_key(&self, hash: &str) -> String {
        sign::tracker_key(sign::TrackerKeyInput {
            hash,
            mid: &self.mid,
            userid: self.userid,
            appid: self.appid,
            platform: self.platform,
        })
    }

    /// `cryptoMd5`: JSON payloads are serialized first.
    #[must_use]
    pub fn md5(&self, data: &CipherInput) -> String {
        crate::crypto::md5_of(data)
    }

    /// Padding-less RSA (login family `pk` param), hex.
    #[must_use]
    pub fn rsa_encrypt_raw(&self, data: &CipherInput) -> String {
        rsa_encrypt_raw(data, None, self.platform).unwrap_or_default()
    }

    /// RSA PKCS#1 v1.5 (encrypted-POST family `p` param), lowercase hex.
    #[must_use]
    pub fn rsa_encrypt_hex(&self, data: &CipherInput) -> String {
        rsa_encrypt_hex(data, self.platform)
    }

    /// sessionCipher encryption (encrypted-POST family body).
    #[must_use]
    pub fn session_cipher(&self, data: &CipherInput) -> SessionCipherResult {
        session_cipher_encrypt(data)
    }

    /// sessionCipher decryption (base64 ciphertext).
    pub fn session_cipher_decrypt(&self, payload: &str, key: &str) -> Result<CipherOutput> {
        session_cipher_decrypt(payload, key)
    }

    /// `cryptoAes` encryption (login family `params` field).
    #[must_use]
    pub fn aes_hex(&self, data: &CipherInput, key: Option<&str>, iv: Option<&str>) -> AesHexResult {
        aes_hex_encrypt(data, key, iv)
    }

    /// `cryptoAes` decryption (hex ciphertext).
    pub fn aes_hex_decrypt(&self, data: &str, key: &str, iv: Option<&str>) -> Result<CipherOutput> {
        aes_hex_decrypt(data, key, iv)
    }

    /// Issue a regular gateway request: identity params, signature and
    /// envelope unwrap included.
    ///
    /// Returns the unwrapped `data`.
    pub fn request(&self, path: &str, options: GatewayRequest) -> BoxFuture<'_, Result<Value>> {
        let path = path.to_owned();
        Box::pin(async move {
            self.client
                .gateway_request(
                    &path,
                    options.method.unwrap_or(Method::Get),
                    options.query.clone(),
                    options.json.clone(),
                    options.text.clone(),
                    options.router.clone(),
                    options.headers.clone(),
                    options.base_url.clone(),
                    options.sign,
                    options.clear_params,
                )
                .await
        })
    }

    /// Call another catalog endpoint from inside a handler.
    pub fn call_inner(&self, id: &str, params: Value) -> BoxFuture<'_, Result<Value>> {
        let id = id.to_owned();
        Box::pin(async move { self.client.call(&id, params).await })
    }

    /// Issue a raw request: no identity injection, no envelope unwrap —
    /// for special protocols (image hosts, QR redirects …).
    pub fn raw(&self, options: RawRequest) -> BoxFuture<'_, Result<HttpResponse>> {
        let request = HttpRequest {
            method: options.method.unwrap_or(Method::Get),
            url: options.url,
            query: options.query.clone().unwrap_or_default(),
            headers: options.headers.clone(),
            body: options.body.clone(),
            timeout: options.timeout,
            retries: options.retries,
            redirect: options.redirect,
        };
        Box::pin(async move { self.client.http_layer().request(request).await })
    }

    /// Persist login results (token / userid / VIP info) into the client
    /// session.
    pub fn apply_login(&self, data: &Value) {
        self.client.apply_session_from_login(data);
    }

    /// Dispatch a regular endpoint (used by `simple_endpoint`).
    pub(crate) async fn dispatch_regular(
        &self,
        endpoint: &'static crate::catalog::def::EndpointSpec,
        built: crate::catalog::def::Built,
    ) -> Result<Value> {
        crate::catalog::def::dispatch_regular(self, endpoint, built).await
    }
}

/// Current UNIX time in seconds.
pub(crate) fn now_secs() -> u64 {
    now_secs_public()
}

/// Current UNIX time in seconds (public re-export for internal crate users).
pub fn now_secs_public() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// Build a context from a client.
#[must_use]
pub fn call_ctx(client: &KuGouClient) -> CallCtx<'_> {
    let identity = client.base_query();
    let session = client.session();
    let dfid = session
        .as_ref()
        .map_or_else(|| "-".to_owned(), |s| s.dfid.clone());
    let mid = identity.get_str("mid").unwrap_or_else(|| "-".to_owned());
    let uuid = identity.get_str("uuid").unwrap_or_else(|| "-".to_owned());
    let (token, userid) = session
        .as_ref()
        .map_or_else(|| (String::new(), 0), |s| (s.token.clone(), s.userid));

    CallCtx {
        clienttime: now_secs(),
        platform: client.platform_id(),
        appid: client.appid(),
        clientver: client.clientver(),
        srcappid: SRCAPPID,
        mid,
        dfid,
        uuid,
        token,
        userid,
        vip_type: session.as_ref().and_then(|s| s.vip_type).unwrap_or(0),
        vip_token: session
            .as_ref()
            .and_then(|s| s.vip_token.clone())
            .unwrap_or_default(),
        client,
    }
}
