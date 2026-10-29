//! Endpoint catalog types and dispatch machinery.
//!
//! A catalog entry describes *how to call one `KuGou` endpoint*:
//!
//! - regular entries: static method / url / router / salt metadata plus a
//!   typed query/body projection ([`simple_endpoint`]),
//! - special entries: a fully custom async handler
//!   ([`handler_endpoint`]) — login family, encrypted-POST family, composite
//!   flows.
//!
//! Both kinds funnel through the same dispatch: parameters are strongly typed
//! per endpoint; the dynamic `client.call(id, json)` path deserializes into the
//! endpoint's parameter struct before dispatch.

use futures::future::BoxFuture;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::http::Method;
use crate::query::QueryRecord;
use crate::sign::SignKind;

use super::ctx::CallCtx;

/// Signature salt selection for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Salt {
    /// Android signature (default).
    #[default]
    Android,
    /// Web (h5) signature.
    Web,
    /// No signature.
    None,
}

/// Static metadata for a catalog endpoint.
#[derive(Debug, Clone)]
pub struct EndpointSpec {
    /// Catalog id (module name with underscores → dots; multi-operation modules
    /// may override).
    pub id: &'static str,
    /// Reference `KuGouMusicApi` module name (for coverage reconciliation).
    pub module: &'static str,
    /// HTTP method.
    pub method: Method,
    /// Request path (starts with `/`; goes through the gateway unless `origin`
    /// overrides protocol and host).
    pub path: &'static str,
    /// Origin override (default `https://gateway.kugou.com`).
    pub origin: Option<&'static str>,
    /// `x-router` header value.
    pub router: Option<&'static str>,
    /// Signature salt.
    pub salt: Salt,
    /// Inject the trackercdn `key` (computed from the `hash` param, then
    /// included in the signature).
    pub tracker_key: bool,
    /// Do not inject the default identity parameters.
    pub clear_params: bool,
    /// Use only the headers declared by the entry.
    pub clear_headers: bool,
    /// Requires a logged-in session (documentation-only marker, like the TS
    /// `auth` flag).
    pub auth: bool,
    /// Note (protocol family / known issues).
    pub note: Option<&'static str>,
}

/// Marker type for catalog endpoints that take no parameters.
///
/// Deserializes from an empty object () or ; unknown fields are
/// ignored so loose dynamic payloads pass through.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoParams {}

/// Build a spec with defaults for the rarely-used fields.
#[must_use]
pub const fn spec(
    id: &'static str,
    module: &'static str,
    method: Method,
    path: &'static str,
) -> EndpointSpec {
    EndpointSpec {
        id,
        module,
        method,
        path,
        origin: None,
        router: None,
        salt: Salt::Android,
        tracker_key: false,
        clear_params: false,
        clear_headers: false,
        auth: false,
        note: None,
    }
}

/// The request a regular endpoint's projection produced.
///
/// All fields are optional: an empty [`Built`] simply means "call `spec.path`
/// with the identity query and android signature".
#[derive(Debug, Default)]
pub struct Built {
    /// Relative path override (dynamic URLs / conditional branches).
    pub path: Option<String>,
    /// Absolute URL override (wins over path/origin).
    pub url: Option<String>,
    /// Origin override (e.g. a CDN host).
    pub origin: Option<String>,
    /// Endpoint query (merged over the identity query unless `clear_params`).
    pub query: QueryRecord,
    /// JSON body (serialized compactly).
    pub json: Option<Value>,
    /// Raw text body (wins over `json`; sent as-is).
    pub text: Option<String>,
    /// Extra headers (x-router / KG-THash / custom User-Agent …), appended
    /// after the spec router header so they can override it.
    pub headers: Vec<(String, String)>,
}

impl Built {
    /// An empty built request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a query parameter.
    #[must_use]
    pub fn query(
        mut self,
        key: impl Into<String>,
        value: impl Into<crate::query::QueryValue>,
    ) -> Self {
        self.query.set(key, value);
        self
    }

    /// Set the JSON body.
    #[must_use]
    pub fn json(mut self, body: Value) -> Self {
        self.json = Some(body);
        self
    }

    /// Set a raw text body.
    #[must_use]
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.text = Some(body.into());
        self
    }

    /// Append a header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Use an absolute URL.
    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Use a relative path.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// A registered endpoint: static spec plus the type-erased invoker.
pub struct RegisteredEndpoint {
    /// Static metadata.
    pub spec: &'static EndpointSpec,
    /// Type-erased dispatch: dynamic JSON params → typed params → handler.
    pub invoke: InvokeFn,
}

/// The type-erased invoker stored in the registry.
pub type InvokeFn = Box<
    dyn for<'a, 'c> Fn(&'a CallCtx<'c>, &'a Value) -> BoxFuture<'a, Result<Value>> + Send + Sync,
>;

/// Deserialize dynamic params into `P`.
fn parse_params<P: DeserializeOwned>(endpoint: &'static EndpointSpec, value: &Value) -> Result<P> {
    serde_json::from_value(value.clone()).map_err(|e| Error::InvalidParams {
        endpoint: endpoint.id.to_owned(),
        message: e.to_string(),
    })
}

/// Register an endpoint with a fully custom async handler (login family,
/// encrypted POST family, composite flows …).
pub fn handler_endpoint<P, H>(endpoint: &'static EndpointSpec, handler: H) -> RegisteredEndpoint
where
    P: DeserializeOwned + Send + Sync + 'static,
    H: for<'a, 'c> Fn(&'a CallCtx<'c>, &'a P) -> BoxFuture<'a, Result<Value>>
        + Send
        + Sync
        + 'static,
{
    let handler = std::sync::Arc::new(handler);
    RegisteredEndpoint {
        spec: endpoint,
        invoke: Box::new(move |ctx: &CallCtx<'_>, params: &Value| {
            let handler = std::sync::Arc::clone(&handler);
            Box::pin(async move {
                let p: P = parse_params(endpoint, params)?;
                handler(ctx, &p).await
            })
        }),
    }
}

/// Register a regular endpoint: static metadata plus a typed, synchronous
/// query/body projection. The generic gateway dispatch (identity params,
/// tracker key, signature, envelope unwrap) is applied around the projection.
pub fn simple_endpoint<P, F>(endpoint: &'static EndpointSpec, build: F) -> RegisteredEndpoint
where
    P: DeserializeOwned + Send + Sync + 'static,
    F: Fn(&P, &CallCtx<'_>) -> Built + Send + Sync + 'static,
{
    let build = std::sync::Arc::new(build);
    handler_endpoint(endpoint, move |ctx, p| {
        let request = build(p, ctx);
        Box::pin(async move { ctx.dispatch_regular(endpoint, request).await })
    })
}

/// The generic gateway dispatch shared by every regular endpoint — a faithful
/// port of the TS `KuGouClient.call` non-handler path.
pub(crate) async fn dispatch_regular(
    ctx: &CallCtx<'_>,
    endpoint: &'static EndpointSpec,
    built: Built,
) -> Result<Value> {
    let client = ctx.client;

    // 1. Query: identity params (unless clear_params) merged with the
    //    endpoint's projection.
    let mut query = if endpoint.clear_params {
        built.query.clone()
    } else {
        let mut q = client.base_query();
        q.merge(&built.query);
        q
    };

    // 2. trackercdn key injection (before signing, so the signature covers it).
    if endpoint.tracker_key {
        let hash = query.get_str("hash").filter(|h| !h.is_empty());
        if let Some(hash) = hash {
            let mid = query
                .get_str("mid")
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| ctx.mid.clone());
            let userid = query
                .get("userid")
                .and_then(numeric_value)
                .unwrap_or(ctx.userid);
            let appid = query
                .get("appid")
                .and_then(numeric_value)
                .unwrap_or(ctx.appid);
            let key = crate::sign::tracker_key(crate::sign::TrackerKeyInput {
                hash: &hash.to_lowercase(),
                mid: &mid,
                userid,
                appid,
                platform: ctx.platform,
            });
            query.set("key", key);
        }
    }

    // 3. Body: raw text wins over JSON.
    let body: Option<String> = match (built.text, built.json) {
        (Some(text), _) => Some(text),
        (None, Some(json)) => Some(serde_json::to_string(&json)?),
        (None, None) => None,
    };

    // 4. Headers: common (unless clear_headers) + router + endpoint extras.
    let mut headers: Vec<(String, String)> = if endpoint.clear_headers {
        Vec::new()
    } else {
        client.common_headers()
    };
    if let Some(router) = endpoint.router {
        headers.push(("x-router".to_owned(), router.to_owned()));
    }
    headers.extend(built.headers.clone());
    if body.is_some()
        && !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
    {
        headers.push(("Content-Type".to_owned(), "application/json".to_owned()));
    }

    // 5. Signature (android by default, web, or none).
    let sign_kind = match endpoint.salt {
        Salt::None => None,
        Salt::Android => Some(SignKind::Android),
        Salt::Web => Some(SignKind::Web),
    };
    if let Some(kind) = sign_kind
        && !query.contains_key("signature")
    {
        let signature = crate::sign::signature(
            &query,
            crate::sign::SignOptions {
                kind,
                platform: ctx.platform,
                data: body.as_deref(),
            },
        );
        query.set("signature", signature);
    }

    // 6. URL: absolute override > origin override + path override > spec.
    let origin = built
        .origin
        .as_deref()
        .or(endpoint.origin)
        .unwrap_or(client.base_url());
    let path = built.path.as_deref().unwrap_or(endpoint.path);
    let url = built
        .url
        .clone()
        .unwrap_or_else(|| format!("{origin}{path}"));

    let request = crate::http::HttpRequest {
        method: endpoint.method,
        url,
        query,
        headers,
        body: body.map(crate::http::Body::Text),
        timeout: None,
        retries: None,
        redirect: crate::http::Redirect::Follow,
    };
    let result = client.http_layer().request(request).await?;

    if !result.ok() {
        return Err(Error::Http {
            status: result.status,
            context: format!("{} {}", endpoint.method.as_str(), endpoint.path),
            payload: result.json(),
        });
    }
    let parsed = result.json().ok_or_else(|| Error::InvalidResponse {
        context: format!("{} {}", endpoint.method.as_str(), endpoint.path),
        body: Some(result.text()),
    })?;
    unwrap_envelope(parsed)
}

/// Numeric coercion for query values (`Number(v)`).
fn numeric_value(value: &crate::query::QueryValue) -> Option<u64> {
    match value {
        crate::query::QueryValue::UInt(u) => Some(*u),
        crate::query::QueryValue::Int(i) => u64::try_from(*i).ok(),
        crate::query::QueryValue::Float(f) => {
            if f.fract() == 0.0 && *f >= 0.0 {
                Some(*f as u64)
            } else {
                None
            }
        }
        crate::query::QueryValue::Str(s) => s.parse().ok(),
        _ => None,
    }
}

/// Unwrap the response envelope; non-envelope shapes pass through unchanged.
///
/// Two envelope families are supported:
///
/// - gateway family `{status, error_code, data}`
/// - tracker family `{status, errcode, errmsg}` (e.g. `/v5/url` failures)
pub fn unwrap_envelope(parsed: Value) -> Result<Value> {
    let Value::Object(record) = &parsed else {
        return Ok(parsed);
    };

    // tracker family: failures carry errcode / errmsg (errcode may be a
    // string). A status of 1 or 200 counts as success — several services
    // (lyrics search) answer with HTTP-style `{status: 200, errcode: 200,
    // errmsg: "OK"}` success envelopes.
    if let Some(errcode) = record.get("errcode") {
        let errcode_num = json_number(errcode);
        let status_ok = record
            .get("status")
            .is_some_and(|s| json_eq_u64(s, 1) || json_eq_u64(s, 200));
        if errcode_num != Some(0) && !status_ok {
            let errmsg = record.get("errmsg").and_then(Value::as_str).unwrap_or("");
            return Err(Error::Api {
                code: errcode_num.unwrap_or(0),
                message: if errmsg.is_empty() {
                    format!("errcode={}", display_json(errcode))
                } else {
                    format!("errcode={} {errmsg}", display_json(errcode))
                },
                payload: Some(parsed),
            });
        }
    }

    // gateway family
    if let (Some(status), Some(error_code)) = (record.get("status"), record.get("error_code")) {
        // strict `===` semantics: a string "0" error_code is NOT zero
        let ok =
            (json_eq_u64(status, 1) || json_eq_u64(status, 200)) && error_code.as_u64() == Some(0);
        if ok {
            return Ok(record.get("data").cloned().unwrap_or(parsed));
        }
        return Err(Error::Api {
            code: error_code.as_i64().unwrap_or(0),
            message: format!("error_code={}", display_json(error_code)),
            payload: Some(parsed),
        });
    }

    Ok(parsed)
}

fn json_eq_u64(value: &Value, expected: u64) -> bool {
    value.as_u64() == Some(expected)
}

/// `Number(v)`-style numeric coercion (strings allowed).
fn json_number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                i64::try_from(u).ok()
            } else if let Some(i) = n.as_i64() {
                Some(i)
            } else {
                let f = n.as_f64()?;
                if f.fract() == 0.0 {
                    Some(f as i64)
                } else {
                    None
                }
            }
        }
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn display_json(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gateway_envelope_ok() {
        let out = unwrap_envelope(json!({"status": 1, "error_code": 0, "data": {"a": 1}})).unwrap();
        assert_eq!(out, json!({"a": 1}));
    }

    #[test]
    fn gateway_envelope_200_ok() {
        let out = unwrap_envelope(json!({"status": 200, "error_code": 0, "data": [1, 2]})).unwrap();
        assert_eq!(out, json!([1, 2]));
    }

    #[test]
    fn gateway_envelope_without_data_returns_record() {
        let out = unwrap_envelope(json!({"status": 1, "error_code": 0})).unwrap();
        assert_eq!(out, json!({"status": 1, "error_code": 0}));
    }

    #[test]
    fn gateway_envelope_business_error() {
        let err = unwrap_envelope(json!({
            "status": 0, "error_code": 30001, "error_text": "签名错误", "data": null
        }))
        .unwrap_err();
        let Error::Api { code, payload, .. } = err else {
            panic!("expected Api error");
        };
        assert_eq!(code, 30001);
        assert_eq!(payload.unwrap()["error_text"], "签名错误");
    }

    #[test]
    fn tracker_envelope_string_errcode() {
        let err = unwrap_envelope(json!({
            "errcode": "20006", "status": 0, "errmsg": "err signature"
        }))
        .unwrap_err();
        let Error::Api { code, message, .. } = err else {
            panic!("expected Api error");
        };
        assert_eq!(code, 20006);
        assert!(message.contains("err signature"));
    }

    #[test]
    fn tracker_envelope_risk_control() {
        let err = unwrap_envelope(json!({
            "fail_process": ["pkg"], "errcode": 20028, "status": 2, "error": "本次请求需要验证"
        }))
        .unwrap_err();
        let Error::Api { code, payload, .. } = err else {
            panic!("expected Api error");
        };
        assert_eq!(code, 20028);
        assert_eq!(payload.unwrap()["fail_process"][0], "pkg");
    }

    #[test]
    fn non_envelope_passthrough() {
        assert_eq!(
            unwrap_envelope(json!({"plain": true})).unwrap(),
            json!({"plain": true})
        );
        assert_eq!(unwrap_envelope(json!([1])).unwrap(), json!([1]));
        assert_eq!(unwrap_envelope(json!(null)).unwrap(), json!(null));
    }

    #[test]
    fn string_error_code_is_an_error() {
        // strict `===`: error_code "0" (string) is treated as an error
        let err = unwrap_envelope(json!({"status": 1, "error_code": "0"})).unwrap_err();
        let Error::Api { code, .. } = err else {
            panic!("expected Api error");
        };
        assert_eq!(code, 0);
    }
}
