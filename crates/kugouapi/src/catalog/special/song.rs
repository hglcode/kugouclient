//! Special song endpoints: the playback-URL family.
//!
//! Covers the classic trackercdn `/v5/url`, the auth variant, the new
//! `/v6/priv_url` protocol, authorization retrieval and the composite
//! auth + url chain, plus the `privilege.lite` quality-aggregation flow.
//!
//! Ported from `special/song.ts`.

use serde_json::{Map, Value, json};

use crate::catalog::ctx::GatewayRequest;
use crate::catalog::def::{Built, EndpointSpec, handler_endpoint, simple_endpoint, spec};
use crate::error::{Error, Result};
use crate::js::{self, FlexI64, FlexString};
use crate::query::QueryRecord;

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

const MAGIC_QUALITIES: [&str; 6] = ["piano", "acappella", "subwoofer", "ancient", "dj", "surnay"];

fn normalize_quality(quality: Option<&str>) -> String {
    let q = quality.unwrap_or("");
    if MAGIC_QUALITIES.contains(&q) {
        format!("magic_{q}")
    } else {
        q.to_owned()
    }
}

/// `song_url` / `song_url_auth` shared tracker platform parameters.
fn tracker_page_ids(ppage_id: Option<&str>, lite: bool) -> (u64, String, u64) {
    if lite {
        (
            967_177_915,
            ppage_id
                .filter(|p| !p.is_empty())
                .unwrap_or("356753938,823673182,967485191")
                .to_owned(),
            411,
        )
    } else {
        (151_369_488, "463467626,350369493,788954147".to_owned(), 2)
    }
}

kugou_params! {
    /// Parameters for `song.url`.
    pub SongUrlEndpointParams {
        required {
            /// File hash.
            hash: String => "hash",
        }
        optional {
            /// Quality (default 128; magic channels auto-prefixed).
            quality: String => "quality",
            /// Album id.
            album_id: FlexString => "album_id",
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
            /// Trial-segment flag.
            free_part: bool => "free_part",
            /// Trial-segment flag alias (`IsFreePart` wire name).
            is_free_part: bool => "IsFreePart",
            /// `ppage_id` override (lite).
            ppage_id: String => "ppage_id",
        }
    }
}

kugou_params! {
    /// Parameters for `song.url.auth`.
    pub SongUrlAuthParams {
        required {
            /// File hash.
            hash: String => "hash",
        }
        optional {
            /// Auth token (from `song.auth`).
            auth: String => "auth",
            /// Open time (from `song.auth`).
            open_time: FlexI64 => "open_time",
            /// Quality.
            quality: String => "quality",
            /// Album id.
            album_id: FlexString => "album_id",
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
            /// `ppage_id` override (lite).
            ppage_id: String => "ppage_id",
        }
    }
}

kugou_params! {
    /// Parameters for `song.url.new` (`/v6/priv_url`).
    pub SongUrlNewParams {
        required {
            /// File hash.
            hash: String => "hash",
        }
        optional {
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
            /// Trial-segment flag.
            free_part: bool => "free_part",
            /// Trial-segment flag alias (`IsFreePart` wire name).
            is_free_part: bool => "IsFreePart",
        }
    }
}

kugou_params! {
    /// Parameters for `song.auth`.
    pub SongAuthParams {
        required {
            /// File hash.
            hash: String => "hash",
        }
        optional {
            /// Authorization string.
            auth: String => "auth",
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
        }
    }
}

kugou_params! {
    /// Parameters for `privilege.lite`.
    pub PrivilegeLiteParams {
        required {
            /// Comma-separated file hashes.
            hash: String => "hash",
        }
        optional {
            /// Comma-separated album ids.
            album_id: String => "album_id",
            /// Comma-separated album audio ids.
            album_audio_id: String => "album_audio_id",
            /// Quality list (`all` or comma-separated).
            qualities: String => "qualities",
        }
    }
}

const EP_SONG_URL: EndpointSpec = EndpointSpec {
    router: Some("trackercdn.kugou.com"),
    tracker_key: true,
    ..spec("song.url", "song_url", crate::http::Method::Get, "/v5/url")
};
const EP_SONG_URL_AUTH: EndpointSpec = EndpointSpec {
    tracker_key: true,
    auth: true,
    ..spec(
        "song.url.auth",
        "song_url_auth",
        crate::http::Method::Get,
        "/tracker/v5/url",
    )
};
const EP_SONG_URL_NEW: EndpointSpec = EndpointSpec {
    router: Some("tracker.kugou.com"),
    auth: true,
    ..spec(
        "song.url.new",
        "song_url_new",
        crate::http::Method::Post,
        "/v6/priv_url",
    )
};
const EP_SONG_AUTH: EndpointSpec = EndpointSpec {
    router: Some("trackercdngz.kugou.com"),
    auth: true,
    ..spec(
        "song.auth",
        "song_auth",
        crate::http::Method::Get,
        "/v1/authorization",
    )
};
const EP_SONG_URL_AUTH_MERGE: EndpointSpec = EndpointSpec {
    router: Some("trackercdngz.kugou.com"),
    auth: true,
    ..spec(
        "song.url.auth.merge",
        "song_url_auth_merge",
        crate::http::Method::Get,
        "/v1/authorization",
    )
};
const EP_PRIVILEGE_LITE: EndpointSpec = EndpointSpec {
    router: Some("media.store.kugou.com"),
    ..spec(
        "privilege.lite",
        "privilege_lite",
        crate::http::Method::Post,
        "/v2/get_res_privilege/lite",
    )
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        // ── /v5/url (classic trackercdn playback URL) ──
        simple_endpoint(&EP_SONG_URL, |p: &SongUrlEndpointParams, ctx| {
            let (page_id, ppage_ids, pid) = tracker_page_ids(
                p.ppage_id.as_deref(),
                ctx.platform == crate::config::Platform::Lite,
            );
            let quality = {
                let q = normalize_quality(p.quality.as_deref());
                if q.is_empty() { "128".to_owned() } else { q }
            };
            let mut query = QueryRecord::new();
            query.set("album_id", flex_string_or(p.album_id.as_ref(), "0"));
            query.set("area_code", 1);
            query.set("hash", p.hash.to_lowercase());
            query.set("ssa_flag", "is_fromtrack");
            query.set("version", 11430);
            query.set("page_id", page_id);
            query.set("quality", quality);
            query.set("album_audio_id", js::unwrap_or_flex(&p.album_audio_id, 0));
            query.set("behavior", "play");
            query.set("pid", pid);
            query.set("cmd", 26);
            query.set("pidversion", 3001);
            query.set(
                "IsFreePart",
                i64::from(p.free_part.unwrap_or(false) || p.is_free_part.unwrap_or(false)),
            );
            query.set("ppage_id", ppage_ids);
            query.set("cdnBackup", 1);
            query.set("module", "");
            query.set("clientver", 11430);
            Built {
                query,
                ..Built::default()
            }
        }),
        // ── /tracker/v5/url (auth playback URL) ──
        simple_endpoint(&EP_SONG_URL_AUTH, |p: &SongUrlAuthParams, ctx| {
            let (page_id, ppage_ids, pid) = tracker_page_ids(
                p.ppage_id.as_deref(),
                ctx.platform == crate::config::Platform::Lite,
            );
            let quality = {
                let q = normalize_quality(p.quality.as_deref());
                if q.is_empty() { "128".to_owned() } else { q }
            };
            let mut query = QueryRecord::new();
            query.set("album_id", flex_string_or(p.album_id.as_ref(), "0"));
            query.set("area_code", 1);
            query.set("module", "");
            query.set("hash", p.hash.to_lowercase());
            query.set("need_m", 0);
            query.set("ssa_flag", "is_fromtrack");
            query.set("version", 11430);
            query.set("open_time", js::or_flex(&p.open_time, 0));
            query.set("ptype", 0);
            query.set("need_ogg", 1);
            query.set("page_id", page_id);
            query.set("auth", p.auth.clone().unwrap_or_default());
            query.set("mtype", 0);
            query.set("quality", quality);
            query.set("album_audio_id", js::unwrap_or_flex(&p.album_audio_id, 0));
            query.set("behavior", "play");
            query.set("pid", pid);
            query.set("module_id", 51);
            query.set("cmd", 26);
            query.set("ppage_id", ppage_ids);
            query.set("clientver", 11561);
            query.set("pidversion", 3001);
            query.set("cdnBackup", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
        // ── /v6/priv_url (new protocol) ──
        simple_endpoint(&EP_SONG_URL_NEW, |p: &SongUrlNewParams, ctx| {
            let body = json!({
                "area_code": "1",
                "behavior": "play",
                "qualities": [
                    "128", "320", "flac", "high", "multitrack",
                    "viper_atmos", "viper_tape", "viper_clear", "super",
                ],
                "resource": {
                    "album_audio_id": p.album_audio_id.as_ref().and_then(FlexI64::value),
                    "collect_list_id": "3",
                    "collect_time": crate::crypto::now_ms(),
                    "hash": p.hash,
                    "id": 0,
                    "page_id": 1,
                    "type": "audio",
                },
                "token": ctx.token,
                "tracker_param": {
                    "all_m": 1,
                    "auth": "",
                    "is_free_part": i64::from(p.free_part.unwrap_or(false)),
                    // NOTE: the key always uses the lite salt (reference parity)
                    "key": crate::sign::md5(&format!(
                        "{}185672dd44712f60bb1736df5a377e82{}{}{}",
                        p.hash, ctx.appid, ctx.mid, ctx.userid
                    )),
                    "module_id": 0,
                    "need_climax": 1,
                    "need_xcdn": 1,
                    "open_time": "",
                    "pid": "411",
                    "pidversion": "3001",
                    "priv_vip_type": "6",
                    "viptoken": ctx.vip_token,
                },
                "userid": ctx.userid.to_string(),
                "vip": ctx.vip_type,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        // ── /v1/authorization (song auth) ──
        simple_endpoint(&EP_SONG_AUTH, |p: &SongAuthParams, _| {
            let mut query = QueryRecord::new();
            query.set("authorization", p.auth.clone().unwrap_or_default());
            query.set("module_id", 51);
            query.set("album_audio_id", js::unwrap_or_flex(&p.album_audio_id, 0));
            query.set("clientver", 11561);
            query.set("hash", p.hash.to_lowercase());
            Built {
                query,
                ..Built::default()
            }
        }),
        // ── composite: song.auth + song.url.auth ──
        handler_endpoint(&EP_SONG_URL_AUTH_MERGE, song_url_auth_merge_handler),
        // ── /v2/get_res_privilege/lite (quality privilege aggregation) ──
        handler_endpoint(&EP_PRIVILEGE_LITE, privilege_lite_handler),
    ]
}

fn flex_string_or(value: Option<&FlexString>, default: &str) -> String {
    match value {
        Some(v) => v.value().map_or_else(|| default.to_owned(), str::to_owned),
        None => default.to_owned(),
    }
}

fn song_url_auth_merge_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a SongAuthParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let hash = p.hash.to_lowercase();
        let album_audio_id = js::unwrap_or_flex(&p.album_audio_id, 0);

        let mut query = QueryRecord::new();
        query.set("authorization", p.auth.clone().unwrap_or_default());
        query.set("module_id", 51);
        query.set("album_audio_id", album_audio_id);
        query.set("clientver", 11561);
        query.set("hash", hash.clone());
        let auth_data = ctx
            .request(
                "/v1/authorization",
                GatewayRequest {
                    query: Some(query),
                    ..GatewayRequest::default()
                },
            )
            .await?;

        let auth = auth_data
            .get("auth")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let open_time = auth_data
            .get("open_time")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if auth.is_empty() && open_time == 0 {
            return Err(Error::Api {
                code: 0,
                message: "failed to obtain auth and open_time".to_owned(),
                payload: Some(auth_data),
            });
        }

        let mut params = Map::new();
        params.insert("hash".to_owned(), json!(hash));
        params.insert("album_audio_id".to_owned(), json!(album_audio_id));
        params.insert("auth".to_owned(), json!(auth));
        params.insert("open_time".to_owned(), json!(open_time));
        if let Some(extra) = p.auth.as_ref() {
            params
                .entry("authorization".to_owned())
                .or_insert(json!(extra));
        }
        ctx.call_inner("song.url.auth", Value::Object(params)).await
    })
}

const UPSTREAM_QUALITIES: [&str; 9] = [
    "128",
    "320",
    "flac",
    "high",
    "viper_atmos",
    "viper_tape",
    "viper_clear",
    "super",
    "multitrack",
];

const QUALITY_ORDER: [&str; 9] = [
    "128",
    "320",
    "flac",
    "high",
    "super",
    "viper_clear",
    "viper_tape",
    "viper_atmos",
    "multitrack",
];

const QUALITY_VIP_TYPE: [(&str, &str); 3] = [
    ("viper_tape", "suvip"),
    ("viper_clear", "suvip"),
    ("viper_atmos", "suvip"),
];

fn quality_levels(quality: &str) -> Vec<i64> {
    match quality {
        "128" => vec![1, 2],
        "320" => vec![4],
        "flac" => vec![5],
        "high" => vec![6],
        "super" => vec![7],
        _ => Vec::new(),
    }
}

fn format_size(bytes: &Value) -> String {
    let n = bytes.as_f64().unwrap_or(0.0);
    if n <= 0.0 {
        return String::new();
    }
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    if n >= GB {
        format!("{:.1}G", n / GB)
    } else if n >= MB {
        format!("{:.1}M", n / MB)
    } else {
        format!("{:.1}K", n / 1024.0)
    }
}

fn pick_relate<'g>(relate_goods: &'g [Value], quality: &str) -> Option<&'g Value> {
    for level in quality_levels(quality) {
        if let Some(hit) = relate_goods.iter().find(|item| {
            item.get("level").and_then(Value::as_i64) == Some(level)
                && item.get("quality").and_then(Value::as_str) == Some(quality)
        }) {
            return Some(hit);
        }
    }
    relate_goods
        .iter()
        .find(|item| item.get("quality").and_then(Value::as_str) == Some(quality))
}

fn ogg_of(quality: &str, resource: &Value) -> (String, u64) {
    let trans_param = resource.get("trans_param").cloned().unwrap_or(Value::Null);
    let pick = |hash_key: &str, size_key: &str| {
        (
            trans_param
                .get(hash_key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            trans_param
                .get(size_key)
                .and_then(Value::as_u64)
                .unwrap_or(0),
        )
    };
    match quality {
        "128" => pick("ogg_128_hash", "ogg_128_filesize"),
        "320" => pick("ogg_320_hash", "ogg_320_filesize"),
        _ => (String::new(), 0),
    }
}

fn num_or(value: Option<&Value>, default: i64) -> i64 {
    value
        .and_then(|v| match v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        })
        .unwrap_or(default)
}

fn str_or(value: Option<&Value>, default: &str) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => default.to_owned(),
    }
}

/// The `privilege.lite` quality-vip summary (field-compatible with the
/// reference implementation).
fn build_quality_vip(
    hash: &str,
    album_id: &str,
    album_audio_id: &str,
    resource: &Value,
    qualities: &[String],
    lite: bool,
) -> Value {
    let relate_goods: Vec<Value> = resource
        .get("relate_goods")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut list: Vec<Value> = Vec::with_capacity(qualities.len());
    for quality in qualities {
        let relate_item = pick_relate(&relate_goods, quality);
        let exists = relate_item.is_some();
        let relate = relate_item.cloned().unwrap_or(Value::Null);
        let privilege = num_or(relate.get("privilege"), -1);
        let neesuvip = exists && privilege != 0;
        let free_listen = lite && quality == "128" && neesuvip;
        let status = num_or(relate.get("status"), 0);
        let obtainable = free_listen || (exists && !neesuvip && status != 0);
        let (ogg_hash, ogg_size) = ogg_of(quality, resource);
        let relate_size = relate
            .get("info")
            .and_then(|info| info.get("filesize"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let size = if exists {
            if ogg_size != 0 { ogg_size } else { relate_size }
        } else {
            0
        };
        let level = num_or(relate.get("level"), 0);
        let bitrate = num_or(relate.get("info").and_then(|i| i.get("bitrate")), 0);
        let extname = if exists {
            if ogg_hash.is_empty() {
                str_or(relate.get("info").and_then(|i| i.get("extname")), "")
            } else {
                "ogg".to_owned()
            }
        } else {
            String::new()
        };
        let relate_hash = str_or(relate.get("hash"), "");
        let require_vip_type = if exists && !obtainable && neesuvip {
            QUALITY_VIP_TYPE
                .iter()
                .find(|(q, _)| *q == quality.as_str())
                .map_or_else(|| "concept".to_owned(), |(_, v)| v.to_string())
        } else {
            String::new()
        };
        list.push(json!({
            "quality": quality,
            "exists": exists,
            "level": level,
            "bitrate": bitrate,
            "extname": extname,
            "hash": if exists {
                if ogg_hash.is_empty() { relate_hash.clone() } else { ogg_hash.clone() }
            } else { String::new() },
            "size": size,
            "size_text": format_size(&json!(size)),
            "privilege": privilege,
            "status": status,
            "pay_type": num_or(relate.get("pay_type"), -1),
            "price": num_or(relate.get("price"), 0),
            "fail_process": num_or(relate.get("fail_process"), 0),
            "need_vip": neesuvip,
            "obtainable": obtainable,
            "free_listen": free_listen,
            "require_vip_type": require_vip_type,
            "msg": str_or(relate.get("_msg"), ""),
        }));
    }

    let free_qualities: Vec<&str> = list
        .iter()
        .filter(|item| item["obtainable"].as_bool().unwrap_or(false))
        .filter_map(|item| item["quality"].as_str())
        .collect();
    let vip_qualities: Vec<&str> = list
        .iter()
        .filter(|item| {
            item["exists"].as_bool().unwrap_or(false)
                && item["need_vip"].as_bool().unwrap_or(false)
                && !item["obtainable"].as_bool().unwrap_or(false)
        })
        .filter_map(|item| item["quality"].as_str())
        .collect();
    let blocked_qualities: Vec<&str> = list
        .iter()
        .filter(|item| {
            item["exists"].as_bool().unwrap_or(false)
                && !item["obtainable"].as_bool().unwrap_or(false)
                && !item["need_vip"].as_bool().unwrap_or(false)
        })
        .filter_map(|item| item["quality"].as_str())
        .collect();
    let missing_qualities: Vec<&str> = list
        .iter()
        .filter(|item| !item["exists"].as_bool().unwrap_or(false))
        .filter_map(|item| item["quality"].as_str())
        .collect();
    let suvip_qualities: Vec<&str> = list
        .iter()
        .filter(|item| item["require_vip_type"].as_str() == Some("suvip"))
        .filter_map(|item| item["quality"].as_str())
        .collect();
    let free_song = !list.is_empty()
        && list
            .iter()
            .all(|item| !item["need_vip"].as_bool().unwrap_or(false));
    let mut best_sorted: Vec<&Value> = list
        .iter()
        .filter(|item| item["obtainable"].as_bool().unwrap_or(false))
        .collect();
    best_sorted.sort_by(|a, b| {
        let la = a["level"].as_i64().unwrap_or(0);
        let lb = b["level"].as_i64().unwrap_or(0);
        let qa = QUALITY_ORDER
            .iter()
            .position(|q| Some(*q) == a["quality"].as_str())
            .unwrap_or(usize::MAX);
        let qb = QUALITY_ORDER
            .iter()
            .position(|q| Some(*q) == b["quality"].as_str())
            .unwrap_or(usize::MAX);
        la.cmp(&lb).then(qa.cmp(&qb))
    });
    let best_free = best_sorted
        .last()
        .and_then(|item| item["quality"].as_str())
        .unwrap_or("")
        .to_owned();

    json!({
        "hash": hash,
        "album_id": str_or(resource.get("album_id"), album_id),
        "album_audio_id": str_or(resource.get("album_audio_id"), album_audio_id),
        "name": str_or(resource.get("name"), ""),
        "singername": str_or(resource.get("singername"), ""),
        "albumname": str_or(resource.get("albumname"), ""),
        "free_song": free_song,
        "free_listen": list.iter().any(|item| item["free_listen"].as_bool().unwrap_or(false)),
        "qualities": list,
        "free_qualities": free_qualities,
        "vip_qualities": vip_qualities,
        "super_vip_qualities": suvip_qualities,
        "blocked_qualities": blocked_qualities,
        "missing_qualities": missing_qualities,
        "best_free_quality": best_free,
    })
}

fn privilege_lite_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PrivilegeLiteParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let hash_list: Vec<String> = p
            .hash
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let album_id_list: Vec<&str> = p.album_id.as_deref().unwrap_or("").split(',').collect();
        let album_audio_id_list: Vec<&str> = p
            .album_audio_id
            .as_deref()
            .unwrap_or("")
            .split(',')
            .collect();
        let qualities_param = p.qualities.as_deref().unwrap_or("").trim().to_lowercase();
        let qualities: Vec<String> = if qualities_param == "all" {
            UPSTREAM_QUALITIES.iter().map(|s| (*s).to_owned()).collect()
        } else if qualities_param.is_empty() {
            "128,320,flac,high,viper_tape,viper_clear,viper_atmos"
                .split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            qualities_param
                .split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        };

        let resource: Vec<Value> = hash_list
            .iter()
            .enumerate()
            .map(|(index, hash)| {
                json!({
                    "type": "audio",
                    "page_id": 0,
                    "hash": hash,
                    "album_id": js::or_flex(
                        &Some(FlexI64::from(
                            album_id_list
                                .get(index)
                                .copied()
                                .unwrap_or("0")
                                .parse::<i64>()
                                .unwrap_or(0),
                        )),
                        0,
                    ),
                })
            })
            .collect();

        let data = ctx
            .request(
                "/v2/get_res_privilege/lite",
                GatewayRequest {
                    method: Some(crate::http::Method::Post),
                    router: Some("media.store.kugou.com".to_owned()),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    json: Some(json!({
                        "appid": ctx.appid,
                        "area_code": 1,
                        "behavior": "play",
                        "clientver": ctx.clientver,
                        "need_hash_offset": 1,
                        "relate": 1,
                        "support_verify": 1,
                        "resource": resource,
                        "qualities": UPSTREAM_QUALITIES,
                    })),
                    ..GatewayRequest::default()
                },
            )
            .await?;

        let empty: Vec<Value> = Vec::new();
        let list = data.as_array().unwrap_or(&empty);
        let quality_vip: Vec<Value> = hash_list
            .iter()
            .enumerate()
            .map(|(index, hash)| {
                let matched = list
                    .iter()
                    .find(|x| {
                        x.get("hash")
                            .and_then(Value::as_str)
                            .is_some_and(|h| h.to_lowercase() == *hash)
                    })
                    .or_else(|| list.get(index))
                    .cloned()
                    .unwrap_or(Value::Null);
                build_quality_vip(
                    hash,
                    album_id_list.get(index).copied().unwrap_or(""),
                    album_audio_id_list.get(index).copied().unwrap_or(""),
                    &matched,
                    &qualities,
                    ctx.platform == crate::config::Platform::Lite,
                )
            })
            .collect();
        Ok(json!({ "quality_vip": quality_vip }))
    })
}

impl crate::api::SongApi<'_> {
    /// Classic playback URL (`song.url`) — typed response.
    pub async fn url(&self, params: crate::api::SongUrlParams) -> Result<crate::api::SongUrlData> {
        self.client.call_typed("song.url", params).await
    }

    /// Classic playback URL with full parameter control (`song.url`).
    pub async fn url_raw(&self, params: SongUrlEndpointParams) -> Result<Value> {
        self.client.call_value("song.url", params).await
    }

    /// Auth playback URL (`song.url.auth`).
    pub async fn url_auth(&self, params: SongUrlAuthParams) -> Result<Value> {
        self.client.call_value("song.url.auth", params).await
    }

    /// New-protocol playback URL (`song.url.new`).
    pub async fn url_new(&self, params: SongUrlNewParams) -> Result<Value> {
        self.client.call_value("song.url.new", params).await
    }

    /// Song authorization (`song.auth`).
    pub async fn auth(&self, params: SongAuthParams) -> Result<Value> {
        self.client.call_value("song.auth", params).await
    }

    /// Composite auth + playback URL (`song.url.auth.merge`).
    pub async fn url_auth_merge(&self, params: SongAuthParams) -> Result<Value> {
        self.client.call_value("song.url.auth.merge", params).await
    }

    /// Quality privilege aggregation (`privilege.lite`).
    pub async fn privilege_lite(&self, params: PrivilegeLiteParams) -> Result<Value> {
        self.client.call_value("privilege.lite", params).await
    }
}
