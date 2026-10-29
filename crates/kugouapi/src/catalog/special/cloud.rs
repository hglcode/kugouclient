//! Encrypted-POST family (collect / playlist delete / cloud) and the
//! cloudlist protocol.
//!
//! Two protocol families:
//!
//! - **encrypted POST**: session-cipher body, `p` = RSA({aes,uid,token}),
//!   `key` = signParamsKey(clienttime); the response is a ciphertext under the
//!   same key.
//! - **cloudlist**: `p` = RSA({aes,uid,token}), `key` =
//!   md5(appid+salt+clientver+clienttime); request and response bodies are
//!   AES-CBC with key/iv derived from md5(aes).
//!
//! Ported from `special/cloud.ts`. Binary parameters (`data`) are passed as
//! base64 strings in the dynamic JSON surface.

use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use md5::{Digest, Md5};
use serde_json::{Value, json};

use crate::catalog::ctx::{GatewayRequest, RawRequest, SignControl};
use crate::catalog::def::{Built, EndpointSpec, Salt, handler_endpoint, simple_endpoint, spec};
use crate::crypto::{CipherInput, CipherOutput, random_string, random_thash};
use crate::error::{Error, Result};
use crate::http::{Body, Method};
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;
use crate::sign::{SignOptions, cloud_key, md5, signature};
use crate::util::split_list;

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

// ─────────────────────────────────────────────────────────────
// Parameters
// ─────────────────────────────────────────────────────────────

kugou_params! {
    /// Parameters for `mv.collect` / `mv.collect.del`.
    pub MvCollectParams {
        required {
            /// MV id.
            id: FlexI64 => "id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `playlist.del`.
    pub PlaylistDelParams {
        required {
            /// List id.
            listid: FlexI64 => "listid",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `playlist.add`.
    pub PlaylistAddParams {
        required {
            /// New playlist name.
            name: String => "name",
        }
        optional {
            /// Playlist type (0 = own).
            list_type: FlexI64 => "type",
            /// Creation source.
            source: FlexI64 => "source",
            /// Private flag (type 0).
            is_pri: FlexI64 => "is_pri",
            /// Creator user id (shared lists).
            list_create_userid: FlexI64 => "list_create_userid",
            /// Creator list id (shared lists).
            list_create_listid: FlexI64 => "list_create_listid",
            /// Creator gid (shared lists).
            list_create_gid: String => "list_create_gid",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.tracks.add`.
    pub PlaylistTracksAddParams {
        required {
            /// Target list id.
            listid: FlexI64 => "listid",
        }
        optional {
            /// `name|hash|album_id|mixsongid` rows, comma separated.
            data: String => "data",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.pic`.
    pub PlaylistPicParams {
        required {
            /// `listid|type|pic` rows, comma separated.
            data: String => "data",
        }
        optional {
            /// Total version.
            total_ver: FlexI64 => "total_ver",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.sort`.
    pub PlaylistSortParams {
        required {
            /// `listid|type|sort` rows, comma separated.
            data: String => "data",
        }
        optional {
            /// Total version.
            total_ver: FlexI64 => "total_ver",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.tracks.sort`.
    pub PlaylistTracksSortParams {
        required {
            /// Target list id.
            listid: FlexI64 => "listid",
        }
        optional {
            /// `fileid|sort` rows, comma separated.
            data: String => "data",
            /// List version.
            list_ver: FlexI64 => "list_ver",
            /// List type.
            list_type: FlexI64 => "type",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.update`.
    pub PlaylistUpdateParams {
        required {
            /// Target list id.
            listid: FlexI64 => "listid",
        }
        optional {
            /// Total version.
            total_ver: FlexI64 => "total_ver",
            /// List type.
            list_type: FlexI64 => "type",
            /// Sort.
            sort: FlexI64 => "sort",
            /// Tags.
            tags: String => "tags",
            /// Introduction.
            intro: String => "intro",
            /// New name.
            name: String => "name",
            /// New cover (`stdmusic/…`).
            pic: String => "pic",
        }
    }
}

kugou_params! {
    /// Parameters for `user.cloud`.
    pub UserCloudParams {
        required {}
        optional {
            /// Page number (default 1).
            page: FlexI64 => "page",
            /// Page size (default 30).
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `user.cloud.del`.
    pub UserCloudDelParams {
        required {
            /// Comma-separated file ids (or `kv_ids`).
            fileids: String => "fileids",
        }
        optional {
            /// Comma-separated album audio ids.
            album_audio_ids: String => "album_audio_ids",
            /// Fallback mixsong id.
            mixid: FlexI64 => "mixid",
        }
    }
}

/// Parameters for `user.cloud.match` (binary `data` accepted as base64).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserCloudMatchParams {
    /// Comma-separated hashes (or file names).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Comma-separated file names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Comma-separated album audio ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_audio_ids: Option<String>,
    /// Fallback album audio id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_audio_id: Option<FlexI64>,
    /// Fallback mixsong id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixid: Option<FlexI64>,
    /// File bytes as base64 (hashed to md5 when no hash is given).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

kugou_params! {
    /// Parameters for `user.cloud.url`.
    pub UserCloudUrlParams {
        required {
            /// File hash.
            hash: String => "hash",
        }
        optional {
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
            /// Audio id.
            audio_id: FlexI64 => "audio_id",
            /// File name.
            name: String => "name",
        }
    }
}

/// Parameters for `user.cloud.upload` (binary `data` as base64).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserCloudUploadParams {
    /// File bytes as base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// File name / local hash (defaults to the file's md5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Extension (default `mp3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extendname: Option<String>,
    /// Disable the auto metadata match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_match: Option<FlexI64>,
    /// Known standard hash (skips matching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash_std: Option<String>,
    /// Known audio id (skips matching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_id: Option<FlexI64>,
    /// Known album audio id (skips matching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_audio_id: Option<FlexI64>,
    /// Fallback mixsong id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixid: Option<FlexI64>,
    /// Author name override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    /// Track name override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_name: Option<String>,
    /// Track name override (alternate key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub songname: Option<String>,
    /// Display name override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Bitrate hint (default 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<FlexI64>,
    /// Duration in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timelen: Option<FlexI64>,
    /// List version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_ver: Option<FlexI64>,
}

kugou_params! {
    /// Parameters for `playlist.pic.upload` (binary `data` as base64).
    pub PlaylistPicUploadParams {
        required {
            /// Image bytes as base64.
            data: String => "data",
        }
        optional {
            /// Upload type (default `custom`).
            upload_type: String => "type",
            /// Extension (default `.jpg`).
            extend_name: String => "extendName",
            /// Md5 override.
            md5: String => "md5",
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Specs
// ─────────────────────────────────────────────────────────────

macro_rules! enc_spec {
    ($id:literal, $module:literal, $method:expr, $path:literal, $origin:literal) => {
        EndpointSpec {
            origin: Some($origin),
            salt: Salt::None,
            auth: true,
            ..spec($id, $module, $method, $path)
        }
    };
}

const EP_MV_COLLECT: EndpointSpec = enc_spec!(
    "mv.collect",
    "mv_collect",
    Method::Post,
    "/v1/collect",
    "https://collectservice.kugou.com"
);
const EP_MV_COLLECT_DEL: EndpointSpec = enc_spec!(
    "mv.collect.del",
    "mv_collect_del",
    Method::Post,
    "/v1/cancel_collect",
    "https://collectservice.kugou.com"
);
const EP_PLAYLIST_DEL: EndpointSpec = EndpointSpec {
    router: Some("cloudlist.service.kugou.com"),
    salt: Salt::None,
    auth: true,
    ..spec(
        "playlist.del",
        "playlist_del",
        Method::Post,
        "/v2/delete_list",
    )
};
const EP_PLAYLIST_ADD: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "playlist.add",
        "playlist_add",
        Method::Post,
        "/cloudlist.service/v5/add_list",
    )
};
const EP_PLAYLIST_TRACKS_ADD: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "playlist.tracks.add",
        "playlist_tracks_add",
        Method::Post,
        "/cloudlist.service/v6/add_song",
    )
};
const EP_PLAYLIST_PIC: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "playlist.pic",
        "playlist_pic",
        Method::Post,
        "/v1/modify_list_pic",
    )
};
const EP_PLAYLIST_SORT: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "playlist.sort",
        "playlist_sort",
        Method::Post,
        "/v1/modify_list_sort",
    )
};
const EP_PLAYLIST_TRACKS_SORT: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "playlist.tracks.sort",
        "playlist_tracks_sort",
        Method::Post,
        "/v1/modify_song_sort",
    )
};
const EP_PLAYLIST_UPDATE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "playlist.update",
        "playlist_update",
        Method::Post,
        "/v1/modify_list",
    )
};
const EP_USER_CLOUD: EndpointSpec = enc_spec!(
    "user.cloud",
    "user_cloud",
    Method::Post,
    "/v1/get_list",
    "https://mcloudservice.kugou.com"
);
const EP_USER_CLOUD_DEL: EndpointSpec = enc_spec!(
    "user.cloud.del",
    "user_cloud_del",
    Method::Post,
    "/v1/del_files",
    "https://mcloudservice.kugou.com"
);
const EP_USER_CLOUD_MATCH: EndpointSpec = EndpointSpec {
    origin: Some("http://kmr.service.kugou.com"),
    router: Some("kmr.service.kugou.com"),
    salt: Salt::None,
    ..spec(
        "user.cloud.match",
        "user_cloud_match",
        Method::Post,
        "/v2/album_audio/audio",
    )
};
const EP_USER_CLOUD_URL: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "user.cloud.url",
        "user_cloud_url",
        Method::Get,
        "/bsstrackercdngz/v2/query_musicclound_url",
    )
};
const EP_USER_CLOUD_UPLOAD: EndpointSpec = enc_spec!(
    "user.cloud.upload",
    "user_cloud_upload",
    Method::Post,
    "/v1/add_files",
    "https://mcloudservice.kugou.com"
);
const EP_PLAYLIST_PIC_UPLOAD: EndpointSpec = EndpointSpec {
    origin: Some("http://imgphp.kugou.com/imageupload"),
    salt: Salt::None,
    ..spec(
        "playlist.pic.upload",
        "playlist_pic_upload",
        Method::Post,
        "/stream.php",
    )
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        handler_endpoint(&EP_MV_COLLECT, mv_collect_handler),
        handler_endpoint(&EP_MV_COLLECT_DEL, mv_collect_del_handler),
        handler_endpoint(&EP_PLAYLIST_DEL, playlist_del_handler),
        handler_endpoint(&EP_PLAYLIST_ADD, playlist_add_handler),
        handler_endpoint(&EP_PLAYLIST_TRACKS_ADD, playlist_tracks_add_handler),
        handler_endpoint(&EP_PLAYLIST_PIC, playlist_pic_handler),
        handler_endpoint(&EP_PLAYLIST_SORT, playlist_sort_handler),
        handler_endpoint(&EP_PLAYLIST_TRACKS_SORT, playlist_tracks_sort_handler),
        handler_endpoint(&EP_PLAYLIST_UPDATE, playlist_update_handler),
        handler_endpoint(&EP_USER_CLOUD, user_cloud_handler),
        handler_endpoint(&EP_USER_CLOUD_DEL, user_cloud_del_handler),
        handler_endpoint(&EP_USER_CLOUD_MATCH, user_cloud_match_handler),
        simple_endpoint(&EP_USER_CLOUD_URL, |p: &UserCloudUrlParams, _| {
            let hash = p.hash.to_lowercase();
            let mut query = QueryRecord::new();
            query.set("hash", hash.clone());
            query.set("ssa_flag", "is_fromtrack");
            query.set("version", "20102");
            query.set("ssl", 0);
            query.set("album_audio_id", js::unwrap_or_flex(&p.album_audio_id, 0));
            query.set("pid", 20026);
            query.set("audio_id", js::unwrap_or_flex(&p.audio_id, 0));
            query.set("kv_id", 2);
            query.set("key", cloud_key(&hash, 20026));
            query.set("bucket", "musicclound");
            query.set("name", p.name.clone().unwrap_or_default());
            query.set("with_res_tag", 0);
            Built {
                query,
                ..Built::default()
            }
        }),
        handler_endpoint(&EP_USER_CLOUD_UPLOAD, user_cloud_upload_handler),
        handler_endpoint(&EP_PLAYLIST_PIC_UPLOAD, playlist_pic_upload_handler),
    ]
}

// ─────────────────────────────────────────────────────────────
// Shared pipelines
// ─────────────────────────────────────────────────────────────

/// The encrypted-POST pipeline (session cipher body + RSA `p` + params key).
async fn encrypted_post(
    ctx: &CallCtx<'_>,
    url: &str,
    origin: Option<&str>,
    data: Value,
    extra_query: Option<QueryRecord>,
    headers: Vec<(String, String)>,
    no_mid: bool,
) -> Result<Value> {
    let encrypted = ctx.session_cipher(&CipherInput::Json(data));
    let p = ctx
        .rsa_encrypt_hex(&CipherInput::Json(json!({
            "aes": encrypted.key,
            "uid": ctx.userid,
            "token": ctx.token,
        })))
        .to_uppercase();

    let mut query = QueryRecord::new();
    query.set("clienttime", ctx.clienttime);
    if !no_mid {
        query.set("mid", ctx.mid.clone());
    }
    query.set("key", ctx.sign_params_key(ctx.clienttime.to_string()));
    query.set("clientver", ctx.clientver);
    query.set("appid", ctx.appid);
    query.set("p", p);
    if let Some(extra) = extra_query {
        query.merge(&extra);
    }

    let mut request_headers = vec![("Content-Type".to_owned(), "application/json".to_owned())];
    request_headers.extend(headers);
    let result = ctx
        .raw(RawRequest {
            method: Some(Method::Post),
            url: format!("{}{url}", origin.unwrap_or("https://gateway.kugou.com")),
            query: Some(query),
            headers: request_headers,
            body: Some(Body::Bytes(
                BASE64
                    .decode(encrypted.body)
                    .map_err(|e| Error::Crypto(e.to_string()))?,
            )),
            retries: Some(0),
            ..RawRequest::new("")
        })
        .await?;

    if !result.ok() {
        return Err(Error::Http {
            status: result.status,
            context: format!("POST {url}"),
            payload: result.json(),
        });
    }
    let payload = BASE64.encode(&result.bytes);
    match ctx.session_cipher_decrypt(&payload, &encrypted.key) {
        Ok(output) => Ok(output.into_value()),
        Err(_) => Ok(result.json().unwrap_or(Value::Null)),
    }
}

/// The cloudlist pipeline (aligned with the reference `createCloudRequest`).
async fn cloud_request(ctx: &CallCtx<'_>, url: &str, data: Value) -> Result<Value> {
    let (appid, clientver, appkey) = if ctx.platform == crate::config::Platform::Lite {
        (3116, 11440, "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA")
    } else {
        (1005, 20489, "OIlwieks28dk2k092lksi2UIkp")
    };
    let clienttime = crate::catalog::ctx::now_secs_public();
    let aes = random_string(6).to_lowercase();
    let digest = md5(&aes);
    let aes_key = digest[0..16].to_owned();
    let aes_iv = digest[16..32].to_owned();
    let key = md5(&format!("{appid}{appkey}{clientver}{clienttime}")).to_lowercase();
    let p = ctx
        .rsa_encrypt_hex(&CipherInput::Json(json!({
            "aes": aes,
            "uid": ctx.userid,
            "token": ctx.token,
        })))
        .to_uppercase();

    let body_bytes = serde_json::to_vec(&data)?;
    let cipher = Aes128CbcEnc::new_from_slices(aes_key.as_bytes(), aes_iv.as_bytes())
        .map_err(|e| Error::Crypto(e.to_string()))?;
    let encrypted = cipher.encrypt_padded_vec::<Pkcs7>(&body_bytes);

    let mut query = QueryRecord::new();
    query.set("appid", appid);
    query.set("clientver", clientver);
    query.set("mid", ctx.mid.clone());
    query.set("clienttime", clienttime);
    query.set("key", key);
    query.set("dfid", ctx.dfid.clone());
    query.set("p", p);

    let result = ctx
        .raw(RawRequest {
            method: Some(Method::Post),
            url: format!("https://gateway.kugou.com{url}"),
            query: Some(query),
            headers: vec![
                (
                    "Content-Type".to_owned(),
                    "application/json;charset=utf-8".to_owned(),
                ),
                (
                    "x-router".to_owned(),
                    "cloudlist.service.kugou.com".to_owned(),
                ),
                ("dfid".to_owned(), ctx.dfid.clone()),
                ("clienttime".to_owned(), clienttime.to_string()),
                ("mid".to_owned(), ctx.mid.clone()),
                ("kg-rc".to_owned(), "1".to_owned()),
                ("kg-thash".to_owned(), "5d816a0".to_owned()),
                ("kg-rec".to_owned(), "1".to_owned()),
            ],
            body: Some(Body::Bytes(encrypted)),
            retries: Some(0),
            ..RawRequest::new("")
        })
        .await?;

    if !result.ok() {
        return Err(Error::Http {
            status: result.status,
            context: format!("POST {url}"),
            payload: result.json(),
        });
    }

    let mut text = String::new();
    let decipher = Aes128CbcDec::new_from_slices(aes_key.as_bytes(), aes_iv.as_bytes())
        .map_err(|e| Error::Crypto(e.to_string()))?;
    if let Ok(plain) = decipher.decrypt_padded_vec::<Pkcs7>(&result.bytes) {
        text = String::from_utf8_lossy(&plain).into_owned();
    }
    if text.is_empty() {
        text = result.text();
    }
    serde_json::from_str(&text).map_err(|_| Error::InvalidResponse {
        context: format!("cloudlist {url}"),
        body: Some(text.chars().take(200).collect()),
    })
}

/// `"a|b,c|d"` → object rows (the cloudlist input format).
fn parse_pipe_list(value: &str, fields: &[&str]) -> Vec<serde_json::Map<String, Value>> {
    let value_json = Value::String(value.to_owned());
    split_list(&value_json)
        .iter()
        .map(|item| {
            let parts: Vec<&str> = item.split('|').collect();
            let mut row = serde_json::Map::new();
            for (i, field) in fields.iter().enumerate() {
                row.insert(
                    (*field).to_owned(),
                    Value::String(parts.get(i).copied().unwrap_or("").to_owned()),
                );
            }
            row
        })
        .collect()
}

fn file_md5(data: &[u8]) -> String {
    hex::encode(Md5::digest(data))
}

fn with_protocol(host: &str) -> String {
    if host.starts_with("http://") || host.starts_with("https://") {
        host.to_owned()
    } else {
        format!("http://{host}")
    }
}

/// A BSS upload request (android signature + common client headers).
async fn bss_request(
    ctx: &CallCtx<'_>,
    url: &str,
    method: Method,
    query: QueryRecord,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
) -> Result<Value> {
    let mut query = query;
    query.set(
        "signature",
        signature(
            &query,
            SignOptions {
                data: Some(""),
                ..SignOptions::default()
            },
        ),
    );
    let mut request_headers = vec![
        (
            "User-Agent".to_owned(),
            format!("Android15-1070-{}-201-0-wifi", ctx.clientver),
        ),
        ("KG-RC".to_owned(), "1".to_owned()),
        ("KG-Rec".to_owned(), "1".to_owned()),
        ("KG-THash".to_owned(), random_thash()),
    ];
    request_headers.extend(headers);
    let result = ctx
        .raw(RawRequest {
            method: Some(method),
            url: url.to_owned(),
            query: Some(query),
            headers: request_headers,
            body: body.map(Body::Bytes),
            retries: Some(0),
            ..RawRequest::new("")
        })
        .await?;
    Ok(result.json().unwrap_or(Value::Null))
}

/// Normalized match info for `user.cloud.match` / `user.cloud.upload`.
fn normalize_match(candidate: &Value) -> Option<Value> {
    if !candidate.is_object() {
        return None;
    }
    let audio_info = candidate.get("audio_info").cloned().unwrap_or(Value::Null);
    let album_audio_id = candidate
        .get("album_audio_id")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let audio_id = audio_info
        .get("audio_id")
        .and_then(Value::as_u64)
        .or_else(|| candidate.get("audio_id").and_then(Value::as_u64))
        .unwrap_or(0);
    let hash_std = audio_info
        .get("hash")
        .and_then(Value::as_str)
        .or_else(|| candidate.get("hash").and_then(Value::as_str))
        .unwrap_or("")
        .to_owned();
    if album_audio_id == 0 && audio_id == 0 && hash_std.is_empty() {
        return None;
    }
    Some(json!({
        "album_audio_id": album_audio_id,
        "audio_id": audio_id,
        "hash_std": hash_std,
        "hash": hash_std,
        "author_name": candidate.get("author_name").and_then(Value::as_str).unwrap_or(""),
        "audio_name": candidate
            .get("ori_audio_name").and_then(Value::as_str)
            .or_else(|| candidate.get("audio_name").and_then(Value::as_str))
            .or_else(|| candidate.get("songname").and_then(Value::as_str))
            .unwrap_or(""),
        "suffix_audio_name": candidate.get("suffix_audio_name").and_then(Value::as_str).unwrap_or(""),
    }))
}

// ─────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────

fn mv_collect_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a MvCollectParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        encrypted_post(
            ctx,
            "/v1/collect",
            Some("https://collectservice.kugou.com"),
            json!({ "ctype": 2, "data": [{ "obj_id": p.id.value() }] }),
            None,
            vec![
                (
                    "User-Agent".to_owned(),
                    "Android9-1070-18-0-MV/Care-wifi".to_owned(),
                ),
                ("KG-THash".to_owned(), random_thash()),
            ],
            false,
        )
        .await
    })
}

fn mv_collect_del_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a MvCollectParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        encrypted_post(
            ctx,
            "/v1/cancel_collect",
            Some("https://collectservice.kugou.com"),
            json!({ "ctype": 2, "data": [{ "obj_id": p.id.value() }] }),
            None,
            vec![
                (
                    "User-Agent".to_owned(),
                    "Android9-1070-18-0-MV/UnCare-wifi".to_owned(),
                ),
                ("KG-THash".to_owned(), random_thash()),
            ],
            false,
        )
        .await
    })
}

fn playlist_del_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistDelParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut extra = QueryRecord::new();
        extra.set("last_area", "gztx");
        extra.set("last_time", ctx.clienttime);
        encrypted_post(
            ctx,
            "/v2/delete_list",
            None,
            json!({
                "listid": p.listid.value(),
                "total_ver": 0,
                "type": 1,
            }),
            Some(extra),
            Vec::new(),
            false,
        )
        .await
    })
}

fn playlist_add_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistAddParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let list_type = js::or_flex(&p.list_type, 0);
        let body = json!({
            "userid": ctx.userid,
            "token": ctx.token,
            "total_ver": 0,
            "name": p.name,
            "type": list_type,
            "source": match p.source.as_ref().and_then(FlexI64::value) {
                Some(0) => 0,
                Some(v) => v,
                None => 1,
            },
            "is_pri": if list_type == 0 { js::or_flex(&p.is_pri, 0) } else { 0 },
            "list_create_userid": p.list_create_userid.as_ref().and_then(FlexI64::value),
            "list_create_listid": p.list_create_listid.as_ref().and_then(FlexI64::value),
            "list_create_gid": p.list_create_gid.clone().unwrap_or_default(),
            "from_shupinmv": 0,
        });
        let query = if list_type == 0 {
            let mut q = QueryRecord::new();
            q.set("last_time", ctx.clienttime);
            q.set("last_area", "gztx");
            q.set("userid", ctx.userid);
            q.set("token", ctx.token.clone());
            q
        } else {
            QueryRecord::new()
        };
        ctx.request(
            "/cloudlist.service/v5/add_list",
            GatewayRequest {
                method: Some(Method::Post),
                json: Some(body),
                query: Some(query),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn playlist_tracks_add_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistTracksAddParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let resource: Vec<Value> = p
            .data
            .clone()
            .unwrap_or_default()
            .split(',')
            .map(|s| {
                let parts: Vec<&str> = s.split('|').collect();
                json!({
                    "number": 1,
                    "name": parts.first().copied().unwrap_or(""),
                    "hash": parts.get(1).copied().unwrap_or(""),
                    "size": 0,
                    "sort": 0,
                    "timelen": 0,
                    "bitrate": 0,
                    "album_id": parts.get(2).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                    "mixsongid": parts.get(3).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                })
            })
            .collect();
        let mut query = QueryRecord::new();
        query.set("last_time", ctx.clienttime);
        query.set("last_area", "gztx");
        query.set("userid", ctx.userid);
        query.set("token", ctx.token.clone());
        ctx.request(
            "/cloudlist.service/v6/add_song",
            GatewayRequest {
                method: Some(Method::Post),
                json: Some(json!({
                    "userid": ctx.userid,
                    "token": ctx.token,
                    "listid": p.listid.value(),
                    "list_ver": 0,
                    "type": 0,
                    "slow_upload": 1,
                    "scene": "false;null",
                    "data": resource,
                })),
                query: Some(query),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn playlist_pic_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistPicParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let resource: Vec<Value> = p
            .data
            .split(',')
            .map(|s| {
                let parts: Vec<&str> = s.split('|').collect();
                let listid = parts.first().copied().unwrap_or("");
                let list_type = parts.get(1).copied().unwrap_or("");
                let pic = parts.get(2).copied().unwrap_or("");
                let cover = if pic.starts_with("stdmusic/") {
                    pic.to_owned()
                } else {
                    let tail = pic.rsplit('/').next().unwrap_or(pic);
                    format!("stdmusic/{tail}")
                };
                json!({
                    "listid": listid.parse::<i64>().unwrap_or(0),
                    "type": list_type.parse::<i64>().unwrap_or(0),
                    "pic": cover,
                })
            })
            .collect();
        cloud_request(
            ctx,
            "/v1/modify_list_pic",
            json!({
                "total_ver": js::or_flex(&p.total_ver, 0),
                "data": resource,
            }),
        )
        .await
    })
}

fn playlist_sort_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistSortParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let resource: Vec<Value> = parse_pipe_list(&p.data, &["listid", "type", "sort"])
            .into_iter()
            .map(|row| {
                json!({
                    "listid": row["listid"].as_str().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                    "type": row["type"].as_str().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                    "sort": row["sort"].as_str().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                })
            })
            .collect();
        cloud_request(
            ctx,
            "/v1/modify_list_sort",
            json!({
                "total_ver": js::or_flex(&p.total_ver, 0),
                "data": resource,
            }),
        )
        .await
    })
}

fn playlist_tracks_sort_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistTracksSortParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let resource: Vec<Value> = parse_pipe_list(
            p.data.as_deref().unwrap_or(""),
            &["fileid", "sort"],
        )
        .into_iter()
        .map(|row| {
            json!({
                "fileid": row["fileid"].as_str().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                "sort": row["sort"].as_str().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
            })
        })
        .collect();
        cloud_request(
            ctx,
            "/v1/modify_song_sort",
            json!({
                "listid": p.listid.value(),
                "list_ver": js::or_flex(&p.list_ver, 0),
                "type": js::or_flex(&p.list_type, 0),
                "data": resource,
            }),
        )
        .await
    })
}

fn playlist_update_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistUpdateParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut body = serde_json::Map::new();
        body.insert("total_ver".to_owned(), json!(js::or_flex(&p.total_ver, 0)));
        body.insert("listid".to_owned(), json!(p.listid.value()));
        body.insert("type".to_owned(), json!(js::or_flex(&p.list_type, 0)));
        body.insert("sort".to_owned(), json!(js::or_flex(&p.sort, 0)));
        body.insert("tags".to_owned(), json!(p.tags.clone().unwrap_or_default()));
        body.insert(
            "intro".to_owned(),
            json!(p.intro.clone().unwrap_or_default()),
        );
        if let Some(name) = &p.name {
            body.insert("name".to_owned(), json!(name));
        }
        if let Some(pic) = &p.pic {
            body.insert("pic".to_owned(), json!(pic));
        }
        cloud_request(ctx, "/v1/modify_list", Value::Object(body)).await
    })
}

fn user_cloud_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserCloudParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        encrypted_post(
            ctx,
            "/v1/get_list",
            Some("https://mcloudservice.kugou.com"),
            json!({
                "page": js::unwrap_or_flex(&p.page, 1),
                "pagesize": js::unwrap_or_flex(&p.page_size, 30),
                "getkmr": 1,
            }),
            None,
            Vec::new(),
            false,
        )
        .await
    })
}

fn user_cloud_del_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserCloudDelParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let fileids = split_list(&Value::String(p.fileids.clone()));
        if fileids.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "missing fileids / kv_id".to_owned(),
                payload: None,
            });
        }
        let album_audio_ids = split_list(&Value::String(
            p.album_audio_ids.clone().unwrap_or_default(),
        ));
        let data: Vec<Value> = fileids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let fallback = album_audio_ids
                    .get(index)
                    .or_else(|| album_audio_ids.first())
                    .and_then(|v| v.parse::<i64>().ok())
                    .or_else(|| p.mixid.as_ref().and_then(FlexI64::value))
                    .unwrap_or(0);
                json!({
                    "kv_id": id.parse::<i64>().unwrap_or(0),
                    "album_audio_id": fallback,
                })
            })
            .collect();
        encrypted_post(
            ctx,
            "/v1/del_files",
            Some("https://mcloudservice.kugou.com"),
            json!({ "data": data }),
            None,
            Vec::new(),
            false,
        )
        .await
    })
}

fn user_cloud_match_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserCloudMatchParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut hashes = split_list(&Value::String(
            p.hash
                .clone()
                .or_else(|| p.filename.clone())
                .unwrap_or_default(),
        ));
        if hashes.is_empty()
            && let Some(data_b64) = &p.data
            && let Ok(bytes) = BASE64.decode(data_b64)
            && !bytes.is_empty()
        {
            hashes.push(file_md5(&bytes));
        }
        if hashes.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "missing hash".to_owned(),
                payload: None,
            });
        }
        let album_audio_ids = split_list(&Value::String(
            p.album_audio_ids.clone().unwrap_or_default(),
        ));
        let mut album_audio_ids: Vec<String> = album_audio_ids;
        if album_audio_ids.is_empty()
            && let Some(single) = p
                .album_audio_id
                .as_ref()
                .and_then(FlexI64::value)
                .filter(|v| *v != 0)
                .or_else(|| p.mixid.as_ref().and_then(FlexI64::value))
        {
            album_audio_ids.push(single.to_string());
        }

        let data: Vec<Value> = hashes
            .iter()
            .enumerate()
            .map(|(index, hash)| {
                let mut item = serde_json::Map::new();
                item.insert("hash".to_owned(), json!(hash.to_lowercase()));
                let album_audio_id = album_audio_ids
                    .get(index)
                    .or_else(|| album_audio_ids.first());
                if let Some(id) = album_audio_id
                    && let Ok(value) = id.parse::<i64>()
                    && value > 0
                {
                    item.insert("album_audio_id".to_owned(), json!(id));
                }
                Value::Object(item)
            })
            .collect();

        let list = ctx
            .request(
                "/v2/album_audio/audio",
                GatewayRequest {
                    method: Some(Method::Post),
                    base_url: Some("http://kmr.service.kugou.com".to_owned()),
                    router: Some("kmr.service.kugou.com".to_owned()),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    sign: SignControl::None,
                    clear_params: true,
                    json: Some(json!({
                        "appid": ctx.appid,
                        "clienttime": ctx.clienttime,
                        "clientver": ctx.clientver,
                        "data": data,
                        "dfid": ctx.dfid,
                        "key": ctx.sign_params_key(ctx.clienttime.to_string()),
                        "mid": ctx.mid,
                        "show_privilege": 0,
                        "show_author_alias": 0,
                        "show_rel_album_audio_info": 0,
                        "show_remarks": 0,
                    })),
                    ..GatewayRequest::default()
                },
            )
            .await?;

        let empty: Vec<Value> = Vec::new();
        let items = list.as_array().unwrap_or(&empty);
        let match_list: Vec<Value> = items
            .iter()
            .filter_map(|item| {
                let candidate = if item.is_array() {
                    item.as_array().and_then(|a| a.first())
                } else {
                    Some(item)
                }?;
                normalize_match(candidate)
            })
            .collect();
        let first = match_list.first().cloned().unwrap_or(Value::Null);
        Ok(json!({
            "data": list,
            "match_list": match_list,
            "match": first,
        }))
    })
}

fn user_cloud_upload_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a UserCloudUploadParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let file_data = match &p.data {
            Some(b64) => BASE64.decode(b64).map_err(|e| Error::InvalidParams {
                endpoint: "user.cloud.upload".to_owned(),
                message: format!("data is not valid base64: {e}"),
            })?,
            None => Vec::new(),
        };
        if file_data.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "missing file bytes (params.data, base64)".to_owned(),
                payload: None,
            });
        }
        let filename = p
            .filename
            .clone()
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| file_md5(&file_data))
            .to_lowercase();
        let extendname = p
            .extendname
            .clone()
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "mp3".to_owned())
            .trim_start_matches('.')
            .to_owned();
        let bucket = "musicclound";

        // Optional auto metadata match.
        let has_explicit = p.hash_std.as_ref().is_some_and(|h| !h.is_empty())
            && p.audio_id.as_ref().and_then(FlexI64::value).unwrap_or(0) != 0
            && p.album_audio_id
                .as_ref()
                .and_then(FlexI64::value)
                .or_else(|| p.mixid.as_ref().and_then(FlexI64::value))
                .unwrap_or(0)
                != 0;
        let need_match = match p.auto_match.as_ref().and_then(FlexI64::value) {
            Some(0) => false,
            _ => !has_explicit,
        };
        let mut match_info: Option<Value> = None;
        if need_match {
            let mut params = serde_json::to_value(p)?;
            if let Some(obj) = params.as_object_mut() {
                obj.remove("data");
                obj.insert("hash".to_owned(), json!(filename));
            }
            if let Ok(match_res) = ctx.call_inner("user.cloud.match", params).await
                && let Some(found) = match_res.get("match")
                && !found.is_null()
            {
                match_info = Some(found.clone());
            }
        }

        let hash_std = p
            .hash_std
            .clone()
            .filter(|h| !h.is_empty())
            .or_else(|| {
                match_info
                    .as_ref()
                    .and_then(|m| m.get("hash_std"))
                    .and_then(Value::as_str)
                    .map(std::borrow::ToOwned::to_owned)
            })
            .unwrap_or_else(|| filename.clone())
            .to_lowercase();
        let audio_id = p
            .audio_id
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|v| *v != 0)
            .or_else(|| {
                match_info
                    .as_ref()
                    .and_then(|m| m.get("audio_id"))
                    .and_then(Value::as_u64)
                    .map(|v| v as i64)
            })
            .unwrap_or(0);
        let album_audio_id = p
            .album_audio_id
            .as_ref()
            .and_then(FlexI64::value)
            .or_else(|| p.mixid.as_ref().and_then(FlexI64::value))
            .filter(|v| *v != 0)
            .or_else(|| {
                match_info
                    .as_ref()
                    .and_then(|m| m.get("album_audio_id"))
                    .and_then(Value::as_u64)
                    .map(|v| v as i64)
            })
            .unwrap_or(0);
        let author_name = p
            .author_name
            .clone()
            .filter(|a| !a.is_empty())
            .or_else(|| {
                match_info
                    .as_ref()
                    .and_then(|m| m.get("author_name"))
                    .and_then(Value::as_str)
                    .map(std::borrow::ToOwned::to_owned)
            })
            .unwrap_or_default();
        let track_name = p
            .track_name
            .clone()
            .filter(|t| !t.is_empty())
            .or_else(|| p.songname.clone().filter(|t| !t.is_empty()))
            .or_else(|| {
                match_info
                    .as_ref()
                    .and_then(|m| m.get("audio_name"))
                    .and_then(Value::as_str)
                    .map(std::borrow::ToOwned::to_owned)
            })
            .unwrap_or_else(|| filename.clone());
        let name = p.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| {
            if author_name.is_empty() {
                format!("{track_name}.{extendname}")
            } else {
                format!("{author_name} - {track_name}.{extendname}")
            }
        });

        // BSS: auth → init → parts → complete.
        let mut auth_query = QueryRecord::new();
        auth_query.set("bucket", bucket);
        auth_query.set("filename", filename.clone());
        auth_query.set("method", "POST");
        auth_query.set(
            "loginType",
            i64::from(!ctx.token.is_empty() && ctx.userid != 0),
        );
        auth_query.set(
            "buVerifyCode",
            ctx.md5(&CipherInput::Text(format!(
                "{}{bucket}8ae10344e9738dcb",
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
        auth_query.set("clienttime", ctx.clienttime);
        let auth_res = bss_request(
            ctx,
            "/bsstrackercdngz/v1/upload/auth",
            Method::Get,
            auth_query,
            Vec::new(),
            None,
        )
        .await?;
        let authorization = auth_res
            .pointer("/data/data/authorization")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if authorization.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: auth_res.to_string(),
                payload: Some(auth_res),
            });
        }

        let mut init_query = QueryRecord::new();
        init_query.set("bucket", bucket);
        init_query.set("filename", filename.clone());
        init_query.set("ssl", 1);
        init_query.set("extendname", extendname.clone());
        init_query.set("version", ctx.clientver);
        init_query.set("userid", ctx.userid);
        init_query.set("token", ctx.token.clone());
        init_query.set("authorization", authorization.clone());
        init_query.set("dfid", ctx.dfid.clone());
        init_query.set("mid", ctx.mid.clone());
        init_query.set("uuid", ctx.uuid.clone());
        init_query.set("appid", ctx.appid);
        init_query.set("clientver", ctx.clientver);
        init_query.set("clienttime", ctx.clienttime);
        let init_res = bss_request(
            ctx,
            "http://bssulbig.kugou.com/v2/multipart/initiate/music",
            Method::Post,
            init_query,
            vec![("Authorization".to_owned(), authorization.clone())],
            None,
        )
        .await?;
        let init_data = init_res
            .pointer("/data/data")
            .cloned()
            .unwrap_or(Value::Null);
        let external_host = init_data
            .get("external_host")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let upload_id = init_data
            .get("upload_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let mut bss_file_hash = init_data
            .get("x-bss-filename")
            .and_then(Value::as_str)
            .unwrap_or(&filename)
            .to_owned();

        if !upload_id.is_empty() {
            if external_host.is_empty() {
                return Err(Error::Api {
                    code: 0,
                    message: init_res.to_string(),
                    payload: Some(init_res),
                });
            }
            const PART_SIZE: usize = 1024 * 1024;
            let part_count = file_data.len().div_ceil(PART_SIZE).max(1);
            for i in 0..part_count {
                let start = i * PART_SIZE;
                let end = ((i + 1) * PART_SIZE).min(file_data.len());
                let part = file_data[start..end].to_vec();

                let mut part_query = QueryRecord::new();
                part_query.set("bucket", bucket);
                part_query.set("authorization", authorization.clone());
                part_query.set("filename", filename.clone());
                part_query.set("partnumber", (i + 1) as u64);
                part_query.set("upload_id", upload_id.clone());
                part_query.set("body_empty", 1);
                part_query.set("version", ctx.clientver);
                part_query.set("userid", ctx.userid);
                part_query.set("token", ctx.token.clone());
                part_query.set("dfid", ctx.dfid.clone());
                part_query.set("mid", ctx.mid.clone());
                part_query.set("uuid", ctx.uuid.clone());
                part_query.set("appid", ctx.appid);
                part_query.set("clientver", ctx.clientver);
                part_query.set("clienttime", ctx.clienttime);
                let upload_res = bss_request(
                    ctx,
                    &format!("{}/v3/multipart/upload", with_protocol(&external_host)),
                    Method::Post,
                    part_query,
                    vec![
                        ("Authorization".to_owned(), authorization.clone()),
                        (
                            "Content-Type".to_owned(),
                            "application/octet-stream".to_owned(),
                        ),
                    ],
                    Some(part),
                )
                .await?;
                if upload_res.get("status").and_then(Value::as_i64) != Some(1) {
                    return Err(Error::Api {
                        code: 0,
                        message: upload_res.to_string(),
                        payload: Some(upload_res),
                    });
                }
            }

            let mut complete_query = QueryRecord::new();
            complete_query.set("bucket", bucket);
            complete_query.set("authorization", authorization.clone());
            complete_query.set("filename", filename.clone());
            complete_query.set("partnumber", part_count as u64);
            complete_query.set("upload_id", upload_id.clone());
            complete_query.set("md5", filename.clone());
            complete_query.set("version", ctx.clientver);
            complete_query.set("userid", ctx.userid);
            complete_query.set("token", ctx.token.clone());
            complete_query.set("if_id3", 1);
            complete_query.set("dfid", ctx.dfid.clone());
            complete_query.set("mid", ctx.mid.clone());
            complete_query.set("uuid", ctx.uuid.clone());
            complete_query.set("appid", ctx.appid);
            complete_query.set("clientver", ctx.clientver);
            complete_query.set("clienttime", ctx.clienttime);
            let complete_res = bss_request(
                ctx,
                &format!("{}/v3/multipart/complete", with_protocol(&external_host)),
                Method::Post,
                complete_query,
                vec![("Authorization".to_owned(), authorization.clone())],
                None,
            )
            .await?;
            if complete_res.get("status").and_then(Value::as_i64) != Some(1) {
                return Err(Error::Api {
                    code: 0,
                    message: complete_res.to_string(),
                    payload: Some(complete_res),
                });
            }
            bss_file_hash = complete_res
                .pointer("/data/data/x-bss-filename")
                .and_then(Value::as_str)
                .unwrap_or(&bss_file_hash)
                .to_owned();
        }

        let body = encrypted_post(
            ctx,
            "/v1/add_files",
            Some("https://mcloudservice.kugou.com"),
            json!({
                "data": [
                    {
                        "name": name,
                        "ext": extendname,
                        "author_name": author_name,
                        "hash": bss_file_hash,
                        "hash_std": hash_std,
                        "audio_id": audio_id,
                        "bitrate": js::or_flex(&p.bitrate, 4),
                        "album_audio_id": album_audio_id,
                        "size": file_data.len(),
                        "timelen": js::or_flex(&p.timelen, 0),
                    },
                ],
                "list_ver": js::or_flex(&p.list_ver, 0),
            }),
            None,
            Vec::new(),
            false,
        )
        .await
        .unwrap_or(Value::Null);

        let mut out = match body {
            Value::Object(map) => serde_json::Map::from_iter(map),
            _ => serde_json::Map::new(),
        };
        out.insert(
            "uploadInfo".to_owned(),
            json!({
                "authorization": authorization,
                "external_host": external_host,
                "upload_id": upload_id,
                "hash": bss_file_hash,
                "local_hash": filename,
                "hash_std": hash_std,
                "audio_id": audio_id,
                "album_audio_id": album_audio_id,
                "matched": match_info.is_some(),
                "filesize": file_data.len(),
            }),
        );
        Ok(Value::Object(out))
    })
}

fn playlist_pic_upload_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistPicUploadParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let image = BASE64.decode(&p.data).map_err(|e| Error::InvalidParams {
            endpoint: "playlist.pic.upload".to_owned(),
            message: format!("data is not valid base64: {e}"),
        })?;
        if image.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "missing image bytes (params.data, base64)".to_owned(),
                payload: None,
            });
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        // Treat the epoch as UTC (matching the reference's local-date intent).
        let days = now / 86_400;
        let (year, month, day) = civil_from_days(days as i64);
        let date_str = format!("{year}{month:02}{day:02}");

        let mut query = QueryRecord::new();
        query.set(
            "type",
            p.upload_type
                .clone()
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| "custom".to_owned()),
        );
        query.set(
            "extendName",
            p.extend_name
                .clone()
                .filter(|e| !e.is_empty())
                .unwrap_or_else(|| ".jpg".to_owned()),
        );
        query.set(
            "md5",
            p.md5.clone().filter(|m| !m.is_empty()).unwrap_or_else(|| {
                ctx.md5(&CipherInput::Text(format!("{date_str}hewry678WEK23D")))
            }),
        );
        query.set("jsonResponse", 1);

        let result = ctx
            .raw(RawRequest {
                method: Some(Method::Post),
                url: "http://imgphp.kugou.com/imageupload/stream.php".to_owned(),
                query: Some(query),
                headers: vec![(
                    "Content-Type".to_owned(),
                    "application/octet-stream".to_owned(),
                )],
                body: Some(Body::Bytes(image)),
                timeout: Some(std::time::Duration::from_secs(30)),
                ..RawRequest::new("")
            })
            .await?;
        let body = result.json().unwrap_or(Value::Null);
        if body.get("IsSuccess").and_then(Value::as_bool) == Some(true) {
            return Ok(json!({
                "status": 1,
                "FileName": body.get("FileName").cloned().unwrap_or(Value::Null),
            }));
        }
        Err(Error::Api {
            code: 0,
            message: body
                .get("Message")
                .and_then(Value::as_str)
                .unwrap_or("upload failed")
                .to_owned(),
            payload: Some(body),
        })
    })
}

/// Days-since-epoch → (year, month, day), UTC (Howard Hinnant's algorithm).
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

impl crate::api::CloudApi<'_> {
    /// Collect an MV (`mv.collect`).
    pub async fn mv_collect(&self, params: MvCollectParams) -> Result<Value> {
        self.client.call_value("mv.collect", params).await
    }

    /// Cancel an MV collection (`mv.collect.del`).
    pub async fn mv_collect_del(&self, params: MvCollectParams) -> Result<Value> {
        self.client.call_value("mv.collect.del", params).await
    }

    /// Delete a playlist (`playlist.del`).
    pub async fn playlist_del(&self, params: PlaylistDelParams) -> Result<Value> {
        self.client.call_value("playlist.del", params).await
    }

    /// Create a playlist (`playlist.add`).
    pub async fn playlist_add(&self, params: PlaylistAddParams) -> Result<Value> {
        self.client.call_value("playlist.add", params).await
    }

    /// Add tracks to a playlist (`playlist.tracks.add`).
    pub async fn playlist_tracks_add(&self, params: PlaylistTracksAddParams) -> Result<Value> {
        self.client.call_value("playlist.tracks.add", params).await
    }

    /// Change a playlist cover (`playlist.pic`).
    pub async fn playlist_pic(&self, params: PlaylistPicParams) -> Result<Value> {
        self.client.call_value("playlist.pic", params).await
    }

    /// Sort playlists (`playlist.sort`).
    pub async fn playlist_sort(&self, params: PlaylistSortParams) -> Result<Value> {
        self.client.call_value("playlist.sort", params).await
    }

    /// Sort playlist tracks (`playlist.tracks.sort`).
    pub async fn playlist_tracks_sort(&self, params: PlaylistTracksSortParams) -> Result<Value> {
        self.client.call_value("playlist.tracks.sort", params).await
    }

    /// Edit a playlist (`playlist.update`).
    pub async fn playlist_update(&self, params: PlaylistUpdateParams) -> Result<Value> {
        self.client.call_value("playlist.update", params).await
    }

    /// Cloud file list (`user.cloud`).
    pub async fn user_cloud(&self, params: UserCloudParams) -> Result<Value> {
        self.client.call_value("user.cloud", params).await
    }

    /// Delete cloud files (`user.cloud.del`).
    pub async fn user_cloud_del(&self, params: UserCloudDelParams) -> Result<Value> {
        self.client.call_value("user.cloud.del", params).await
    }

    /// Match cloud file metadata (`user.cloud.match`).
    pub async fn user_cloud_match(&self, params: UserCloudMatchParams) -> Result<Value> {
        self.client.call_value("user.cloud.match", params).await
    }

    /// Cloud playback URL (`user.cloud.url`).
    pub async fn user_cloud_url(&self, params: UserCloudUrlParams) -> Result<Value> {
        self.client.call_value("user.cloud.url", params).await
    }

    /// Upload a file to the cloud (`user.cloud.upload`) — BSS multipart with
    /// an encrypted registration.
    pub async fn user_cloud_upload(&self, params: UserCloudUploadParams) -> Result<Value> {
        self.client.call_value("user.cloud.upload", params).await
    }

    /// Upload a cover image (`playlist.pic.upload`).
    pub async fn playlist_pic_upload(&self, params: PlaylistPicUploadParams) -> Result<Value> {
        self.client.call_value("playlist.pic.upload", params).await
    }
}

// keep the CipherOutput import referenced (used by the encrypted-post path)
const _: Option<CipherOutput> = None;
