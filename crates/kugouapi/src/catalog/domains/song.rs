//! Song-domain regular endpoints.
//!
//! Ported from `domains/song.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;
use crate::sign::md5;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `audio.accompany.matching`.
    pub AudioAccompanyMatchingParams {
        required {
            /// File hash.
            hash: String => "hash",
        }
        optional {
            /// Accompany mix id.
            mix_id: FlexI64 => "mixId",
            /// File name.
            file_name: String => "fileName",
        }
    }
}

kugou_params! {
    /// Parameters for `audio.ktv.total`.
    pub AudioKtvTotalParams {
        required {
            /// Song hash.
            song_hash: String => "songHash",
        }
        optional {
            /// Song id.
            song_id: FlexI64 => "songId",
            /// Singer name.
            singer_name: String => "singerName",
        }
    }
}

kugou_params! {
    /// Parameters for `song.climax`.
    pub SongClimaxParams {
        required {
            /// Comma-separated file hashes.
            hash: String => "hash",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `song.ranking`.
    pub SongRankingParams {
        required {
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `song.ranking.filter`.
    pub SongRankingFilterParams {
        required {
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

/// The `*s&iN#G70*`-salted sign used by the sing7 CDN endpoints:
/// `md5(sorted "k=v" pairs joined by "&" + salt)[8..24]`.
fn sing7_sign(pairs: &[(&str, String)]) -> String {
    let mut sorted: Vec<(&str, String)> = pairs.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let joined = sorted
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    md5(&format!("{joined}*s&iN#G70*"))[8..24].to_owned()
}

const EP_AUDIO_ACCOMPANY_MATCHING: EndpointSpec = EndpointSpec {
    origin: Some("https://nsongacsing.kugou.com"),
    salt: crate::catalog::def::Salt::None,
    clear_params: true,
    ..spec(
        "audio.accompany.matching",
        "audio_accompany_matching",
        crate::http::Method::Get,
        "/sing7/accompanywan/json/v2/cdn/optimal_matching_accompany_2_listen.do",
    )
};
const EP_AUDIO_KTV_TOTAL: EndpointSpec = EndpointSpec {
    origin: Some("https://acsing.service.kugou.com"),
    salt: crate::catalog::def::Salt::None,
    clear_params: true,
    ..spec(
        "audio.ktv.total",
        "audio_ktv_total",
        crate::http::Method::Get,
        "/sing7/listenguide/json/v2/cdn/listenguide/get_total_opus_num_v02.do",
    )
};
const EP_SONG_CLIMAX: EndpointSpec = EndpointSpec {
    origin: Some("https://expendablekmrcdn.kugou.com"),
    ..spec(
        "song.climax",
        "song_climax",
        crate::http::Method::Get,
        "/v1/audio_climax/audio",
    )
};
const EP_SONG_RANKING: EndpointSpec = spec(
    "song.ranking",
    "song_ranking",
    crate::http::Method::Get,
    "/grow/v1/song_ranking/play_page/ranking_info",
);
const EP_SONG_RANKING_FILTER: EndpointSpec = spec(
    "song.ranking.filter",
    "song_ranking_filter",
    crate::http::Method::Get,
    "/grow/v1/song_ranking/unlock/v2/ranking_filter",
);

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(
            &EP_AUDIO_ACCOMPANY_MATCHING,
            |p: &AudioAccompanyMatchingParams, ctx| {
                let pairs = [
                    ("isteen", "0".to_owned()),
                    ("mixId", js::unwrap_or_flex(&p.mix_id, 0).to_string()),
                    ("usemkv", "1".to_owned()),
                    ("platform", "2".to_owned()),
                    ("fileName", p.file_name.clone().unwrap_or_default()),
                    ("hash", p.hash.clone()),
                    ("version", "12375".to_owned()),
                    ("appid", ctx.appid.to_string()),
                ];
                let sign = sing7_sign(&pairs);
                let mut query = QueryRecord::new();
                query.set("isteen", 0);
                query.set("mixId", js::unwrap_or_flex(&p.mix_id, 0));
                query.set("usemkv", 1);
                query.set("platform", 2);
                query.set("fileName", p.file_name.clone().unwrap_or_default());
                query.set("hash", p.hash.clone());
                query.set("version", 12375);
                query.set("appid", ctx.appid);
                query.set("sign", sign);
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_AUDIO_KTV_TOTAL, |p: &AudioKtvTotalParams, ctx| {
            let pairs = [
                ("isteen", "0".to_owned()),
                ("songId", js::unwrap_or_flex(&p.song_id, 0).to_string()),
                ("usemkv", "1".to_owned()),
                ("platform", "2".to_owned()),
                ("singerName", p.singer_name.clone().unwrap_or_default()),
                ("songHash", p.song_hash.clone()),
                ("version", "12375".to_owned()),
                ("appid", ctx.appid.to_string()),
            ];
            let sign = sing7_sign(&pairs);
            let mut query = QueryRecord::new();
            query.set("isteen", 0);
            query.set("songId", js::unwrap_or_flex(&p.song_id, 0));
            query.set("usemkv", 1);
            query.set("platform", 2);
            query.set("singerName", p.singer_name.clone().unwrap_or_default());
            query.set("songHash", p.song_hash.clone());
            query.set("version", 12375);
            query.set("appid", ctx.appid);
            query.set("sign", sign);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SONG_CLIMAX, |p: &SongClimaxParams, _| {
            let data: Vec<Value> = p.hash.split(',').map(|s| json!({ "hash": s })).collect();
            let mut query = QueryRecord::new();
            query.set("data", serde_json::to_string(&data).unwrap_or_default());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SONG_RANKING, |p: &SongRankingParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("album_audio_id", p.album_audio_id.value());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SONG_RANKING_FILTER, |p: &SongRankingFilterParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("album_audio_id", p.album_audio_id.value());
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::SongApi<'_> {
    /// Accompany matching (`audio.accompany.matching`).
    pub async fn accompany_matching(&self, params: AudioAccompanyMatchingParams) -> Result<Value> {
        self.client
            .call_value("audio.accompany.matching", params)
            .await
    }

    /// KTV totals (`audio.ktv.total`).
    pub async fn ktv_total(&self, params: AudioKtvTotalParams) -> Result<Value> {
        self.client.call_value("audio.ktv.total", params).await
    }

    /// Climax segments (`song.climax`).
    pub async fn climax(&self, params: SongClimaxParams) -> Result<Value> {
        self.client.call_value("song.climax", params).await
    }

    /// Song ranking info (`song.ranking`).
    pub async fn ranking(&self, params: SongRankingParams) -> Result<Value> {
        self.client.call_value("song.ranking", params).await
    }

    /// Song ranking filter (`song.ranking.filter`).
    pub async fn ranking_filter(&self, params: SongRankingFilterParams) -> Result<Value> {
        self.client.call_value("song.ranking.filter", params).await
    }
}
