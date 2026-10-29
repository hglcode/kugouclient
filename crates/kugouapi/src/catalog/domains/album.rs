//! Album-domain endpoints.
//!
//! Ported from `domains/album.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64};

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `album.detail`.
    pub AlbumDetailParams {
        required {
            /// Album id.
            id: FlexI64 => "id",
        }
        optional {
            /// Purchase flag.
            is_buy: FlexI64 => "is_buy",
        }
    }
}

kugou_params! {
    /// Parameters for `album.songs`.
    pub AlbumSongsParams {
        required {
            /// Album id.
            id: FlexI64 => "id",
        }
        optional {
            /// Purchase flag.
            is_buy: FlexI64 => "is_buy",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

const EP_ALBUM_DETAIL: EndpointSpec = EndpointSpec {
    router: Some("openapi.kugou.com"),
    ..spec(
        "album.detail",
        "album_detail",
        crate::http::Method::Post,
        "/kmr/v2/albums",
    )
};
const EP_ALBUM_SHOP: EndpointSpec = spec(
    "album.shop",
    "album_shop",
    crate::http::Method::Get,
    "/zhuanjidata/v3/album_shop_v2/get_classify_data",
);
const EP_ALBUM_SONGS: EndpointSpec = EndpointSpec {
    router: Some("openapi.kugou.com"),
    ..spec(
        "album.songs",
        "album_songs",
        crate::http::Method::Post,
        "/v1/album_audio/lite",
    )
};

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_ALBUM_DETAIL, |p: &AlbumDetailParams, _| {
            let body = json!({
                "data": [{ "album_id": p.id.value() }],
                "is_buy": js::or_flex(&p.is_buy, 0),
                "fields": "album_id,album_name,publish_date,sizable_cover,intro,language,is_publish,heat,type,quality,authors,exclusive,author_name,trans_param",
            });
            Built {
                json: Some(body),
                headers: vec![("kg-tid".to_owned(), "255".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ALBUM_SHOP, |_: &NoParams, _| Built::default()),
        simple_endpoint(&EP_ALBUM_SONGS, |p: &AlbumSongsParams, _| {
            let body = json!({
                "album_id": p.id.value(),
                "is_buy": match p.is_buy.as_ref().and_then(FlexI64::value) { Some(v) if v != 0 => v.to_string(), _ => String::new() },
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30),
            });
            Built {
                json: Some(body),
                headers: vec![("kg-tid".to_owned(), "255".to_owned())],
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::AlbumApi<'_> {
    /// Album detail (`album.detail`).
    pub async fn detail(&self, params: AlbumDetailParams) -> Result<Value> {
        self.client.call_value("album.detail", params).await
    }

    /// Album shop classification (`album.shop`).
    pub async fn shop(&self) -> Result<Value> {
        self.client
            .call_value("album.shop", NoParams::default())
            .await
    }

    /// Album songs (`album.songs`).
    pub async fn songs(&self, params: AlbumSongsParams) -> Result<Value> {
        self.client.call_value("album.songs", params).await
    }
}
