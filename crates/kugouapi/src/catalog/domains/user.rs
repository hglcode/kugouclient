//! User-domain endpoints (blacklist / favourites / follows / history / info …).
//!
//! Ported from `domains/user.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, Salt, simple_endpoint, spec};
use crate::crypto::CipherInput;
use crate::error::Result;
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `blacklist.list`.
    pub BlacklistListParams {
        required {}
        optional {
            /// Module id for the `KG-TID` header (default 473).
            module_id: FlexI64 => "moduleId",
            /// Blacklist source.
            source: FlexI64 => "source",
            /// Label: song (default) / singer.
            label: String => "label",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size (max 500).
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `favorite.count`.
    pub FavoriteCountParams {
        required {
            /// Comma-separated mixsong ids.
            mixsongids: String => "mixsongids",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `playhistory.upload`.
    pub PlayhistoryUploadParams {
        required {
            /// Mixsong id.
            mxid: FlexI64 => "mxid",
        }
        optional {
            /// Operation time (UNIX seconds).
            time: FlexI64 => "time",
            /// Play count.
            pc: FlexI64 => "pc",
        }
    }
}

kugou_params! {
    /// Parameters for `user.detail`.
    pub UserDetailParams {
        required {}
        optional {}
    }
}

kugou_params! {
    /// Parameters for `user.follow.add` / `user.follow.del`.
    pub UserFollowActionParams {
        required {
            /// Target user id.
            tuid: FlexI64 => "tuid",
        }
        optional {
            /// Follow source.
            source: FlexI64 => "source",
        }
    }
}

kugou_params! {
    /// Parameters for `user.follow.message`.
    pub UserFollowMessageParams {
        required {
            /// Peer user id.
            id: FlexI64 => "id",
        }
        optional {
            /// Max message id.
            maxid: FlexI64 => "maxid",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Message tag override.
            tag: String => "tag",
        }
    }
}

kugou_params! {
    /// Parameters for `user.history`.
    pub UserHistoryParams {
        required {}
        optional {
            /// Break-point parameter.
            bp: String => "bp",
        }
    }
}

kugou_params! {
    /// Parameters for `user.listen`.
    pub UserListenParams {
        required {}
        optional {
            /// List type.
            list_type: FlexI64 => "type",
        }
    }
}

kugou_params! {
    /// Parameters for `user.playlist`.
    pub UserPlaylistParams {
        required {}
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `user.purchased.albums` / `user.purchased.songs`.
    pub UserPurchasedParams {
        required {}
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `user.video.collect` / `user.video.love`.
    pub UserVideoParams {
        required {}
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

macro_rules! follow_headers {
    ($ctx:expr) => {{
        let clientver = if $ctx.platform == crate::config::Platform::Lite {
            11440
        } else {
            $ctx.clientver
        };
        vec![
            ("KG-FAKE-TYPE".to_owned(), "0,0".to_owned()),
            ("KG-FAKE-SUBTYPE".to_owned(), "0,0".to_owned()),
            ("KG-RFB".to_owned(), "0".to_owned()),
            (
                "User-Agent".to_owned(),
                format!("Android15-AndroidPhone-{clientver}-201-0-User-wifi"),
            ),
            ("KG-FAKE".to_owned(), $ctx.userid.to_string()),
        ]
    }};
}

/// The lite-aware appid used by the relation endpoints.
fn relation_appid(ctx: &CallCtx<'_>) -> u64 {
    if ctx.platform == crate::config::Platform::Lite {
        3116
    } else {
        ctx.appid
    }
}

/// The lite-aware clientver used by the relation endpoints.
fn relation_clientver(ctx: &CallCtx<'_>) -> u64 {
    if ctx.platform == crate::config::Platform::Lite {
        11440
    } else {
        ctx.clientver
    }
}

const EP_BLACKLIST_LIST: EndpointSpec = EndpointSpec {
    origin: Some("https://relationuser.kugou.com"),
    ..spec(
        "blacklist.list",
        "blacklist_list",
        crate::http::Method::Post,
        "/v2/get_list_items",
    )
};
const EP_FAVORITE_COUNT: EndpointSpec = spec(
    "favorite.count",
    "favorite_count",
    crate::http::Method::Get,
    "/count/v1/audio/mget_collect",
);
const EP_PLAYHISTORY_UPLOAD: EndpointSpec = spec(
    "playhistory.upload",
    "playhistory_upload",
    crate::http::Method::Post,
    "/playhistory/v1/upload_songs",
);
const EP_USER_DETAIL: EndpointSpec = EndpointSpec {
    router: Some("usercenter.kugou.com"),
    ..spec(
        "user.detail",
        "user_detail",
        crate::http::Method::Post,
        "/v3/get_my_info",
    )
};
const EP_USER_FANS: EndpointSpec = EndpointSpec {
    origin: Some("https://relationuser.kugou.com"),
    salt: Salt::None,
    clear_params: true,
    ..spec(
        "user.fans",
        "user_fans",
        crate::http::Method::Post,
        "/v2/fans_list",
    )
};
const EP_USER_FOLLOW: EndpointSpec = EndpointSpec {
    router: Some("relationuser.kugou.com"),
    ..spec(
        "user.follow",
        "user_follow",
        crate::http::Method::Post,
        "/v4/follow_list",
    )
};
const EP_USER_FOLLOW_ADD: EndpointSpec = EndpointSpec {
    router: Some("relation.user.kugou.com"),
    salt: Salt::None,
    clear_params: true,
    ..spec(
        "user.follow.add",
        "user_follow_add",
        crate::http::Method::Post,
        "/v2/follow",
    )
};
const EP_USER_FOLLOW_DEL: EndpointSpec = EndpointSpec {
    router: Some("relationuser.kugou.com"),
    salt: Salt::None,
    clear_params: true,
    ..spec(
        "user.follow.del",
        "user_follow_del",
        crate::http::Method::Post,
        "/v1/unfollow",
    )
};
const EP_USER_FOLLOW_MESSAGE: EndpointSpec = spec(
    "user.follow.message",
    "user_follow_message",
    crate::http::Method::Get,
    "/msg.mobile/v3/msgtag/history",
);
const EP_USER_FRIENDS: EndpointSpec = EndpointSpec {
    origin: Some("https://relationuser.kugou.com"),
    salt: Salt::None,
    clear_params: true,
    ..spec(
        "user.friends",
        "user_friends",
        crate::http::Method::Post,
        "/v1/friends_list",
    )
};
const EP_USER_HISTORY: EndpointSpec = spec(
    "user.history",
    "user_history",
    crate::http::Method::Post,
    "/playhistory/v1/get_songs",
);
const EP_USER_INFO: EndpointSpec = EndpointSpec {
    origin: Some("http://relation.user.kugou.com"),
    ..spec(
        "user.info",
        "user_info",
        crate::http::Method::Post,
        "/v1/get_my_userinfo",
    )
};
const EP_USER_LISTEN: EndpointSpec = EndpointSpec {
    origin: Some("https://listenservice.kugou.com"),
    ..spec(
        "user.listen",
        "user_listen",
        crate::http::Method::Post,
        "/v2/get_list",
    )
};
const EP_USER_PLAYLIST: EndpointSpec = EndpointSpec {
    router: Some("cloudlist.service.kugou.com"),
    ..spec(
        "user.playlist",
        "user_playlist",
        crate::http::Method::Post,
        "/v7/get_all_list",
    )
};
const EP_USER_PURCHASED_ALBUMS: EndpointSpec = spec(
    "user.purchased.albums",
    "user_purchased_albums",
    crate::http::Method::Post,
    "/openapi/v1/copyright/get_album_goods",
);
const EP_USER_PURCHASED_SONGS: EndpointSpec = spec(
    "user.purchased.songs",
    "user_purchased_songs",
    crate::http::Method::Post,
    "/openapi/copyright/v1/audio/get_goods",
);
const EP_USER_UPDATE: EndpointSpec = EndpointSpec {
    origin: Some("http://update.user.kugou.com"),
    salt: Salt::None,
    clear_params: true,
    ..spec(
        "user.update",
        "user_update",
        crate::http::Method::Post,
        "/v1/update_userinfo",
    )
};
const EP_USER_VIDEO_COLLECT: EndpointSpec = spec(
    "user.video.collect",
    "user_video_collect",
    crate::http::Method::Post,
    "/collectservice/v2/collect_list_mixvideo",
);
const EP_USER_VIDEO_LOVE: EndpointSpec = spec(
    "user.video.love",
    "user_video_love",
    crate::http::Method::Get,
    "/m.comment.service/v1/get_user_like_video",
);
const EP_USER_VIP_DETAIL: EndpointSpec = EndpointSpec {
    origin: Some("https://kugouvip.kugou.com"),
    ..spec(
        "user.vip.detail",
        "user_vip_detail",
        crate::http::Method::Get,
        "/v1/get_union_vip",
    )
};

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_BLACKLIST_LIST, |p: &BlacklistListParams, ctx| {
            let is_singer = p.label.as_deref() == Some("singer");
            let source = js::or_chain(
                &[p.source.as_ref().and_then(FlexI64::value)],
                if is_singer { 4 } else { 3 },
            );
            let body = json!({
                "userid": ctx.userid,
                "source": source,
                "label": if is_singer { "singer" } else { "song" },
                "p": ctx.rsa_encrypt_raw(&CipherInput::Text(
                    serde_json::to_string(&json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                    }))
                    .unwrap_or_default(),
                )),
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30).min(500),
            });
            Built {
                json: Some(body),
                headers: vec![(
                    "KG-TID".to_owned(),
                    js::or_flex(&p.module_id, 473).to_string(),
                )],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_FAVORITE_COUNT, |p: &FavoriteCountParams, _| {
            let mut query = QueryRecord::new();
            query.set("mixsongids", p.mixsongids.clone());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_PLAYHISTORY_UPLOAD,
            |p: &PlayhistoryUploadParams, ctx| {
                let mut query = QueryRecord::new();
                query.set("plat", 3);
                let body = json!({
                    "songs": [
                        {
                            "mxid": p.mxid.value(),
                            "op": 1,
                            "ot": js::or_flex(&p.time, ctx.clienttime as i64),
                            "pc": js::or_flex(&p.pc, 1),
                        },
                    ],
                    "token": ctx.token,
                    "userid": ctx.userid,
                });
                Built {
                    json: Some(body),
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_USER_DETAIL, |_: &UserDetailParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("plat", 1);
            let body = json!({
                "visit_time": ctx.clienttime,
                "usertype": 1,
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "token": ctx.token,
                        "clienttime": ctx.clienttime,
                    })))
                    .to_uppercase(),
                "userid": ctx.userid,
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_FANS, |_: &NoParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("dfid", ctx.dfid.clone());
            let body = json!({
                "userid": ctx.userid,
                "dfid": ctx.dfid,
                "plat": 1,
                "nickimg": 1,
                "appid": relation_appid(ctx),
                "clientver": relation_clientver(ctx),
                "mid": ctx.mid,
                "clienttime": ctx.clienttime,
                "key": ctx.sign_params_key_with(
                    ctx.clienttime.to_string(),
                    Some(relation_appid(ctx)),
                    Some(relation_clientver(ctx)),
                ),
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                    })))
                    .to_uppercase(),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_FOLLOW, |_: &NoParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("plat", 1);
            let body = json!({
                "merge": 2,
                "need_iden_type": 1,
                "ext_params": "k_pic,jumptype,singerid,score",
                "userid": ctx.userid,
                "type": 0,
                "id_type": 0,
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                    })))
                    .to_uppercase(),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_FOLLOW_ADD, |p: &UserFollowActionParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("dfid", ctx.dfid.clone());
            let body = json!({
                "userid": ctx.userid,
                "source": js::or_flex(&p.source, 0),
                "clienttime": ctx.clienttime,
                "mid": ctx.mid,
                "key": ctx.sign_params_key_with(
                    ctx.clienttime.to_string(),
                    Some(relation_appid(ctx)),
                    Some(relation_clientver(ctx)),
                ),
                "dfid": ctx.dfid,
                "clientver": relation_clientver(ctx),
                "appid": relation_appid(ctx),
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                        "t_userid": p.tuid.value(),
                    })))
                    .to_uppercase(),
            });
            Built {
                json: Some(body),
                headers: follow_headers!(ctx),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_FOLLOW_DEL, |p: &UserFollowActionParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("dfid", ctx.dfid.clone());
            let body = json!({
                "userid": ctx.userid,
                "source": js::or_flex(&p.source, 0),
                "clienttime": ctx.clienttime,
                "mid": ctx.mid,
                "key": ctx.sign_params_key_with(
                    ctx.clienttime.to_string(),
                    Some(relation_appid(ctx)),
                    Some(relation_clientver(ctx)),
                ),
                "dfid": ctx.dfid,
                "clientver": relation_clientver(ctx),
                "appid": relation_appid(ctx),
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                        "t_userid": p.tuid.value(),
                    })))
                    .to_uppercase(),
            });
            Built {
                json: Some(body),
                headers: follow_headers!(ctx),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_USER_FOLLOW_MESSAGE,
            |p: &UserFollowMessageParams, ctx| {
                let mut query = QueryRecord::new();
                query.set("filter", 1);
                query.set("maxid", js::unwrap_or_flex(&p.maxid, 0));
                query.set("pagesize", js::unwrap_or_flex(&p.page_size, 30));
                query.set(
                    "tag",
                    p.tag.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| {
                        format!("chat:{}_{}", p.id.value().unwrap_or(0), ctx.userid)
                    }),
                );
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_USER_FRIENDS, |_: &NoParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("dfid", ctx.dfid.clone());
            let body = json!({
                "userid": ctx.userid,
                "dfid": ctx.dfid,
                "plat": 1,
                "nickimg": 1,
                "appid": relation_appid(ctx),
                "clientver": relation_clientver(ctx),
                "mid": ctx.mid,
                "clienttime": ctx.clienttime,
                "key": ctx.sign_params_key_with(
                    ctx.clienttime.to_string(),
                    Some(relation_appid(ctx)),
                    Some(relation_clientver(ctx)),
                ),
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                    })))
                    .to_uppercase(),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_HISTORY, |p: &UserHistoryParams, ctx| {
            let mut body = serde_json::Map::new();
            body.insert("token".to_owned(), json!(ctx.token));
            body.insert("userid".to_owned(), json!(ctx.userid));
            body.insert("source_classify".to_owned(), json!("app"));
            body.insert("to_subdivide_sr".to_owned(), json!(1));
            if let Some(bp) = &p.bp {
                body.insert("bp".to_owned(), json!(bp));
            }
            Built {
                json: Some(Value::Object(body)),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_INFO, |_: &NoParams, ctx| {
            let body = json!({
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                    })))
                    .to_uppercase(),
                "appid": ctx.appid,
                "mid": ctx.mid,
                "clientver": ctx.clientver,
                "source": 0,
                "clienttime": ctx.clienttime,
                "uuid": "-",
                "userid": ctx.userid,
                "key": ctx.sign_params_key(ctx.clienttime),
            });
            Built {
                json: Some(body),
                headers: vec![("Host".to_owned(), "relation.user.kugou.com".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_LISTEN, |p: &UserListenParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("clienttime", ctx.clienttime);
            query.set("plat", 0);
            let body = json!({
                "t_userid": ctx.userid,
                "userid": ctx.userid,
                "list_type": js::or_flex(&p.list_type, 0),
                "area_code": 1,
                "cover": 2,
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Json(json!({
                        "clienttime": ctx.clienttime,
                        "token": ctx.token,
                    })))
                    .to_uppercase(),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_PLAYLIST, |p: &UserPlaylistParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("plat", 1);
            query.set("userid", ctx.userid);
            query.set("token", ctx.token.clone());
            let body = json!({
                "userid": ctx.userid,
                "token": ctx.token,
                "total_ver": 979,
                "type": 2,
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_PURCHASED_ALBUMS, |p: &UserPurchasedParams, ctx| {
            let body = json!({
                "appid": ctx.appid,
                "userid": ctx.userid,
                "token": ctx.token,
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 15),
                "clientver": ctx.clientver.to_string(),
                "deleted": 0,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_PURCHASED_SONGS, |p: &UserPurchasedParams, ctx| {
            let body = json!({
                "appid": ctx.appid,
                "userid": ctx.userid,
                "token": ctx.token,
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 50),
                "clientver": ctx.clientver.to_string(),
                "deleted": 0,
                "need_audio_info": 1,
                "area_code": "1",
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_UPDATE, |_: &NoParams, ctx| {
            let body = json!({
                "clienttime": ctx.clienttime,
                "appid": relation_appid(ctx),
                "clientver": relation_clientver(ctx),
                "mid": ctx.mid,
                "uuid": "-",
                "userid": ctx.userid,
                "key": ctx.sign_params_key_with(
                    ctx.clienttime,
                    Some(relation_appid(ctx)),
                    Some(relation_clientver(ctx)),
                ),
                "p": ctx
                    .rsa_encrypt_raw(&CipherInput::Text(
                        serde_json::to_string(&json!({
                            "clienttime": ctx.clienttime,
                            "token": ctx.token,
                        }))
                        .unwrap_or_default(),
                    ))
                    .to_uppercase(),
                "data": {},
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_VIDEO_COLLECT, |p: &UserVideoParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("plat", 1);
            let body = json!({
                "userid": ctx.userid,
                "token": ctx.token,
                "page": js::unwrap_or_flex(&p.page, 1),
                "pagesize": js::unwrap_or_flex(&p.page_size, 30),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_VIDEO_LOVE, |p: &UserVideoParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("kugouid", ctx.userid);
            query.set("pagesize", js::unwrap_or_flex(&p.page_size, 30));
            query.set("load_video_info", 1);
            query.set("p", 1);
            query.set("plat", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_USER_VIP_DETAIL, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("busi_type", "concept");
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::UserApi<'_> {
    /// Blacklist (`blacklist.list`).
    pub async fn blacklist_list(&self, params: BlacklistListParams) -> Result<Value> {
        self.client.call_value("blacklist.list", params).await
    }

    /// Favourite counts (`favorite.count`).
    pub async fn favorite_count(&self, params: FavoriteCountParams) -> Result<Value> {
        self.client.call_value("favorite.count", params).await
    }

    /// Upload play history (`playhistory.upload`).
    pub async fn playhistory_upload(&self, params: PlayhistoryUploadParams) -> Result<Value> {
        self.client.call_value("playhistory.upload", params).await
    }

    /// My info (`user.detail`).
    pub async fn detail(&self) -> Result<Value> {
        self.client
            .call_value("user.detail", UserDetailParams::default())
            .await
    }

    /// Fans list (`user.fans`).
    pub async fn fans(&self) -> Result<Value> {
        self.client
            .call_value("user.fans", NoParams::default())
            .await
    }

    /// Follow list (`user.follow`).
    pub async fn follow(&self) -> Result<Value> {
        self.client
            .call_value("user.follow", NoParams::default())
            .await
    }

    /// Follow a user (`user.follow.add`).
    pub async fn follow_add(&self, params: UserFollowActionParams) -> Result<Value> {
        self.client.call_value("user.follow.add", params).await
    }

    /// Unfollow a user (`user.follow.del`).
    pub async fn follow_del(&self, params: UserFollowActionParams) -> Result<Value> {
        self.client.call_value("user.follow.del", params).await
    }

    /// Follow messages (`user.follow.message`).
    pub async fn follow_message(&self, params: UserFollowMessageParams) -> Result<Value> {
        self.client.call_value("user.follow.message", params).await
    }

    /// Friends list (`user.friends`).
    pub async fn friends(&self) -> Result<Value> {
        self.client
            .call_value("user.friends", NoParams::default())
            .await
    }

    /// Play history (`user.history`).
    pub async fn history(&self, params: UserHistoryParams) -> Result<Value> {
        self.client.call_value("user.history", params).await
    }

    /// My userinfo (`user.info`).
    pub async fn info(&self) -> Result<Value> {
        self.client
            .call_value("user.info", NoParams::default())
            .await
    }

    /// Listen lists (`user.listen`).
    pub async fn listen(&self, params: UserListenParams) -> Result<Value> {
        self.client.call_value("user.listen", params).await
    }

    /// My playlists (`user.playlist`).
    pub async fn playlist(&self, params: UserPlaylistParams) -> Result<Value> {
        self.client.call_value("user.playlist", params).await
    }

    /// Purchased albums (`user.purchased.albums`).
    pub async fn purchased_albums(&self, params: UserPurchasedParams) -> Result<Value> {
        self.client
            .call_value("user.purchased.albums", params)
            .await
    }

    /// Purchased songs (`user.purchased.songs`).
    pub async fn purchased_songs(&self, params: UserPurchasedParams) -> Result<Value> {
        self.client.call_value("user.purchased.songs", params).await
    }

    /// Update userinfo (`user.update`).
    pub async fn update(&self) -> Result<Value> {
        self.client
            .call_value("user.update", NoParams::default())
            .await
    }

    /// Collected videos (`user.video.collect`).
    pub async fn video_collect(&self, params: UserVideoParams) -> Result<Value> {
        self.client.call_value("user.video.collect", params).await
    }

    /// Liked videos (`user.video.love`).
    pub async fn video_love(&self, params: UserVideoParams) -> Result<Value> {
        self.client.call_value("user.video.love", params).await
    }

    /// VIP detail (`user.vip.detail`).
    pub async fn vip_detail(&self) -> Result<Value> {
        self.client
            .call_value("user.vip.detail", NoParams::default())
            .await
    }
}
