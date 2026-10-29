//! Verticals-domain endpoints (IP / longaudio / youth / team basics).
//!
//! Ported from `domains/verticals.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64, FlexString};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `ip.dateil`.
    pub IpDateilParams {
        required {
            /// Comma-separated IP ids.
            id: String => "id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `ip.playlist`.
    pub IpPlaylistParams {
        required {
            /// IP id.
            id: FlexString => "id",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `ip.zone.home`.
    pub IpZoneHomeParams {
        required {
            /// Zone id.
            id: FlexI64 => "id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `longaudio.album.audios`.
    pub LongaudioAlbumAudiosParams {
        required {
            /// Album id.
            album_id: FlexI64 => "album_id",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `longaudio.album.detail`.
    pub LongaudioAlbumDetailParams {
        required {
            /// Comma-separated album ids.
            album_id: String => "album_id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `longaudio.album.list`.
    pub LongaudioAlbumListParams {
        required {}
        optional {
            /// Gender filter.
            gender: FlexI64 => "gender",
            /// Sort.
            sort: FlexI64 => "sort",
            /// Tag id (default 906).
            tag_id: FlexI64 => "tag_id",
            /// Status filter.
            status: FlexI64 => "status",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size (default 20).
            page_size: FlexI64 => "page_size",
        }
    }
}

kugou_params! {
    /// Parameters for `longaudio.daily.recommend`.
    pub LongaudioDailyRecommendParams {
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
    /// Parameters for `longaudio.search`.
    pub LongaudioSearchParams {
        required {
            /// Search keyword.
            keywords: String => "keywords",
        }
        optional {
            /// Fallback keyword.
            keyword: String => "keyword",
            /// User id override.
            userid: FlexI64 => "userid",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `team.join`.
    pub TeamJoinParams {
        required {
            /// Team code.
            team_code: String => "team_code",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `team.my`.
    pub TeamMyParams {
        required {
            /// Period id.
            period_id: FlexI64 => "period_id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `youth.channel.all`.
    pub YouthChannelAllParams {
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
    /// Parameters for `youth.channel.amway` / `youth.channel.song` /
    /// `youth.channel.song.detail` / `youth.channel.song.list`.
    pub YouthChannelParams {
        required {
            /// `global_collection_id`.
            global_collection_id: FlexString => "global_collection_id",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// File id (`youth.channel.song.detail`).
            fileid: FlexI64 => "fileid",
        }
    }
}

kugou_params! {
    /// Parameters for `youth.channel.detail`.
    pub YouthChannelDetailParams {
        required {
            /// Comma-separated `global_collection_id` values.
            global_collection_id: String => "global_collection_id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `youth.channel.similar`.
    pub YouthChannelSimilarParams {
        required {
            /// Channel id.
            channel_id: FlexI64 => "channel_id",
        }
        optional {
            /// VIP type override.
            vip_type: FlexI64 => "vip_type",
        }
    }
}

kugou_params! {
    /// Parameters for `youth.day.vip`.
    pub YouthDayVipParams {
        required {
            /// Receive days.
            receive_day: FlexI64 => "receive_day",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `youth.day.vip.upgrade`.
    pub YouthDayVipUpgradeParams {
        required {}
        optional {
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

kugou_params! {
    /// Parameters for `youth.listen.song`.
    pub YouthListenSongParams {
        required {}
        optional {
            /// Mixsong id (default 666075191).
            mixsongid: FlexI64 => "mixsongid",
        }
    }
}

kugou_params! {
    /// Parameters for `youth.user.song`.
    pub YouthUserSongParams {
        required {
            /// User id.
            userid: FlexI64 => "userid",
        }
        optional {
            /// Song type filter.
            list_type: FlexI64 => "type",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

const EP_IP_DATEIL: EndpointSpec = spec(
    "ip.dateil",
    "ip_dateil",
    crate::http::Method::Post,
    "/openapi/v1/ip",
);
const EP_IP_PLAYLIST: EndpointSpec = spec(
    "ip.playlist",
    "ip_playlist",
    crate::http::Method::Post,
    "/ocean/v6/pubsongs/list_info_for_ip",
);
const EP_IP_ZONE: EndpointSpec = EndpointSpec {
    router: Some("yuekucategory.kugou.com"),
    ..spec(
        "ip.zone",
        "ip_zone",
        crate::http::Method::Get,
        "/v1/zone/index",
    )
};
const EP_IP_ZONE_HOME: EndpointSpec = EndpointSpec {
    router: Some("yuekucategory.kugou.com"),
    ..spec(
        "ip.zone.home",
        "ip_zone_home",
        crate::http::Method::Get,
        "/v1/zone/home",
    )
};
const EP_LONGAUDIO_ALBUM_AUDIOS: EndpointSpec = EndpointSpec {
    router: Some("openapi.kugou.com"),
    ..spec(
        "longaudio.album.audios",
        "longaudio_album_audios",
        crate::http::Method::Post,
        "/longaudio/v2/album_audios",
    )
};
const EP_LONGAUDIO_ALBUM_DETAIL: EndpointSpec = spec(
    "longaudio.album.detail",
    "longaudio_album_detail",
    crate::http::Method::Post,
    "/openapi/v2/broadcast",
);
const EP_LONGAUDIO_ALBUM_LIST: EndpointSpec = spec(
    "longaudio.album.list",
    "longaudio_album_list",
    crate::http::Method::Get,
    "/longaudio/v1/album/list",
);
const EP_LONGAUDIO_DAILY_RECOMMEND: EndpointSpec = spec(
    "longaudio.daily.recommend",
    "longaudio_daily_recommend",
    crate::http::Method::Post,
    "/longaudio/v1/home_new/daily_recommend",
);
const EP_LONGAUDIO_RANK_RECOMMEND: EndpointSpec = spec(
    "longaudio.rank.recommend",
    "longaudio_rank_recommend",
    crate::http::Method::Get,
    "/longaudio/v1/home_new/rank_card_recommend",
);
const EP_LONGAUDIO_SEARCH: EndpointSpec = spec(
    "longaudio.search",
    "longaudio_search",
    crate::http::Method::Get,
    "/complexsearch/v4/search/song",
);
const EP_LONGAUDIO_TAG_LIST: EndpointSpec = EndpointSpec {
    router: Some("longaudio.kugou.com"),
    ..spec(
        "longaudio.tag.list",
        "longaudio_tag_list",
        crate::http::Method::Get,
        "/v3/list_audiobook_tags",
    )
};
const EP_LONGAUDIO_VIP_RECOMMEND: EndpointSpec = spec(
    "longaudio.vip.recommend",
    "longaudio_vip_recommend",
    crate::http::Method::Post,
    "/longaudio/v1/home_new/vip_select_recommend",
);
const EP_LONGAUDIO_WEEK_RECOMMEND: EndpointSpec = spec(
    "longaudio.week.recommend",
    "longaudio_week_recommend",
    crate::http::Method::Post,
    "/longaudio/v1/home_new/week_new_albums_recommend",
);
const EP_TEAM_JOIN: EndpointSpec = spec(
    "team.join",
    "team_join",
    crate::http::Method::Post,
    "/youth/v1/ut/join_team",
);
const EP_TEAM_MY: EndpointSpec = spec(
    "team.my",
    "team_my",
    crate::http::Method::Post,
    "/youth/v1/ut/create_team",
);
const EP_YOUTH_CHANNEL_ALL: EndpointSpec = spec(
    "youth.channel.all",
    "youth_channel_all",
    crate::http::Method::Get,
    "/youth/v2/channel/channel_all_list",
);
const EP_YOUTH_CHANNEL_AMWAY: EndpointSpec = spec(
    "youth.channel.amway",
    "youth_channel_amway",
    crate::http::Method::Get,
    "/youth/api/amway/v2/index",
);
const EP_YOUTH_CHANNEL_DETAIL: EndpointSpec = spec(
    "youth.channel.detail",
    "youth_channel_detail",
    crate::http::Method::Post,
    "/youth/api/channel/v1/channel_list_by_id",
);
const EP_YOUTH_CHANNEL_SIMILAR: EndpointSpec = spec(
    "youth.channel.similar",
    "youth_channel_similar",
    crate::http::Method::Post,
    "/youth/v1/channel/get_friendly_channel",
);
const EP_YOUTH_CHANNEL_SONG: EndpointSpec = spec(
    "youth.channel.song",
    "youth_channel_song",
    crate::http::Method::Get,
    "/youth/api/channel/v1/channel_get_song_audit_passed",
);
const EP_YOUTH_CHANNEL_SONG_DETAIL: EndpointSpec = spec(
    "youth.channel.song.detail",
    "youth_channel_song_detail",
    crate::http::Method::Get,
    "/youth/v2/post/get_song_detail",
);
const EP_YOUTH_CHANNEL_SONG_LIST: EndpointSpec = EndpointSpec {
    origin: Some("https://youth.kugou.com"),
    ..spec(
        "youth.channel.song.list",
        "youth_channel_song_list",
        crate::http::Method::Get,
        "/api/channel/v1/channel_get_song_list",
    )
};
const EP_YOUTH_DAY_VIP: EndpointSpec = spec(
    "youth.day.vip",
    "youth_day_vip",
    crate::http::Method::Post,
    "/youth/v1/recharge/receive_vip_listen_song",
);
const EP_YOUTH_DAY_VIP_UPGRADE: EndpointSpec = spec(
    "youth.day.vip.upgrade",
    "youth_day_vip_upgrade",
    crate::http::Method::Post,
    "/youth/v1/listen_song/upgrade_vip_reward",
);
const EP_YOUTH_DYNAMIC: EndpointSpec = spec(
    "youth.dynamic",
    "youth_dynamic",
    crate::http::Method::Get,
    "/youth/v3/user/get_dynamic",
);
const EP_YOUTH_DYNAMIC_RECENT: EndpointSpec = spec(
    "youth.dynamic.recent",
    "youth_dynamic_recent",
    crate::http::Method::Get,
    "/youth/v3/user/recent_dynamic",
);
const EP_YOUTH_LISTEN_SONG: EndpointSpec = spec(
    "youth.listen.song",
    "youth_listen_song",
    crate::http::Method::Post,
    "/youth/v2/report/listen_song",
);
const EP_YOUTH_MONTH_VIP_RECORD: EndpointSpec = spec(
    "youth.month.vip.record",
    "youth_month_vip_record",
    crate::http::Method::Get,
    "/youth/v1/activity/get_month_vip_record",
);
const EP_YOUTH_UNION_VIP: EndpointSpec = EndpointSpec {
    origin: Some("https://kugouvip.kugou.com"),
    ..spec(
        "youth.union.vip",
        "youth_union_vip",
        crate::http::Method::Get,
        "/v1/get_union_vip",
    )
};
const EP_YOUTH_USER_SONG: EndpointSpec = spec(
    "youth.user.song",
    "youth_user_song",
    crate::http::Method::Get,
    "/youth/v1/get_user_song_public",
);
const EP_YOUTH_VIP: EndpointSpec = spec(
    "youth.vip",
    "youth_vip",
    crate::http::Method::Post,
    "/youth/v1/ad/play_report",
);

fn youth_identity_query(ctx: &crate::catalog::ctx::CallCtx<'_>) -> QueryRecord {
    let mut query = QueryRecord::new();
    query.set("srcappid", ctx.srcappid);
    query.set("clientver", ctx.clientver);
    query.set("clienttime", ctx.clienttime);
    query.set("mid", ctx.mid.clone());
    query.set("uuid", ctx.uuid.clone());
    query.set("dfid", ctx.dfid.clone());
    query.set("appid", ctx.appid);
    query.set("userid", ctx.userid);
    query.set("token", ctx.token.clone());
    query
}

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_IP_DATEIL, |p: &IpDateilParams, _| {
            let data: Vec<Value> = p.id.split(',').map(|s| json!({ "ip_id": s })).collect();
            let body = json!({ "data": data, "is_publish": 1 });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_IP_PLAYLIST, |p: &IpPlaylistParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("ip", p.id.value());
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_IP_ZONE, |_: &NoParams, _| Built::default()),
        simple_endpoint(&EP_IP_ZONE_HOME, |p: &IpZoneHomeParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("id", p.id.value());
            query.set("share", 0);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_LONGAUDIO_ALBUM_AUDIOS,
            |p: &LongaudioAlbumAudiosParams, _| {
                let body = json!({
                    "album_id": p.album_id.value(),
                    "area_code": 1,
                    "tagid": 0,
                    "page": js::or_flex(&p.page, 1),
                    "pagesize": js::or_flex(&p.page_size, 30),
                });
                Built {
                    json: Some(body),
                    headers: vec![("KG-TID".to_owned(), "78".to_owned())],
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_LONGAUDIO_ALBUM_DETAIL,
            |p: &LongaudioAlbumDetailParams, _| {
                let data: Vec<Value> = p
                    .album_id
                    .split(',')
                    .map(|s| json!({ "album_id": s }))
                    .collect();
                let body = json!({
                    "data": data,
                    "show_album_tag": 1,
                    "fields": "album_name,album_id,category,authors,sizable_cover,intro,author_name,trans_param,album_tag,mix_intro,full_intro,is_publish",
                });
                Built {
                    json: Some(body),
                    headers: vec![("KG-TID".to_owned(), "78".to_owned())],
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_LONGAUDIO_ALBUM_LIST,
            |p: &LongaudioAlbumListParams, _| {
                let mut query = QueryRecord::new();
                query.set("api_ver", 2);
                query.set("gender", js::or_flex(&p.gender, 0).max(0));
                query.set("sort", js::or_flex(&p.sort, 0).max(0));
                query.set("tag_id", js::or_flex(&p.tag_id, 906).max(0));
                query.set("free", 1);
                query.set("status", js::or_flex(&p.status, 0).max(0));
                query.set("page", js::or_flex(&p.page, 1).max(1));
                query.set("page_size", js::or_flex(&p.page_size, 20).max(1));
                query.set("clientver", 20789);
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_LONGAUDIO_DAILY_RECOMMEND,
            |p: &LongaudioDailyRecommendParams, _| {
                let mut query = QueryRecord::new();
                query.set("module_id", 1);
                query.set("size", js::or_flex(&p.page_size, 30));
                query.set("page", js::or_flex(&p.page, 1));
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_LONGAUDIO_RANK_RECOMMEND, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("platform", "ios");
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_LONGAUDIO_SEARCH, |p: &LongaudioSearchParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("area_code", 1);
            query.set("albumhide", 1);
            query.set("com_user_type", 0);
            query.set("privilegefilter", 0);
            query.set("dopicfull", 1);
            query.set("filter", 12);
            query.set("platform", "AndroidFilter");
            query.set("tag", "em");
            query.set("recver", 2);
            query.set("iscorrection", 1);
            query.set("search_ability", 223);
            query.set("sec_aggre", 1);
            query.set("sec_aggre_bitmap", 0);
            query.set("mode_ability", 1);
            query.set("nocollect", 1);
            query.set("user_type", 0);
            query.set(
                "userid",
                js::or_chain(
                    &[
                        p.userid.as_ref().and_then(FlexI64::value),
                        Some(ctx.userid as i64),
                    ],
                    0,
                )
                .to_string(),
            );
            query.set(
                "keyword",
                p.keyword
                    .clone()
                    .filter(|k| !k.is_empty())
                    .unwrap_or_else(|| p.keywords.clone()),
            );
            query.set("page", js::or_flex(&p.page, 1).max(1));
            query.set("pagesize", js::or_flex(&p.page_size, 30).max(1));
            query.set("clientver", 20789);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_LONGAUDIO_TAG_LIST, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("platform", "android");
            query.set("clientver", 20789);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_LONGAUDIO_VIP_RECOMMEND, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("position", "2");
            query.set("clientver", 12329);
            let body = json!({ "album_playlist": [] });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_LONGAUDIO_WEEK_RECOMMEND, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("clientver", 12329);
            let body = json!({ "album_playlist": [] });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_TEAM_JOIN, |p: &TeamJoinParams, ctx| {
            let body = json!({ "team_code": p.team_code });
            Built {
                json: Some(body),
                headers: vec![("Host".to_owned(), "gateway.kugou.com".to_owned())],
                query: youth_identity_query(ctx),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_TEAM_MY, |p: &TeamMyParams, ctx| {
            let body = json!({ "period_id": p.period_id.value() });
            Built {
                json: Some(body),
                headers: vec![("Host".to_owned(), "gateway.kugou.com".to_owned())],
                query: youth_identity_query(ctx),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YOUTH_CHANNEL_ALL, |p: &YouthChannelAllParams, _| {
            let mut query = QueryRecord::new();
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("type", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YOUTH_CHANNEL_AMWAY, |p: &YouthChannelParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("global_collection_id", p.global_collection_id.value());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_YOUTH_CHANNEL_DETAIL,
            |p: &YouthChannelDetailParams, _| {
                let data: Vec<Value> = p
                    .global_collection_id
                    .split(',')
                    .map(|s| json!({ "global_collection_id": s }))
                    .collect();
                let body = json!({ "data": data });
                Built {
                    json: Some(body),
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_YOUTH_CHANNEL_SIMILAR,
            |p: &YouthChannelSimilarParams, ctx| {
                let mut query = QueryRecord::new();
                query.set_opt("channel_id", p.channel_id.value());
                let body = json!({
                    "area_code": 1,
                    "playlist_ver": 2,
                    "vip_type": js::or_chain(
                        &[p.vip_type.as_ref().and_then(FlexI64::value), Some(ctx.vip_type)],
                        0,
                    ),
                    "platform": "ios",
                });
                Built {
                    json: Some(body),
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_YOUTH_CHANNEL_SONG, |p: &YouthChannelParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("global_collection_id", p.global_collection_id.value());
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            query.set("is_filter", 0);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_YOUTH_CHANNEL_SONG_DETAIL,
            |p: &YouthChannelParams, _| {
                let mut query = QueryRecord::new();
                query.set_opt("global_collection_id", p.global_collection_id.value());
                query.set_opt("fileid", p.fileid.as_ref().and_then(FlexI64::value));
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_YOUTH_CHANNEL_SONG_LIST, |p: &YouthChannelParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("global_collection_id", p.global_collection_id.value());
            query.set("pagesize", js::or_flex(&p.page_size, 20));
            query.set("page", js::or_flex(&p.page, 1));
            query.set("types", 2);
            query.set("is_filter", 0);
            query.set("apiver", 3);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YOUTH_DAY_VIP, |p: &YouthDayVipParams, _| {
            let mut query = QueryRecord::new();
            query.set("source_id", 90139);
            query.set_opt("receive_day", p.receive_day.value());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_YOUTH_DAY_VIP_UPGRADE,
            |p: &YouthDayVipUpgradeParams, ctx| {
                let mut query = QueryRecord::new();
                query.set(
                    "kugouid",
                    js::or_chain(
                        &[
                            p.userid.as_ref().and_then(FlexI64::value),
                            Some(ctx.userid as i64),
                        ],
                        0,
                    ),
                );
                query.set("ad_type", 1);
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_YOUTH_DYNAMIC, |_: &NoParams, _| Built::default()),
        simple_endpoint(&EP_YOUTH_DYNAMIC_RECENT, |_: &NoParams, _| Built::default()),
        simple_endpoint(&EP_YOUTH_LISTEN_SONG, |p: &YouthListenSongParams, _| {
            let mut query = QueryRecord::new();
            query.set("clientver", 10566);
            let body = json!({ "mixsongid": js::or_flex(&p.mixsongid, 666_075_191) });
            Built {
                json: Some(body),
                headers: vec![(
                    "user-agent".to_owned(),
                    "Android13-1070-10566-201-0-ReportPlaySongToServerProtocol-wifi".to_owned(),
                )],
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YOUTH_MONTH_VIP_RECORD, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("latest_limit", 100);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YOUTH_UNION_VIP, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("busi_type", "concept");
            query.set("opt_product_types", "dvip,qvip");
            query.set("product_type", "svip");
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YOUTH_USER_SONG, |p: &YouthUserSongParams, _| {
            let mut query = QueryRecord::new();
            query.set("filter_video", 0);
            query.set("type", js::or_flex(&p.list_type, 0));
            query.set_opt("userid", p.userid.value());
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            query.set("is_filter", 0);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YOUTH_VIP, |_: &NoParams, _| {
            let now = crate::crypto::now_ms();
            let body = json!({
                "ad_id": 12_307_537_187_i64,
                "play_end": now,
                "play_start": now - 30_000,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::VerticalsApi<'_> {
    /// IP detail (`ip.dateil`).
    pub async fn ip_dateil(&self, params: IpDateilParams) -> Result<Value> {
        self.client.call_value("ip.dateil", params).await
    }

    /// IP playlists (`ip.playlist`).
    pub async fn ip_playlist(&self, params: IpPlaylistParams) -> Result<Value> {
        self.client.call_value("ip.playlist", params).await
    }

    /// IP zone index (`ip.zone`).
    pub async fn ip_zone(&self) -> Result<Value> {
        self.client.call_value("ip.zone", NoParams::default()).await
    }

    /// IP zone home (`ip.zone.home`).
    pub async fn ip_zone_home(&self, params: IpZoneHomeParams) -> Result<Value> {
        self.client.call_value("ip.zone.home", params).await
    }

    /// Longaudio album audios (`longaudio.album.audios`).
    pub async fn longaudio_album_audios(
        &self,
        params: LongaudioAlbumAudiosParams,
    ) -> Result<Value> {
        self.client
            .call_value("longaudio.album.audios", params)
            .await
    }

    /// Longaudio album detail (`longaudio.album.detail`).
    pub async fn longaudio_album_detail(
        &self,
        params: LongaudioAlbumDetailParams,
    ) -> Result<Value> {
        self.client
            .call_value("longaudio.album.detail", params)
            .await
    }

    /// Longaudio album list (`longaudio.album.list`).
    pub async fn longaudio_album_list(&self, params: LongaudioAlbumListParams) -> Result<Value> {
        self.client.call_value("longaudio.album.list", params).await
    }

    /// Longaudio daily recommendation (`longaudio.daily.recommend`).
    pub async fn longaudio_daily_recommend(
        &self,
        params: LongaudioDailyRecommendParams,
    ) -> Result<Value> {
        self.client
            .call_value("longaudio.daily.recommend", params)
            .await
    }

    /// Longaudio rank recommendation (`longaudio.rank.recommend`).
    pub async fn longaudio_rank_recommend(&self) -> Result<Value> {
        self.client
            .call_value("longaudio.rank.recommend", NoParams::default())
            .await
    }

    /// Longaudio search (`longaudio.search`).
    pub async fn longaudio_search(&self, params: LongaudioSearchParams) -> Result<Value> {
        self.client.call_value("longaudio.search", params).await
    }

    /// Longaudio tag list (`longaudio.tag.list`).
    pub async fn longaudio_tag_list(&self) -> Result<Value> {
        self.client
            .call_value("longaudio.tag.list", NoParams::default())
            .await
    }

    /// Longaudio VIP recommendation (`longaudio.vip.recommend`).
    pub async fn longaudio_vip_recommend(&self) -> Result<Value> {
        self.client
            .call_value("longaudio.vip.recommend", NoParams::default())
            .await
    }

    /// Longaudio week recommendation (`longaudio.week.recommend`).
    pub async fn longaudio_week_recommend(&self) -> Result<Value> {
        self.client
            .call_value("longaudio.week.recommend", NoParams::default())
            .await
    }

    /// Join a team (`team.join`).
    pub async fn team_join(&self, params: TeamJoinParams) -> Result<Value> {
        self.client.call_value("team.join", params).await
    }

    /// Create a team (`team.my`).
    pub async fn team_my(&self, params: TeamMyParams) -> Result<Value> {
        self.client.call_value("team.my", params).await
    }

    /// Youth channel list (`youth.channel.all`).
    pub async fn youth_channel_all(&self, params: YouthChannelAllParams) -> Result<Value> {
        self.client.call_value("youth.channel.all", params).await
    }

    /// Youth channel amway (`youth.channel.amway`).
    pub async fn youth_channel_amway(&self, params: YouthChannelParams) -> Result<Value> {
        self.client.call_value("youth.channel.amway", params).await
    }

    /// Youth channel detail (`youth.channel.detail`).
    pub async fn youth_channel_detail(&self, params: YouthChannelDetailParams) -> Result<Value> {
        self.client.call_value("youth.channel.detail", params).await
    }

    /// Youth friendly channels (`youth.channel.similar`).
    pub async fn youth_channel_similar(&self, params: YouthChannelSimilarParams) -> Result<Value> {
        self.client
            .call_value("youth.channel.similar", params)
            .await
    }

    /// Youth channel songs (`youth.channel.song`).
    pub async fn youth_channel_song(&self, params: YouthChannelParams) -> Result<Value> {
        self.client.call_value("youth.channel.song", params).await
    }

    /// Youth channel song detail (`youth.channel.song.detail`).
    pub async fn youth_channel_song_detail(&self, params: YouthChannelParams) -> Result<Value> {
        self.client
            .call_value("youth.channel.song.detail", params)
            .await
    }

    /// Youth channel song list (`youth.channel.song.list`).
    pub async fn youth_channel_song_list(&self, params: YouthChannelParams) -> Result<Value> {
        self.client
            .call_value("youth.channel.song.list", params)
            .await
    }

    /// Youth day VIP (`youth.day.vip`).
    pub async fn youth_day_vip(&self, params: YouthDayVipParams) -> Result<Value> {
        self.client.call_value("youth.day.vip", params).await
    }

    /// Youth day VIP upgrade (`youth.day.vip.upgrade`).
    pub async fn youth_day_vip_upgrade(&self, params: YouthDayVipUpgradeParams) -> Result<Value> {
        self.client
            .call_value("youth.day.vip.upgrade", params)
            .await
    }

    /// Youth dynamics (`youth.dynamic`).
    pub async fn youth_dynamic(&self) -> Result<Value> {
        self.client
            .call_value("youth.dynamic", NoParams::default())
            .await
    }

    /// Recent youth dynamics (`youth.dynamic.recent`).
    pub async fn youth_dynamic_recent(&self) -> Result<Value> {
        self.client
            .call_value("youth.dynamic.recent", NoParams::default())
            .await
    }

    /// Report listened song (`youth.listen.song`).
    pub async fn youth_listen_song(&self, params: YouthListenSongParams) -> Result<Value> {
        self.client.call_value("youth.listen.song", params).await
    }

    /// Monthly VIP record (`youth.month.vip.record`).
    pub async fn youth_month_vip_record(&self) -> Result<Value> {
        self.client
            .call_value("youth.month.vip.record", NoParams::default())
            .await
    }

    /// Union VIP products (`youth.union.vip`).
    pub async fn youth_union_vip(&self) -> Result<Value> {
        self.client
            .call_value("youth.union.vip", NoParams::default())
            .await
    }

    /// User's public songs (`youth.user.song`).
    pub async fn youth_user_song(&self, params: YouthUserSongParams) -> Result<Value> {
        self.client.call_value("youth.user.song", params).await
    }

    /// Youth ad play report (`youth.vip`).
    pub async fn youth_vip(&self) -> Result<Value> {
        self.client
            .call_value("youth.vip", NoParams::default())
            .await
    }
}
