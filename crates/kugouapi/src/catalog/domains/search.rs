//! Search-domain regular endpoints.
//!
//! Ported from `domains/search.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, simple_endpoint, spec};
use crate::error::Result;
use crate::js::FlexI64;
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `search.complex`.
    pub SearchComplexParams {
        required {
            /// Search keyword.
            keywords: String => "keywords",
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
    /// Parameters for `search.default`.
    pub SearchDefaultParams {
        required {}
        optional {
            /// VIP type override.
            vip_type: FlexI64 => "vip_type",
        }
    }
}

kugou_params! {
    /// Parameters for `search.suggest`.
    pub SearchSuggestParams {
        required {
            /// Search keyword.
            keywords: String => "keywords",
        }
        optional {
            /// Album tip count (default 10).
            album_tip_count: FlexI64 => "albumTipCount",
            /// Correction tip count (default 10).
            correct_tip_count: FlexI64 => "correctTipCount",
            /// MV tip count (default 10).
            mv_tip_count: FlexI64 => "mvTipCount",
            /// Music tip count (default 10).
            music_tip_count: FlexI64 => "musicTipCount",
        }
    }
}

kugou_params! {
    /// Parameters for `search.lyric`.
    pub SearchLyricParams {
        required {}
        optional {
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
            /// Duration in seconds.
            duration: FlexI64 => "duration",
            /// File hash.
            hash: String => "hash",
            /// Search keyword.
            keywords: String => "keywords",
            /// Vocal gender hint (`yes` / `no`).
            man: String => "man",
        }
    }
}

const EP_SEARCH_COMPLEX: EndpointSpec = EndpointSpec {
    origin: Some("https://complexsearch.kugou.com"),
    ..spec(
        "search.complex",
        "search_complex",
        crate::http::Method::Get,
        "/v6/search/complex",
    )
};
const EP_SEARCH_DEFAULT: EndpointSpec = spec(
    "search.default",
    "search_default",
    crate::http::Method::Post,
    "/searchnofocus/v1/search_no_focus_word",
);
const EP_SEARCH_HOT: EndpointSpec = spec(
    "search.hot",
    "search_hot",
    crate::http::Method::Get,
    "/api/v3/search/hot_tab",
);
const EP_SEARCH_LYRIC: EndpointSpec = EndpointSpec {
    origin: Some("https://lyrics.kugou.com"),
    clear_params: true,
    ..spec(
        "search.lyric",
        "search_lyric",
        crate::http::Method::Get,
        "/v1/search",
    )
};
const EP_SEARCH_SUGGEST: EndpointSpec = spec(
    "search.suggest",
    "search_suggest",
    crate::http::Method::Get,
    "/v2/getSearchTip",
);

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_SEARCH_COMPLEX, |p: &SearchComplexParams, _| {
            let mut query = QueryRecord::new();
            query.set("platform", "AndroidFilter");
            query.set("keyword", p.keywords.clone());
            query.set("page", crate::js::or_flex(&p.page, 1));
            query.set("pagesize", crate::js::or_flex(&p.page_size, 30));
            query.set("cursor", 0);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SEARCH_DEFAULT, |p: &SearchDefaultParams, ctx| {
            let body = json!({
                "plat": 0,
                "userid": ctx.userid,
                "tags": "{}",
                "vip_type": crate::js::or_chain(
                    &[p.vip_type.as_ref().and_then(FlexI64::value), Some(ctx.vip_type)],
                    65530,
                ),
                "m_type": 0,
                "own_ads": {},
                "ability": "3",
                "sources": [],
                "bitmap": 2,
                "mode": "normal",
            });
            let mut query = QueryRecord::new();
            query.set("clientver", 12329);
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SEARCH_HOT, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("navid", 1);
            query.set("plat", 2);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SEARCH_LYRIC, |p: &SearchLyricParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("album_audio_id", crate::js::or_flex(&p.album_audio_id, 0));
            query.set("appid", ctx.appid);
            query.set("clientver", ctx.clientver);
            query.set("duration", crate::js::or_flex(&p.duration, 0));
            query.set("hash", p.hash.clone().unwrap_or_default());
            query.set("keyword", p.keywords.clone().unwrap_or_default());
            query.set("lrctxt", 1);
            query.set("man", p.man.clone().unwrap_or_else(|| "no".to_owned()));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SEARCH_SUGGEST, |p: &SearchSuggestParams, _| {
            let mut query = QueryRecord::new();
            query.set("keyword", p.keywords.clone());
            query.set("AlbumTipCount", crate::js::or_flex(&p.album_tip_count, 10));
            query.set(
                "CorrectTipCount",
                crate::js::or_flex(&p.correct_tip_count, 10),
            );
            query.set("MVTipCount", crate::js::or_flex(&p.mv_tip_count, 10));
            query.set("MusicTipCount", crate::js::or_flex(&p.music_tip_count, 10));
            query.set("radiotip", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::SearchApi<'_> {
    /// Complex search (`search.complex`).
    pub async fn complex(&self, params: SearchComplexParams) -> Result<Value> {
        self.client.call_value("search.complex", params).await
    }

    /// Default (no-focus) search (`search.default`).
    pub async fn default_search(&self, params: SearchDefaultParams) -> Result<Value> {
        self.client.call_value("search.default", params).await
    }

    /// Hot search tabs (`search.hot`).
    pub async fn hot(&self) -> Result<Value> {
        self.client
            .call_value("search.hot", NoParams::default())
            .await
    }

    /// Lyric search (`search.lyric`).
    pub async fn lyric_search(&self, params: SearchLyricParams) -> Result<Value> {
        self.client.call_value("search.lyric", params).await
    }

    /// Search suggestions (`search.suggest`).
    pub async fn suggest(&self, params: SearchSuggestParams) -> Result<Value> {
        self.client.call_value("search.suggest", params).await
    }
}
