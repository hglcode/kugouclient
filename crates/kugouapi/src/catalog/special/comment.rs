//! Comment / barrage family (the `_comment` protocol).
//!
//! Everything goes through `m.comment.service.kugou.com/index.php`: `r` selects
//! the route, `code` selects the resource type (song / album / playlist /
//! barrage), `key` = signParamsKey(clienttime[+mid(+data)]); the android
//! signature is disabled for this family.
//!
//! Ported from `special/comment.ts`.

use serde_json::{Value, json};

use crate::catalog::ctx::{GatewayRequest, SignControl};
use crate::catalog::def::{EndpointSpec, Salt, handler_endpoint, spec};
use crate::error::{Error, Result};
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;
use crate::util::{compact, first_value};

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

const COMMENT_HOST: &str = "m.comment.service.kugou.com";
const SONG_COMMENT_CODE: &str = "fc4be23b4e972707f36b8a828a93ba8a";
const ALBUM_COMMENT_CODE: &str = "94f1792ced1df89aa68a7939eaf2efca";
const PLAYLIST_COMMENT_CODE: &str = "ca53b96fe5a1d9c22d71c8f522ef7c4f";
const SONG_BARRAGE_CODE: &str = "articulossong";
const VIDEO_BARRAGE_CODE: &str = "db3664c219a6e350b00ab08d7f723a79";

/// Shared parameters for the comment / barrage family. All fields are
/// optional; handlers apply the reference fallback chains.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommentParams {
    /// Resource id (`special_id` / `childrenid` / `id` chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_id: Option<FlexI64>,
    /// Alias for the resource id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub childrenid: Option<FlexI64>,
    /// Alias for the resource id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FlexI64>,
    /// Resource name (`name` / `song_name` / `album_name` / `playlist_name` /
    /// `childrenname` chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Song name alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub song_name: Option<String>,
    /// Album name alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_name: Option<String>,
    /// Playlist name alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlist_name: Option<String>,
    /// Video name alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_name: Option<String>,
    /// Children name alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub childrenname: Option<String>,
    /// Mixsong id (`mixsongid` / `album_audio_id` chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixsongid: Option<FlexI64>,
    /// Album audio id alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_audio_id: Option<FlexI64>,
    /// File hash (`hash` / `schash` / `mvhash` / `extdata` chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Hash alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schash: Option<String>,
    /// Hash alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mvhash: Option<String>,
    /// Hash alias (send lookups).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extdata: Option<String>,
    /// Video id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_id: Option<FlexI64>,
    /// Comment content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Parent comment id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<FlexI64>,
    /// Target comment id (replies / deletion).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tid: Option<FlexI64>,
    /// Comment id (`cid` / `comment_id` chain, deletion).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cid: Option<FlexI64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Comment id alias.
    pub comment_id: Option<FlexI64>,
    /// Page number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<FlexI64>,
    /// Page alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<FlexI64>,
    /// Page size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagesize: Option<FlexI64>,
    /// Protocol version override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ver: Option<FlexI64>,
    /// Show classify flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_classify: Option<FlexI64>,
    /// Show hotword list flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_hotword_list: Option<FlexI64>,
    /// Explicit `code` override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Resource type: song (default) / album / playlist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    /// Resource type alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resourceType: Option<String>,
    /// Top-level reply flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_t: Option<FlexI64>,
    /// Top-level reply flag alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isT: Option<FlexI64>,
    /// Replied user name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_user_name: Option<String>,
    /// Replied user name alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puser: Option<String>,
    /// Replied content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_content: Option<String>,
    /// Replied content alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pcontent: Option<String>,
    /// Comment title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Extra extdata for the content payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_extdata: Option<String>,
    /// Attached images.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Value>,
    /// Source flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FlexI64>,
}

fn opt_i64(values: &[&Option<FlexI64>]) -> Option<i64> {
    for value in values {
        if let Some(v) = value.as_ref().and_then(FlexI64::value)
            && v != 0
        {
            return Some(v);
        }
    }
    None
}

fn opt_str(values: &[&Option<String>]) -> Option<String> {
    for value in values {
        if let Some(v) = value.as_ref().filter(|s| !s.is_empty()) {
            return Some(v.clone());
        }
    }
    None
}

/// The comment-family auth parameters (reference `getListAuthParams`).
fn list_auth_params(p: &CommentParams, ctx: &CallCtx<'_>, query: &mut QueryRecord) {
    query.set("kugouid", ctx.userid);
    let ver = p
        .ver
        .as_ref()
        .and_then(FlexI64::value)
        .filter(|v| *v != 0)
        .unwrap_or(6);
    query.set("ver", ver);
    query.set("clienttoken", ctx.token.clone());
    query.set("appid", ctx.appid);
    query.set("clientver", ctx.clientver);
    query.set("mid", ctx.mid.clone());
    query.set("clienttime", ctx.clienttime);
    query.set("key", ctx.sign_params_key(ctx.clienttime.to_string()));
    query.set("uuid", ctx.uuid.clone());
    query.set("dfid", ctx.dfid.clone());
}

/// The comment-family request (index.php, no signature, cleared defaults).
async fn comment_request(
    ctx: &CallCtx<'_>,
    query: QueryRecord,
    method: crate::http::Method,
    data: Option<String>,
) -> Result<Value> {
    let mut headers = Vec::new();
    if data.is_some() {
        headers.push((
            "Content-Type".to_owned(),
            "application/json; charset=UTF-8".to_owned(),
        ));
    }
    ctx.request(
        "/index.php",
        GatewayRequest {
            method: Some(method),
            router: Some(COMMENT_HOST.to_owned()),
            sign: SignControl::None,
            clear_params: true,
            query: Some(query),
            text: data,
            headers,
            ..GatewayRequest::default()
        },
    )
    .await
}

fn resolve_comment_code(p: &CommentParams) -> String {
    if let Some(code) = opt_str(&[&p.code]) {
        return code;
    }
    let resource_type = p
        .resource_type
        .clone()
        .or_else(|| p.resourceType.clone())
        .unwrap_or_else(|| "song".to_owned())
        .to_lowercase();
    match resource_type.as_str() {
        "album" => ALBUM_COMMENT_CODE.to_owned(),
        "playlist" => PLAYLIST_COMMENT_CODE.to_owned(),
        _ => SONG_COMMENT_CODE.to_owned(),
    }
}

/// Extract the resource id / name from a comment-list response.
fn extract_resolved_resource(body: &Value) -> (Option<i64>, Option<String>) {
    let record = body.as_object().cloned().unwrap_or_default();
    let list = body.get("list").and_then(Value::as_array);
    let first = list.and_then(|l| l.first()).cloned().unwrap_or(Value::Null);

    let id = record
        .get("childrenid")
        .cloned()
        .or_else(|| first.get("special_child_id").cloned());
    let id = match id {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    };
    let name = first_value(&[
        first
            .get("special_child_name")
            .cloned()
            .unwrap_or(Value::Null),
        first.get("song_show_text").cloned().unwrap_or(Value::Null),
    ])
    .and_then(|v| v.as_str().map(std::borrow::ToOwned::to_owned));
    (id, name)
}

const EP_SONG_BARRAGE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec(
        "song.barrage",
        "song_barrage",
        crate::http::Method::Get,
        "/index.php",
    )
};
const EP_SONG_BARRAGE_SEND: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "song.barrage.send",
        "song_barrage_send",
        crate::http::Method::Post,
        "/index.php",
    )
};
const EP_VIDEO_BARRAGE: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec(
        "video.barrage",
        "video_barrage",
        crate::http::Method::Get,
        "/index.php",
    )
};
const EP_VIDEO_BARRAGE_SEND: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "video.barrage.send",
        "video_barrage_send",
        crate::http::Method::Post,
        "/index.php",
    )
};
const EP_COMMENT_FLOOR: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    ..spec(
        "comment.floor",
        "comment_floor",
        crate::http::Method::Post,
        "/mcomment/v1/hot_replylist",
    )
};
const EP_COMMENT_MUSIC_SEND: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "comment.music.send",
        "comment_music_send",
        crate::http::Method::Post,
        "/index.php",
    )
};
const EP_COMMENT_ALBUM_SEND: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "comment.album.send",
        "comment_album_send",
        crate::http::Method::Post,
        "/index.php",
    )
};
const EP_COMMENT_PLAYLIST_SEND: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "comment.playlist.send",
        "comment_playlist_send",
        crate::http::Method::Post,
        "/index.php",
    )
};
const EP_COMMENT_FLOOR_SEND: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "comment.floor.send",
        "comment_floor_send",
        crate::http::Method::Post,
        "/index.php",
    )
};
const EP_COMMENT_MUSIC_DEL: EndpointSpec = EndpointSpec {
    salt: Salt::None,
    auth: true,
    ..spec(
        "comment.music.del",
        "comment_music_del",
        crate::http::Method::Get,
        "/index.php",
    )
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        handler_endpoint(&EP_SONG_BARRAGE, song_barrage_handler),
        handler_endpoint(&EP_SONG_BARRAGE_SEND, song_barrage_send_handler),
        handler_endpoint(&EP_VIDEO_BARRAGE, video_barrage_handler),
        handler_endpoint(&EP_VIDEO_BARRAGE_SEND, video_barrage_send_handler),
        handler_endpoint(&EP_COMMENT_FLOOR, comment_floor_handler),
        handler_endpoint(&EP_COMMENT_MUSIC_SEND, comment_send_handler::<0>),
        handler_endpoint(&EP_COMMENT_ALBUM_SEND, comment_send_handler::<1>),
        handler_endpoint(&EP_COMMENT_PLAYLIST_SEND, comment_send_handler::<2>),
        handler_endpoint(&EP_COMMENT_FLOOR_SEND, comment_floor_send_handler),
        handler_endpoint(&EP_COMMENT_MUSIC_DEL, comment_music_del_handler),
    ]
}

fn barrage_list_handler<'a, const VIDEO: bool>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let special_id = if VIDEO {
            opt_i64(&[&p.video_id, &p.childrenid, &p.id])
        } else {
            opt_i64(&[&p.special_id, &p.childrenid, &p.id])
        };
        let hash = if VIDEO {
            opt_str(&[&p.hash, &p.mvhash, &p.extdata])
        } else {
            opt_str(&[&p.hash, &p.schash, &p.extdata])
        };
        if special_id.is_none() && hash.is_none() {
            return Err(Error::Api {
                code: 0,
                message: if VIDEO {
                    "video_id or hash is required".to_owned()
                } else {
                    "special_id or hash is required".to_owned()
                },
                payload: None,
            });
        }

        let mut query = QueryRecord::new();
        query.set("r", "comments/getCommentWithLike");
        query.set(
            "code",
            if VIDEO {
                VIDEO_BARRAGE_CODE
            } else {
                SONG_BARRAGE_CODE
            },
        );
        query.set_opt("childrenid", special_id);
        if special_id.is_none()
            && let Some(hash) = &hash
        {
            query.set("extdata", hash.clone());
        }
        let name_fields: &[&Option<String>] = if VIDEO {
            &[&p.name, &p.video_name, &p.childrenname]
        } else {
            &[&p.name, &p.song_name, &p.childrenname]
        };
        let name = opt_str(name_fields);
        query.set_opt("childrenname", name);
        query.set_opt("mixsongid", opt_i64(&[&p.mixsongid, &p.album_audio_id]));
        query.set(
            "p",
            p.page
                .as_ref()
                .and_then(FlexI64::value)
                .or_else(|| p.p.as_ref().and_then(FlexI64::value))
                .unwrap_or(1),
        );
        query.set(
            "pagesize",
            p.pagesize.as_ref().and_then(FlexI64::value).unwrap_or(20),
        );
        list_auth_params(p, ctx, &mut query);
        comment_request(ctx, query, crate::http::Method::Get, None).await
    })
}

fn song_barrage_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    barrage_list_handler::<false>(ctx, p)
}

fn video_barrage_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    barrage_list_handler::<true>(ctx, p)
}

fn barrage_send_handler<'a, const VIDEO: bool>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let content = p.content.clone().unwrap_or_default();
        if content.trim().is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "content must not be empty".to_owned(),
                payload: None,
            });
        }
        let mut special_id = if VIDEO {
            opt_i64(&[&p.video_id, &p.childrenid, &p.id])
        } else {
            opt_i64(&[&p.special_id, &p.childrenid, &p.id])
        };
        let name_fields: &[&Option<String>] = if VIDEO {
            &[&p.name, &p.video_name, &p.childrenname]
        } else {
            &[&p.name, &p.song_name, &p.childrenname]
        };
        let mut name = opt_str(name_fields);

        if special_id.is_none() {
            let hash = if VIDEO {
                opt_str(&[&p.hash, &p.mvhash, &p.extdata])
            } else {
                opt_str(&[&p.hash, &p.schash, &p.extdata])
            };
            let Some(hash) = hash else {
                return Err(Error::Api {
                    code: 0,
                    message: if VIDEO {
                        "video_id or hash is required".to_owned()
                    } else {
                        "special_id or hash is required".to_owned()
                    },
                    payload: None,
                });
            };
            let mut query = QueryRecord::new();
            query.set("r", "comments/getCommentWithLike");
            query.set(
                "code",
                if VIDEO {
                    VIDEO_BARRAGE_CODE
                } else {
                    SONG_BARRAGE_CODE
                },
            );
            query.set("extdata", hash);
            query.set_opt("childrenname", name.clone());
            query.set_opt("mixsongid", opt_i64(&[&p.mixsongid, &p.album_audio_id]));
            query.set("p", 1);
            query.set("pagesize", 1);
            list_auth_params(p, ctx, &mut query);
            let lookup = comment_request(ctx, query, crate::http::Method::Get, None).await?;
            let (resolved_id, resolved_name) = extract_resolved_resource(&lookup);
            special_id = resolved_id;
            if name.is_none() {
                name = resolved_name;
            }
        }
        let Some(special_id) = special_id else {
            return Err(Error::Api {
                code: 0,
                message: if VIDEO {
                    "failed to resolve the video barrage id from hash".to_owned()
                } else {
                    "failed to resolve the song barrage special_id from hash".to_owned()
                },
                payload: None,
            });
        };

        let mut query = QueryRecord::new();
        query.set("r", "comments/addcomment");
        query.set(
            "code",
            if VIDEO {
                VIDEO_BARRAGE_CODE
            } else {
                SONG_BARRAGE_CODE
            },
        );
        query.set("childrenid", special_id);
        query.set_opt("childrenname", name);
        let ver = p
            .ver
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|v| *v != 0)
            .unwrap_or(0);
        if ver != 0 {
            query.set("ver", ver.to_string());
        } else {
            query.set("ver", "1.02");
        }
        query.set("content", content);
        query.set_opt("pid", p.pid.as_ref().and_then(FlexI64::value));
        query.set("clientver", ctx.clientver);
        query.set("mid", ctx.mid.clone());
        query.set("clienttoken", ctx.token.clone());
        query.set("kugouid", ctx.userid);
        query.set("appid", ctx.appid);
        comment_request(ctx, query, crate::http::Method::Post, None).await
    })
}

fn song_barrage_send_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    barrage_send_handler::<false>(ctx, p)
}

fn video_barrage_send_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    barrage_send_handler::<true>(ctx, p)
}

fn comment_floor_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let resource_type = p
            .resource_type
            .clone()
            .or_else(|| p.resourceType.clone())
            .unwrap_or_default()
            .to_lowercase();
        let code = match opt_str(&[&p.code]) {
            Some(code) => code,
            None => match resource_type.as_str() {
                "playlist" => PLAYLIST_COMMENT_CODE.to_owned(),
                "album" => ALBUM_COMMENT_CODE.to_owned(),
                _ => SONG_COMMENT_CODE.to_owned(),
            },
        };
        let use_service = resource_type == "playlist"
            || resource_type == "album"
            || code == PLAYLIST_COMMENT_CODE
            || code == ALBUM_COMMENT_CODE;

        let mut query = QueryRecord::new();
        query.set_opt("childrenid", p.special_id.as_ref().and_then(FlexI64::value));
        query.set("need_show_image", 1);
        query.set("p", js::or_flex(&p.page, 1));
        query.set("pagesize", js::or_flex(&p.pagesize, 30));
        query.set("show_classify", js::unwrap_or_flex(&p.show_classify, 1));
        query.set(
            "show_hotword_list",
            js::unwrap_or_flex(&p.show_hotword_list, 1),
        );
        query.set("code", code);
        query.set_opt("tid", p.tid.as_ref().and_then(FlexI64::value));
        let mixsongid = opt_i64(&[&p.mixsongid, &p.album_audio_id]);
        query.set_opt("mixsongid", mixsongid);

        ctx.request(
            if use_service {
                "/m.comment.service/v1/hot_replylist"
            } else {
                "/mcomment/v1/hot_replylist"
            },
            GatewayRequest {
                method: Some(crate::http::Method::Post),
                query: Some(query),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

/// The shared send-comment logic (`commentsv3/add` with lookups).
async fn send_comment(ctx: &CallCtx<'_>, p: &CommentParams, code: &str, kind: u8) -> Result<Value> {
    let content = p.content.clone().unwrap_or_default();
    if content.trim().is_empty() {
        return Err(Error::Api {
            code: 0,
            message: "content must not be empty".to_owned(),
            payload: None,
        });
    }
    let mixsongid = opt_i64(&[&p.mixsongid, &p.album_audio_id]);
    let mut special_id = opt_i64(&[&p.special_id, &p.childrenid, &p.id]);
    let mut name = opt_str(&[
        &p.name,
        &p.song_name,
        &p.album_name,
        &p.playlist_name,
        &p.childrenname,
    ]);

    if (special_id.is_none() || name.is_none()) && mixsongid.is_some() {
        let lookup_id = match kind {
            0 => "comment.music",
            1 => "comment.album",
            _ => "comment.playlist",
        };
        let mut params = serde_json::to_value(p)?;
        if let Some(obj) = params.as_object_mut() {
            if let Some(id) = special_id {
                obj.insert("id".to_owned(), json!(id));
            }
            obj.insert("mixsongid".to_owned(), json!(mixsongid));
            obj.insert("page".to_owned(), json!(1));
            obj.insert("pagesize".to_owned(), json!(1));
        }
        let lookup = ctx.call_inner(lookup_id, params).await?;
        let (resolved_id, resolved_name) = extract_resolved_resource(&lookup);
        special_id = special_id.or(resolved_id);
        if name.is_none() {
            name = resolved_name;
        }
    }
    let Some(special_id) = special_id else {
        return Err(Error::Api {
            code: 0,
            message: "failed to resolve the comment resource id; pass mixsongid or special_id"
                .to_owned(),
            payload: None,
        });
    };

    // content payload (compact: drop null / empty-string fields)
    let mut content_data = serde_json::Map::new();
    content_data.insert("content".to_owned(), json!(content));
    if let Some(mixsongid) = mixsongid {
        content_data.insert("album_audio_id".to_owned(), json!(mixsongid));
    }
    if let Some(title) = opt_str(&[&p.title]) {
        content_data.insert("title".to_owned(), json!(title));
    }
    if let Some(extdata) = opt_str(&[&p.content_extdata]) {
        content_data.insert("extdata".to_owned(), json!(extdata));
    }
    content_data.insert(
        "images".to_owned(),
        p.images
            .clone()
            .filter(serde_json::Value::is_array)
            .unwrap_or_else(|| json!([])),
    );
    let content_data = compact(&content_data);
    let data = serde_json::to_string(&json!({ "data": content_data }))?;

    let mut query = QueryRecord::new();
    query.set("r", "commentsv3/add");
    query.set("code", code);
    query.set("childrenid", special_id);
    query.set_opt("childrenname", name);
    query.set("kugouid", ctx.userid);
    query.set(
        "ver",
        p.ver
            .as_ref()
            .and_then(FlexI64::value)
            .filter(|v| *v != 0)
            .unwrap_or(6),
    );
    query.set("clienttoken", ctx.token.clone());
    query.set("appid", ctx.appid);
    query.set("clientver", ctx.clientver);
    query.set("mid", ctx.mid.clone());
    query.set("clienttime", ctx.clienttime);
    query.set(
        "key",
        ctx.sign_params_key(format!("{}{}{}", ctx.clienttime, ctx.mid, data)),
    );
    query.set("uuid", ctx.uuid.clone());
    query.set("dfid", ctx.dfid.clone());
    query.set_opt("source", p.source.as_ref().and_then(FlexI64::value));
    comment_request(ctx, query, crate::http::Method::Post, Some(data)).await
}

fn comment_send_handler<'a, const KIND: u8>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let code = match KIND {
            0 => SONG_COMMENT_CODE,
            1 => ALBUM_COMMENT_CODE,
            _ => PLAYLIST_COMMENT_CODE,
        };
        send_comment(ctx, p, code, KIND).await
    })
}

fn comment_floor_send_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let content = p.content.clone().unwrap_or_default();
        if content.trim().is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "content must not be empty".to_owned(),
                payload: None,
            });
        }
        let special_id =
            opt_i64(&[&p.special_id, &p.childrenid, &p.id]).ok_or_else(|| Error::Api {
                code: 0,
                message: "special_id must not be empty".to_owned(),
                payload: None,
            })?;
        let tid = opt_i64(&[&p.tid]).ok_or_else(|| Error::Api {
            code: 0,
            message: "tid must not be empty".to_owned(),
            payload: None,
        })?;
        let code = resolve_comment_code(p);
        let mut name = opt_str(&[
            &p.name,
            &p.song_name,
            &p.album_name,
            &p.playlist_name,
            &p.childrenname,
        ]);
        if name.is_none() {
            let mut params = serde_json::to_value(p)?;
            if let Some(obj) = params.as_object_mut() {
                obj.insert("special_id".to_owned(), json!(special_id));
                obj.insert("code".to_owned(), json!(code));
                obj.insert("page".to_owned(), json!(1));
                obj.insert("pagesize".to_owned(), json!(1));
            }
            let lookup = ctx.call_inner("comment.floor", params).await?;
            let (_, resolved_name) = extract_resolved_resource(&lookup);
            name = resolved_name;
        }

        let pid = p.pid.as_ref().and_then(FlexI64::value).unwrap_or(0);
        let is_top_level = p
            .is_t
            .as_ref()
            .or(p.isT.as_ref())
            .and_then(FlexI64::value)
            .unwrap_or(i64::from(pid == 0));
        let reply_user_name = opt_str(&[&p.reply_user_name, &p.puser]);
        let reply_content = opt_str(&[&p.reply_content, &p.pcontent]);
        let final_content =
            if let (Some(user), Some(text)) = (reply_user_name.as_ref(), reply_content.as_ref()) {
                if content.contains("//@") {
                    content.clone()
                } else {
                    format!("{content}//@{user}:{text}")
                }
            } else {
                content.clone()
            };

        let mut query = QueryRecord::new();
        query.set("r", "commentsv2/reply");
        query.set("code", code);
        query.set("childrenid", special_id);
        query.set_opt("childrenname", name);
        query.set("kugouid", ctx.userid);
        query.set(
            "ver",
            p.ver
                .as_ref()
                .and_then(FlexI64::value)
                .filter(|v| *v != 0)
                .unwrap_or(6),
        );
        query.set("clienttoken", ctx.token.clone());
        query.set("appid", ctx.appid);
        query.set("clientver", ctx.clientver);
        query.set("mid", ctx.mid.clone());
        query.set("clienttime", ctx.clienttime);
        query.set(
            "key",
            ctx.sign_params_key(format!("{}{}", ctx.clienttime, ctx.mid)),
        );
        query.set("uuid", ctx.uuid.clone());
        query.set("dfid", ctx.dfid.clone());
        query.set_opt("extdata", p.extdata.clone().filter(|e| !e.is_empty()));
        query.set("content", final_content);
        query.set("tid", tid);
        query.set("is_t", is_top_level);
        query.set("pid", pid);
        query.set_opt("source", p.source.as_ref().and_then(FlexI64::value));
        comment_request(ctx, query, crate::http::Method::Post, None).await
    })
}

fn comment_music_del_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a CommentParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let cid = opt_i64(&[&p.cid, &p.comment_id]).ok_or_else(|| Error::Api {
            code: 0,
            message: "cid (comment id) must not be empty".to_owned(),
            payload: None,
        })?;
        let mixsongid = opt_i64(&[&p.mixsongid, &p.album_audio_id]);
        let mut special_id = opt_i64(&[&p.special_id, &p.childrenid]);
        if special_id.is_none() && mixsongid.is_some() {
            let mut params = serde_json::to_value(p)?;
            if let Some(obj) = params.as_object_mut() {
                obj.insert("mixsongid".to_owned(), json!(mixsongid));
                obj.insert("page".to_owned(), json!(1));
                obj.insert("pagesize".to_owned(), json!(1));
            }
            let lookup = ctx.call_inner("comment.music", params).await?;
            let (resolved_id, _) = extract_resolved_resource(&lookup);
            special_id = resolved_id;
        }
        let Some(special_id) = special_id else {
            return Err(Error::Api {
                code: 0,
                message: "failed to resolve the comment resource id; pass special_id or mixsongid"
                    .to_owned(),
                payload: None,
            });
        };

        let mut query = QueryRecord::new();
        query.set("r", "commentsv2/delcomment");
        query.set("code", resolve_comment_code(p));
        query.set("childrenid", special_id);
        query.set("cid", cid);
        query.set_opt("tid", p.tid.as_ref().and_then(FlexI64::value));
        query.set("kugouid", ctx.userid);
        query.set(
            "ver",
            p.ver
                .as_ref()
                .and_then(FlexI64::value)
                .filter(|v| *v != 0)
                .unwrap_or(6),
        );
        query.set("clienttoken", ctx.token.clone());
        query.set("appid", ctx.appid);
        query.set("clientver", ctx.clientver);
        query.set("mid", ctx.mid.clone());
        query.set("clienttime", ctx.clienttime);
        query.set(
            "key",
            ctx.sign_params_key(format!("{}{}", ctx.clienttime, ctx.mid)),
        );
        query.set("uuid", ctx.uuid.clone());
        query.set("dfid", ctx.dfid.clone());
        comment_request(ctx, query, crate::http::Method::Get, None).await
    })
}

impl crate::api::CommentApi<'_> {
    /// Song barrage list (`song.barrage`).
    pub async fn song_barrage(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("song.barrage", params).await
    }

    /// Send a song barrage (`song.barrage.send`).
    pub async fn song_barrage_send(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("song.barrage.send", params).await
    }

    /// Video barrage list (`video.barrage`).
    pub async fn video_barrage(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("video.barrage", params).await
    }

    /// Send a video barrage (`video.barrage.send`).
    pub async fn video_barrage_send(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("video.barrage.send", params).await
    }

    /// Floor (reply-thread) comments (`comment.floor`).
    pub async fn floor(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("comment.floor", params).await
    }

    /// Send a song comment (`comment.music.send`).
    pub async fn music_send(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("comment.music.send", params).await
    }

    /// Send an album comment (`comment.album.send`).
    pub async fn album_send(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("comment.album.send", params).await
    }

    /// Send a playlist comment (`comment.playlist.send`).
    pub async fn playlist_send(&self, params: CommentParams) -> Result<Value> {
        self.client
            .call_value("comment.playlist.send", params)
            .await
    }

    /// Reply to a floor comment (`comment.floor.send`).
    pub async fn floor_send(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("comment.floor.send", params).await
    }

    /// Delete a comment (`comment.music.del`).
    pub async fn music_del(&self, params: CommentParams) -> Result<Value> {
        self.client.call_value("comment.music.del", params).await
    }
}
