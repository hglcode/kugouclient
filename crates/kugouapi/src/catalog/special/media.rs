//! Media-domain special endpoints: audio / album aggregates (signParamsKey
//! protocol), the image family (self-computed signatures with URL-embedded
//! parameters), lyrics (KRC decode), song recognition and artist follow
//! (AES + RSA).
//!
//! Ported from `special/media.ts`.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Map, Value, json};

use crate::catalog::ctx::{GatewayRequest, RawRequest, SignControl};
use crate::catalog::def::{Built, EndpointSpec, Salt, handler_endpoint, simple_endpoint, spec};
use crate::crypto::CipherInput;
use crate::crypto::decode_krc;
use crate::error::{Error, Result};
use crate::http::{Body, Method};
use crate::js::{self, FlexI64};
use crate::query::{QueryRecord, QueryValue};
use crate::sign::{SignOptions, md5, signature};

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

const STANDARD_SALT: &str = "OIlwieks28dk2k092lksi2UIkp";

/// The reference `signatureStandardParams` (objects serialized as JSON).
fn signature_standard(params: &QueryRecord) -> String {
    let mut pairs: Vec<(&str, &QueryValue)> = params.iter().collect();
    pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let canonical: String = pairs.iter().fold(String::new(), |mut acc, (k, v)| {
        acc.push_str(k);
        acc.push('=');
        acc.push_str(&v.to_wire_string());
        acc
    });
    md5(&format!("{STANDARD_SALT}{canonical}{STANDARD_SALT}"))
}

/// Render params as a sorted, URL-encoded query string.
fn render_sorted_query(params: &QueryRecord) -> String {
    let mut pairs: Vec<(&str, &QueryValue)> = params.iter().collect();
    pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencode(&v.to_wire_string())))
        .collect::<Vec<_>>()
        .join("&")
}

/// `encodeURIComponent` equivalent.
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(*byte as char),
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

/// The image family: parameters embedded in the URL plus a self-computed
/// signature.
async fn signed_image_url(
    ctx: &CallCtx<'_>,
    path: &str,
    params: QueryRecord,
    origin: &str,
    extra_headers: Vec<(String, String)>,
) -> Result<Value> {
    let sign = signature_standard(&params);
    let query = render_sorted_query(&params);
    let mut request_query = QueryRecord::new();
    request_query.set("signature", sign);
    ctx.request(
        &format!("{path}?{query}"),
        GatewayRequest {
            method: Some(Method::Get),
            base_url: Some(origin.to_owned()),
            sign: SignControl::None,
            clear_params: true,
            query: Some(request_query),
            headers: extra_headers,
            ..GatewayRequest::default()
        },
    )
    .await
}

/// The kmr audio / album shared payload (ms timestamp + signParamsKey).
fn kmr_payload(ctx: &CallCtx<'_>, rows: Vec<Value>) -> Map<String, Value> {
    let date_time = crate::crypto::now_ms();
    let mut body = Map::new();
    body.insert("appid".to_owned(), json!(ctx.appid));
    body.insert("clienttime".to_owned(), json!(date_time));
    body.insert("clientver".to_owned(), json!(ctx.clientver));
    body.insert("data".to_owned(), json!(rows));
    body.insert("dfid".to_owned(), json!(ctx.dfid));
    body.insert("key".to_owned(), json!(ctx.sign_params_key(date_time)));
    body.insert("mid".to_owned(), json!(ctx.mid));
    if !ctx.token.is_empty() {
        body.insert("token".to_owned(), json!(ctx.token));
    }
    if ctx.userid != 0 {
        body.insert("userid".to_owned(), json!(ctx.userid));
    }
    body
}

kugou_params! {
    /// Parameters for `audio` (batch audio details).
    pub AudioParams {
        required {
            /// Comma-separated hashes.
            hash: String => "hash",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `album` (batch album details).
    pub AlbumParams {
        required {
            /// Comma-separated album ids.
            album_id: String => "album_id",
        }
        optional {
            /// Field selection.
            fields: String => "fields",
        }
    }
}

kugou_params! {
    /// Parameters for `album.dycover`.
    pub AlbumDycoverParams {
        required {
            /// Comma-separated album audio ids.
            album_audio_id: String => "album_audio_id",
        }
        optional {
            /// Comma-separated album ids.
            album_id: String => "album_id",
            /// Cover query type (default `audioPlay`).
            query: String => "query",
        }
    }
}

/// Parameters for `audio.match` (binary fingerprint as base64).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioMatchParams {
    /// Fingerprint bytes as base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// User id override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userid: Option<FlexI64>,
}

kugou_params! {
    /// Parameters for `audio.related`.
    pub AudioRelatedParams {
        required {
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
        }
        optional {
            /// `1` selects the aggregated `/v2` endpoint.
            show_detail: FlexI64 => "show_detail",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Show type.
            show_type: FlexI64 => "show_type",
            /// Sort: all / hot / new.
            sort: String => "sort",
            /// Type filter.
            list_type: FlexI64 => "type",
        }
    }
}

kugou_params! {
    /// Parameters for `fm.songs` (radio images).
    pub FmSongsParams {
        required {
            /// Comma-separated fm ids.
            fmid: String => "fmid",
        }
        optional {
            /// Default fm type (default 2).
            fm_type: FlexI64 => "type",
            /// Comma-separated per-item fm types.
            fmtype: String => "fmtype",
            /// Default offset (default -1).
            offset: FlexI64 => "offset",
            /// Comma-separated per-item offsets.
            fmoffset: String => "fmoffset",
            /// Default size (default 20).
            size: FlexI64 => "size",
            /// Comma-separated per-item sizes.
            fmsize: String => "fmsize",
        }
    }
}

kugou_params! {
    /// Parameters for `images`.
    pub ImagesParams {
        required {
            /// Comma-separated hashes.
            hash: String => "hash",
        }
        optional {
            /// Comma-separated album ids.
            album_id: String => "album_id",
            /// Comma-separated album audio ids.
            album_audio_id: String => "album_audio_id",
            /// Image count (default 5).
            count: FlexI64 => "count",
        }
    }
}

kugou_params! {
    /// Parameters for `images.audio`.
    pub ImagesAudioParams {
        required {
            /// Comma-separated hashes.
            hash: String => "hash",
        }
        optional {
            /// Comma-separated audio ids.
            audio_id: String => "audio_id",
            /// Comma-separated album audio ids.
            album_audio_id: String => "album_audio_id",
            /// Comma-separated file names.
            filename: String => "filename",
            /// Image count (default 5).
            count: FlexI64 => "count",
        }
    }
}

kugou_params! {
    /// Parameters for `lyric`.
    pub LyricParams {
        required {
            /// Lyric id.
            id: FlexI64 => "id",
            /// Lyric access key.
            accesskey: String => "accesskey",
        }
        optional {
            /// Client type (default `android`).
            client: String => "client",
            /// Format: krc (default) / lrc.
            fmt: String => "fmt",
            /// Attach `decodeContent` (base64 / KRC-decoded text).
            decode: bool => "decode",
        }
    }
}

kugou_params! {
    /// Parameters for `artist.follow` / `artist.unfollow`.
    pub ArtistFollowParams {
        required {
            /// Singer id.
            id: FlexI64 => "id",
        }
        optional {}
    }
}

const EP_AUDIO: EndpointSpec = EndpointSpec {
    origin: Some("http://kmr.service.kugou.com"),
    router: Some("kmr.service.kugou.com"),
    ..spec("audio", "audio", Method::Post, "/v1/audio/audio")
};
const EP_ALBUM: EndpointSpec = EndpointSpec {
    origin: Some("http://kmr.service.kugou.com"),
    router: Some("kmr.service.kugou.com"),
    ..spec("album", "album", Method::Post, "/v1/album")
};
const EP_ALBUM_DYCOVER: EndpointSpec = EndpointSpec {
    origin: Some("https://kmrcdn.service.kugou.com"),
    salt: Salt::None,
    clear_headers: true,
    ..spec(
        "album.dycover",
        "album_dycover",
        Method::Get,
        "/v2/album/audio",
    )
};
const EP_AUDIO_MATCH: EndpointSpec = spec(
    "audio.match",
    "audio_match",
    Method::Post,
    "/fingerprint.service/v1/music_trackid_mulit",
);
const EP_AUDIO_RELATED: EndpointSpec = EndpointSpec {
    origin: Some("https://listkmrp3cdnretry.kugou.com"),
    salt: Salt::None,
    ..spec(
        "audio.related",
        "audio_related",
        Method::Get,
        "/v3/album_audio/related",
    )
};
const EP_FM_SONGS: EndpointSpec = EndpointSpec {
    router: Some("fm.service.kugou.com"),
    ..spec("fm.songs", "fm_songs", Method::Post, "/v1/fm_info")
};
const EP_IMAGES: EndpointSpec = EndpointSpec {
    origin: Some("https://expendablekmr.kugou.com"),
    salt: Salt::None,
    ..spec("images", "images", Method::Get, "/container/v2/image")
};
const EP_IMAGES_AUDIO: EndpointSpec = EndpointSpec {
    origin: Some("https://expendablekmr.kugou.com"),
    salt: Salt::None,
    ..spec(
        "images.audio",
        "images_audio",
        Method::Get,
        "/v2/author_image/audio",
    )
};
const EP_LYRIC: EndpointSpec = EndpointSpec {
    origin: Some("https://lyrics.kugou.com"),
    ..spec("lyric", "lyric", Method::Get, "/download")
};
const EP_ARTIST_FOLLOW: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "artist.follow",
        "artist_follow",
        Method::Post,
        "/followservice/v3/follow_singer",
    )
};
const EP_ARTIST_UNFOLLOW: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "artist.unfollow",
        "artist_unfollow",
        Method::Post,
        "/followservice/v3/unfollow_singer",
    )
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_AUDIO, |p: &AudioParams, ctx| {
            let rows: Vec<Value> = p
                .hash
                .split(',')
                .map(|s| json!({ "hash": s, "audio_id": 0 }))
                .collect();
            Built {
                json: Some(Value::Object(kmr_payload(ctx, rows))),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                ..Built::default()
            }
        }),
        handler_endpoint(&EP_ALBUM, album_handler),
        handler_endpoint(&EP_ALBUM_DYCOVER, album_dycover_handler),
        handler_endpoint(&EP_AUDIO_MATCH, audio_match_handler),
        handler_endpoint(&EP_AUDIO_RELATED, audio_related_handler),
        simple_endpoint(&EP_FM_SONGS, |p: &FmSongsParams, ctx| {
            let default_fmtype = js::or_flex(&p.fm_type, 2);
            let default_offset = js::or_flex(&p.offset, -1);
            let default_size = js::or_flex(&p.size, 20);
            let mut rows: Vec<Map<String, Value>> = p
                .fmid
                .split(',')
                .map(|s| {
                    let mut row = Map::new();
                    row.insert("fields".to_owned(), json!("imgUrl100,imgUrl50"));
                    row.insert("fmid".to_owned(), json!(s));
                    row.insert("fmtype".to_owned(), json!(default_fmtype));
                    row.insert("offset".to_owned(), json!(default_offset));
                    row.insert("size".to_owned(), json!(default_size));
                    row
                })
                .collect();
            let mut merge = |values: &str, key: &str, fallback: i64| {
                for (index, value) in values.split(',').enumerate() {
                    if let Some(row) = rows.get_mut(index) {
                        let parsed = value.parse::<i64>().unwrap_or(0);
                        if parsed != 0 {
                            row.insert(key.to_owned(), json!(parsed));
                        } else {
                            row.insert(key.to_owned(), json!(fallback));
                        }
                    }
                }
            };
            merge(p.fmtype.as_deref().unwrap_or(""), "fmtype", default_fmtype);
            merge(
                p.fmoffset.as_deref().unwrap_or(""),
                "offset",
                default_offset,
            );
            merge(p.fmsize.as_deref().unwrap_or(""), "size", default_size);

            let date_time = crate::crypto::now_ms();
            let mut body = Map::new();
            body.insert("appid".to_owned(), json!(ctx.appid));
            body.insert("clienttime".to_owned(), json!(date_time));
            body.insert("clientver".to_owned(), json!(ctx.clientver));
            body.insert(
                "data".to_owned(),
                json!(rows.into_iter().map(Value::Object).collect::<Vec<_>>()),
            );
            body.insert("dfid".to_owned(), json!(ctx.dfid));
            body.insert("key".to_owned(), json!(ctx.sign_params_key(date_time)));
            body.insert("mid".to_owned(), json!(ctx.mid));
            if ctx.userid != 0 {
                body.insert("userid".to_owned(), json!(ctx.userid));
            }
            if !ctx.token.is_empty() {
                body.insert("token".to_owned(), json!(ctx.token));
            }
            Built {
                json: Some(Value::Object(body)),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                ..Built::default()
            }
        }),
        handler_endpoint(&EP_IMAGES, images_handler),
        handler_endpoint(&EP_IMAGES_AUDIO, images_audio_handler),
        handler_endpoint(&EP_LYRIC, lyric_handler),
        handler_endpoint(&EP_ARTIST_FOLLOW, artist_follow_handler),
        handler_endpoint(&EP_ARTIST_UNFOLLOW, artist_unfollow_handler),
    ]
}

fn album_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a AlbumParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let rows: Vec<Value> = p
            .album_id
            .split(',')
            .map(|s| json!({ "album_id": s, "album_name": "", "author_name": "" }))
            .collect();
        let mut body = kmr_payload(ctx, rows);
        body.insert(
            "fields".to_owned(),
            json!(p.fields.clone().unwrap_or_default()),
        );
        ctx.request(
            "/v1/album",
            GatewayRequest {
                method: Some(Method::Post),
                base_url: Some("http://kmr.service.kugou.com".to_owned()),
                router: Some("kmr.service.kugou.com".to_owned()),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                json: Some(Value::Object(body)),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn album_dycover_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a AlbumDycoverParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let album_ids: Vec<&str> = p.album_id.as_deref().unwrap_or("").split(',').collect();
        let data: Vec<Value> = p
            .album_audio_id
            .split(',')
            .enumerate()
            .map(|(index, s)| {
                let album_id = album_ids
                    .get(index)
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(0);
                let mut obj = Map::new();
                obj.insert(
                    "album_audio_id".to_owned(),
                    json!(s.parse::<i64>().unwrap_or(0)),
                );
                if album_id > 0 {
                    obj.insert("album_id".to_owned(), json!(album_id));
                }
                Value::Object(obj)
            })
            .collect();

        let mut params = QueryRecord::new();
        params.set("appid", ctx.appid);
        params.set("clientver", ctx.clientver);
        params.set_json("data", json!(data));
        params.set("isCdn", 1);
        params.set(
            "query",
            p.query.clone().unwrap_or_else(|| "audioPlay".to_owned()),
        );
        signed_image_url(
            ctx,
            "/v2/album/audio",
            params,
            "https://kmrcdn.service.kugou.com",
            vec![
                ("dfid".to_owned(), "-".to_owned()),
                ("mid".to_owned(), "undefined".to_owned()),
                (
                    "clienttime".to_owned(),
                    crate::catalog::ctx::now_secs_public().to_string(),
                ),
            ],
        )
        .await
    })
}

fn audio_match_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a AudioMatchParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let data = match &p.data {
            Some(b64) => BASE64.decode(b64).map_err(|e| Error::InvalidParams {
                endpoint: "audio.match".to_owned(),
                message: format!("data is not valid base64: {e}"),
            })?,
            None => Vec::new(),
        };
        if data.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "missing fingerprint bytes (params.data, base64)".to_owned(),
                payload: None,
            });
        }
        let mut query = QueryRecord::new();
        query.set("fpid", crate::crypto::now_ms());
        query.set("area_code", 1);
        query.set("include_unpublish", 1);
        query.set(
            "useid",
            p.userid
                .as_ref()
                .and_then(FlexI64::value)
                .filter(|u| *u != 0)
                .unwrap_or(ctx.userid as i64),
        );
        query.set("multi_result", 1);
        let sign = signature(
            &query,
            SignOptions {
                platform: ctx.platform,
                ..SignOptions::default()
            },
        );
        query.set("signature", sign);

        let result = ctx
            .raw(RawRequest {
                method: Some(Method::Post),
                url: "https://gateway.kugou.com/fingerprint.service/v1/music_trackid_mulit"
                    .to_owned(),
                query: Some(query),
                headers: vec![
                    (
                        "Content-Type".to_owned(),
                        "application/octet-stream".to_owned(),
                    ),
                    ("User-Agent".to_owned(), "KuGou/11490 (Android)".to_owned()),
                ],
                body: Some(Body::Bytes(data)),
                retries: Some(0),
                ..RawRequest::new("")
            })
            .await?;
        if !result.ok() {
            return Err(Error::Http {
                status: result.status,
                context: "POST /fingerprint.service/v1/music_trackid_mulit".to_owned(),
                payload: result.json(),
            });
        }
        Ok(result.json().unwrap_or(Value::Null))
    })
}

fn audio_related_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a AudioRelatedParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let show_detail = js::unwrap_or_flex(&p.show_detail, 0) == 1;
        let sort_type = match p.sort.as_deref() {
            Some("hot") => 2,
            Some("new") => 3,
            _ => 1,
        };

        let mut data = QueryRecord::new();
        data.set("album_audio_id", p.album_audio_id.value().unwrap_or(0));
        data.set("appid", 1005);
        data.set("area_code", 1);
        data.set("clientver", 12329);
        if !show_detail {
            data.set("page", js::or_flex(&p.page, 1));
            data.set("pagesize", js::or_flex(&p.page_size, 30));
            data.set("show_input", 1);
            data.set("show_type", js::or_flex(&p.show_type, 0));
            data.set("sort", sort_type);
            data.set("type", js::or_flex(&p.list_type, 0));
        }
        data.set("version", 1);
        let sign = signature_standard(&data);
        data.set("signature", sign);

        let path = if show_detail {
            "/v2/audio_related/total"
        } else {
            "/v3/album_audio/related"
        };
        let query = render_sorted_query(&data);
        ctx.request(
            &format!("{path}?{query}"),
            GatewayRequest {
                method: Some(Method::Get),
                base_url: Some("https://listkmrp3cdnretry.kugou.com".to_owned()),
                sign: SignControl::None,
                clear_params: true,
                query: Some(QueryRecord::new()),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn images_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a ImagesParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let album_ids: Vec<&str> = p.album_id.as_deref().unwrap_or("").split(',').collect();
        let album_audio_ids: Vec<&str> = p
            .album_audio_id
            .as_deref()
            .unwrap_or("")
            .split(',')
            .collect();
        let data: Vec<Value> = p
            .hash
            .split(',')
            .map(|s| json!({ "album_id": 0, "hash": s, "album_audio_id": 0 }))
            .enumerate()
            .map(|(index, mut row)| {
                if let Some(album_id) = album_ids.get(index) {
                    row["album_id"] = json!(if album_id.is_empty() {
                        json!(0)
                    } else {
                        json!(album_id)
                    });
                }
                if let Some(album_audio_id) = album_audio_ids.get(index) {
                    row["album_audio_id"] = json!(if album_audio_id.is_empty() {
                        json!(0)
                    } else {
                        json!(album_audio_id)
                    });
                }
                row
            })
            .collect();

        let mut params = QueryRecord::new();
        params.set("album_image_type", "-3");
        params.set("appid", ctx.appid);
        params.set("clientver", ctx.clientver);
        params.set("author_image_type", "3,4,5");
        params.set("count", js::or_flex(&p.count, 5));
        params.set_json("data", json!(data));
        params.set("isCdn", 1);
        params.set("publish_time", 1);
        signed_image_url(
            ctx,
            "/container/v2/image",
            params,
            "https://expendablekmr.kugou.com",
            Vec::new(),
        )
        .await
    })
}

fn images_audio_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a ImagesAudioParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let audio_ids: Vec<&str> = p.audio_id.as_deref().unwrap_or("").split(',').collect();
        let album_audio_ids: Vec<&str> = p
            .album_audio_id
            .as_deref()
            .unwrap_or("")
            .split(',')
            .collect();
        let filenames: Vec<&str> = p.filename.as_deref().unwrap_or("").split(',').collect();
        let data: Vec<Value> = p
            .hash
            .split(',')
            .map(|s| json!({ "audio_id": 0, "hash": s, "album_audio_id": 0, "filename": "" }))
            .enumerate()
            .map(|(index, mut row)| {
                if let Some(audio_id) = audio_ids.get(index) {
                    row["audio_id"] = json!(if audio_id.is_empty() {
                        json!(0)
                    } else {
                        json!(audio_id)
                    });
                }
                if let Some(album_audio_id) = album_audio_ids.get(index) {
                    row["album_audio_id"] = json!(if album_audio_id.is_empty() {
                        json!(0)
                    } else {
                        json!(album_audio_id)
                    });
                }
                if let Some(filename) = filenames.get(index) {
                    row["filename"] = json!(filename);
                }
                row
            })
            .collect();

        let mut params = QueryRecord::new();
        params.set("appid", ctx.appid);
        params.set("clientver", ctx.clientver);
        params.set("count", js::or_flex(&p.count, 5));
        params.set_json("data", json!(data));
        params.set("isCdn", 1);
        params.set("publish_time", 1);
        params.set("show_authors", 1);
        signed_image_url(
            ctx,
            "/v2/author_image/audio",
            params,
            "https://expendablekmr.kugou.com",
            Vec::new(),
        )
        .await
    })
}

fn lyric_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a LyricParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut query = QueryRecord::new();
        query.set("ver", 1);
        query.set(
            "client",
            p.client.clone().unwrap_or_else(|| "android".to_owned()),
        );
        query.set_opt("id", p.id.value());
        query.set("accesskey", p.accesskey.clone());
        query.set("fmt", p.fmt.clone().unwrap_or_else(|| "krc".to_owned()));
        query.set("charset", "utf8");
        let mut data = ctx
            .request(
                "/download",
                GatewayRequest {
                    base_url: Some("https://lyrics.kugou.com".to_owned()),
                    query: Some(query),
                    ..GatewayRequest::default()
                },
            )
            .await?;

        if p.decode.unwrap_or(false)
            && let Some(content) = data.get("content").and_then(Value::as_str)
        {
            let is_lrc = p.fmt.as_deref() == Some("lrc")
                || data.get("contenttype").and_then(Value::as_i64) != Some(0);
            let decoded = if is_lrc {
                String::from_utf8_lossy(&BASE64.decode(content).unwrap_or_default()).into_owned()
            } else {
                decode_krc(content)
            };
            if let Some(obj) = data.as_object_mut() {
                obj.insert("decodeContent".to_owned(), json!(decoded));
            }
        }
        Ok(data)
    })
}

async fn follow_singer(ctx: &CallCtx<'_>, singer_id: i64, url: &str) -> Result<Value> {
    let clienttime = crate::catalog::ctx::now_secs_public();
    let encrypt = ctx.aes_hex(
        &CipherInput::Json(json!({
            "singerid": singer_id,
            "token": ctx.token,
        })),
        None,
        None,
    );
    let mut query = QueryRecord::new();
    query.set("clienttime", clienttime);
    ctx.request(
        url,
        GatewayRequest {
            method: Some(Method::Post),
            json: Some(json!({
                "plat": 0,
                "userid": ctx.userid,
                "singerid": singer_id,
                "source": 7,
                "p": ctx.rsa_encrypt_hex(&CipherInput::Json(json!({
                    "clienttime": clienttime,
                    "key": encrypt.key,
                }))),
                "params": encrypt.str,
            })),
            query: Some(query),
            ..GatewayRequest::default()
        },
    )
    .await
}

fn artist_follow_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a ArtistFollowParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        follow_singer(
            ctx,
            p.id.value().unwrap_or(0),
            "/followservice/v3/follow_singer",
        )
        .await
    })
}

fn artist_unfollow_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a ArtistFollowParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        follow_singer(
            ctx,
            p.id.value().unwrap_or(0),
            "/followservice/v3/unfollow_singer",
        )
        .await
    })
}

impl crate::api::MediaApi<'_> {
    /// Batch audio details (`audio`).
    pub async fn audio(&self, params: AudioParams) -> Result<Value> {
        self.client.call_value("audio", params).await
    }

    /// Batch album details (`album`).
    pub async fn album(&self, params: AlbumParams) -> Result<Value> {
        self.client.call_value("album", params).await
    }

    /// Album dynamic covers (`album.dycover`).
    pub async fn album_dycover(&self, params: AlbumDycoverParams) -> Result<Value> {
        self.client.call_value("album.dycover", params).await
    }

    /// Song recognition (`audio.match`).
    pub async fn audio_match(&self, params: AudioMatchParams) -> Result<Value> {
        self.client.call_value("audio.match", params).await
    }

    /// Related audios (`audio.related`).
    pub async fn audio_related(&self, params: AudioRelatedParams) -> Result<Value> {
        self.client.call_value("audio.related", params).await
    }

    /// Lyric download (`lyric`).
    pub async fn lyric(&self, params: LyricParams) -> Result<Value> {
        self.client.call_value("lyric", params).await
    }

    /// Song / album images (`images`).
    pub async fn images(&self, params: ImagesParams) -> Result<Value> {
        self.client.call_value("images", params).await
    }

    /// Audio author images (`images.audio`).
    pub async fn images_audio(&self, params: ImagesAudioParams) -> Result<Value> {
        self.client.call_value("images.audio", params).await
    }

    /// Follow a singer (`artist.follow`).
    pub async fn artist_follow(&self, params: ArtistFollowParams) -> Result<Value> {
        self.client.call_value("artist.follow", params).await
    }

    /// Unfollow a singer (`artist.unfollow`).
    pub async fn artist_unfollow(&self, params: ArtistFollowParams) -> Result<Value> {
        self.client.call_value("artist.unfollow", params).await
    }
}
