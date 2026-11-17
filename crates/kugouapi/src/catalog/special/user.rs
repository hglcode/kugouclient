//! User-domain special endpoints: grade info, listen reporting (the CSCC
//! protocol), private chat, avatar upload, preference config, visitors,
//! blacklist editing and batch VIP info.
//!
//! Ported from `special/user.ts`.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use md5::{Digest as _, Md5};
use serde_json::{Map, Value, json};

use crate::catalog::ctx::{GatewayRequest, RawRequest, SignControl};
use crate::catalog::def::{Built, EndpointSpec, Salt, handler_endpoint, simple_endpoint, spec};
use crate::crypto::CipherInput;
use crate::error::{Error, Result};
use crate::http::{Body, Method};
use crate::js::{self, FlexI64, FlexString};
use crate::query::QueryRecord;
use crate::sign::{SignOptions, md5, signature};
use crate::util::{is_image_buffer, normalize_base64};

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

const CSCC_APPKEY: &str = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA";
const CSCC_VERSION: &str = "10597";

kugou_params! {
    /// Parameters for `user.grade.info`.
    pub UserGradeInfoParams {
        required {}
        optional {
            /// Query type (default 1).
            list_type: FlexI64 => "type",
            /// Report: elapsed seconds.
            d_sec: FlexI64 => "d_sec",
            /// Report: diff seconds.
            diff_sec: FlexI64 => "diff_sec",
            /// Report: y type.
            y_type: FlexI64 => "y_type",
            /// Report: m type.
            m_type: FlexI64 => "m_type",
        }
    }
}

/// Parameters for `user.listen.report` (CSCC).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserListenReportParams {
    /// Mixsong id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixsongid: Option<FlexI64>,
    /// Event: `start` or `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Play-state description (default `完整播放`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Played duration (event end).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<FlexI64>,
    /// Grade sync: elapsed seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d_sec: Option<FlexI64>,
    /// Grade sync: diff seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_sec: Option<FlexI64>,
    /// UUID override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Mid override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid: Option<String>,
    /// User id override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userid: Option<FlexI64>,
    /// Token override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Device model override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    /// Device model alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dev: Option<String>,
    /// System version (default `9`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_version: Option<String>,
    /// Screen width (default 1920).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_width: Option<FlexI64>,
    /// Screen height (default 1080).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_height: Option<FlexI64>,
    /// Local IP (default `0.0.0.0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_ip: Option<String>,
}

/// Parameters for `user.follow.chat`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserFollowChatParams {
    /// Target user id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tuid: Option<FlexI64>,
    /// Target chat tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Message type: 201 text (default), 202 image, 205 sticker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msgtype: Option<FlexI64>,
    /// Message text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert: Option<String>,
    /// Sender nickname.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// Message source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FlexI64>,
    /// Follow source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_source: Option<FlexI64>,
    /// Push notification text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nt: Option<String>,
    /// Location payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Value>,
    /// Group id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groupid: Option<FlexI64>,
    /// Media URL (image / sticker / other).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Sticker thumbnail URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbUrl: Option<String>,
    /// Image bytes as base64 (msgtype 202, when no url).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Image bytes as base64 / dataURL (alias of `data`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imgFile: Option<String>,
    /// Image extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extendname: Option<String>,
    /// Image width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<FlexI64>,
    /// Image height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<FlexI64>,
    /// Send the original image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_original: Option<bool>,
    /// `is_original` alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isOriginal: Option<bool>,
    /// Original size (bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_size: Option<FlexI64>,
    /// `original_size` alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub originalSize: Option<FlexI64>,
    /// Media duration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timelen: Option<FlexI64>,
    /// Fake id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fakeid: Option<FlexI64>,
    /// Ingress path (`source_path` / `ingress` chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Ingress alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingress: Option<String>,
    /// Retry flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<bool>,
}

/// Parameters for `user.update.avatar` (image as base64 / dataURL).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserUpdateAvatarParams {
    /// Image bytes as base64 / dataURL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imgFile: Option<String>,
    /// Alias of `imgFile`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub img: Option<String>,
    /// Alias of `imgFile`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Upload file name (default `avatar.jpg`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Parameters for `user.preference.update`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserPreferenceUpdateParams {
    /// Preferred mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Value>,
    /// Gender preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<Value>,
    /// Age preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<Value>,
    /// Language preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<Value>,
    /// Style preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Value>,
    /// Song language list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub song_lang: Option<Value>,
    /// Style-rec tag list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stylerec_taglist: Option<Value>,
    /// Wish tag list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wish_taglist: Option<Value>,
    /// Inactive tag list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inactive_taglist: Option<Value>,
    /// Inactive singer list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inactive_singerlist: Option<Value>,
}

kugou_params! {
    /// Parameters for `user.visitors`.
    pub UserVisitorsParams {
        required {}
        optional {
            /// Target user id (`t_userid` / `visit_userid` chain).
            t_userid: FlexI64 => "t_userid",
            /// Target user id alias.
            visit_userid: FlexI64 => "visit_userid",
            /// App id override.
            appid: FlexI64 => "appid",
            /// Client version override.
            clientver: FlexI64 => "clientver",
            /// Page number.
            page: FlexI64 => "page",
            /// Business type.
            busi_type: FlexI64 => "busi_type",
            /// KG-DEVID header.
            kg_devid: FlexI64 => "kg_devid",
            /// KG-CLIENTTIMEMS header.
            kg_clienttimems: FlexI64 => "kg_clienttimems",
        }
    }
}

/// Parameters for `blacklist`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlacklistParams {
    /// Label: song (default) / singer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Explicit items (`[{k, v}]` or its JSON string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Value>,
    /// Song hash (auto item construction).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Singer id (auto item construction).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub singerid: Option<FlexI64>,
    /// Song name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Mixsong id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixsongid: Option<FlexI64>,
    /// Source override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FlexI64>,
    /// Delete instead of add.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isDelete: Option<bool>,
    /// `KG-TID` module id (default 473).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moduleId: Option<FlexI64>,
}

kugou_params! {
    /// Parameters for `user.batch.union.vipinfo`.
    pub UserBatchUnionVipinfoParams {
        required {
            /// User ids (array, or comma/space separated string; max 20).
            useridlist: FlexString => "useridlist",
        }
        optional {}
    }
}

const EP_USER_GRADE_INFO: EndpointSpec = EndpointSpec {
    origin: Some("http://userinfo.user.kugou.com"),
    salt: Salt::None,
    auth: true,
    ..spec(
        "user.grade.info",
        "user_grade_info",
        Method::Post,
        "/v2/get_grade_info",
    )
};
const EP_USER_LISTEN_REPORT: EndpointSpec = EndpointSpec {
    origin: Some("http://d.kugou.com"),
    salt: Salt::None,
    auth: true,
    ..spec(
        "user.listen.report",
        "user_listen_report",
        Method::Post,
        "/v2/post",
    )
};
const EP_USER_FOLLOW_CHAT: EndpointSpec = EndpointSpec {
    router: Some("msg.mobile.kugou.com"),
    auth: true,
    ..spec(
        "user.follow.chat",
        "user_follow_chat",
        Method::Post,
        "/v1/chat/send",
    )
};
const EP_USER_UPDATE_AVATAR: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "user.update.avatar",
        "user_update_avatar",
        Method::Post,
        "/v1/profile",
    )
};
const EP_USER_PREFERENCE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "user.preference",
        "user_preference",
        Method::Post,
        "/userpreferservice/v1/get_user_conf",
    )
};
const EP_USER_PREFERENCE_UPDATE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "user.preference.update",
        "user_preference_update",
        Method::Post,
        "/userpreferservice/v1/update_user_conf",
    )
};
const EP_USER_VISITORS: EndpointSpec = EndpointSpec {
    origin: Some("http://usercenter.kugou.com"),
    salt: Salt::None,
    auth: true,
    ..spec(
        "user.visitors",
        "user_visitors",
        Method::Post,
        "/v2/get_visitors",
    )
};
const EP_BLACKLIST: EndpointSpec = EndpointSpec {
    origin: Some("https://relationuser.kugou.com"),
    auth: true,
    ..spec(
        "blacklist",
        "blacklist",
        Method::Post,
        "/v1/edit_list_items",
    )
};
const EP_USER_BATCH_UNION_VIPINFO: EndpointSpec = EndpointSpec {
    origin: Some("https://kugouvip.kugou.com"),
    auth: true,
    ..spec(
        "user.batch.union.vipinfo",
        "user_batch_union_vipinfo",
        Method::Post,
        "/v2/batch_union_vipinfo",
    )
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        handler_endpoint(&EP_USER_GRADE_INFO, user_grade_info_handler),
        handler_endpoint(&EP_USER_LISTEN_REPORT, user_listen_report_handler),
        handler_endpoint(&EP_USER_FOLLOW_CHAT, user_follow_chat_handler),
        handler_endpoint(&EP_USER_UPDATE_AVATAR, user_update_avatar_handler),
        handler_endpoint(&EP_USER_PREFERENCE, user_preference_handler),
        handler_endpoint(&EP_USER_PREFERENCE_UPDATE, user_preference_update_handler),
        handler_endpoint(&EP_USER_VISITORS, user_visitors_handler),
        handler_endpoint(&EP_BLACKLIST, blacklist_handler),
        simple_endpoint(
            &EP_USER_BATCH_UNION_VIPINFO,
            |p: &UserBatchUnionVipinfoParams, _| {
                let raw = p.useridlist.value().unwrap_or("");
                let list: Vec<String> = raw
                    .split([',', ' '])
                    .filter(|s| !s.is_empty())
                    .map(std::borrow::ToOwned::to_owned)
                    .collect();
                let mut ids: Vec<String> = Vec::new();
                for item in list {
                    let id = item.trim();
                    if let Ok(value) = id.parse::<i64>() {
                        if value <= 0 {
                            continue;
                        }
                    } else {
                        continue;
                    }
                    if !ids.contains(&id.to_owned()) {
                        ids.push(id.to_owned());
                    }
                    if ids.len() >= 20 {
                        break;
                    }
                }
                let mut query = QueryRecord::new();
                query.set("userids", ids.join(","));
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
    ]
}

fn random_hex7() -> String {
    crate::crypto::random_thash()
}

// ─────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────

fn user_grade_info_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserGradeInfoParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let clienttime = crate::catalog::ctx::now_secs_public();
        let key = md5(&format!("3116{CSCC_APPKEY}10597{clienttime}"));

        let mut data = Map::new();
        data.insert("mid".to_owned(), json!(ctx.mid));
        data.insert("type".to_owned(), json!(js::or_flex(&p.list_type, 1)));
        data.insert("uuid".to_owned(), json!(ctx.uuid));
        data.insert("userid".to_owned(), json!(ctx.userid));

        let is_report = p.d_sec.is_some() && p.diff_sec.is_some();
        let p_param = if is_report {
            let d_sec = js::unwrap_or_flex(&p.d_sec, 0);
            let diff_sec = js::unwrap_or_flex(&p.diff_sec, 0);
            let y_type = js::or_flex(&p.y_type, 0);
            let m_type = js::or_flex(&p.m_type, 0);
            let md5_value = md5(&format!("{d_sec}{diff_sec}{y_type}{m_type}"));
            data.insert("d_sec".to_owned(), json!(d_sec));
            data.insert("diff_sec".to_owned(), json!(diff_sec));
            data.insert("y_type".to_owned(), json!(y_type));
            data.insert("m_type".to_owned(), json!(m_type));
            ctx.rsa_encrypt_raw(&CipherInput::Json(json!({
                "token": ctx.token,
                "md5": md5_value,
            })))
            .to_uppercase()
        } else {
            ctx.rsa_encrypt_raw(&CipherInput::Text(
                serde_json::to_string(&json!({
                    "clienttime": clienttime,
                    "userid": ctx.userid,
                }))
                .unwrap_or_default(),
            ))
            .to_uppercase()
        };
        data.insert("p".to_owned(), json!(p_param));
        data.insert("appid".to_owned(), json!("3116"));
        data.insert("clientver".to_owned(), json!("10597"));
        data.insert("clienttime".to_owned(), json!(clienttime));
        data.insert("key".to_owned(), json!(key));

        let mut query = QueryRecord::new();
        query.set("dfid", ctx.dfid.clone());
        ctx.request(
            "/v2/get_grade_info",
            GatewayRequest {
                method: Some(Method::Post),
                base_url: Some("http://userinfo.user.kugou.com".to_owned()),
                sign: SignControl::None,
                clear_params: true,
                query: Some(query),
                json: Some(Value::Object(data)),
                headers: vec![
                    (
                        "Content-Type".to_owned(),
                        "text/plain; charset=ISO-8859-1".to_owned(),
                    ),
                    (
                        "User-Agent".to_owned(),
                        "Android15-1070-10597-201-0-get_user_grade_info-wifi".to_owned(),
                    ),
                    ("KG-THash".to_owned(), random_hex7()),
                    ("KG-Rec".to_owned(), "1".to_owned()),
                    ("KG-RC".to_owned(), "1".to_owned()),
                ],
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

// ── CSCC session cache ──

#[derive(Debug, Clone)]
struct CsccSession {
    cookie: String,
    clienttime: String,
    serverstr: String,
    field2: String,
    expires: u64,
}

static CSCC_SESSIONS: LazyLock<Mutex<HashMap<String, CsccSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

async fn cscc_request(
    ctx: &CallCtx<'_>,
    url: &str,
    method: Method,
    query: QueryRecord,
    data: Option<Vec<u8>>,
    kind: &str,
    system_version: &str,
) -> Result<crate::http::HttpResponse> {
    let mut headers = Vec::new();
    if method == Method::Post {
        headers.push((
            "Content-Type".to_owned(),
            "application/octet-stream".to_owned(),
        ));
    }
    if url != "/v3/qrydid" {
        headers.push(("KG-Rec".to_owned(), "1".to_owned()));
        headers.push((
            "User-Agent".to_owned(),
            format!("Android{system_version}-1070-{CSCC_VERSION}-18-0-Cscc{kind}-wifi"),
        ));
    }
    ctx.raw(RawRequest {
        method: Some(method),
        url: format!("http://d.kugou.com{url}"),
        query: Some(query),
        headers,
        body: data.map(Body::Bytes),
        retries: Some(0),
        ..RawRequest::new("")
    })
    .await
}

/// The CSCC handshake: qrydid (device registration) → gen (session).
async fn cscc_handshake(
    ctx: &CallCtx<'_>,
    uuid: &str,
    mid: &str,
    device_model: &str,
    screen_width: i64,
    screen_height: i64,
    system_version: &str,
) -> Result<CsccSession> {
    let mut t = crate::catalog::ctx::now_secs_public().to_string();
    let device_data = serde_json::to_string(&json!({
        "machine": device_model,
        "mid": mid,
        "uuid": uuid,
        "wh": [screen_width, screen_height],
    }))?;
    let mut query = QueryRecord::new();
    query.set("appid", "and02");
    query.set("_t", t.clone());
    query.set(
        "sign",
        md5(&format!(
            "_t{t}appidand02pbKC7zn{{4U*ydo2M1Rir{device_data}"
        )),
    );
    let device = cscc_request(
        ctx,
        "/v3/qrydid",
        Method::Post,
        query,
        Some(device_data.into_bytes()),
        "Gen",
        system_version,
    )
    .await?;
    let device_body = device.json().unwrap_or(Value::Null);
    let deviceid = device_body
        .pointer("/data/deviceid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if deviceid.is_empty() {
        return Err(Error::Api {
            code: 0,
            message: "CSCC did not return a deviceid".to_owned(),
            payload: Some(device_body),
        });
    }

    let field2 = {
        use rand::RngExt;
        let bytes: [u8; 16] = rand::rng().random();
        hex::encode(bytes)
    };
    use rsa::pkcs8::DecodePublicKey;
    let public_key = rsa::PublicKey::from_public_key_pem(crate::crypto::LITE_RSA_PUBLIC_KEY)
        .map_err(|e| Error::Crypto(e.to_string()))?;
    use rsa::Pkcs1v15Encrypt;
    let s = BASE64.encode({
        let mut rng = rand::rng();
        public_key
            .encrypt(
                &mut rng,
                Pkcs1v15Encrypt,
                format!("{uuid}\t{field2}\t{}\t{deviceid}", crate::crypto::now_ms()).as_bytes(),
            )
            .map_err(|e| Error::Crypto(e.to_string()))?
    });

    t = crate::catalog::ctx::now_secs_public().to_string();
    let mut query = QueryRecord::new();
    query.set("s", s.clone());
    query.set("appid", "3116");
    query.set("_t", t.clone());
    query.set("sign", md5(&format!("_t{t}appid3116s{s}{CSCC_APPKEY}")));
    let gen_resp = cscc_request(
        ctx,
        "/v2/gen",
        Method::Get,
        query,
        None,
        "Gen",
        system_version,
    )
    .await?;
    let gen_body = gen_resp.json().unwrap_or(Value::Null);
    let encrypted = gen_body
        .get("data")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let cipher_bytes = BASE64
        .decode(encrypted)
        .map_err(|e| Error::Crypto(format!("CSCC session base64: {e}")))?;
    let decipher =
        Aes256CbcDec::new_from_slices(field2.as_bytes(), &uuid.as_bytes()[..16.min(uuid.len())])
            .map_err(|e| Error::Crypto(e.to_string()))?;
    let plain = decipher
        .decrypt_padded_vec::<Pkcs7>(&cipher_bytes)
        .map_err(|e| Error::Crypto(format!("CSCC session decrypt: {e}")))?;
    let session: Value = serde_json::from_slice(&plain)
        .map_err(|e| Error::Crypto(format!("CSCC session json: {e}")))?;
    let cookie = session
        .get("cookie")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let clienttime = session.get("clienttime").map_or_default(|v| match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => String::new(),
    });
    let serverstr = session
        .get("serverstr")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if cookie.is_empty() || clienttime.is_empty() || serverstr.is_empty() {
        return Err(Error::Api {
            code: 0,
            message: "invalid CSCC session response".to_owned(),
            payload: Some(session),
        });
    }
    Ok(CsccSession {
        cookie,
        clienttime,
        serverstr,
        field2,
        expires: crate::crypto::now_ms() + 30 * 60 * 1000,
    })
}

fn deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = encoder.write_all(data);
    encoder.finish().unwrap_or_default()
}

fn user_listen_report_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserListenReportParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let uuid = p
            .uuid
            .clone()
            .filter(|u| !u.is_empty())
            .or_else(|| Some(ctx.uuid.clone()).filter(|u| !u.is_empty() && *u != "-"))
            .unwrap_or_default()
            .replace('-', "");
        let mid = p
            .mid
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| ctx.mid.clone());
        let userid = p
            .userid
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|u| *u != 0)
            .map_or_else(|| ctx.userid.to_string(), |u| u.to_string());
        let token = p
            .token
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| ctx.token.clone());
        let song = p
            .mixsongid
            .as_ref()
            .and_then(FlexI64::value)
            .map_or_default(|v| v.to_string());
        let event = p.event.clone().unwrap_or_default();
        let state = p
            .state
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "完整播放".to_owned());
        let sync = p.d_sec.is_some() || p.diff_sec.is_some();
        let device_model = p
            .device_model
            .clone()
            .or_else(|| p.dev.clone())
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| {
                let device = ctx.client.device_info().device.clone();
                if device.is_empty() {
                    "KuGouMusicApi".to_owned()
                } else {
                    device
                }
            });
        let system_version = p
            .system_version
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "9".to_owned());
        let screen_width = js::or_flex(&p.screen_width, 1920);
        let screen_height = js::or_flex(&p.screen_height, 1080);
        let local_ip = p
            .local_ip
            .clone()
            .filter(|i| !i.is_empty())
            .unwrap_or_else(|| "0.0.0.0".to_owned());

        if uuid.is_empty() || mid.is_empty() || userid.is_empty() || token.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "token, userid, mid and uuid are required (login first)".to_owned(),
                payload: None,
            });
        }
        if song.is_empty() || (event != "start" && event != "end") {
            return Err(Error::Api {
                code: 0,
                message: "a valid mixsongid and event=start|end are required".to_owned(),
                payload: None,
            });
        }

        // Session (cached 30 minutes).
        let cache_key = md5(&serde_json::to_string(&json!([
            uuid,
            mid,
            userid,
            token,
            device_model,
            system_version,
            screen_width,
            screen_height,
        ]))?);
        let cached = CSCC_SESSIONS
            .lock()
            .expect("cscc session lock")
            .get(&cache_key)
            .filter(|s| s.expires > crate::crypto::now_ms())
            .cloned();
        let session = if let Some(session) = cached {
            session
        } else {
            let session = cscc_handshake(
                ctx,
                &uuid,
                &mid,
                &device_model,
                screen_width,
                screen_height,
                &system_version,
            )
            .await?;
            CSCC_SESSIONS
                .lock()
                .expect("cscc session lock")
                .insert(cache_key, session.clone());
            session
        };

        // Report payload.
        let mut fields: Vec<String> = if event == "start" {
            vec![
                "type_id=20431".to_owned(),
                "action=play".to_owned(),
                "fo3=3878,4320,4326,5747,5749,7229,7289,7863".to_owned(),
                "spt=0".to_owned(),
                "sty=手动".to_owned(),
                format!("mixsongid={song}"),
                "source=46".to_owned(),
                "ivar1=1".to_owned(),
                "fo=/专辑播放页".to_owned(),
            ]
        } else {
            vec![
                "type_id=4".to_owned(),
                "action=play".to_owned(),
                "fo3=3878,4320,4326,5747,5749,7229,7289,7863".to_owned(),
                format!(
                    "duration={}",
                    p.duration.as_ref().and_then(FlexI64::value).unwrap_or(0)
                ),
                "svar3=0".to_owned(),
                "svar2=0".to_owned(),
                "fo=我的音乐/主态/自建歌单/RU".to_owned(),
                "sty=手动".to_owned(),
                format!("state={state}"),
                format!("mixsongid={song}"),
                "source=46".to_owned(),
                "ivar3=0".to_owned(),
                "type=1".to_owned(),
                "fs=1.0".to_owned(),
                "ivar1=1".to_owned(),
            ]
        };
        let ss3 = { md5(&uuid::Uuid::new_v4().to_string()) };
        fields.extend([
            format!("mid={mid}"),
            format!("uuid={uuid}"),
            "ss1=1".to_owned(),
            "ss2=1".to_owned(),
            format!("sys={system_version}"),
            format!("mod={device_model}"),
            "channelid=18".to_owned(),
            format!("ip={local_ip}"),
            "net=1".to_owned(),
            format!("ver={CSCC_VERSION}"),
            "gitversion=7aa8a76".to_owned(),
            format!("time={}", crate::crypto::now_ms()),
            format!("userid={userid}"),
            format!("ss3={ss3}"),
        ]);
        let event_buffer = fields.join("&").into_bytes();

        let line1 = [
            "4",
            "1",
            &uuid,
            "0",
            "3116",
            CSCC_VERSION,
            "18",
            &system_version,
            &device_model,
            &uuid,
            &mid,
            "0",
            "000000000000000000000000000000000000",
        ]
        .join("\t");
        let line2 = [
            event_buffer.len().to_string(),
            "10048".to_owned(),
            "0".to_owned(),
            crate::catalog::ctx::now_secs_public().to_string(),
            "1".to_owned(),
            userid.clone(),
            "0".to_owned(),
            "0".to_owned(),
        ]
        .join("\t");
        let mut plain: Vec<u8> = format!("{line1}\r\n{line2}\r\n").into_bytes();
        plain.extend(deflate(&event_buffer));

        let key = md5(&format!(
            "{uuid}{}{}{}",
            session.clienttime, session.field2, session.serverstr
        ));
        let cipher =
            Aes256CbcEnc::new_from_slices(key.as_bytes(), &uuid.as_bytes()[..16.min(uuid.len())])
                .map_err(|e| Error::Crypto(e.to_string()))?;
        let encrypted = cipher.encrypt_padded_vec::<Pkcs7>(&deflate(&plain));

        let mut query = QueryRecord::new();
        query.set("cookie", session.cookie.clone());
        query.set("length", plain.len().to_string());
        query.set("appid", "3116");
        query.set("_t", crate::catalog::ctx::now_secs_public().to_string());
        let sorted: String = {
            let mut pairs: Vec<(String, String)> = query
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_wire_string()))
                .collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs.iter().fold(String::new(), |mut acc, (k, v)| {
                acc.push_str(k);
                acc.push_str(v);
                acc
            })
        };
        let sign = {
            let mut hasher = Md5::new();
            hasher.update(format!("{sorted}{CSCC_APPKEY}").as_bytes());
            hasher.update(&encrypted);
            hex::encode(hasher.finalize())
        };
        query.set("sign", sign);
        cscc_request(
            ctx,
            "/v2/post",
            Method::Post,
            query,
            Some(encrypted),
            "Post",
            &system_version,
        )
        .await?;

        if !sync {
            return Ok(json!({ "status": 1, "data": { "accepted": true } }));
        }

        // Grade sync.
        let mut grade = Value::Null;
        let mut grade_synced = false;
        let mut sync_params = serde_json::to_value(p)?;
        if let Some(obj) = sync_params.as_object_mut() {
            obj.insert("uuid".to_owned(), json!(uuid));
            obj.insert("mid".to_owned(), json!(mid));
            obj.insert("userid".to_owned(), json!(userid));
            obj.insert("token".to_owned(), json!(token));
        }
        if let Ok(result) = ctx.call_inner("user.grade.info", sync_params).await {
            grade = result;
            grade_synced = true;
        }
        Ok(json!({
            "status": 1,
            "data": {
                "report": { "accepted": true },
                "grade": grade,
                "grade_synced": grade_synced,
            },
        }))
    })
}

/// Upload a chat image through the BSS image bucket.
async fn upload_chat_image(
    ctx: &CallCtx<'_>,
    image: Vec<u8>,
    extendname: Option<&String>,
    is_original: bool,
) -> Result<String> {
    let extendname = extendname
        .filter(|e| !e.is_empty())
        .unwrap_or(&"jpg".to_owned())
        .trim_start_matches('.')
        .to_owned();

    let clienttime = crate::catalog::ctx::now_secs_public();
    let mut auth_query = QueryRecord::new();
    auth_query.set("bucket", "imagemsg");
    auth_query.set("filename", "");
    auth_query.set("method", "POST");
    auth_query.set("loginType", i64::from(!ctx.token.is_empty()));
    auth_query.set(
        "buVerifyCode",
        ctx.md5(&CipherInput::Text(format!(
            "{}imagemsgb8a37aa141c842f9",
            ctx.appid
        ))),
    );
    auth_query.set("extranet", 1);
    auth_query.set("userid", ctx.userid);
    auth_query.set("token", ctx.token.clone());
    auth_query.set("version", ctx.clientver);
    auth_query.set("dfid", ctx.dfid.clone());
    auth_query.set("mid", ctx.mid.clone());
    auth_query.set("uuid", ctx.uuid.clone());
    auth_query.set("appid", ctx.appid);
    auth_query.set("clientver", ctx.clientver);
    auth_query.set("clienttime", clienttime);
    auth_query.set(
        "signature",
        signature(
            &auth_query,
            SignOptions {
                data: Some(""),
                platform: ctx.platform,
                ..SignOptions::default()
            },
        ),
    );

    let auth_resp = ctx
        .raw(RawRequest {
            method: Some(Method::Get),
            url: "https://encounter.kugou.com/v1/upload/auth".to_owned(),
            query: Some(auth_query),
            headers: vec![
                (
                    "x-router".to_owned(),
                    "bsstrackercdngz.kugou.com".to_owned(),
                ),
                ("Host".to_owned(), "gateway.kugou.com".to_owned()),
            ],
            retries: Some(0),
            ..RawRequest::new("")
        })
        .await?;
    let auth_data = auth_resp.json().unwrap_or(Value::Null);
    let authorization = auth_data
        .pointer("/data/authorization")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if authorization.is_empty() {
        return Err(Error::Api {
            code: 0,
            message: auth_data.to_string(),
            payload: Some(auth_data),
        });
    }

    let mut upload_query = QueryRecord::new();
    upload_query.set("bucket", "imagemsg");
    upload_query.set("authorization", authorization.clone());
    upload_query.set("extendname", extendname);
    upload_query.set("userid", ctx.userid);
    upload_query.set("token", ctx.token.clone());
    upload_query.set("dfid", ctx.dfid.clone());
    upload_query.set("mid", ctx.mid.clone());
    upload_query.set("appid", ctx.appid);
    upload_query.set("clientver", ctx.clientver);
    upload_query.set("body_empty", 1);
    upload_query.set(
        "signature",
        signature(
            &upload_query,
            SignOptions {
                data: Some(""),
                platform: ctx.platform,
                ..SignOptions::default()
            },
        ),
    );

    let upload_resp = ctx
        .raw(RawRequest {
            method: Some(Method::Post),
            url: "https://bssul.service.kugou.com/v3/upload".to_owned(),
            query: Some(upload_query),
            headers: vec![
                ("Authorization".to_owned(), authorization),
                (
                    "Content-Type".to_owned(),
                    "application/octet-stream".to_owned(),
                ),
                (
                    "User-Agent".to_owned(),
                    format!(
                        "Android15-AndroidPhone-{}-201-0-UploadImageChatMsg-wifi",
                        ctx.clientver
                    ),
                ),
            ],
            body: Some(Body::Bytes(image)),
            retries: Some(0),
            ..RawRequest::new("")
        })
        .await?;
    let upload_data = upload_resp.json().unwrap_or(Value::Null);
    let bss_filename = upload_data
        .pointer("/data/x-bss-filename")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if upload_data.get("status").and_then(Value::as_i64) != Some(1) || bss_filename.is_empty() {
        return Err(Error::Api {
            code: 0,
            message: upload_data.to_string(),
            payload: Some(upload_data),
        });
    }
    let bucket_name = upload_data
        .pointer("/data/x-bss-bucket")
        .and_then(Value::as_str)
        .unwrap_or("");
    let host = if is_original {
        "bssdlbig.kugou.com"
    } else {
        "bssdl.kugou.com"
    };
    Ok(format!("http://{bucket_name}.{host}/{bss_filename}"))
}

fn user_follow_chat_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserFollowChatParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let tag = p.tag.clone().unwrap_or_default();
        let tuid = js::or_flex(&p.tuid, 0);
        if tag.is_empty() && tuid == 0 {
            return Err(Error::Api {
                code: 0,
                message: "tuid or tag is required".to_owned(),
                payload: None,
            });
        }
        let msgtype = js::or_flex(&p.msgtype, 201);

        let mut message = Map::new();
        message.insert("msgtype".to_owned(), json!(msgtype));
        let alert = p.alert.clone().unwrap_or_default();
        message.insert("alert".to_owned(), json!(alert));
        message.insert(
            "nickname".to_owned(),
            json!(p.nickname.clone().unwrap_or_default()),
        );
        message.insert("source".to_owned(), json!(js::or_flex(&p.source, 0)));
        message.insert(
            "follow_source".to_owned(),
            json!(js::or_flex(&p.follow_source, 0)),
        );
        message.insert("nt".to_owned(), json!(p.nt.clone().unwrap_or_default()));
        if let Some(location) = &p.location
            && !location.is_null()
        {
            message.insert("location".to_owned(), location.clone());
        }
        if let Some(groupid) = p.groupid.as_ref().and_then(FlexI64::value)
            && groupid != 0
        {
            message.insert("groupid".to_owned(), json!(groupid));
        }

        let is_original = p.is_original.unwrap_or(false) || p.isOriginal.unwrap_or(false);
        if msgtype == 202 {
            let alert = message
                .get("alert")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if alert.is_empty() {
                message.insert("alert".to_owned(), json!("[图片]"));
            }
            let mut url = p.url.clone().unwrap_or_default();
            if url.is_empty() {
                let image: Option<Vec<u8>> = if let Some(data) = &p.data {
                    BASE64.decode(data).ok()
                } else if let Some(img_file) = &p.imgFile {
                    normalize_base64(img_file)
                } else {
                    None
                };
                let Some(image) = image.filter(|i| !i.is_empty()) else {
                    return Err(Error::Api {
                        code: 0,
                        message: "image messages need a url, or image bytes via data / imgFile (base64/dataURL)".to_owned(),
                        payload: None,
                    });
                };
                url = upload_chat_image(ctx, image, p.extendname.as_ref(), is_original).await?;
            }
            message.insert("url".to_owned(), json!(url));
            if let Some(width) = p.width.as_ref().and_then(FlexI64::value)
                && width != 0
            {
                message.insert("width".to_owned(), json!(width));
            }
            if let Some(height) = p.height.as_ref().and_then(FlexI64::value)
                && height != 0
            {
                message.insert("height".to_owned(), json!(height));
            }
            message.insert("isOriginal".to_owned(), json!(is_original));
            if is_original {
                let original_size = p
                    .original_size
                    .as_ref()
                    .and_then(FlexI64::value)
                    .or_else(|| p.originalSize.as_ref().and_then(FlexI64::value))
                    .unwrap_or(0);
                if original_size != 0 {
                    message.insert("originalSize".to_owned(), json!(original_size));
                }
            }
        } else if msgtype == 205 {
            let alert = message
                .get("alert")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if alert.is_empty() {
                message.insert("alert".to_owned(), json!("[表情]"));
            }
            let url = p.url.clone().unwrap_or_default();
            let thumb_url = p.thumbUrl.clone().unwrap_or_default();
            if url.is_empty() || thumb_url.is_empty() {
                return Err(Error::Api {
                    code: 0,
                    message: "sticker messages need url and thumbUrl".to_owned(),
                    payload: None,
                });
            }
            message.insert("url".to_owned(), json!(url));
            message.insert("thumbUrl".to_owned(), json!(thumb_url));
        } else if msgtype == 201 {
            if message
                .get("alert")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                return Err(Error::Api {
                    code: 0,
                    message: "text messages need alert content".to_owned(),
                    payload: None,
                });
            }
        } else {
            if let Some(url) = p.url.clone().filter(|u| !u.is_empty()) {
                message.insert("url".to_owned(), json!(url));
            }
            if let Some(thumb_url) = p.thumbUrl.clone().filter(|u| !u.is_empty()) {
                message.insert("thumbUrl".to_owned(), json!(thumb_url));
            }
            if let Some(timelen) = p.timelen.as_ref().and_then(FlexI64::value)
                && timelen != 0
            {
                message.insert("timelen".to_owned(), json!(timelen));
            }
            let has_alert = message
                .get("alert")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty());
            let has_url = message.get("url").is_some();
            if !has_alert && !has_url {
                return Err(Error::Api {
                    code: 0,
                    message: format!("msgtype={msgtype} needs alert or url"),
                    payload: None,
                });
            }
        }
        if let Some(fakeid) = p.fakeid.as_ref().and_then(FlexI64::value)
            && fakeid != 0
        {
            message.insert("fakeid".to_owned(), json!(fakeid.to_string()));
        }

        let source_path = p
            .source_path
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| p.ingress.clone().filter(|s| !s.is_empty()))
            .unwrap_or_default();
        let mut body = Map::new();
        if tag.is_empty() {
            body.insert("tuid".to_owned(), json!(tuid));
        } else {
            body.insert("tag".to_owned(), json!(tag));
        }
        if !source_path.is_empty() {
            body.insert("ingress".to_owned(), json!(source_path));
        }
        if p.retry.unwrap_or(false) {
            body.insert("retry".to_owned(), json!(1));
        }
        body.insert("message".to_owned(), Value::Object(message));

        // The reference escapes "/" as "\/" before sending.
        let data = serde_json::to_string(&Value::Object(body))?.replace('/', "\\/");
        ctx.request(
            "/v1/chat/send",
            GatewayRequest {
                method: Some(Method::Post),
                router: Some("msg.mobile.kugou.com".to_owned()),
                headers: vec![
                    (
                        "User-Agent".to_owned(),
                        format!(
                            "Android15-AndroidPhone-{}-201-0-ChatSend-wifi",
                            ctx.clientver
                        ),
                    ),
                    (
                        "Content-Type".to_owned(),
                        "text/plain; charset=UTF-8".to_owned(),
                    ),
                ],
                query: Some(QueryRecord::new()),
                text: Some(data),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn user_update_avatar_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserUpdateAvatarParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let raw = p
            .imgFile
            .clone()
            .or_else(|| p.img.clone())
            .or_else(|| p.file.clone());
        let image = raw
            .as_deref()
            .and_then(normalize_base64)
            .filter(|i| !i.is_empty());
        let Some(image) = image else {
            return Err(Error::Api {
                code: 0,
                message: "invalid image data (imgFile must be base64 / dataURL of a JPEG/PNG/GIF/BMP/WEBP)".to_owned(),
                payload: None,
            });
        };
        if !is_image_buffer(&image) {
            return Err(Error::Api {
                code: 0,
                message: "image magic bytes not recognized (JPEG/PNG/GIF/BMP/WEBP)".to_owned(),
                payload: None,
            });
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let days = now / 86_400;
        let (year, month, day) = civil_from_days(days as i64);
        let ymd = format!("{year}{month:02}{day:02}");
        let md5_token = md5(&format!("{ymd}hewry678WEK23D"));

        // multipart upload to the imgphp host; the file field is fixed.
        let boundary = format!("----KuGouFormBoundary{}", random_hex7());
        let mut body: Vec<u8> = Vec::new();
        let push_field = |name: &str, value: &str, body: &mut Vec<u8>| {
            body.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                )
                .as_bytes(),
            );
        };
        push_field("type", "kugouicon", &mut body);
        push_field("md5", &md5_token, &mut body);
        let filename = p
            .filename
            .clone()
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| "avatar.jpg".to_owned());
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(&image);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let upload_resp = ctx
            .raw(RawRequest {
                method: Some(Method::Post),
                url: "http://imgphp.kugou.com/imageupload/post.php".to_owned(),
                headers: vec![(
                    "Content-Type".to_owned(),
                    format!("multipart/form-data; boundary={boundary}"),
                )],
                body: Some(Body::Bytes(body)),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        let upload_json = upload_resp.json().unwrap_or(Value::Null);
        let uploaded_name = upload_json
            .get("FileName")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if upload_json.get("IsSuccess").and_then(Value::as_bool) != Some(true)
            || uploaded_name.is_empty()
        {
            let message = upload_json
                .get("Message")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .cloned()
                .unwrap_or(json!("image host upload failed"));
            return Err(Error::Api {
                code: 0,
                message: message.to_string(),
                payload: Some(upload_json),
            });
        }

        let mut params = serde_json::to_value(p)?;
        if let Some(obj) = params.as_object_mut() {
            obj.insert("photo".to_owned(), json!(uploaded_name));
        }
        let result = ctx.call_inner("user.update", params).await?;
        let mut out = match result {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        out.insert("photo".to_owned(), json!(uploaded_name));
        out.insert(
            "pic".to_owned(),
            json!(format!("http://c1.kgimg.com/v2/kugouicon/{uploaded_name}")),
        );
        Ok(Value::Object(out))
    })
}

fn user_preference_handler<'a>(
    ctx: &'a CallCtx<'_>,
    _p: &'a crate::catalog::def::NoParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let clienttime = crate::catalog::ctx::now_secs_public();
        let str_g2 = {
            use rand::RngExt;
            let bytes: [u8; 8] = rand::rng().random();
            hex::encode(bytes)
        };
        let body = json!({
            "plat": 1,
            "userid": ctx.userid,
            "p": ctx.rsa_encrypt_hex(&CipherInput::Text(format!(
                "{{\"key\":\"{str_g2}\",\"clienttime\":\"{clienttime}\"}}"
            ))),
            "params": ctx.aes_hex(
                &CipherInput::Text(format!("{{\"token\":\"{}\"}}", ctx.token)),
                Some(&str_g2),
                None,
            )
            .str,
        });
        let mut query = QueryRecord::new();
        query.set("appid", ctx.appid);
        query.set("clientver", ctx.clientver);
        query.set("clienttime", clienttime);
        query.set("mid", ctx.mid.clone());
        query.set("dfid", ctx.dfid.clone());
        query.set("uuid", ctx.uuid.clone());
        ctx.request(
            "/userpreferservice/v1/get_user_conf",
            GatewayRequest {
                method: Some(Method::Post),
                sign: SignControl::None,
                clear_params: true,
                query: Some(query),
                json: Some(body),
                headers: vec![
                    ("Content-Type".to_owned(), "application/json".to_owned()),
                    (
                        "User-Agent".to_owned(),
                        format!("Android15-1070-{}-201-0-get_user_conf-wifi", ctx.clientver),
                    ),
                    ("KG-Rec".to_owned(), "1".to_owned()),
                    ("KG-RC".to_owned(), "1".to_owned()),
                ],
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn user_preference_update_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserPreferenceUpdateParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let clienttime = crate::catalog::ctx::now_secs_public();
        let fields: [(&str, &Option<Value>); 10] = [
            ("mode", &p.mode),
            ("gender", &p.gender),
            ("age", &p.age),
            ("lang", &p.lang),
            ("style", &p.style),
            ("song_lang", &p.song_lang),
            ("stylerec_taglist", &p.stylerec_taglist),
            ("wish_taglist", &p.wish_taglist),
            ("inactive_taglist", &p.inactive_taglist),
            ("inactive_singerlist", &p.inactive_singerlist),
        ];
        let mut data = serde_json::Map::new();
        for (key, value) in fields {
            if let Some(value) = value {
                if value.is_null() {
                    continue;
                }
                let rendered = if value.is_object() || value.is_array() {
                    serde_json::to_string(value)?
                } else {
                    match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    }
                };
                data.insert(key.to_owned(), json!(rendered));
            }
        }
        if data.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "at least one preference field is required (gender/age/lang/style …)"
                    .to_owned(),
                payload: None,
            });
        }

        let str_g2 = {
            use rand::RngExt;
            let bytes: [u8; 8] = rand::rng().random();
            hex::encode(bytes)
        };
        let body = json!({
            "plat": 1,
            "userid": ctx.userid,
            "p": ctx.rsa_encrypt_hex(&CipherInput::Text(format!(
                "{{\"key\":\"{str_g2}\",\"clienttime\":\"{clienttime}\"}}"
            ))),
            "data": data,
            "params": ctx.aes_hex(
                &CipherInput::Text(format!("{{\"token\":\"{}\"}}", ctx.token)),
                Some(&str_g2),
                None,
            )
            .str,
        });
        let mut query = QueryRecord::new();
        query.set("appid", ctx.appid);
        query.set("clientver", ctx.clientver);
        query.set("clienttime", clienttime);
        query.set("mid", ctx.mid.clone());
        query.set("dfid", ctx.dfid.clone());
        query.set("uuid", ctx.uuid.clone());
        ctx.request(
            "/userpreferservice/v1/update_user_conf",
            GatewayRequest {
                method: Some(Method::Post),
                sign: SignControl::None,
                clear_params: true,
                query: Some(query),
                json: Some(body),
                headers: vec![
                    ("Content-Type".to_owned(), "application/json".to_owned()),
                    (
                        "User-Agent".to_owned(),
                        format!(
                            "Android15-1070-{}-201-0-update_user_conf-wifi",
                            ctx.clientver
                        ),
                    ),
                    ("KG-Rec".to_owned(), "1".to_owned()),
                    ("KG-RC".to_owned(), "1".to_owned()),
                ],
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn user_visitors_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserVisitorsParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let appid_used = p
            .appid
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|a| *a != 0)
            .unwrap_or(ctx.appid as i64) as u64;
        let clientver_used = p
            .clientver
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|c| *c != 0)
            .unwrap_or(if appid_used == 3116 { 10672 } else { 20809 });
        let t_userid = p
            .t_userid
            .as_ref()
            .or(p.visit_userid.as_ref())
            .and_then(FlexI64::value)
            .filter(|u| *u != 0)
            .unwrap_or(ctx.userid as i64) as u64;
        let clienttime = crate::catalog::ctx::now_secs_public();

        let p_plain = format!(
            "{{\"clienttime\":{clienttime},\"token\":\"{}\",\"t_userid\":{t_userid}}}",
            ctx.token
        );
        let mut padded = vec![0u8; 128];
        let plain_bytes = p_plain.as_bytes();
        if plain_bytes.len() > 128 {
            return Err(Error::Crypto("visitors p payload too long".to_owned()));
        }
        padded[..plain_bytes.len()].copy_from_slice(plain_bytes);
        use rsa::pkcs8::DecodePublicKey;
        let public_key = rsa::PublicKey::from_public_key_pem(crate::crypto::LITE_RSA_PUBLIC_KEY)
            .map_err(|e| Error::Crypto(e.to_string()))?;
        let p_param =
            hex::encode(crate::crypto::raw_modpow_public(&public_key, &padded)?).to_uppercase();

        let mut data = Map::new();
        data.insert("userid".to_owned(), json!(ctx.userid));
        data.insert("t_userid".to_owned(), json!(t_userid));
        data.insert("p".to_owned(), json!(p_param));
        data.insert("page".to_owned(), json!(js::or_flex(&p.page, 1)));
        if let Some(busi_type) = p.busi_type.as_ref().and_then(FlexI64::value)
            && busi_type != 0
        {
            data.insert("busi_type".to_owned(), json!(busi_type));
        }

        let mut query = QueryRecord::new();
        query.set("dfid", ctx.dfid.clone());
        query.set("plat", "1");
        query.set("clienttime", clienttime.to_string());
        query.set("mid", ctx.mid.clone());
        query.set("uuid", ctx.uuid.clone());
        query.set("clientver", clientver_used.to_string());
        query.set("appid", appid_used.to_string());
        let salt = if appid_used == 3116 {
            "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA"
        } else {
            "OIlwieks28dk2k092lksi2UIkp"
        };
        let sorted: String = {
            let mut pairs: Vec<(String, String)> = query
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_wire_string()))
                .collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs.iter().fold(String::new(), |mut acc, (k, v)| {
                acc.push_str(k);
                acc.push('=');
                acc.push_str(v);
                acc
            })
        };
        let data_str = serde_json::to_string(&Value::Object(data.clone()))?;
        query.set("signature", md5(&format!("{salt}{sorted}{data_str}{salt}")));

        let mut headers = vec![
            (
                "Content-Type".to_owned(),
                "text/plain; charset=ISO-8859-1".to_owned(),
            ),
            (
                "User-Agent".to_owned(),
                format!("Android15-1070-{clientver_used}-201-0-User-wifi"),
            ),
            ("KG-RC".to_owned(), "1".to_owned()),
            ("KG-Rec".to_owned(), "1".to_owned()),
            ("KG-THash".to_owned(), "3980f3c".to_owned()),
        ];
        if let Some(kg_devid) = p.kg_devid.as_ref().and_then(FlexI64::value)
            && kg_devid != 0
        {
            headers.push(("KG-DEVID".to_owned(), kg_devid.to_string()));
        }
        if let Some(kg_clienttimems) = p.kg_clienttimems.as_ref().and_then(FlexI64::value)
            && kg_clienttimems != 0
        {
            headers.push(("KG-CLIENTTIMEMS".to_owned(), kg_clienttimems.to_string()));
        }

        ctx.request(
            "/v2/get_visitors",
            GatewayRequest {
                method: Some(Method::Post),
                base_url: Some("http://usercenter.kugou.com".to_owned()),
                sign: SignControl::None,
                clear_params: true,
                query: Some(query),
                json: Some(Value::Object(data)),
                headers,
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn blacklist_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a BlacklistParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let label = if p.label.as_deref() == Some("singer") {
            "singer"
        } else {
            "song"
        };
        let clienttime = crate::catalog::ctx::now_secs_public();
        let timestamp = clienttime.to_string();

        let mut items = p.items.clone();
        if let Some(Value::String(text)) = &items {
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                items = Some(parsed);
            } else {
                items = None;
            }
        }
        let items = match items {
            Some(Value::Array(list)) if !list.is_empty() => list,
            _ => {
                if label == "song" {
                    vec![json!({
                        "k": p.hash.clone().unwrap_or_default().to_lowercase(),
                        "v": serde_json::to_string(&json!({
                            "n": p.name.clone().unwrap_or_default(),
                            "m": p.mixsongid.as_ref().and_then(FlexI64::value).map_or_default(|v| v.to_string()),
                            "t": timestamp,
                        }))?,
                    })]
                } else {
                    vec![json!({
                        "k": p.singerid.as_ref().and_then(FlexI64::value).unwrap_or(0).to_string(),
                        "v": serde_json::to_string(&json!({
                            "n": p.name.clone().unwrap_or_default(),
                            "t": timestamp,
                        }))?,
                    })]
                }
            }
        };

        let body = json!({
            "userid": ctx.userid,
            "source": js::or_chain(
                &[p.source.as_ref().and_then(FlexI64::value)],
                if label == "song" { 3 } else { 4 },
            ),
            "label": label,
            "items": items,
            "action": if p.isDelete.unwrap_or(false) { "delete" } else { "add" },
            "p": ctx.rsa_encrypt_raw(&CipherInput::Text(
                serde_json::to_string(&json!({
                    "clienttime": clienttime,
                    "token": ctx.token,
                }))?,
            )),
        });
        ctx.request(
            "/v1/edit_list_items",
            GatewayRequest {
                method: Some(Method::Post),
                base_url: Some("https://relationuser.kugou.com".to_owned()),
                json: Some(body),
                headers: vec![(
                    "KG-TID".to_owned(),
                    js::or_flex(&p.moduleId, 473).to_string(),
                )],
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

/// Days-since-epoch → (year, month, day), UTC.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

impl crate::api::UserApi<'_> {
    /// Grade info / grade sync report (`user.grade.info`).
    pub async fn grade_info(&self, params: UserGradeInfoParams) -> Result<Value> {
        self.client.call_value("user.grade.info", params).await
    }

    /// CSCC listen reporting (`user.listen.report`).
    pub async fn listen_report(&self, params: UserListenReportParams) -> Result<Value> {
        self.client.call_value("user.listen.report", params).await
    }

    /// Send a private message (`user.follow.chat`).
    pub async fn follow_chat(&self, params: UserFollowChatParams) -> Result<Value> {
        self.client.call_value("user.follow.chat", params).await
    }

    /// Upload an avatar (`user.update.avatar`).
    pub async fn update_avatar(&self, params: UserUpdateAvatarParams) -> Result<Value> {
        self.client.call_value("user.update.avatar", params).await
    }

    /// Read user preferences (`user.preference`).
    pub async fn preference(&self) -> Result<Value> {
        self.client
            .call_value("user.preference", crate::catalog::def::NoParams::default())
            .await
    }

    /// Update user preferences (`user.preference.update`).
    pub async fn preference_update(&self, params: UserPreferenceUpdateParams) -> Result<Value> {
        self.client
            .call_value("user.preference.update", params)
            .await
    }

    /// Visitors list (`user.visitors`).
    pub async fn visitors(&self, params: UserVisitorsParams) -> Result<Value> {
        self.client.call_value("user.visitors", params).await
    }

    /// Edit the blacklist (`blacklist`).
    pub async fn blacklist(&self, params: BlacklistParams) -> Result<Value> {
        self.client.call_value("blacklist", params).await
    }

    /// Batch VIP info (`user.batch.union.vipinfo`).
    pub async fn batch_union_vipinfo(&self, params: UserBatchUnionVipinfoParams) -> Result<Value> {
        self.client
            .call_value("user.batch.union.vipinfo", params)
            .await
    }
}
