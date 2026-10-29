//! Discover-domain endpoints (rank / top / everyday / home / recommend).
//!
//! Ported from `domains/discover.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `ai.recommend`.
    pub AiRecommendParams {
        required {}
        optional {
            /// Comma-separated mixsong ids.
            album_audio_id: String => "album_audio_id",
        }
    }
}

kugou_params! {
    /// Parameters for `ai.recommend.song`.
    pub AiRecommendSongParams {
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
    /// Parameters for `brush`.
    pub BrushParams {
        required {}
        optional {
            /// VIP type override.
            vip_type: FlexI64 => "vipType",
            /// Song pool id.
            song_pool_id: FlexI64 => "song_pool_id",
            /// Feed mode.
            mode: String => "mode",
        }
    }
}

kugou_params! {
    /// Parameters for `everyday.history`.
    pub EverydayHistoryParams {
        required {}
        optional {
            /// History mode (default `list`).
            mode: String => "mode",
            /// Platform (default `ios`).
            platform: String => "platform",
            /// History name.
            history_name: String => "history_name",
            /// Date filter.
            date: String => "date",
        }
    }
}

kugou_params! {
    /// Parameters for `everyday.recommend` / `recommend.songs`.
    pub EverydayRecommendParams {
        required {}
        optional {
            /// Platform (default `ios`).
            platform: String => "platform",
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

kugou_params! {
    /// Parameters for `everyday.style.recommend`.
    pub EverydayStyleRecommendParams {
        required {}
        optional {
            /// Tag ids.
            tagids: String => "tagids",
        }
    }
}

kugou_params! {
    /// Parameters for `home.discover`.
    pub HomeDiscoverParams {
        required {}
        optional {
            /// Support mode (default `only_song`).
            support: String => "support",
            /// Recall type (default `song`).
            recall_type: String => "recall_type",
            /// Today play count.
            today_play_num: FlexI64 => "today_play_num",
            /// Page size (default 4).
            page_size: FlexI64 => "pagesize",
            /// Extra routing array.
            go_ky_extra: Value => "go_ky_extra",
        }
    }
}

kugou_params! {
    /// Parameters for `lastest.songs.listen`.
    pub LastestSongsListenParams {
        required {}
        optional {
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `rank.audio`.
    pub RankAudioParams {
        required {
            /// Rank id.
            rankid: FlexI64 => "rankid",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Rank category id.
            rank_cid: FlexI64 => "rank_cid",
        }
    }
}

kugou_params! {
    /// Parameters for `rank.info` / `rank.vol`.
    pub RankParams {
        required {
            /// Rank id.
            rankid: FlexI64 => "rankid",
        }
        optional {
            /// Rank category id.
            rank_cid: FlexI64 => "rank_cid",
            /// Album image flag.
            album_img: FlexI64 => "album_img",
            /// Zone filter.
            zone: String => "zone",
        }
    }
}

kugou_params! {
    /// Parameters for `rank.list`.
    pub RankListParams {
        required {}
        optional {
            /// Include songs flag (default 1).
            withsong: FlexI64 => "withsong",
        }
    }
}

kugou_params! {
    /// Parameters for `server.now`.
    pub ServerNowParams {
        required {}
        optional {}
    }
}

kugou_params! {
    /// Parameters for `top.album`.
    pub TopAlbumParams {
        required {}
        optional {
            /// Token override.
            token: String => "token",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `top.card`.
    pub TopCardParams {
        required {}
        optional {
            /// Card id (default 1).
            card_id: FlexI64 => "card_id",
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

kugou_params! {
    /// Parameters for `top.playlist`.
    pub TopPlaylistParams {
        required {}
        optional {
            /// User id override.
            userid: FlexI64 => "userid",
            /// Module id (default 1).
            module_id: FlexI64 => "module_id",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Include tags (default 1).
            withtag: FlexI64 => "withtag",
            /// Include songs (default 1).
            withsong: FlexI64 => "withsong",
            /// Sort (default 1).
            sort: FlexI64 => "sort",
            /// Category id.
            category_id: FlexI64 => "category_id",
        }
    }
}

kugou_params! {
    /// Parameters for `top.song`.
    pub TopSongParams {
        required {}
        optional {
            /// Rank id (default 21608).
            list_type: FlexI64 => "type",
            /// User id override.
            userid: FlexI64 => "userid",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

const EP_AI_RECOMMEND: EndpointSpec = EndpointSpec {
    router: Some("songlistairec.kugou.com"),
    clear_params: true,
    ..spec(
        "ai.recommend",
        "ai_recommend",
        crate::http::Method::Post,
        "/recommend",
    )
};
const EP_AI_RECOMMEND_SONG: EndpointSpec = spec(
    "ai.recommend.song",
    "ai_recommend_song",
    crate::http::Method::Post,
    "/concepts/v1/ai/recommend_song",
);
const EP_BRUSH: EndpointSpec = spec(
    "brush",
    "brush",
    crate::http::Method::Post,
    "/genesisapi/v1/newepoch_song_rec/feed",
);
const EP_EVERYDAY_FRIEND: EndpointSpec = EndpointSpec {
    origin: Some("https://acsing.service.kugou.com"),
    ..spec(
        "everyday.friend",
        "everyday_friend",
        crate::http::Method::Post,
        "/sing7/relation/json/v3/friend_rec_by_using_song_list",
    )
};
const EP_EVERYDAY_HISTORY: EndpointSpec = EndpointSpec {
    router: Some("everydayrec.service.kugou.com"),
    ..spec(
        "everyday.history",
        "everyday_history",
        crate::http::Method::Post,
        "/everyday/api/v1/get_history",
    )
};
const EP_EVERYDAY_RECOMMEND: EndpointSpec = EndpointSpec {
    router: Some("everydayrec.service.kugou.com"),
    ..spec(
        "everyday.recommend",
        "everyday_recommend",
        crate::http::Method::Post,
        "/everyday_song_recommend",
    )
};
const EP_EVERYDAY_STYLE_RECOMMEND: EndpointSpec = spec(
    "everyday.style.recommend",
    "everyday_style_recommend",
    crate::http::Method::Post,
    "/everydayrec.service/everyday_style_recommend",
);
const EP_HOME_DISCOVER: EndpointSpec = spec(
    "home.discover",
    "home_discover",
    crate::http::Method::Post,
    "/homediscoverrec/v1/client/home_discover_rec",
);
const EP_LASTEST_SONGS_LISTEN: EndpointSpec = spec(
    "lastest.songs.listen",
    "lastest_songs_listen",
    crate::http::Method::Post,
    "/playque/devque/v1/get_latest_songs",
);
const EP_RANK_AUDIO: EndpointSpec = spec(
    "rank.audio",
    "rank_audio",
    crate::http::Method::Post,
    "/openapi/kmr/v2/rank/audio",
);
const EP_RANK_INFO: EndpointSpec = spec(
    "rank.info",
    "rank_info",
    crate::http::Method::Get,
    "/ocean/v6/rank/info",
);
const EP_RANK_LIST: EndpointSpec = spec(
    "rank.list",
    "rank_list",
    crate::http::Method::Get,
    "/ocean/v6/rank/list",
);
const EP_RANK_TOP: EndpointSpec = spec(
    "rank.top",
    "rank_top",
    crate::http::Method::Get,
    "/mobileservice/api/v5/rank/rec_rank_list",
);
const EP_RANK_VOL: EndpointSpec = spec(
    "rank.vol",
    "rank_vol",
    crate::http::Method::Get,
    "/ocean/v6/rank/vol",
);
const EP_RECOMMEND_SONGS: EndpointSpec = EndpointSpec {
    router: Some("everydayrec.service.kugou.com"),
    ..spec(
        "recommend.songs",
        "recommend_songs",
        crate::http::Method::Post,
        "/everyday_song_recommend",
    )
};
const EP_SERVER_NOW: EndpointSpec = EndpointSpec {
    router: Some("usercenter.kugou.com"),
    ..spec(
        "server.now",
        "server_now",
        crate::http::Method::Post,
        "/v1/server_now",
    )
};
const EP_TOP_ALBUM: EndpointSpec = spec(
    "top.album",
    "top_album",
    crate::http::Method::Post,
    "/musicadservice/v1/mobile_newalbum_sp",
);
const EP_TOP_CARD: EndpointSpec = spec(
    "top.card",
    "top_card",
    crate::http::Method::Post,
    "/singlecardrec.service/v1/single_card_recommend",
);
const EP_TOP_IP: EndpointSpec = EndpointSpec {
    origin: Some("http://musicadservice.kugou.com"),
    ..spec(
        "top.ip",
        "top_ip",
        crate::http::Method::Post,
        "/v1/daily_recommend",
    )
};
const EP_TOP_PLAYLIST: EndpointSpec = EndpointSpec {
    router: Some("specialrec.service.kugou.com"),
    ..spec(
        "top.playlist",
        "top_playlist",
        crate::http::Method::Post,
        "/v2/special_recommend",
    )
};
const EP_TOP_SONG: EndpointSpec = spec(
    "top.song",
    "top_song",
    crate::http::Method::Post,
    "/musicadservice/container/v1/newsong_publish",
);

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_AI_RECOMMEND, |p: &AiRecommendParams, ctx| {
            let recommend_source: Vec<Value> = p
                .album_audio_id
                .clone()
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse::<i64>().ok().map(|n| json!({ "ID": n })))
                .collect();
            let body = json!({
                "platform": "ios",
                "clientver": ctx.clientver,
                "clienttime": ctx.clienttime,
                "userid": ctx.userid,
                "client_playlist": [],
                "source_type": 2,
                "playlist_ver": 2,
                "area_code": 1,
                "appid": ctx.appid,
                "key": ctx.sign_params_key(ctx.clienttime.to_string()),
                "mid": ctx.mid,
                "recommend_source": recommend_source,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_AI_RECOMMEND_SONG, |p: &AiRecommendSongParams, _| {
            let body = json!({
                "apiver": 2,
                "source": 2,
                "pagesize": js::or_flex(&p.page_size, 30),
                "page": js::or_flex(&p.page, 1),
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_BRUSH, |p: &BrushParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("sort_type", 1);
            query.set("platform", "ios");
            query.set("page", 1);
            query.set("content_ver", 4);
            query.set("clientver", 11850);
            let body = json!({
                "behaviors": [],
                "abtest": { "abtest": { "shuashua": { "commentcard": 2 } } },
                "personal_recommend_params": {
                    "userid": ctx.userid,
                    "appid": ctx.appid,
                    "playlist_ver": 2,
                    "clienttime": ctx.clienttime,
                    "mid": ctx.mid,
                    "new_sync_point": ctx.clienttime,
                    "module_id": 1,
                    "action": "login",
                    "vip_type": js::or_chain(
                        &[Some(ctx.vip_type), p.vip_type.as_ref().and_then(FlexI64::value)],
                        0,
                    ),
                    "vip_flags": 3,
                    "recommend_source_locked": 0,
                    "song_pool_id": js::or_flex(&p.song_pool_id, 0),
                    "callerid": 0,
                    "m_type": 1,
                    "kguid": ctx.userid,
                    "platform": "ios",
                    "area_code": 1,
                    "fakem": "ca981cfc583a4c37f28d2d49000013c16a0a",
                    "clientver": 11850,
                    "mode": p.mode.clone().unwrap_or_else(|| "normal".to_owned()),
                    "active_swtich": "on",
                    "key": ctx.sign_params_key(ctx.clienttime),
                },
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_EVERYDAY_FRIEND, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("channel", 130);
            query.set("isteen", 0);
            query.set("platform", 2);
            query.set("usemkv", 1);
            let body = json!({
                "list": [
                    {
                        "user_id": 853_927_886,
                        "mixsong_ids": [
                            290083753, 251724346, 571554587, 250126644, 208831644, 40328518,
                            250504076, 581706850, 318347675, 585258401, 288481998, 407414475,
                            28239430, 280584633, 291957521, 64556644, 243149863, 488725103,
                            32114153, 39951172, 29019580, 40397606, 327507651, 32029382,
                            32218359, 340353127, 276448762, 177071956, 100031397, 249251602,
                        ],
                    },
                ],
            });
            Built {
                json: Some(body),
                headers: vec![("pid".to_owned(), "126556797".to_owned())],
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_EVERYDAY_HISTORY, |p: &EverydayHistoryParams, _| {
            let mut query = QueryRecord::new();
            query.set("mode", p.mode.clone().unwrap_or_else(|| "list".to_owned()));
            query.set(
                "platform",
                p.platform.clone().unwrap_or_else(|| "ios".to_owned()),
            );
            query.set_opt("history_name", p.history_name.clone());
            query.set_opt("date", p.date.clone());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_EVERYDAY_RECOMMEND, |p: &EverydayRecommendParams, _| {
            let mut query = QueryRecord::new();
            query.set(
                "platform",
                p.platform.clone().unwrap_or_else(|| "ios".to_owned()),
            );
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_EVERYDAY_STYLE_RECOMMEND,
            |p: &EverydayStyleRecommendParams, _| {
                let mut query = QueryRecord::new();
                query.set("tagids", p.tagids.clone().unwrap_or_default());
                let body = json!({});
                Built {
                    json: Some(body),
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_HOME_DISCOVER, |p: &HomeDiscoverParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("module_key", "home_discover_rec");
            query.set("module_id", 1);
            query.set("area_code", 1);
            query.set("platform", "android");
            query.set("userid", ctx.userid);
            query.set("clientver", 20809);
            let go_ky_extra = p.go_ky_extra.clone().unwrap_or_else(|| {
                json!([
                    { "key": "network", "val": "2" },
                    { "key": "play_mode", "val": "2" },
                    { "key": "no_mv_ret", "val": "1" },
                ])
            });
            let body = json!({
                "support": p.support.clone().unwrap_or_else(|| "only_song".to_owned()),
                "userid": ctx.userid,
                "recall_type": p.recall_type.clone().unwrap_or_else(|| "song".to_owned()),
                "today_play_num": js::or_flex(&p.today_play_num, 0),
                "pagesize": js::or_flex(&p.page_size, 4),
                "go_ky_extra": go_ky_extra,
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_LASTEST_SONGS_LISTEN,
            |p: &LastestSongsListenParams, ctx| {
                let body = json!({
                    "area_code": "1",
                    "sources": ["pc", "mobile", "tv", "car"],
                    "userid": ctx.userid,
                    "ret_info": 1,
                    "token": ctx.token,
                    "pagesize": js::or_flex(&p.page_size, 30),
                });
                Built {
                    json: Some(body),
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_RANK_AUDIO, |p: &RankAudioParams, _| {
            let body = json!({
                "show_portrait_mv": 1,
                "show_type_total": 1,
                "filter_original_remarks": 1,
                "area_code": 1,
                "pagesize": js::or_flex(&p.page_size, 30),
                "rank_cid": js::or_flex(&p.rank_cid, 0),
                "type": 1,
                "page": js::or_flex(&p.page, 1),
                "rank_id": p.rankid.value(),
            });
            Built {
                json: Some(body),
                headers: vec![("kg-tid".to_owned(), "369".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_RANK_INFO, |p: &RankParams, _| {
            let mut query = QueryRecord::new();
            query.set("rank_cid", js::or_flex(&p.rank_cid, 0));
            query.set_opt("rankid", p.rankid.value());
            query.set("with_album_img", js::or_flex(&p.album_img, 1));
            query.set("zone", p.zone.clone().unwrap_or_default());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_RANK_LIST, |p: &RankListParams, _| {
            let mut query = QueryRecord::new();
            query.set("plat", 2);
            query.set("withsong", js::or_flex(&p.withsong, 1));
            query.set("parentid", 0);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_RANK_TOP, |_: &NoParams, _| Built::default()),
        simple_endpoint(&EP_RANK_VOL, |p: &RankParams, _| {
            let mut query = QueryRecord::new();
            query.set("rank_cid", js::or_flex(&p.rank_cid, 0));
            query.set_opt("rankid", p.rankid.value());
            query.set("ranktype", 1);
            query.set("type", 0);
            query.set("plat", 2);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_RECOMMEND_SONGS, |p: &EverydayRecommendParams, ctx| {
            let userid = js::or_chain(
                &[
                    p.userid.as_ref().and_then(FlexI64::value),
                    Some(ctx.userid as i64),
                ],
                0,
            );
            let body = json!({
                "platform": p.platform.clone().unwrap_or_else(|| "android".to_owned()),
                "userid": userid.to_string(),
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SERVER_NOW, |_: &ServerNowParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("plat", 3);
            let body = json!({ "token": ctx.token, "userid": ctx.userid });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_TOP_ALBUM, |p: &TopAlbumParams, ctx| {
            let body = json!({
                "apiver": 20,
                "token": p
                    .token
                    .clone()
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| ctx.token.clone()),
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30),
                "withpriv": 1,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_TOP_CARD, |p: &TopCardParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("card_id", js::or_flex(&p.card_id, 1));
            query.set("fakem", "60f7ebf1f812edbac3c63a7310001701760f");
            query.set("area_code", 1);
            query.set("platform", "ios");
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "platform": "android",
                "clienttime": ctx.clienttime,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "key": ctx.sign_params_key(ctx.clienttime),
                "fakem": "60f7ebf1f812edbac3c63a7310001701760f",
                "area_code": 1,
                "mid": ctx.mid,
                "uuid": "-",
                "client_playlist": [],
                "u_info": "a0c35cd40af564444b5584c2754dedec",
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_TOP_IP, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("clientver", 12349);
            query.set("area_code", 1);
            let body = json!({ "tags": {} });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_TOP_PLAYLIST, |p: &TopPlaylistParams, ctx| {
            let body = json!({
                "appid": ctx.appid,
                "mid": ctx.mid,
                "clientver": ctx.clientver,
                "platform": "android",
                "clienttime": ctx.clienttime,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "module_id": js::or_flex(&p.module_id, 1),
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30),
                "key": ctx.sign_params_key(ctx.clienttime.to_string()),
                "special_recommend": {
                    "withtag": js::or_flex(&p.withtag, 1),
                    "withsong": js::or_flex(&p.withsong, 1),
                    "sort": js::or_flex(&p.sort, 1),
                    "ugc": 1,
                    "is_selected": 0,
                    "withrecommend": 1,
                    "area_code": 1,
                    "categoryid": js::or_flex(&p.category_id, 0),
                },
                "req_multi": 1,
                "retrun_min": 5,
                "return_special_falg": 1,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_TOP_SONG, |p: &TopSongParams, ctx| {
            let body = json!({
                "rank_id": js::or_flex(&p.list_type, 21608),
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30),
                "tags": [],
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::DiscoverApi<'_> {
    /// AI playlist recommendation (`ai.recommend`).
    pub async fn ai_recommend(&self, params: AiRecommendParams) -> Result<Value> {
        self.client.call_value("ai.recommend", params).await
    }

    /// AI song recommendation (`ai.recommend.song`).
    pub async fn ai_recommend_song(&self, params: AiRecommendSongParams) -> Result<Value> {
        self.client.call_value("ai.recommend.song", params).await
    }

    /// Brush feed (`brush`).
    pub async fn brush(&self, params: BrushParams) -> Result<Value> {
        self.client.call_value("brush", params).await
    }

    /// Everyday friend recommendation (`everyday.friend`).
    pub async fn everyday_friend(&self) -> Result<Value> {
        self.client
            .call_value("everyday.friend", NoParams::default())
            .await
    }

    /// Everyday history (`everyday.history`).
    pub async fn everyday_history(&self, params: EverydayHistoryParams) -> Result<Value> {
        self.client.call_value("everyday.history", params).await
    }

    /// Everyday recommendation (`everyday.recommend`).
    pub async fn everyday_recommend(&self, params: EverydayRecommendParams) -> Result<Value> {
        self.client.call_value("everyday.recommend", params).await
    }

    /// Everyday style recommendation (`everyday.style.recommend`).
    pub async fn everyday_style_recommend(
        &self,
        params: EverydayStyleRecommendParams,
    ) -> Result<Value> {
        self.client
            .call_value("everyday.style.recommend", params)
            .await
    }

    /// Home discover (`home.discover`).
    pub async fn home_discover(&self, params: HomeDiscoverParams) -> Result<Value> {
        self.client.call_value("home.discover", params).await
    }

    /// Latest listened songs (`lastest.songs.listen`).
    pub async fn lastest_songs_listen(&self, params: LastestSongsListenParams) -> Result<Value> {
        self.client.call_value("lastest.songs.listen", params).await
    }

    /// Rank audios (`rank.audio`).
    pub async fn rank_audio(&self, params: RankAudioParams) -> Result<Value> {
        self.client.call_value("rank.audio", params).await
    }

    /// Rank info (`rank.info`).
    pub async fn rank_info(&self, params: RankParams) -> Result<Value> {
        self.client.call_value("rank.info", params).await
    }

    /// Rank list (`rank.list`).
    pub async fn rank_list(&self) -> Result<Value> {
        self.client
            .call_value("rank.list", RankListParams::default())
            .await
    }

    /// Recommended rank list (`rank.top`).
    pub async fn rank_top(&self) -> Result<Value> {
        self.client
            .call_value("rank.top", NoParams::default())
            .await
    }

    /// Rank volumes (`rank.vol`).
    pub async fn rank_vol(&self, params: RankParams) -> Result<Value> {
        self.client.call_value("rank.vol", params).await
    }

    /// Recommended songs (`recommend.songs`).
    pub async fn recommend_songs(&self, params: EverydayRecommendParams) -> Result<Value> {
        self.client.call_value("recommend.songs", params).await
    }

    /// Server time (`server.now`).
    pub async fn server_now(&self) -> Result<Value> {
        self.client
            .call_value("server.now", ServerNowParams::default())
            .await
    }

    /// New albums (`top.album`).
    pub async fn top_album(&self, params: TopAlbumParams) -> Result<Value> {
        self.client.call_value("top.album", params).await
    }

    /// Single-card recommendation (`top.card`).
    pub async fn top_card(&self, params: TopCardParams) -> Result<Value> {
        self.client.call_value("top.card", params).await
    }

    /// Daily IP recommendation (`top.ip`).
    pub async fn top_ip(&self) -> Result<Value> {
        self.client.call_value("top.ip", NoParams::default()).await
    }

    /// Special playlist recommendation (`top.playlist`).
    pub async fn top_playlist(&self, params: TopPlaylistParams) -> Result<Value> {
        self.client.call_value("top.playlist", params).await
    }

    /// New song publish (`top.song`).
    pub async fn top_song(&self, params: TopSongParams) -> Result<Value> {
        self.client.call_value("top.song", params).await
    }
}
