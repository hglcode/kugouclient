//! Radio / FM-domain endpoints.
//!
//! Ported from `domains/radio.ts`.

use serde_json::{Map, Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `fm.class`.
    pub FmClassParams {
        required {}
        optional {}
    }
}

kugou_params! {
    /// Parameters for `fm.image`.
    pub FmImageParams {
        required {
            /// Comma-separated fm ids.
            fmid: String => "fmid",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `fm.recommend`.
    pub FmRecommendParams {
        required {}
        optional {}
    }
}

kugou_params! {
    /// Parameters for `pc.diantai`.
    pub PcDiantaiParams {
        required {}
        optional {}
    }
}

kugou_params! {
    /// Parameters for `personal.fm`.
    pub PersonalFmParams {
        required {}
        optional {
            /// Action (default `play`).
            action: String => "action",
            /// Song pool id.
            song_pool_id: FlexI64 => "song_pool_id",
            /// Platform (default `ios`).
            platform: String => "platform",
            /// Remaining song count.
            remain_songcnt: FlexI64 => "remain_songcnt",
            /// Overplay flag.
            is_overplay: bool => "is_overplay",
            /// Feed mode (default `normal`).
            mode: String => "mode",
            /// VIP type override.
            vip_type: FlexI64 => "vipType",
            /// Song hash.
            hash: String => "hash",
            /// Song id.
            songid: FlexI64 => "songid",
            /// Play time.
            playtime: FlexI64 => "playtime",
            /// Current mark.
            cur_mark: FlexI64 => "cur_mark",
        }
    }
}

kugou_params! {
    /// Parameters for `yueku.banner`.
    pub YuekuBannerParams {
        required {}
        optional {
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

const EP_FM_CLASS: EndpointSpec = EndpointSpec {
    router: Some("fm.service.kugou.com"),
    ..spec(
        "fm.class",
        "fm_class",
        crate::http::Method::Post,
        "/v1/class_fm_song",
    )
};
const EP_FM_IMAGE: EndpointSpec = EndpointSpec {
    router: Some("fm.service.kugou.com"),
    ..spec(
        "fm.image",
        "fm_image",
        crate::http::Method::Post,
        "/v1/fm_info",
    )
};
const EP_FM_RECOMMEND: EndpointSpec = EndpointSpec {
    router: Some("fm.service.kugou.com"),
    ..spec(
        "fm.recommend",
        "fm_recommend",
        crate::http::Method::Post,
        "/v1/rcmd_list",
    )
};
const EP_PC_DIANTAI: EndpointSpec = EndpointSpec {
    origin: Some("https://adservice.kugou.com"),
    ..spec(
        "pc.diantai",
        "pc_diantai",
        crate::http::Method::Post,
        "/v3/pc_diantai",
    )
};
const EP_PERSONAL_FM: EndpointSpec = EndpointSpec {
    router: Some("persnfm.service.kugou.com"),
    ..spec(
        "personal.fm",
        "personal_fm",
        crate::http::Method::Post,
        "/v2/personal_recommend",
    )
};
const EP_YUEKU: EndpointSpec = EndpointSpec {
    router: Some("service.mobile.kugou.com"),
    ..spec(
        "yueku",
        "yueku",
        crate::http::Method::Get,
        "/v1/yueku/recommend_v2",
    )
};
const EP_YUEKU_BANNER: EndpointSpec = spec(
    "yueku.banner",
    "yueku_banner",
    crate::http::Method::Post,
    "/ads.gateway/v3/listen_banner",
);
const EP_YUEKU_FM: EndpointSpec = EndpointSpec {
    router: Some("fm.service.kugou.com"),
    ..spec(
        "yueku.fm",
        "yueku_fm",
        crate::http::Method::Get,
        "/v1/time_fm_info",
    )
};

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_FM_CLASS, |_: &FmClassParams, ctx| {
            let body = json!({
                "kguid": ctx.userid,
                "clienttime": ctx.clienttime,
                "mid": ctx.mid,
                "platform": "android",
                "clientver": ctx.clientver,
                "uid": ctx.userid,
                "get_tracker": 1,
                "key": ctx.sign_params_key(ctx.clienttime),
                "appid": ctx.appid,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_FM_IMAGE, |p: &FmImageParams, ctx| {
            let data: Vec<Value> = p
                .fmid
                .split(',')
                .map(|s| {
                    json!({
                        "fields": "imgUrl100,imgUrl50",
                        "fmid": s,
                        "fmtype": 2,
                    })
                })
                .collect();
            let body = json!({
                "appid": ctx.appid,
                "clienttime": ctx.clienttime,
                "clientver": ctx.clientver,
                "data": data,
                "dfid": ctx.dfid,
                "key": ctx.sign_params_key(ctx.clienttime),
                "mid": ctx.mid,
            });
            let mut body = body;
            if ctx.userid != 0 {
                body["userid"] = json!(ctx.userid);
            }
            if !ctx.token.is_empty() {
                body["token"] = json!(ctx.token);
            }
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_FM_RECOMMEND, |_: &FmRecommendParams, ctx| {
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "clienttime": ctx.clienttime,
                "mid": ctx.mid,
                "key": ctx.sign_params_key(ctx.clienttime),
                "rcmdsongcount": 1,
                "level": 0,
                "area_code": 1,
                "get_tracker": 1,
                "uid": 0,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_PC_DIANTAI, |_: &PcDiantaiParams, ctx| {
            let body = json!({
                "isvip": 0,
                "userid": ctx.userid,
                "vipType": 0,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_PERSONAL_FM, |p: &PersonalFmParams, ctx| {
            let mut body = Map::new();
            body.insert("appid".to_owned(), json!(ctx.appid));
            body.insert("clienttime".to_owned(), json!(ctx.clienttime));
            body.insert("mid".to_owned(), json!(ctx.mid));
            body.insert(
                "action".to_owned(),
                json!(p.action.clone().unwrap_or_else(|| "play".to_owned())),
            );
            body.insert("recommend_source_locked".to_owned(), json!(0));
            body.insert(
                "song_pool_id".to_owned(),
                json!(js::or_flex(&p.song_pool_id, 0)),
            );
            body.insert("callerid".to_owned(), json!(0));
            body.insert("m_type".to_owned(), json!(1));
            body.insert(
                "platform".to_owned(),
                json!(p.platform.clone().unwrap_or_else(|| "ios".to_owned())),
            );
            body.insert("area_code".to_owned(), json!(1));
            body.insert(
                "remain_songcnt".to_owned(),
                json!(js::or_flex(&p.remain_songcnt, 0)),
            );
            body.insert("clientver".to_owned(), json!(ctx.clientver));
            body.insert(
                "is_overplay".to_owned(),
                json!(i64::from(p.is_overplay.unwrap_or(false))),
            );
            body.insert(
                "mode".to_owned(),
                json!(p.mode.clone().unwrap_or_else(|| "normal".to_owned())),
            );
            body.insert(
                "fakem".to_owned(),
                json!("ca981cfc583a4c37f28d2d49000013c16a0a"),
            );
            body.insert("key".to_owned(), json!(ctx.sign_params_key(ctx.clienttime)));
            if ctx.userid != 0 {
                body.insert("userid".to_owned(), json!(ctx.userid));
                body.insert("kguid".to_owned(), json!(ctx.userid));
            }
            if !ctx.token.is_empty() {
                body.insert("token".to_owned(), json!(ctx.token));
            }
            let vip_type = js::or_chain(
                &[
                    Some(ctx.vip_type),
                    p.vip_type.as_ref().and_then(FlexI64::value),
                ],
                0,
            );
            if vip_type != 0 {
                body.insert("vip_type".to_owned(), json!(vip_type));
            }
            if let Some(hash) = &p.hash {
                body.insert("hash".to_owned(), json!(hash));
            }
            if let Some(songid) = p.songid.as_ref().and_then(FlexI64::value) {
                body.insert("songid".to_owned(), json!(songid));
            }
            if let Some(playtime) = p.playtime.as_ref().and_then(FlexI64::value) {
                body.insert("playtime".to_owned(), json!(playtime));
            }
            if let Some(cur_mark) = p.cur_mark.as_ref().and_then(FlexI64::value) {
                body.insert("cur_mark".to_owned(), json!(cur_mark));
            }
            Built {
                json: Some(Value::Object(body)),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YUEKU, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("operator", 7);
            query.set("plat", 0);
            query.set("type", 11);
            query.set("area_code", 1);
            query.set("req_multi", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YUEKU_BANNER, |p: &YuekuBannerParams, ctx| {
            let body = json!({
                "plat": 0,
                "channel": 201,
                "operator": 7,
                "networktype": 2,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "vip_type": 0,
                "m_type": 0,
                "tags": [],
                "apiver": 5,
                "ability": 2,
                "mode": "normal",
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_YUEKU_FM, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("operator", 7);
            query.set("plat", 0);
            query.set("type", 11);
            query.set("area_code", 1);
            query.set("req_multi", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::RadioApi<'_> {
    /// FM classification (`fm.class`).
    pub async fn fm_class(&self) -> Result<Value> {
        self.client
            .call_value("fm.class", FmClassParams::default())
            .await
    }

    /// FM images (`fm.image`).
    pub async fn fm_image(&self, params: FmImageParams) -> Result<Value> {
        self.client.call_value("fm.image", params).await
    }

    /// FM recommendation (`fm.recommend`).
    pub async fn fm_recommend(&self) -> Result<Value> {
        self.client
            .call_value("fm.recommend", FmRecommendParams::default())
            .await
    }

    /// PC radio station (`pc.diantai`).
    pub async fn pc_diantai(&self) -> Result<Value> {
        self.client
            .call_value("pc.diantai", PcDiantaiParams::default())
            .await
    }

    /// Personal FM (`personal.fm`).
    pub async fn personal_fm(&self, params: PersonalFmParams) -> Result<Value> {
        self.client.call_value("personal.fm", params).await
    }

    /// Yueku recommendation (`yueku`).
    pub async fn yueku(&self) -> Result<Value> {
        self.client.call_value("yueku", NoParams::default()).await
    }

    /// Yueku banner (`yueku.banner`).
    pub async fn yueku_banner(&self, params: YuekuBannerParams) -> Result<Value> {
        self.client.call_value("yueku.banner", params).await
    }

    /// Yueku time FM (`yueku.fm`).
    pub async fn yueku_fm(&self) -> Result<Value> {
        self.client
            .call_value("yueku.fm", NoParams::default())
            .await
    }
}
