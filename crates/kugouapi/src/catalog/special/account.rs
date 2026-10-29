//! Account / login / register / verification family.
//!
//! Covers password login, SMS login, token refresh, device management, QR
//! login (`KuGou` / QQ / `WeChat`), `register_dev` and the SSA verification flow.
//! Credentials (`token` / `userid` / VIP info) are written into the client
//! session automatically after successful logins.
//!
//! Ported from `special/account.ts` (with `login.device.kick` following the
//! original reference semantics: raw no-padding RSA over the zero-padded
//! token with the lite public key).

use serde_json::{Map, Value, json};

use crate::catalog::ctx::{GatewayRequest, RawRequest, SignControl};
use crate::catalog::def::{Built, EndpointSpec, Salt, handler_endpoint, simple_endpoint, spec};
use crate::config::Platform;
use crate::crypto::{CipherInput, SimulateInput, generate_simulate, random_string};
use crate::error::{Error, Result};
use crate::http::{Method, Redirect};
use crate::js::{self, FlexI64, FlexString};
use crate::query::QueryRecord;
use crate::sign::md5;

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

// login family lite crypto constants (reference implementation values)
const LITE_T2_KEY: &str = "fd14b35e3f81af3817a20ae7adae7020";
const LITE_T2_IV: &str = "17a20ae7adae7020";
const LITE_T1_KEY: &str = "5e4ef500e9597fe004bd09a46d8add98";
const LITE_T1_IV: &str = "04bd09a46d8add98";
const T3: &str = "MCwwLDAsMCwwLDAsMCwwLDA=";

const LOGIN_UA_MOBILE: &str = "Android16-1070-11440-130-0-LOGIN-wifi";
const QQ_UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

// ─────────────────────────────────────────────────────────────
// Parameters
// ─────────────────────────────────────────────────────────────

kugou_params! {
    /// Parameters for `login` (password login).
    pub LoginParams {
        required {
            /// Account username.
            username: String => "username",
        }
        optional {
            /// Account password.
            password: String => "password",
            /// Salt code variant.
            code: String => "code",
            /// Device guid override.
            guid: String => "guid",
            /// MAC override.
            mac: String => "mac",
            /// Device codename override.
            dev: String => "dev",
            /// UUID override.
            uuid: String => "uuid",
        }
    }
}

kugou_params! {
    /// Parameters for `login.cellphone` (SMS login).
    pub LoginCellphoneParams {
        required {
            /// Mobile number.
            mobile: String => "mobile",
            /// SMS verification code.
            code: String => "code",
        }
        optional {
            /// User id (multi-device login).
            userid: FlexI64 => "userid",
            /// Device guid override.
            guid: String => "guid",
            /// MAC override.
            mac: String => "mac",
            /// Device codename override.
            dev: String => "dev",
        }
    }
}

kugou_params! {
    /// Parameters for `login.token` (token refresh).
    pub LoginTokenParams {
        required {}
        optional {
            /// Token override (falls back to the session).
            token: String => "token",
            /// User id override.
            userid: FlexI64 => "userid",
            /// Device guid override.
            guid: String => "guid",
            /// MAC override.
            mac: String => "mac",
            /// Device codename override.
            dev: String => "dev",
        }
    }
}

kugou_params! {
    /// Parameters for `login.device`.
    pub LoginDeviceParams {
        required {}
        optional {
            /// Token override.
            token: String => "token",
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

kugou_params! {
    /// Parameters for `login.device.kick`.
    pub LoginDeviceKickParams {
        required {}
        optional {
            /// Token override (falls back to the session).
            token: String => "token",
            /// User id override.
            userid: FlexI64 => "userid",
            /// Mid override.
            mid: String => "mid",
            /// UUID override.
            uuid: String => "uuid",
            /// Dfid override.
            dfid: String => "dfid",
            /// Target mid.
            t_mid: FlexString => "t_mid",
            /// Target time.
            t: FlexString => "t",
            /// Target appid.
            t_appid: FlexString => "t_appid",
            /// Target clientver.
            t_clientver: FlexString => "t_clientver",
        }
    }
}

kugou_params! {
    /// Parameters for `login.qq` (QQ token login).
    pub LoginQqParams {
        required {
            /// QQ openid (from the QR flow).
            openid: String => "openid",
            /// QQ access token.
            access_token: String => "access_token",
        }
        optional {
            /// Device guid override.
            guid: String => "guid",
            /// MAC override.
            mac: String => "mac",
            /// Device codename override.
            dev: String => "dev",
        }
    }
}

kugou_params! {
    /// Parameters for `login.openplat` (`WeChat` OAuth login).
    pub LoginOpenplatParams {
        required {
            /// `WeChat` OAuth authorization code.
            code: String => "code",
        }
        optional {
            /// `WeChat` app id override.
            wx_appid: String => "wx_appid",
            /// `WeChat` secret override.
            wx_secret: String => "wx_secret",
            /// Device guid override.
            guid: String => "guid",
            /// MAC override.
            mac: String => "mac",
            /// Device codename override.
            dev: String => "dev",
        }
    }
}

kugou_params! {
    /// Parameters for `login.qr.key`.
    pub LoginQrKeyParams {
        required {}
        optional {
            /// `web` selects the web app id (1014) instead of 1001.
            login_type: String => "type",
        }
    }
}

kugou_params! {
    /// Parameters for `login.qr.create`.
    pub LoginQrCreateParams {
        required {
            /// QR key from `login.qr.key`.
            key: String => "key",
        }
        optional {
            /// Whether a `base64` field is expected (kept for parity).
            qrimg: FlexI64 => "qrimg",
        }
    }
}

kugou_params! {
    /// Parameters for `login.qr.check`.
    pub LoginQrCheckParams {
        required {
            /// QR key from `login.qr.key`.
            key: String => "key",
        }
        optional {
            /// Device codename override.
            dev: String => "dev",
            /// Device guid override.
            guid: String => "guid",
            /// UUID override.
            uuid: String => "uuid",
        }
    }
}

kugou_params! {
    /// Parameters for `login.qr.authorize`.
    pub LoginQrAuthorizeParams {
        required {
            /// QR code content.
            qrcode: String => "qrcode",
        }
        optional {
            /// App id override.
            appid: FlexI64 => "appid",
            /// Platform (2 = pad).
            plat: FlexI64 => "plat",
        }
    }
}

kugou_params! {
    /// Parameters for `login.qq.qr.check`.
    pub LoginQqQrCheckParams {
        required {
            /// `qrsig` from `login.qq.qr.create`.
            qrsig: String => "qrsig",
            /// `ptqrtoken` from `login.qq.qr.create`.
            ptqrtoken: String => "ptqrtoken",
            /// `pt_login_sig` from `login.qq.qr.create`.
            pt_login_sig: String => "pt_login_sig",
            /// `pt_openlogin_data` from `login.qq.qr.create`.
            pt_openlogin_data: String => "pt_openlogin_data",
            /// `xlogin_url` from `login.qq.qr.create`.
            xlogin_url: String => "xlogin_url",
            /// Cookie string from `login.qq.qr.create`.
            cookie: String => "cookie",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `login.wx.create` / `login.wx.check`.
    pub LoginWxParams {
        required {
            /// `WeChat` UUID (`login.wx.check`).
            uuid: String => "uuid",
        }
        optional {
            /// `WeChat` app id override (`login.wx.create`).
            wx_appid: String => "wx_appid",
            /// `WeChat` secret override (`login.wx.create`).
            wx_secret: String => "wx_secret",
        }
    }
}

kugou_params! {
    /// Parameters for `login.wx.create` (no uuid needed).
    pub LoginWxCreateParams {
        required {}
        optional {
            /// `WeChat` app id override.
            wx_appid: String => "wx_appid",
            /// `WeChat` secret override.
            wx_secret: String => "wx_secret",
        }
    }
}

/// Parameters for `sidedt` (auto-generated SSA payload) and
/// `verify.user.info`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VerifyParams {
    /// Event id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eventid: Option<Value>,
    /// Verification type (default 23).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_type: Option<FlexI64>,
    /// Verification code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verifycode: Option<String>,
    /// Platform id (default 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platid: Option<FlexI64>,
    /// User id override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userid: Option<FlexI64>,
    /// Mid override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid: Option<String>,
    /// Dfid override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dfid: Option<String>,
    /// Sid payload (verify.user.info only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    /// Edt payload (verify.user.info only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edt: Option<String>,
    /// Any additional fields are passed through.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

// ─────────────────────────────────────────────────────────────
// Specs
// ─────────────────────────────────────────────────────────────

const EP_LOGIN: EndpointSpec = EndpointSpec {
    router: Some("login.user.kugou.com"),
    ..spec("login", "login", Method::Post, "/v9/login_by_pwd")
};
const EP_LOGIN_CELLPHONE: EndpointSpec = EndpointSpec {
    origin: Some("https://loginserviceretry.kugou.com"),
    ..spec(
        "login.cellphone",
        "login_cellphone",
        Method::Post,
        "/v7/login_by_verifycode",
    )
};
const EP_LOGIN_TOKEN: EndpointSpec = EndpointSpec {
    origin: Some("http://login.user.kugou.com"),
    auth: true,
    ..spec(
        "login.token",
        "login_token",
        Method::Post,
        "/v5/login_by_token",
    )
};
const EP_LOGIN_DEVICE: EndpointSpec = EndpointSpec {
    origin: Some("https://userinfoservice.kugou.com"),
    auth: true,
    ..spec("login.device", "login_device", Method::Post, "/v2/get_dev")
};
const EP_LOGIN_DEVICE_KICK: EndpointSpec = spec(
    "login.device.kick",
    "login_device_kick",
    Method::Get,
    "/loginservice/v1/dev_logout",
);
const EP_LOGIN_QQ: EndpointSpec = EndpointSpec {
    router: Some("login.user.kugou.com"),
    ..spec(
        "login.qq",
        "login_qq",
        Method::Post,
        "/v6/login_by_openplat",
    )
};
const EP_LOGIN_OPENPLAT: EndpointSpec = EndpointSpec {
    router: Some("login.user.kugou.com"),
    ..spec(
        "login.openplat",
        "login_openplat",
        Method::Post,
        "/v6/login_by_openplat",
    )
};
const EP_LOGIN_QR_KEY: EndpointSpec = EndpointSpec {
    origin: Some("https://login-user.kugou.com"),
    salt: Salt::Web,
    ..spec("login.qr.key", "login_qr_key", Method::Get, "/v2/qrcode")
};
const EP_LOGIN_QR_CREATE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec("login.qr.create", "login_qr_create", Method::Get, "/")
};
const EP_LOGIN_QR_CHECK: EndpointSpec = EndpointSpec {
    origin: Some("https://login-user.kugou.com"),
    salt: Salt::Web,
    ..spec(
        "login.qr.check",
        "login_qr_check",
        Method::Get,
        "/v2/get_userinfo_qrcode",
    )
};
const EP_LOGIN_QR_AUTHORIZE: EndpointSpec = EndpointSpec {
    origin: Some("https://login-user.kugou.com"),
    salt: Salt::Web,
    auth: true,
    ..spec(
        "login.qr.authorize",
        "login_qr_authorize",
        Method::Post,
        "/v2/scan",
    )
};
const EP_LOGIN_QQ_QR_CREATE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec("login.qq.qr.create", "login_qq_qr_create", Method::Get, "/")
};
const EP_LOGIN_QQ_QR_CHECK: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec("login.qq.qr.check", "login_qq_qr_check", Method::Get, "/")
};
const EP_LOGIN_WX_CREATE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec("login.wx.create", "login_wx_create", Method::Get, "/")
};
const EP_LOGIN_WX_CHECK: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec("login.wx.check", "login_wx_check", Method::Get, "/")
};
const EP_REGISTER_DEV: EndpointSpec = EndpointSpec {
    origin: Some("https://userservice.kugou.com"),
    ..spec(
        "register.dev",
        "register_dev",
        Method::Post,
        "/risk/v3/r_register_dev",
    )
};
const EP_SIDEDT: EndpointSpec = EndpointSpec {
    origin: Some("https://verifyservice.kugou.com"),
    auth: true,
    ..spec("sidedt", "sidedt", Method::Post, "/v4/verify_user_info")
};
const EP_VERIFY_USER_INFO: EndpointSpec = EndpointSpec {
    origin: Some("https://verifyservice.kugou.com"),
    ..spec(
        "verify.user.info",
        "verify_user_info",
        Method::Post,
        "/v4/verify_user_info",
    )
};
const EP_USER_VERIFY: EndpointSpec = EndpointSpec {
    router: Some("trackercdngz.kugou.com"),
    ..spec("user.verify", "user_verify", Method::Get, "/v1/user_verify")
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        handler_endpoint(&EP_LOGIN, login_handler),
        handler_endpoint(&EP_LOGIN_CELLPHONE, login_cellphone_handler),
        handler_endpoint(&EP_LOGIN_TOKEN, login_token_handler),
        handler_endpoint(&EP_LOGIN_DEVICE, login_device_handler),
        handler_endpoint(&EP_LOGIN_DEVICE_KICK, login_device_kick_handler),
        handler_endpoint(&EP_LOGIN_QQ, login_qq_handler),
        handler_endpoint(&EP_LOGIN_OPENPLAT, login_openplat_handler),
        simple_endpoint(&EP_LOGIN_QR_KEY, |p: &LoginQrKeyParams, ctx| {
            let mut query = QueryRecord::new();
            query.set(
                "appid",
                if p.login_type.as_deref() == Some("web") {
                    1014
                } else {
                    1001
                },
            );
            query.set("type", 1);
            query.set("plat", 4);
            query.set(
                "qrcode_txt",
                format!(
                    "https://h5.kugou.com/apps/loginQRCode/html/index.html?appid={}&",
                    ctx.appid
                ),
            );
            query.set("srcappid", ctx.srcappid);
            Built {
                query,
                ..Built::default()
            }
        }),
        handler_endpoint(&EP_LOGIN_QR_CREATE, login_qr_create_handler),
        handler_endpoint(&EP_LOGIN_QR_CHECK, login_qr_check_handler),
        handler_endpoint(&EP_LOGIN_QR_AUTHORIZE, login_qr_authorize_handler),
        handler_endpoint(&EP_LOGIN_QQ_QR_CREATE, login_qq_qr_create_handler),
        handler_endpoint(&EP_LOGIN_QQ_QR_CHECK, login_qq_qr_check_handler),
        handler_endpoint(&EP_LOGIN_WX_CREATE, login_wx_create_handler),
        handler_endpoint(&EP_LOGIN_WX_CHECK, login_wx_check_handler),
        handler_endpoint(&EP_REGISTER_DEV, register_dev_handler),
        handler_endpoint(&EP_SIDEDT, sidedt_handler),
        handler_endpoint(&EP_VERIFY_USER_INFO, verify_user_info_handler),
        simple_endpoint(&EP_USER_VERIFY, |_: &crate::catalog::def::NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("module_id", 51);
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

// ─────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────

/// Device identity (guid / mac / dev) with parameter overrides.
#[allow(clippy::option_if_let_else)]
fn device_identity(
    guid: Option<&str>,
    mac: Option<&str>,
    dev: Option<&str>,
    uuid_override: Option<&str>,
    ctx: &CallCtx<'_>,
) -> (String, String, String) {
    let device = ctx.client.device_info();
    (
        guid.filter(|g| !g.is_empty())
            .or_else(|| uuid_override.filter(|u| !u.is_empty()))
            .map_or_else(
                || {
                    Some(ctx.uuid.clone())
                        .filter(|u| !u.is_empty() && u != "-")
                        .or_else(|| Some(device.uuid.clone()).filter(|u| !u.is_empty()))
                },
                |g| Some(g.to_owned()),
            )
            .unwrap_or_else(|| "-".to_owned()),
        mac.unwrap_or_default().to_owned(),
        dev.filter(|d| !d.is_empty())
            .map_or_else(|| device.device.clone(), str::to_owned),
    )
}

/// Login response settle: decrypt `secu_params`, merge and persist the
/// session.
fn settle_login(ctx: &CallCtx<'_>, data: Value, encrypt_key: &str) -> Result<Value> {
    let mut merged = data;
    if let Some(secu_params) = merged
        .get("secu_params")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        let token = ctx.aes_hex_decrypt(secu_params, encrypt_key, None)?;
        match token.into_value() {
            Value::Object(map) => {
                let obj = merged.as_object_mut().expect("merged is an object");
                for (k, v) in map {
                    obj.insert(k, v);
                }
            }
            Value::String(s) => {
                merged
                    .as_object_mut()
                    .expect("merged is an object")
                    .insert("token".to_owned(), Value::String(s));
            }
            _ => {}
        }
    }
    let has_credentials = merged.get("token").is_some_and(|t| !t.is_null())
        || merged.get("userid").is_some_and(|u| !u.is_null());
    if has_credentials {
        ctx.apply_login(&merged);
    }
    Ok(merged)
}

/// The `/v6/login_by_openplat` shared request (QQ / `WeChat`).
async fn login_by_openplat(
    ctx: &CallCtx<'_>,
    guid: &str,
    mac: &str,
    dev: &str,
    partnerid: i64,
    openid: &str,
    params: &str,
    pk: &str,
    third_appid: Option<&str>,
) -> Result<Value> {
    let date_now = crate::crypto::now_ms();
    let lite = ctx.platform == Platform::Lite;
    let t2 = ctx.aes_hex(
        &CipherInput::Text(format!(
            "{guid}|0f607264fc6318a92b9e13c65db7cd3c|{mac}|{dev}|{date_now}"
        )),
        Some(LITE_T2_KEY),
        Some(LITE_T2_IV),
    );
    let t1 = ctx.aes_hex(
        &CipherInput::Text(format!("|{date_now}")),
        Some(LITE_T1_KEY),
        Some(LITE_T1_IV),
    );

    let mut body = Map::new();
    body.insert("dev".to_owned(), json!(dev));
    let _ = &t2;
    body.insert("force_login".to_owned(), json!(1));
    body.insert("partnerid".to_owned(), json!(partnerid));
    body.insert("clienttime_ms".to_owned(), json!(date_now));
    body.insert("t1".to_owned(), if lite { json!(t1.str) } else { json!(0) });
    body.insert("t2".to_owned(), if lite { json!(t2.str) } else { json!(0) });
    body.insert("t3".to_owned(), json!(T3));
    if let Some(third) = third_appid {
        body.insert("third_appid".to_owned(), json!(third));
    }
    body.insert("openid".to_owned(), json!(openid));
    body.insert("params".to_owned(), json!(params));
    body.insert("pk".to_owned(), json!(pk));

    ctx.request(
        "/v6/login_by_openplat",
        GatewayRequest {
            method: Some(Method::Post),
            router: Some("login.user.kugou.com".to_owned()),
            json: Some(Value::Object(body)),
            ..GatewayRequest::default()
        },
    )
    .await
}

/// QQ qrsig → ptqrtoken (hash33).
fn hash33(s: &str) -> i64 {
    let mut e: i64 = 0;
    for ch in s.chars() {
        e = e.wrapping_add((e << 5).wrapping_add(i64::from(u32::from(ch))));
    }
    e & 2_147_483_647
}

fn sha1_hex(data: &str) -> String {
    use sha1::{Digest, Sha1};
    let digest = Sha1::digest(data.as_bytes());
    hex::encode(digest)
}

/// `decodeURIComponent` with a permissive fallback.
fn decode_uri_component(value: &str) -> String {
    percent_decode(value)
}

/// Minimal percent-decoding (`%XX` and `+` passthrough).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 < bytes.len() {
            let hex_pair = &input[i + 1..i + 3];
            if let Ok(byte) = u8::from_str_radix(hex_pair, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn urlencode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────

fn login_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let date_now = crate::crypto::now_ms();
        let lite = ctx.platform == Platform::Lite;
        let encrypt = ctx.aes_hex(
            &CipherInput::Json(json!({
                "pwd": p.password.clone().unwrap_or_default(),
                "code": p.code.clone().unwrap_or_default(),
                "clienttime_ms": date_now,
            })),
            None,
            None,
        );
        let (guid, mac, dev) = device_identity(
            p.guid.as_deref(),
            p.mac.as_deref(),
            p.dev.as_deref(),
            p.uuid.as_deref(),
            ctx,
        );
        let t2 = ctx.aes_hex(
            &CipherInput::Text(format!(
                "{guid}|0f607264fc6318a92b9e13c65db7cd3c|{mac}|{dev}|{date_now}"
            )),
            Some(LITE_T2_KEY),
            Some(LITE_T2_IV),
        );
        let t1 = ctx.aes_hex(
            &CipherInput::Text(format!("|{date_now}")),
            Some(LITE_T1_KEY),
            Some(LITE_T1_IV),
        );

        let body = json!({
            "plat": 1,
            "support_multi": 1,
            "clienttime_ms": date_now,
            "t1": if lite { json!(t1.str) } else { json!(0) },
            "t2": if lite { json!(t2.str) } else { json!(0) },
            "t3": T3,
            "dev": dev,
            "username": p.username,
            "params": encrypt.str,
            "pk": ctx
                .rsa_encrypt_raw(&CipherInput::Json(json!({
                    "clienttime_ms": date_now,
                    "key": encrypt.key,
                })))
                .to_uppercase(),
        });

        let data = ctx
            .request(
                "/v9/login_by_pwd",
                GatewayRequest {
                    method: Some(Method::Post),
                    router: Some("login.user.kugou.com".to_owned()),
                    json: Some(body),
                    ..GatewayRequest::default()
                },
            )
            .await?;
        settle_login(ctx, data, &encrypt.key)
    })
}

fn login_cellphone_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginCellphoneParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let date_now = crate::crypto::now_ms();
        let lite = ctx.platform == Platform::Lite;
        let encrypt = ctx.aes_hex(
            &CipherInput::Json(json!({
                "mobile": p.mobile,
                "code": p.code,
            })),
            None,
            None,
        );
        let masked_mobile = format!(
            "{}*****{}",
            js::js_substring(&p.mobile, 0, 2),
            js::js_substring(&p.mobile, 10, 11),
        );
        let (guid, mac, dev) = device_identity(
            p.guid.as_deref(),
            p.mac.as_deref(),
            p.dev.as_deref(),
            None,
            ctx,
        );
        let t2 = ctx.aes_hex(
            &CipherInput::Text(format!(
                "{guid}|0f607264fc6318a92b9e13c65db7cd3c|{mac}|{dev}|{date_now}"
            )),
            Some(LITE_T2_KEY),
            Some(LITE_T2_IV),
        );
        let t1 = ctx.aes_hex(
            &CipherInput::Text(format!("|{date_now}")),
            Some(LITE_T1_KEY),
            Some(LITE_T1_IV),
        );

        let mut body = Map::new();
        body.insert("plat".to_owned(), json!(1));
        body.insert("support_multi".to_owned(), json!(1));
        body.insert("t1".to_owned(), if lite { json!(t1.str) } else { json!(0) });
        body.insert("t2".to_owned(), if lite { json!(t2.str) } else { json!(0) });
        body.insert("clienttime_ms".to_owned(), json!(date_now));
        body.insert("mobile".to_owned(), json!(masked_mobile));
        body.insert("key".to_owned(), json!(ctx.sign_params_key(date_now)));
        body.insert(
            "pk".to_owned(),
            json!(
                ctx.rsa_encrypt_raw(&CipherInput::Json(json!({
                    "clienttime_ms": date_now,
                    "key": encrypt.key,
                })))
                .to_uppercase()
            ),
        );
        body.insert("params".to_owned(), json!(encrypt.str));
        if let Some(userid) = p.userid.as_ref().and_then(FlexI64::value) {
            body.insert("userid".to_owned(), json!(userid));
        }
        if lite {
            body.insert("dfid".to_owned(), json!(ctx.dfid));
            body.insert("dev".to_owned(), json!(dev));
            body.insert("gitversion".to_owned(), json!("5f0b7c4"));
        } else {
            body.insert("t3".to_owned(), json!(T3));
        }

        let data = ctx
            .request(
                "/v7/login_by_verifycode",
                GatewayRequest {
                    method: Some(Method::Post),
                    base_url: Some("https://loginserviceretry.kugou.com".to_owned()),
                    json: Some(Value::Object(body)),
                    headers: vec![
                        ("support-calm".to_owned(), "1".to_owned()),
                        ("User-Agent".to_owned(), LOGIN_UA_MOBILE.to_owned()),
                    ],
                    ..GatewayRequest::default()
                },
            )
            .await?;
        settle_login(ctx, data, &encrypt.key)
    })
}

fn login_token_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginTokenParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let date_now = crate::crypto::now_ms();
        let lite = ctx.platform == Platform::Lite;
        let (key, iv) = if lite {
            ("c24f74ca2820225badc01946dba4fdf7", "adc01946dba4fdf7")
        } else {
            ("90b8382a1bb4ccdcf063102053fd75b8", "f063102053fd75b8")
        };
        let token = p
            .token
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| ctx.token.clone());
        let userid = p
            .userid
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|u| *u != 0)
            .unwrap_or(ctx.userid as i64);

        let encrypt = ctx.aes_hex(
            &CipherInput::Json(json!({
                "clienttime": date_now / 1000,
                "token": token,
            })),
            Some(key),
            Some(iv),
        );
        let encrypt_params = ctx.aes_hex(&CipherInput::Json(json!({})), None, None);
        let (guid, mac, dev) = device_identity(
            p.guid.as_deref(),
            p.mac.as_deref(),
            p.dev.as_deref(),
            None,
            ctx,
        );
        let t2 = ctx.aes_hex(
            &CipherInput::Text(format!(
                "{guid}|0f607264fc6318a92b9e13c65db7cd3c|{mac}|{dev}|{date_now}"
            )),
            Some(LITE_T2_KEY),
            Some(LITE_T2_IV),
        );
        let t1 = ctx.aes_hex(
            &CipherInput::Text(format!("|{date_now}")),
            Some(LITE_T1_KEY),
            Some(LITE_T1_IV),
        );

        let mut body = Map::new();
        body.insert("dfid".to_owned(), json!(ctx.dfid));
        body.insert("p3".to_owned(), json!(encrypt.str));
        body.insert("plat".to_owned(), json!(1));
        body.insert("t1".to_owned(), if lite { json!(t1.str) } else { json!(0) });
        body.insert("t2".to_owned(), if lite { json!(t2.str) } else { json!(0) });
        body.insert("t3".to_owned(), json!(T3));
        body.insert(
            "pk".to_owned(),
            json!(ctx.rsa_encrypt_raw(&CipherInput::Json(json!({
                "clienttime_ms": date_now,
                "key": encrypt_params.key,
            })))),
        );
        body.insert("params".to_owned(), json!(encrypt_params.str));
        body.insert("userid".to_owned(), json!(userid.to_string()));
        body.insert("clienttime_ms".to_owned(), json!(date_now));
        if lite {
            body.insert("dev".to_owned(), json!(dev));
        }

        let data = ctx
            .request(
                "/v5/login_by_token",
                GatewayRequest {
                    method: Some(Method::Post),
                    base_url: Some("http://login.user.kugou.com".to_owned()),
                    json: Some(Value::Object(body)),
                    ..GatewayRequest::default()
                },
            )
            .await?;
        settle_login(ctx, data, &encrypt_params.key)
    })
}

fn login_device_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginDeviceParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let clienttime_ms = crate::crypto::now_ms();
        let encrypt = ctx.aes_hex(
            &CipherInput::Json(json!({
                "token": p
                    .token
                    .clone()
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| ctx.token.clone()),
            })),
            None,
            None,
        );
        let body = json!({
            "plat": 1,
            "userid": p
                .userid
                .as_ref()
                .and_then(FlexI64::value)
                .filter(|u| *u != 0)
                .unwrap_or(ctx.userid as i64),
            "clienttime_ms": clienttime_ms,
            "pk": ctx
                .rsa_encrypt_raw(&CipherInput::Json(json!({
                    "clienttime_ms": clienttime_ms,
                    "key": encrypt.key,
                })))
                .to_uppercase(),
            "params": encrypt.str,
        });
        ctx.request(
            "/v2/get_dev",
            GatewayRequest {
                method: Some(Method::Post),
                base_url: Some("https://userinfoservice.kugou.com".to_owned()),
                json: Some(body),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn login_device_kick_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginDeviceKickParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let raw_token = p
            .token
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| ctx.token.clone());
        let prefix = "moc.uoguk.59::";
        let input = format!("{prefix}{raw_token}").into_bytes();
        let mut padded = vec![0u8; 128];
        if input.len() > 128 {
            return Err(Error::Crypto(
                "dev_logout token exceeds the 128-byte RSA slot".to_owned(),
            ));
        }
        padded[..input.len()].copy_from_slice(&input);
        // Original reference semantics: raw no-padding RSA with the lite key.
        let encrypted = {
            use rsa::pkcs8::DecodePublicKey;
            let key = rsa::RsaPublicKey::from_public_key_pem(crate::crypto::LITE_RSA_PUBLIC_KEY)
                .map_err(|e| Error::Crypto(e.to_string()))?;
            hex::encode(crate::crypto::raw_modpow_public(&key, &padded)?).to_uppercase()
        };
        let token = format!("h5{encrypted}");

        let mut query = QueryRecord::new();
        query.set("appid", ctx.appid);
        query.set("clientver", ctx.clientver);
        query.set("clienttime", crate::crypto::now_ms());
        query.set(
            "mid",
            p.mid
                .clone()
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| ctx.mid.clone()),
        );
        query.set(
            "uuid",
            p.uuid
                .clone()
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| ctx.uuid.clone()),
        );
        query.set(
            "dfid",
            p.dfid
                .clone()
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| ctx.dfid.clone()),
        );
        query.set("plat", 1);
        query.set(
            "userid",
            p.userid
                .as_ref()
                .and_then(FlexI64::value)
                .filter(|u| *u != 0)
                .unwrap_or(ctx.userid as i64),
        );
        query.set("token", token);
        query.set("srcappid", ctx.srcappid);
        if let Some(t_mid) = &p.t_mid {
            query.set_opt("t_mid", t_mid.value().map(std::string::ToString::to_string));
        }
        if let Some(t) = &p.t {
            query.set_opt("t", t.value().map(std::string::ToString::to_string));
        }
        if let Some(t_appid) = &p.t_appid {
            query.set_opt(
                "t_appid",
                t_appid.value().map(std::string::ToString::to_string),
            );
        }
        if let Some(t_clientver) = &p.t_clientver {
            query.set_opt(
                "t_clientver",
                t_clientver.value().map(std::string::ToString::to_string),
            );
        }

        ctx.request(
            "/loginservice/v1/dev_logout",
            GatewayRequest {
                method: Some(Method::Get),
                sign: SignControl::Web,
                query: Some(query),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn login_qq_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginQqParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        if p.openid.is_empty() || p.access_token.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "missing openid or access_token".to_owned(),
                payload: None,
            });
        }
        let date_now = crate::crypto::now_ms();
        let encrypt = ctx.aes_hex(
            &CipherInput::Json(json!({ "access_token": p.access_token })),
            None,
            None,
        );
        let third_appid = if ctx.platform == Platform::Lite {
            "101706348"
        } else {
            "205141"
        };
        let (_, mac, dev) = device_identity(
            p.guid.as_deref(),
            p.mac.as_deref(),
            p.dev.as_deref(),
            None,
            ctx,
        );
        let data = login_by_openplat(
            ctx,
            &p.openid,
            &mac,
            &dev,
            1,
            &p.openid,
            &encrypt.str,
            &ctx.rsa_encrypt_raw(&CipherInput::Json(json!({
                "clienttime_ms": date_now,
                "key": encrypt.key,
            })))
            .to_uppercase(),
            Some(third_appid),
        )
        .await?;
        settle_login(ctx, data, &encrypt.key)
    })
}

fn login_openplat_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginOpenplatParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        if p.code.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "missing `WeChat` OAuth code".to_owned(),
                payload: None,
            });
        }
        let lite = ctx.platform == Platform::Lite;
        let appid = p
            .wx_appid
            .clone()
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| crate::config::wx_appid(ctx.platform).appid.to_owned());
        let secret = p
            .wx_secret
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| crate::config::wx_appid(ctx.platform).secret.to_owned());
        let _ = lite;

        let mut query = QueryRecord::new();
        query.set("secret", secret);
        query.set("appid", appid);
        query.set("code", p.code.clone());
        query.set("grant_type", "authorization_code");
        let token_resp = ctx
            .raw(RawRequest {
                method: Some(Method::Post),
                url: "https://api.weixin.qq.com/sns/oauth2/access_token".to_owned(),
                query: Some(query),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        let token_data = token_resp.json().unwrap_or(Value::Null);
        let access_token = token_data
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let openid = token_data
            .get("openid")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if access_token.is_empty() || openid.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: token_data.to_string(),
                payload: Some(token_data),
            });
        }

        let date_now = crate::crypto::now_ms();
        let encrypt = ctx.aes_hex(
            &CipherInput::Json(json!({ "access_token": access_token })),
            None,
            None,
        );
        let (_, mac, dev) = device_identity(
            p.guid.as_deref(),
            p.mac.as_deref(),
            p.dev.as_deref(),
            None,
            ctx,
        );
        let data = login_by_openplat(
            ctx,
            "",
            &mac,
            &dev,
            36,
            &openid,
            &encrypt.str,
            &ctx.rsa_encrypt_raw(&CipherInput::Json(json!({
                "clienttime_ms": date_now,
                "key": encrypt.key,
            })))
            .to_uppercase(),
            None,
        )
        .await?;
        settle_login(ctx, data, &encrypt.key)
    })
}

fn login_qr_create_handler<'a>(
    _ctx: &'a CallCtx<'_>,
    p: &'a LoginQrCreateParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        // Locally construct the login URL; QR image rendering is out of scope
        // (generate an image from `url` with any QR library).
        Ok(json!({
            "url": format!("https://h5.kugou.com/apps/loginQRCode/html/index.html?qrcode={}", p.key),
            "base64": "",
        }))
    })
}

fn login_qr_check_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginQrCheckParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let (_, _, dev) = device_identity(
            p.guid.as_deref(),
            None,
            p.dev.as_deref(),
            p.uuid.as_deref(),
            ctx,
        );
        let mut query = QueryRecord::new();
        query.set("plat", 4);
        query.set("appid", ctx.appid);
        query.set("srcappid", ctx.srcappid);
        query.set("qrcode", p.key.clone());
        query.set("dev", dev);
        let data = ctx
            .request(
                "/v2/get_userinfo_qrcode",
                GatewayRequest {
                    base_url: Some("https://login-user.kugou.com".to_owned()),
                    sign: SignControl::Web,
                    query: Some(query),
                    ..GatewayRequest::default()
                },
            )
            .await?;

        // status=4 means the scan succeeded: persist the session.
        if data.get("status").and_then(Value::as_u64) == Some(4)
            && let Some(token) = data.get("token").and_then(Value::as_str)
        {
            let mut login = Map::new();
            login.insert("token".to_owned(), json!(token));
            if let Some(userid) = data.get("userid") {
                login.insert("userid".to_owned(), userid.clone());
            }
            ctx.apply_login(&Value::Object(login));
        }
        Ok(data)
    })
}

fn login_qr_authorize_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginQrAuthorizeParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let qrcode = &p.qrcode;
        if qrcode.is_empty()
            || !qrcode
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Error::Api {
                code: 0,
                message: "missing or invalid qrcode parameter".to_owned(),
                payload: None,
            });
        }
        if ctx.token.is_empty() || ctx.userid == 0 {
            return Err(Error::Api {
                code: 0,
                message: "missing logged-in token or userid".to_owned(),
                payload: None,
            });
        }
        let request_appid = p
            .appid
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|a| *a != 0)
            .unwrap_or(ctx.appid as i64);

        let mut query = QueryRecord::new();
        query.set("appid", request_appid);
        query.set("srcappid", ctx.srcappid);
        query.set("clientver", ctx.clientver);

        let scan_data = ctx
            .request(
                "/v2/scan",
                GatewayRequest {
                    method: Some(Method::Post),
                    base_url: Some("https://login-user.kugou.com".to_owned()),
                    sign: SignControl::Web,
                    query: Some(query.clone()),
                    json: Some(json!({
                        "plat": if js::unwrap_or_flex(&p.plat, 0) == 2 { 2 } else { 1 },
                        "userid": ctx.userid,
                        "token": ctx.token,
                        "qrcode": qrcode,
                        "type": 1,
                    })),
                    ..GatewayRequest::default()
                },
            )
            .await?;

        let scan_userid = scan_data
            .get("userid")
            .map(|v| match v {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        if scan_userid != ctx.userid.to_string() {
            let mut out = scan_data;
            out["qrcode"] = json!(qrcode);
            out["appid"] = json!(request_appid);
            out["authorized"] = json!(false);
            return Ok(out);
        }

        let authorize_data = ctx
            .request(
                "/v2/authorize",
                GatewayRequest {
                    method: Some(Method::Post),
                    base_url: Some("https://login-user.kugou.com".to_owned()),
                    sign: SignControl::Web,
                    query: Some(query),
                    ..GatewayRequest::default()
                },
            )
            .await?;
        let authorize_userid = authorize_data
            .get("userid")
            .map(|v| match v {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        let mut out = authorize_data;
        out["qrcode"] = json!(qrcode);
        out["appid"] = json!(request_appid);
        out["authorized"] = json!(authorize_userid == ctx.userid.to_string());
        Ok(out)
    })
}

fn login_qq_qr_create_handler<'a>(
    ctx: &'a CallCtx<'_>,
    _p: &'a crate::catalog::def::NoParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let client_id = if ctx.platform == Platform::Lite {
            "101706348"
        } else {
            "205141"
        };
        let time = crate::crypto::now_ms() / 1000;
        let sign = md5(&format!("fe4a24d80fcf253a00676a808f62c2c6_{time}"));

        let auth_params = [
            ("cancel_display", "1".to_owned()),
            ("sdkp", "a".to_owned()),
            ("display", "mobile".to_owned()),
            ("format", "json".to_owned()),
            ("sign", sign),
            ("sdkv", "3.5.11.lite".to_owned()),
            ("response_type", "token".to_owned()),
            ("status_os", "11".to_owned()),
            ("client_id", client_id.to_owned()),
            ("switch", "1".to_owned()),
            ("status_version", "30".to_owned()),
            ("show_download_ui", "true".to_owned()),
            ("pf", "openmobile_android".to_owned()),
            ("scope", "all".to_owned()),
            ("compat_v", "1".to_owned()),
            ("status_machine", "MEIZU+18+Pro".to_owned()),
            ("style", "qr".to_owned()),
            ("time", time.to_string()),
            ("redirect_uri", "auth://tauth.qq.com/".to_owned()),
        ];
        let auth_query: String = auth_params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        let auth_url = format!("https://openmobile.qq.com/oauth2.0/m_authorize?{auth_query}");

        let headers = vec![
            ("User-Agent".to_owned(), QQ_UA.to_owned()),
            (
                "Referer".to_owned(),
                "https://xui.ptlogin2.qq.com/".to_owned(),
            ),
        ];
        let auth_resp = ctx
            .raw(RawRequest {
                url: auth_url,
                headers: headers.clone(),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        let auth_text = auth_resp.text();
        let src = extract_quoted_after(&auth_text, "src = ").ok_or_else(|| Error::Api {
            code: 0,
            message: "m_authorize response missing xlogin url".to_owned(),
            payload: None,
        })?;
        let xlogin_url = decode_hex_escapes(&src);
        let xlogin_query = xlogin_url.split_once('?').map_or("", |(_, q)| q);
        let pt_openlogin_data = urlencode_component(xlogin_query);

        let mut cookie_jar: Vec<(String, String)> = Vec::new();
        let cookie_str = |jar: &[(String, String)]| {
            jar.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ")
        };
        let collect_cookies = |jar: &mut Vec<(String, String)>,
                               resp: &crate::http::HttpResponse| {
            for cookie in resp.headers_all("set-cookie") {
                let kv = cookie.split(';').next().unwrap_or("").trim();
                if let Some(idx) = kv.find('=') {
                    let name = kv[..idx].to_owned();
                    let value = kv[idx + 1..].to_owned();
                    if let Some(slot) = jar.iter_mut().find(|(k, _)| *k == name) {
                        slot.1 = value;
                    } else {
                        jar.push((name, value));
                    }
                }
            }
        };

        let xlogin_resp = ctx
            .raw(RawRequest {
                url: xlogin_url.clone(),
                headers: headers.clone(),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        collect_cookies(&mut cookie_jar, &xlogin_resp);
        let pt_login_sig = cookie_jar
            .iter()
            .find(|(k, _)| k == "pt_login_sig")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        let qr_url = format!(
            "https://xui.ptlogin2.qq.com/ssl/ptqrshow?s=8&e=0&appid=716027609&type=0&t={}&daid=381&pt_3rd_aid={client_id}",
            rand_jitter()
        );
        let qr_resp = ctx
            .raw(RawRequest {
                url: qr_url,
                headers: vec![
                    ("User-Agent".to_owned(), QQ_UA.to_owned()),
                    ("Referer".to_owned(), xlogin_url.clone()),
                    ("Cookie".to_owned(), cookie_str(&cookie_jar)),
                ],
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        collect_cookies(&mut cookie_jar, &qr_resp);
        let qrsig = cookie_jar
            .iter()
            .find(|(k, _)| k == "qrsig")
            .map(|(_, v)| v.clone())
            .ok_or_else(|| Error::Api {
                code: 0,
                message: "qrsig not obtained".to_owned(),
                payload: None,
            })?;

        use base64::Engine as _;
        Ok(json!({
            "qrcode": base64::engine::general_purpose::STANDARD.encode(&qr_resp.bytes),
            "qrsig": qrsig,
            "ptqrtoken": hash33(&qrsig),
            "pt_login_sig": pt_login_sig,
            "pt_openlogin_data": pt_openlogin_data,
            "xlogin_url": xlogin_url,
            "cookie": cookie_str(&cookie_jar),
        }))
    })
}

fn login_qq_qr_check_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginQqQrCheckParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut cookie = p.cookie.clone();
        if !p.pt_login_sig.is_empty() && !cookie.contains("pt_login_sig") {
            let sep = if cookie.is_empty() {
                ""
            } else {
                &format!("{cookie}; ")
            };
            cookie = format!("{sep}pt_login_sig={}", p.pt_login_sig);
        }

        let client_id = if ctx.platform == Platform::Lite {
            "101706348"
        } else {
            "205141"
        };
        let poll_url = format!(
            "https://xui.ptlogin2.qq.com/ssl/ptqrlogin?u1={}&from_ui=1&type=1&ptlang=2052&ptqrtoken={}&daid=381&aid=716027609&pt_3rd_aid={client_id}&pt_openlogin_data={}&device=2&ptopt=1&pt_uistyle=35&jsver=v1.36.0&login_sig={}&r={}",
            urlencode_component("http://connect.qq.com"),
            p.ptqrtoken,
            p.pt_openlogin_data,
            urlencode_component(&p.pt_login_sig),
            rand_jitter()
        );
        let poll_resp = ctx
            .raw(RawRequest {
                url: poll_url,
                headers: vec![
                    ("User-Agent".to_owned(), QQ_UA.to_owned()),
                    (
                        "Referer".to_owned(),
                        if p.xlogin_url.is_empty() {
                            "https://xui.ptlogin2.qq.com/".to_owned()
                        } else {
                            p.xlogin_url.clone()
                        },
                    ),
                    ("Cookie".to_owned(), cookie.clone()),
                ],
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        for set_cookie in poll_resp.headers_all("set-cookie") {
            let kv = set_cookie.split(';').next().unwrap_or("").trim().to_owned();
            if let Some(idx) = kv.find('=') {
                let name = &kv[..idx];
                if !cookie.contains(&format!("{name}=")) {
                    let sep = if cookie.is_empty() {
                        ""
                    } else {
                        &format!("{cookie}; ")
                    };
                    cookie = format!("{sep}{kv}");
                }
            }
        }

        let body = poll_resp.text();
        let inner = extract_between(&body, "ptuiCB(", ")").ok_or_else(|| Error::Api {
            code: 0,
            message: format!(
                "ptqrlogin parse failed: {}",
                body.chars().take(200).collect::<String>()
            ),
            payload: None,
        })?;
        let parts: Vec<String> = extract_single_quoted(inner);
        let code = parts.first().cloned().unwrap_or_default();
        let url = parts.get(2).cloned().unwrap_or_default();
        let msg = parts.get(4).cloned().unwrap_or_default();

        match code.as_str() {
            "66" => Ok(
                json!({ "status": "wait", "qrsig": p.qrsig, "ptqrtoken": p.ptqrtoken, "msg": "等待扫码" }),
            ),
            "65" => Ok(
                json!({ "status": "expired", "qrsig": p.qrsig, "ptqrtoken": p.ptqrtoken, "msg": "二维码已失效，请重新生成" }),
            ),
            "0" => {
                let mut openid = query_param(&url, "openid");
                let mut access_token = query_param(&url, "access_token");
                if openid.is_empty() || access_token.is_empty() {
                    let mut sig_url = if url.starts_with("http") {
                        url.clone()
                    } else {
                        format!("https://{url}")
                    };
                    for _hop in 0..10 {
                        let Ok(sig_resp) = ctx
                            .raw(RawRequest {
                                url: sig_url.clone(),
                                redirect: Redirect::Manual,
                                headers: vec![
                                    ("User-Agent".to_owned(), QQ_UA.to_owned()),
                                    ("Accept".to_owned(), "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".to_owned()),
                                    ("Accept-Language".to_owned(), "zh-CN,zh;q=0.9".to_owned()),
                                    ("Referer".to_owned(), if p.xlogin_url.is_empty() { "https://xui.ptlogin2.qq.com/".to_owned() } else { p.xlogin_url.clone() }),
                                    ("Cookie".to_owned(), cookie.clone()),
                                ],
                                retries: Some(0),
                                ..RawRequest::new("")
                            })
                            .await
                        else {
                            break;
                        };
                        let loc = sig_resp
                            .header("location")
                            .map(std::borrow::ToOwned::to_owned);
                        if let Some(found) = query_param_opt(&sig_url, "openid") {
                            openid = found;
                        }
                        if let Some(found) = query_param_opt(&sig_url, "access_token") {
                            access_token = found;
                        }
                        if !openid.is_empty() && !access_token.is_empty() {
                            break;
                        }
                        let Some(loc) = loc else {
                            let text = sig_resp.text();
                            if let Some(found) = extract_loose_kv(&text, "openid") {
                                openid = found;
                            }
                            if let Some(found) = extract_loose_kv(&text, "access_token") {
                                access_token = found;
                            }
                            break;
                        };
                        sig_url = if loc.starts_with("http") {
                            loc.clone()
                        } else {
                            format!("https://{loc}")
                        };
                        if loc.contains("://") && !loc.starts_with("http") {
                            if let Some(found) = query_param_opt(&sig_url, "openid") {
                                openid = found;
                            }
                            if let Some(found) = query_param_opt(&sig_url, "access_token") {
                                access_token = found;
                            }
                            break;
                        }
                    }
                }
                if openid.is_empty() || access_token.is_empty() {
                    return Err(Error::Api {
                        code: 0,
                        message: format!("login succeeded but openid/access_token missing: {url}"),
                        payload: None,
                    });
                }
                qq_openplat_login(ctx, &openid, &access_token).await
            }
            _ => Ok(
                json!({ "status": code, "qrsig": p.qrsig, "ptqrtoken": p.ptqrtoken, "msg": msg }),
            ),
        }
    })
}

/// The QQ token login shared by `login.qq` and the QR-check success branch.
async fn qq_openplat_login(ctx: &CallCtx<'_>, openid: &str, access_token: &str) -> Result<Value> {
    let date_now = crate::crypto::now_ms();
    let encrypt = ctx.aes_hex(
        &CipherInput::Json(json!({ "access_token": access_token })),
        None,
        None,
    );
    let third_appid = if ctx.platform == Platform::Lite {
        "101706348"
    } else {
        "205141"
    };
    let data = login_by_openplat(
        ctx,
        "",
        "",
        "",
        1,
        openid,
        &encrypt.str,
        &ctx.rsa_encrypt_raw(&CipherInput::Json(json!({
            "clienttime_ms": date_now,
            "key": encrypt.key,
        })))
        .to_uppercase(),
        Some(third_appid),
    )
    .await?;
    settle_login(ctx, data, &encrypt.key)
}

fn login_wx_create_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginWxCreateParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let appid = p
            .wx_appid
            .clone()
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| crate::config::wx_appid(ctx.platform).appid.to_owned());
        let secret = p
            .wx_secret
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| crate::config::wx_appid(ctx.platform).secret.to_owned());

        let mut query = QueryRecord::new();
        query.set("appid", appid.clone());
        query.set("secret", secret);
        query.set("grant_type", "client_credential");
        let token_resp = ctx
            .raw(RawRequest {
                url: "https://api.weixin.qq.com/cgi-bin/token".to_owned(),
                query: Some(query),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        let access_token = token_resp.json().unwrap_or(Value::Null);
        let access_token_str = access_token
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if access_token_str.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: access_token.to_string(),
                payload: Some(access_token),
            });
        }

        let mut query = QueryRecord::new();
        query.set("access_token", access_token_str);
        query.set("type", 2);
        let ticket_resp = ctx
            .raw(RawRequest {
                url: "https://api.weixin.qq.com/cgi-bin/ticket/getticket".to_owned(),
                query: Some(query),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        let ticket_data = ticket_resp.json().unwrap_or(Value::Null);
        if ticket_data.get("errcode").and_then(Value::as_i64) != Some(0) {
            return Err(Error::Api {
                code: 0,
                message: ticket_data.to_string(),
                payload: Some(ticket_data),
            });
        }

        let timestamp = crate::crypto::now_ms();
        let noncestr = md5(&random_string(16));
        let ticket = ticket_data
            .get("ticket")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let signature = sha1_hex(&format!(
            "appid={appid}&noncestr={noncestr}&sdk_ticket={ticket}&timestamp={timestamp}"
        ));

        let mut query = QueryRecord::new();
        query.set("appid", appid);
        query.set("noncestr", noncestr);
        query.set("timestamp", timestamp.to_string());
        query.set("scope", "snsapi_userinfo");
        query.set("signature", signature);
        let connect_resp = ctx
            .raw(RawRequest {
                url: "https://open.weixin.qq.com/connect/sdk/qrconnect".to_owned(),
                query: Some(query),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        let mut connect = connect_resp.json().unwrap_or(Value::Null);
        if connect.get("errcode").and_then(Value::as_i64) != Some(0) {
            return Err(Error::Api {
                code: 0,
                message: connect.to_string(),
                payload: Some(connect),
            });
        }
        let uuid = connect
            .get("uuid")
            .and_then(Value::as_str)
            .map(std::borrow::ToOwned::to_owned);
        if let Some(uuid) = uuid
            && let Some(qrcode) = connect.get_mut("qrcode").and_then(Value::as_object_mut)
        {
            qrcode.insert(
                "qrcodeurl".to_owned(),
                json!(format!(
                    "https://open.weixin.qq.com/connect/confirm?uuid={uuid}"
                )),
            );
        }
        Ok(connect)
    })
}

fn login_wx_check_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LoginWxParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let resp = ctx
            .raw(RawRequest {
                url: format!(
                    "https://long.open.weixin.qq.com/connect/l/qrconnect?f=json&uuid={}",
                    p.uuid
                ),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        Ok(resp.json().unwrap_or(Value::Null))
    })
}

fn register_dev_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a crate::api::RegisterDeviceParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move { ctx.client.register_device_impl(p).await })
}

fn sidedt_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a VerifyParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let value = generate_simulate(SimulateInput {
            mid: Some(
                p.mid
                    .clone()
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| ctx.mid.clone()),
            ),
            userid: Some(
                p.userid
                    .as_ref()
                    .and_then(FlexI64::value)
                    .filter(|u| *u != 0)
                    .and_then(|u| u64::try_from(u).ok())
                    .unwrap_or(ctx.userid),
            ),
            dfid: Some(
                p.dfid
                    .clone()
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| ctx.dfid.clone()),
            ),
            webgl_hash: None,
        })?;

        let mut params = serde_json::to_value(p)?;
        if let Some(obj) = params.as_object_mut() {
            obj.insert("edt".to_owned(), json!(value.edt));
            obj.insert("sid".to_owned(), json!(value.sid));
        }
        ctx.call_inner("verify.user.info", params).await
    })
}

fn verify_user_info_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a VerifyParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let v_type = js::or_flex(&p.v_type, 23);
        let userid = p
            .userid
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|u| *u != 0)
            .unwrap_or(ctx.userid as i64);

        let mut data = Map::new();
        if let Some(eventid) = &p.eventid {
            data.insert("eventid".to_owned(), eventid.clone());
        }
        data.insert("userid".to_owned(), json!(userid));
        data.insert("platid".to_owned(), json!(js::or_flex(&p.platid, 2)));
        data.insert("v_type".to_owned(), json!(v_type));
        data.insert("wasm".to_owned(), json!(1));
        data.insert("i".to_owned(), json!(""));
        data.insert(
            "sid".to_owned(),
            json!(decode_uri_component(p.sid.as_deref().unwrap_or("")).clone()),
        );
        data.insert(
            "edt".to_owned(),
            json!(decode_uri_component(p.edt.as_deref().unwrap_or("")).clone()),
        );

        if v_type == 23 {
            let encrypt = ctx.aes_hex(&CipherInput::Json(json!({})), None, None);
            data.insert(
                "verifycode".to_owned(),
                json!(decode_uri_component(p.verifycode.as_deref().unwrap_or(""))),
            );
            data.insert(
                "pk".to_owned(),
                json!(ctx.rsa_encrypt_raw(&CipherInput::Json(json!({ "key": encrypt.key })))),
            );
            data.insert("params".to_owned(), json!(encrypt.str));
        }
        if v_type == 32 {
            let encrypt = ctx.aes_hex(
                &CipherInput::Json(json!({
                    "code": p.verifycode.clone().unwrap_or_default(),
                })),
                None,
                None,
            );
            data.insert(
                "code".to_owned(),
                json!(decode_uri_component(p.verifycode.as_deref().unwrap_or(""))),
            );
            data.insert(
                "pk".to_owned(),
                json!(ctx.rsa_encrypt_raw(&CipherInput::Json(json!({ "key": encrypt.key })))),
            );
            data.insert("params".to_owned(), json!(encrypt.str));
        }

        let mut query = QueryRecord::new();
        query.set("clientver", 11510);
        ctx.request(
            "/v4/verify_user_info",
            GatewayRequest {
                method: Some(Method::Post),
                base_url: Some("https://verifyservice.kugou.com".to_owned()),
                json: Some(Value::Object(data)),
                query: Some(query),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

// ─────────────────────────────────────────────────────────────
// Small text helpers for the QQ flow
// ─────────────────────────────────────────────────────────────

/// Extract `"value"` following a marker like `src = `.
fn extract_quoted_after(text: &str, marker: &str) -> Option<String> {
    let idx = text.find(marker)?;
    let rest = &text[idx + marker.len()..];
    let start = rest.find('"')?;
    let end = rest[start + 1..].find('"')?;
    Some(rest[start + 1..start + 1 + end].to_owned())
}

/// Decode `\xNN` escapes.
fn decode_hex_escapes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'x') {
            chars.next();
            let mut hex = String::new();
            for _ in 0..2 {
                if let Some(h) = chars.peek().copied()
                    && h.is_ascii_hexdigit()
                {
                    hex.push(h);
                    chars.next();
                }
            }
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                out.push(byte as char);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Extract the payload between `start_marker` and `end_marker`.
fn extract_between<'t>(text: &'t str, start: &str, end: &str) -> Option<&'t str> {
    let s = text.find(start)? + start.len();
    let rest = &text[s..];
    let e = rest.find(end)?;
    Some(&rest[..e])
}

/// Extract all single-quoted segments.
fn extract_single_quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\''
            && let Some(end) = text[i + 1..].find('\'')
        {
            out.push(text[i + 1..i + 1 + end].to_owned());
            // advance past this segment
            for _ in 0..end {
                chars.next();
            }
        }
    }
    out
}

/// Query-parameter extraction (`openid=…&` / `#`-terminated).
fn query_param(url: &str, key: &str) -> String {
    query_param_opt(url, key).unwrap_or_default()
}

fn query_param_opt(url: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=");
    let idx = url.find(&needle)?;
    let rest = &url[idx + needle.len()..];
    let end = rest.find(['&', '#']).unwrap_or(rest.len());
    Some(rest[..end].to_owned())
}

/// Loose `openid` / `access_token` extraction from response bodies.
fn extract_loose_kv(text: &str, key: &str) -> Option<String> {
    let mut idx = 0;
    while let Some(pos) = text[idx..].find(key) {
        let rest = &text[idx + pos + key.len()..];
        let mut chars = rest.chars().peekable();
        // optional quotes, then `:` or `=`
        while matches!(chars.peek(), Some('\'' | '"')) {
            chars.next();
        }
        if matches!(chars.peek(), Some(':' | '=')) {
            chars.next();
            while matches!(chars.peek(), Some('\'' | '"')) {
                chars.next();
            }
            let value: String = chars
                .take_while(|c| {
                    !c.is_whitespace() && *c != '&' && *c != '"' && *c != '\'' && *c != ','
                })
                .collect();
            if !value.is_empty() {
                return Some(value);
            }
        }
        idx += pos + key.len();
    }
    None
}

/// `Math.random()`-style jitter for cache-busting parameters.
fn rand_jitter() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    format!("{:.17}", rng.random::<f64>())
}

impl crate::api::AccountApi<'_> {
    /// Password login (`login`) — persists credentials on success.
    pub async fn login(&self, params: LoginParams) -> Result<Value> {
        self.client.call_value("login", params).await
    }

    /// SMS login (`login.cellphone`) — persists credentials on success.
    pub async fn login_cellphone(&self, params: LoginCellphoneParams) -> Result<Value> {
        self.client.call_value("login.cellphone", params).await
    }

    /// Token refresh (`login.token`).
    pub async fn login_token(&self, params: LoginTokenParams) -> Result<Value> {
        self.client.call_value("login.token", params).await
    }

    /// Device list (`login.device`).
    pub async fn login_device(&self, params: LoginDeviceParams) -> Result<Value> {
        self.client.call_value("login.device", params).await
    }

    /// Kick a device offline (`login.device.kick`).
    pub async fn login_device_kick(&self, params: LoginDeviceKickParams) -> Result<Value> {
        self.client.call_value("login.device.kick", params).await
    }

    /// QQ token login (`login.qq`).
    pub async fn login_qq(&self, params: LoginQqParams) -> Result<Value> {
        self.client.call_value("login.qq", params).await
    }

    /// `WeChat` OAuth login (`login.openplat`).
    pub async fn login_openplat(&self, params: LoginOpenplatParams) -> Result<Value> {
        self.client.call_value("login.openplat", params).await
    }

    /// QR login key (`login.qr.key`).
    pub async fn login_qr_key(&self) -> Result<Value> {
        self.client
            .call_value("login.qr.key", LoginQrKeyParams::default())
            .await
    }

    /// Build the QR login URL (`login.qr.create`).
    pub async fn login_qr_create(&self, params: LoginQrCreateParams) -> Result<Value> {
        self.client.call_value("login.qr.create", params).await
    }

    /// Poll the QR login status (`login.qr.check`).
    pub async fn login_qr_check(&self, params: LoginQrCheckParams) -> Result<Value> {
        self.client.call_value("login.qr.check", params).await
    }

    /// Authorize another device's QR (`login.qr.authorize`).
    pub async fn login_qr_authorize(&self, params: LoginQrAuthorizeParams) -> Result<Value> {
        self.client.call_value("login.qr.authorize", params).await
    }

    /// Create a QQ login QR (`login.qq.qr.create`).
    pub async fn login_qq_qr_create(&self) -> Result<Value> {
        self.client
            .call_value(
                "login.qq.qr.create",
                crate::catalog::def::NoParams::default(),
            )
            .await
    }

    /// Poll the QQ login QR (`login.qq.qr.check`).
    pub async fn login_qq_qr_check(&self, params: LoginQqQrCheckParams) -> Result<Value> {
        self.client.call_value("login.qq.qr.check", params).await
    }

    /// Create a `WeChat` login QR (`login.wx.create`).
    pub async fn login_wx_create(&self, params: LoginWxCreateParams) -> Result<Value> {
        self.client.call_value("login.wx.create", params).await
    }

    /// Poll the `WeChat` login QR (`login.wx.check`).
    pub async fn login_wx_check(&self, params: LoginWxParams) -> Result<Value> {
        self.client.call_value("login.wx.check", params).await
    }

    /// Register the device (`register.dev`) — typed response; the session is
    /// updated automatically.
    pub async fn register_device(
        &self,
        params: crate::api::RegisterDeviceParams,
    ) -> Result<crate::api::RegisterData> {
        self.client.call_typed("register.dev", params).await
    }

    /// SSA verification with an auto-generated payload (`sidedt`).
    pub async fn sidedt(&self, params: VerifyParams) -> Result<Value> {
        self.client.call_value("sidedt", params).await
    }

    /// Submit verification info (`verify.user.info`).
    pub async fn verify_user_info(&self, params: VerifyParams) -> Result<Value> {
        self.client.call_value("verify.user.info", params).await
    }

    /// User verification status (`user.verify`).
    pub async fn user_verify(&self) -> Result<Value> {
        self.client
            .call_value("user.verify", crate::catalog::def::NoParams::default())
            .await
    }
}
