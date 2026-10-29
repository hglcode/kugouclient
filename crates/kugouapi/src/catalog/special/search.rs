//! Search-domain special endpoints: multi-type `search` dispatch, mixed
//! search, IP music, concept-edition cards, channel subscribe and playlist
//! import.
//!
//! Ported from `special/search.ts`.

use serde_json::{Value, json};

use crate::catalog::ctx::{GatewayRequest, SignControl};
use crate::catalog::def::{Built, EndpointSpec, handler_endpoint, simple_endpoint, spec};
use crate::error::{Error, Result};
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;
use crate::sign::md5;

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

const SEARCH_TYPES: [&str; 6] = ["special", "lyric", "song", "album", "author", "mv"];

kugou_params! {
    /// Parameters for `search` (multi-type dispatch).
    pub SearchParams {
        required {}
        optional {
            /// Result type: song (default) / album / special / lyric / author / mv.
            search_type: String => "type",
            /// Primary keyword (`keywords` wins over `keyword`).
            keywords: String => "keywords",
            /// Fallback keyword.
            keyword: String => "keyword",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Error correction flag.
            iscorrection: FlexI64 => "iscorrection",
            /// Privilege filter.
            privilegefilter: FlexI64 => "privilegefilter",
            /// Area code.
            area_code: FlexI64 => "area_code",
            /// `<em>` highlight tag.
            tag: String => "tag",
            /// Album category.
            category: String => "category",
            /// Album sort: 0 = all, 1 = newest.
            sorttype: FlexI64 => "sorttype",
            /// Album search-song flag.
            searchsong: FlexI64 => "searchsong",
        }
    }
}

kugou_params! {
    /// Parameters for `search.mixed`.
    pub SearchMixedParams {
        required {
            /// Search keyword.
            keyword: String => "keyword",
        }
        optional {
            /// Area code (default 1).
            area_code: FlexI64 => "area_code",
            /// Error correction flag.
            iscorrection: FlexI64 => "iscorrection",
            /// Pagination cursor.
            cursor: FlexI64 => "cursor",
            /// Client version override.
            clientver: FlexI64 => "clientver",
            /// VIP status.
            vip_status: FlexI64 => "vip_status",
        }
    }
}

kugou_params! {
    /// Parameters for `ip` (IP music).
    pub IpParams {
        required {
            /// IP id.
            id: FlexI64 => "id",
        }
        optional {
            /// Resource type: `audios` (default) / `albums` / `videos` / `author_list`.
            resource_type: String => "type",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `top.card.youth`.
    pub TopCardYouthParams {
        required {}
        optional {
            /// Card id (3103 routes to `tag_card_recommend` when no tagid).
            card_id: FlexI64 => "card_id",
            /// Tag id.
            tagid: FlexI64 => "tagid",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `youth.channel.sub`.
    pub YouthChannelSubParams {
        required {
            /// Channel id.
            global_collection_id: String => "global_collection_id",
        }
        optional {
            /// `0` unsubscribes (DELETE), anything else subscribes (POST).
            t: FlexI64 => "t",
        }
    }
}

kugou_params! {
    /// Parameters for `import.playlist`.
    pub ImportPlaylistParams {
        required {}
        optional {
            /// Operation: `add_task` (default) / `submit_img` / `task_count` /
            /// `query_task_status` / `query_task`.
            operation: String => "operation",
            /// Import source (default `3`).
            source: FlexI64 => "source",
            /// Task type (`0` = URL import).
            task_type: FlexI64 => "task_type",
            /// Source URL (`task_type` `0`).
            url: String => "url",
            /// Target list id.
            listid: FlexI64 => "listid",
            /// Target list name.
            list_name: String => "list_name",
            /// Task serial number.
            task_sn: String => "task_sn",
            /// Image payload (dataURL or base64).
            img_base64: String => "img_base64",
            /// Task classify (default 1).
            classify: FlexI64 => "classify",
            /// Task ids (array / JSON-array string / comma separated).
            ids: Value => "ids",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Whether to show missed songs.
            show_missed: FlexI64 => "show_missed",
        }
    }
}

const EP_SEARCH: EndpointSpec = EndpointSpec {
    router: Some("complexsearch.kugou.com"),
    ..spec(
        "search",
        "search",
        crate::http::Method::Get,
        "/v2/search/song",
    )
};
const EP_SEARCH_MIXED: EndpointSpec = spec(
    "search.mixed",
    "search_mixed",
    crate::http::Method::Get,
    "/v3/search/mixed",
);
const EP_IP: EndpointSpec = spec(
    "ip",
    "ip",
    crate::http::Method::Post,
    "/openapi/v1/ip/audios",
);
const EP_TOP_CARD_YOUTH: EndpointSpec = spec(
    "top.card.youth",
    "top_card_youth",
    crate::http::Method::Post,
    "/youth/v1/song/single_card_recommend",
);
const EP_YOUTH_CHANNEL_SUB: EndpointSpec = spec(
    "youth.channel.sub",
    "youth_channel_sub",
    crate::http::Method::Delete,
    "/youth/v1/channel_un_subscribe",
);
const EP_IMPORT_PLAYLIST: EndpointSpec = spec(
    "import.playlist",
    "import_playlist",
    crate::http::Method::Post,
    "/assetservice/import/v1/add_task",
);

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        // ── search (type dispatch: song / album / others) ──
        simple_endpoint(&EP_SEARCH, |p: &SearchParams, ctx| {
            let kind = resolve_search_type(p);
            let keyword = resolve_keyword(p);
            let path = match kind.as_str() {
                "song" => "/v2/search/song",
                "album" => "/v1/search/album",
                _ => return search_other_built(&kind, p, keyword),
            };

            let mut query = QueryRecord::new();
            query.set("keyword", keyword);
            query.set("page", js::or_flex(&p.page, 1));
            query.set(
                "pagesize",
                js::or_flex(&p.page_size, if kind == "album" { 20 } else { 30 }),
            );
            query.set("platform", "AndroidFilter");
            query.set("iscorrection", js::unwrap_or_flex(&p.iscorrection, 1));
            if kind == "song" {
                query.set("privilegefilter", js::unwrap_or_flex(&p.privilegefilter, 0));
                query.set("area_code", js::or_flex(&p.area_code, 1));
                query.set("dopicfull", 1);
            } else {
                query.set(
                    "category",
                    p.category.clone().unwrap_or_else(|| "1".to_owned()),
                );
                query.set("sorttype", i64::from(js::is_one_opt(&p.sorttype)));
                query.set("searchsong", i64::from(js::is_one_opt(&p.searchsong)));
            }
            if let Some(tag) = &p.tag {
                query.set("tag", tag.clone());
            }
            if ctx.platform == crate::config::Platform::Lite {
                query.set("clientver", 201);
            }
            Built {
                path: Some(path.to_owned()),
                query,
                ..Built::default()
            }
        }),
        // ── /v3/search/mixed ──
        simple_endpoint(&EP_SEARCH_MIXED, |p: &SearchMixedParams, ctx| {
            let mut query = QueryRecord::new();
            if ctx.platform == crate::config::Platform::Lite {
                query.set("keyword", p.keyword.clone());
                query.set("platform", "AndroidFilter");
                query.set("tag", "em");
                query.set("area_code", js::or_flex(&p.area_code, 1));
                query.set("iscorrection", js::unwrap_or_flex(&p.iscorrection, 1));
                query.set("cursor", js::or_flex(&p.cursor, 0));
                query.set("apiver", 22);
                query.set("osversion", "15");
                query.set("ability", 1);
                query.set("clientver", js::or_flex(&p.clientver, 201));
                query.set(
                    "vip_status",
                    js::or_chain(
                        &[
                            p.vip_status.as_ref().and_then(FlexI64::value),
                            Some(ctx.vip_type),
                        ],
                        0,
                    ),
                );
            } else {
                let time = now_ms();
                query.set("ab_tag", 0);
                query.set("ability", 511);
                query.set("albumhide", 0);
                query.set("apiver", 22);
                query.set("area_code", 1);
                query.set("clientver", js::or_flex(&p.clientver, 20125));
                query.set("cursor", 0);
                query.set("is_gpay", 0);
                query.set("iscorrection", 1);
                query.set("keyword", p.keyword.clone());
                query.set("nocollect", 0);
                query.set("osversion", 16.5);
                query.set("platform", "IOSFilter");
                query.set("recver", 2);
                query.set("req_ai", 1);
                query.set(
                    "requestid",
                    format!(
                        "{}_0",
                        md5(&format!("bdaa53d04e7475feb9024164a47032f9{time}"))
                    ),
                );
                query.set("search_ability", 3);
                query.set("sec_aggre", 1);
                query.set("sec_aggre_bitmap", 0);
                query.set("style_type", 3);
                query.set("tag", "em");
            }
            Built {
                headers: vec![("kg-clienttimems".to_owned(), now_ms().to_string())],
                query,
                ..Built::default()
            }
        }),
        // ── /openapi/v1/ip/{type} ──
        simple_endpoint(&EP_IP, |p: &IpParams, _| {
            let kind = match p.resource_type.as_deref() {
                Some(t) if ["audios", "albums", "videos", "author_list"].contains(&t) => t,
                _ => "audios",
            };
            let body = json!({
                "is_publish": 1,
                "ip_id": p.id.value().unwrap_or(0),
                "sort": 3,
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30),
                "query": 1,
            });
            Built {
                path: Some(format!("/openapi/v1/ip/{kind}")),
                json: Some(body),
                ..Built::default()
            }
        }),
        // ── concept-edition song card recommend ──
        simple_endpoint(&EP_TOP_CARD_YOUTH, |p: &TopCardYouthParams, _| {
            let tag_branch =
                p.card_id.as_ref().and_then(FlexI64::value) == Some(3103) && p.tagid.is_none();
            let path = if tag_branch {
                "/youth/v1/song/tag_card_recommend"
            } else {
                "/youth/v1/song/single_card_recommend"
            };
            let mut query = QueryRecord::new();
            query.set("card_id", js::or_flex(&p.card_id, 3005));
            query.set("area_code", 1);
            query.set("platform", "ios");
            query.set("module_id", 1);
            query.set("ver", "v2");
            query.set("pagesize", js::unwrap_or_flex(&p.page_size, 30));
            query.set("clientver", 12500);
            let body = json!({
                "tagid": js::unwrap_or_flex(&p.tagid, 0),
                "u_info": "",
                "source_mixsong": "",
            });
            Built {
                path: Some(path.to_owned()),
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        // ── concept-edition channel subscribe / unsubscribe ──
        handler_endpoint(&EP_YOUTH_CHANNEL_SUB, youth_channel_sub_handler),
        // ── playlist import (assetservice, operation dispatch) ──
        handler_endpoint(&EP_IMPORT_PLAYLIST, import_playlist_handler),
    ]
}

fn resolve_search_type(p: &SearchParams) -> String {
    match p.search_type.as_deref() {
        Some(t) if SEARCH_TYPES.contains(&t) => t.to_owned(),
        _ => "song".to_owned(),
    }
}

fn resolve_keyword(p: &SearchParams) -> String {
    let primary = p.keywords.clone().filter(|k| !k.is_empty());
    match primary {
        Some(k) => k,
        None => p.keyword.clone().unwrap_or_default(),
    }
}

fn search_other_built(kind: &str, p: &SearchParams, keyword: String) -> Built {
    let mut query = QueryRecord::new();
    query.set("albumhide", 0);
    query.set("iscorrection", 1);
    query.set("keyword", keyword);
    query.set("nocollect", 0);
    query.set("page", js::or_flex(&p.page, 1));
    query.set("pagesize", js::or_flex(&p.page_size, 30));
    query.set("platform", "AndroidFilter");
    Built {
        path: Some(format!("/v1/search/{kind}")),
        query,
        ..Built::default()
    }
}

fn youth_channel_sub_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a YouthChannelSubParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let unsubscribe = p.t.as_ref().and_then(FlexI64::value) == Some(0);
        let path = if unsubscribe {
            "/youth/v1/channel_un_subscribe"
        } else {
            "/youth/v1/channel_subscribe"
        };
        let mut query = QueryRecord::new();
        query.set("global_collection_id", p.global_collection_id.clone());
        query.set("source", 1);
        ctx.request(
            path,
            GatewayRequest {
                method: Some(if unsubscribe {
                    crate::http::Method::Delete
                } else {
                    crate::http::Method::Post
                }),
                query: Some(query),
                ..GatewayRequest::default()
            },
        )
        .await
    })
}

fn import_playlist_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a ImportPlaylistParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let operation = p.operation.clone().unwrap_or_else(|| "add_task".to_owned());

        let common = |json: Value| GatewayRequest {
            method: Some(crate::http::Method::Post),
            query: Some(import_common_query(ctx)),
            json: Some(json),
            headers: vec![(
                "Content-Type".to_owned(),
                "application/json;charset=utf-8".to_owned(),
            )],
            sign: SignControl::Android,
            clear_params: true,
            ..GatewayRequest::default()
        };

        match operation.as_str() {
            "add_task" => {
                let task_type = js::or_flex(&p.task_type, 0);
                let mut data = serde_json::Map::new();
                data.insert("userid".to_owned(), json!(ctx.userid));
                data.insert("token".to_owned(), json!(ctx.token));
                data.insert("source".to_owned(), json!(js::or_flex(&p.source, 3)));
                data.insert("task_type".to_owned(), json!(task_type));
                if task_type == 0 {
                    data.insert("url".to_owned(), json!(p.url.clone().unwrap_or_default()));
                } else {
                    data.insert("listid".to_owned(), json!(js::or_flex(&p.listid, 0)));
                    if let Some(list_name) = &p.list_name {
                        data.insert("list_name".to_owned(), json!(list_name));
                    }
                    data.insert(
                        "task_sn".to_owned(),
                        json!(
                            p.task_sn
                                .clone()
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| format!("{}{}", ctx.userid, now_ms()))
                        ),
                    );
                }
                ctx.request(
                    "/assetservice/import/v1/add_task",
                    common(Value::Object(data)),
                )
                .await
            }
            "submit_img" => {
                let stripped = strip_data_url(&p.img_base64.clone().unwrap_or_default());
                ctx.request(
                    "/assetservice/import/v1/submit_img",
                    common(json!({
                        "userid": ctx.userid,
                        "token": ctx.token,
                        "img_base64": stripped,
                        "task_sn": p.task_sn.clone().unwrap_or_default(),
                    })),
                )
                .await
            }
            "task_count" => {
                ctx.request(
                    "/assetservice/import/v1/task_count",
                    common(json!({
                        "userid": ctx.userid,
                        "token": ctx.token,
                        "classify": js::or_flex(&p.classify, 1),
                    })),
                )
                .await
            }
            "query_task_status" => {
                let ids = parse_ids(p.ids.as_ref());
                ctx.request(
                    "/assetservice/import/v1/query_task_status",
                    common(json!({ "userid": ctx.userid, "token": ctx.token, "ids": ids })),
                )
                .await
            }
            "query_task" => {
                ctx.request(
                    "/pubsongs/v1/query_task",
                    common(json!({
                        "userid": ctx.userid,
                        "token": ctx.token,
                        "listid": js::or_flex(&p.listid, 0).to_string(),
                        "page": (js::or_flex(&p.page, 1)).max(1),
                        "pagesize": (js::or_flex(&p.page_size, 30)).max(1),
                        "show_missed": i32::from(js::unwrap_or_flex(&p.show_missed, 1) != 0),
                    })),
                )
                .await
            }
            other => Err(Error::Api {
                code: 0,
                message: format!("unsupported operation: {other}"),
                payload: None,
            }),
        }
    })
}

fn import_common_query(ctx: &CallCtx<'_>) -> QueryRecord {
    let mut query = QueryRecord::new();
    query.set("appid", ctx.appid.to_string());
    query.set("clientver", ctx.clientver.to_string());
    query.set("clienttime", ctx.clienttime.to_string());
    query.set("mid", ctx.mid.clone());
    query.set("uuid", ctx.uuid.clone());
    query.set("dfid", ctx.dfid.clone());
    query
}

/// Strip a `data:image/...;base64,` prefix.
fn strip_data_url(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("data:")
        && let Some(idx) = rest.find(',')
    {
        let mime = &rest[..idx];
        if mime.starts_with("image/") && mime.ends_with(";base64") {
            return rest[idx + 1..].to_owned();
        }
    }
    input.to_owned()
}

/// `parseIds`: arrays / JSON-array strings / comma-separated strings → ids.
fn parse_ids(value: Option<&Value>) -> Vec<i64> {
    match value {
        Some(Value::Array(items)) => items.iter().filter_map(id_from_value).collect(),
        Some(Value::String(text)) => {
            if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(text) {
                return items.iter().filter_map(id_from_value).collect();
            }
            text.split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect()
        }
        Some(other) => id_from_value(other).map(|id| vec![id]).unwrap_or_default(),
        None => Vec::new(),
    }
}

fn id_from_value(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn now_ms() -> u64 {
    crate::crypto::now_ms()
}

impl crate::api::SearchApi<'_> {
    /// Typed song search — dispatches `search` with song semantics and decodes
    /// the typed [`crate::api::SearchData`].
    pub async fn song(
        &self,
        params: crate::api::SearchSongParams,
    ) -> Result<crate::api::SearchData> {
        self.client.call_typed("search", params).await
    }

    /// Multi-type search (song / album / special / lyric / author / mv).
    pub async fn search(&self, params: SearchParams) -> Result<Value> {
        self.client.call_value("search", params).await
    }

    /// Mixed search (`search.mixed`).
    pub async fn mixed(&self, params: SearchMixedParams) -> Result<Value> {
        self.client.call_value("search.mixed", params).await
    }

    /// IP music (`ip`).
    pub async fn ip(&self, params: IpParams) -> Result<Value> {
        self.client.call_value("ip", params).await
    }

    /// Concept-edition card recommend (`top.card.youth`).
    pub async fn top_card_youth(&self, params: TopCardYouthParams) -> Result<Value> {
        self.client.call_value("top.card.youth", params).await
    }

    /// Channel subscribe / unsubscribe (`youth.channel.sub`).
    pub async fn youth_channel_sub(&self, params: YouthChannelSubParams) -> Result<Value> {
        self.client.call_value("youth.channel.sub", params).await
    }

    /// Playlist import (`import.playlist`).
    pub async fn import_playlist(&self, params: ImportPlaylistParams) -> Result<Value> {
        self.client.call_value("import.playlist", params).await
    }
}
