//! Artist-domain endpoints.
//!
//! Ported from `domains/artist.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `artist.albums`.
    pub ArtistAlbumsParams {
        required {
            /// Author id.
            id: FlexI64 => "id",
        }
        optional {
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Page number.
            page: FlexI64 => "page",
            /// Sort (`hot` or newest).
            sort: String => "sort",
        }
    }
}

kugou_params! {
    /// Parameters for `artist.audios` / `artist.audios.new`.
    pub ArtistAudiosParams {
        required {
            /// Author id.
            id: FlexI64 => "id",
        }
        optional {
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Page number.
            page: FlexI64 => "page",
            /// Sort (`hot` or default).
            sort: String => "sort",
        }
    }
}

kugou_params! {
    /// Parameters for `artist.detail`.
    pub ArtistDetailParams {
        required {
            /// Author id.
            id: FlexI64 => "id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `artist.follow.newsongs`.
    pub ArtistFollowNewsongsParams {
        required {}
        optional {
            /// Last seen album id.
            last_album_id: FlexI64 => "last_album_id",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Sort option (2 = by time).
            opt_sort: FlexI64 => "opt_sort",
        }
    }
}

kugou_params! {
    /// Parameters for `artist.honour`.
    pub ArtistHonourParams {
        required {
            /// Singer id.
            id: FlexI64 => "id",
        }
        optional {
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Page number.
            page: FlexI64 => "page",
        }
    }
}

kugou_params! {
    /// Parameters for `artist.lists`.
    pub ArtistListsParams {
        required {}
        optional {
            /// Musician flag.
            musician: FlexI64 => "musician",
            /// Sex type filter.
            sextypes: FlexI64 => "sextypes",
            /// Category type.
            list_type: FlexI64 => "type",
            /// Hot list size.
            hotsize: FlexI64 => "hotsize",
        }
    }
}

kugou_params! {
    /// Parameters for `artist.videos`.
    pub ArtistVideosParams {
        required {
            /// Author id.
            id: FlexI64 => "id",
        }
        optional {
            /// Tag: official / live / fan / artist / all.
            tag: String => "tag",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Page number.
            page: FlexI64 => "page",
        }
    }
}

kugou_params! {
    /// Parameters for `singer.list`.
    pub SingerListParams {
        required {}
        optional {
            /// Hot list size (default 200).
            hotsize: FlexI64 => "hotsize",
            /// Sex type filter.
            sextype: FlexI64 => "sextype",
            /// Category type.
            list_type: FlexI64 => "type",
        }
    }
}

const EP_ARTIST_ALBUMS: EndpointSpec = EndpointSpec {
    router: Some("openapi.kugou.com"),
    ..spec(
        "artist.albums",
        "artist_albums",
        crate::http::Method::Post,
        "/kmr/v1/author/albums",
    )
};
const EP_ARTIST_AUDIOS: EndpointSpec = EndpointSpec {
    origin: Some("https://openapi.kugou.com"),
    router: Some("openapi.kugou.com"),
    ..spec(
        "artist.audios",
        "artist_audios",
        crate::http::Method::Post,
        "/kmr/v1/audio_group/author",
    )
};
const EP_ARTIST_AUDIOS_NEW: EndpointSpec = spec(
    "artist.audios.new",
    "artist_audios_new",
    crate::http::Method::Get,
    "/openapi/kmr/v2/audio_group/author",
);
const EP_ARTIST_DETAIL: EndpointSpec = EndpointSpec {
    router: Some("openapi.kugou.com"),
    ..spec(
        "artist.detail",
        "artist_detail",
        crate::http::Method::Post,
        "/kmr/v3/author",
    )
};
const EP_ARTIST_FOLLOW_NEWSONGS: EndpointSpec = spec(
    "artist.follow.newsongs",
    "artist_follow_newsongs",
    crate::http::Method::Post,
    "/feed/v1/follow/newsong_album_list",
);
const EP_ARTIST_HONOUR: EndpointSpec = EndpointSpec {
    origin: Some("http://h5activity.kugou.com"),
    ..spec(
        "artist.honour",
        "artist_honour",
        crate::http::Method::Post,
        "/v1/query_singer_honour_detail",
    )
};
const EP_ARTIST_LISTS: EndpointSpec = spec(
    "artist.lists",
    "artist_lists",
    crate::http::Method::Get,
    "/ocean/v6/singer/list",
);
const EP_ARTIST_VIDEOS: EndpointSpec = EndpointSpec {
    origin: Some("https://openapicdn.kugou.com"),
    ..spec(
        "artist.videos",
        "artist_videos",
        crate::http::Method::Get,
        "/kmr/v1/author/videos",
    )
};
const EP_SINGER_LIST: EndpointSpec = spec(
    "singer.list",
    "singer_list",
    crate::http::Method::Get,
    "/ocean/v6/singer/list",
);

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_ARTIST_ALBUMS, |p: &ArtistAlbumsParams, _| {
            let body = json!({
                "author_id": p.id.value(),
                "pagesize": js::or_flex(&p.page_size, 30),
                "page": js::or_flex(&p.page, 1),
                "sort": if p.sort.as_deref() == Some("hot") { 3 } else { 1 },
                "category": 1,
                "area_code": "all",
            });
            Built {
                json: Some(body),
                headers: vec![("kg-tid".to_owned(), "36".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ARTIST_AUDIOS, |p: &ArtistAudiosParams, ctx| {
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "mid": ctx.mid,
                "clienttime": ctx.clienttime,
                "key": ctx.sign_params_key(ctx.clienttime),
                "author_id": p.id.value(),
                "pagesize": js::or_flex(&p.page_size, 30),
                "page": js::or_flex(&p.page, 1),
                "sort": if p.sort.as_deref() == Some("hot") { 1 } else { 2 },
                "area_code": "all",
            });
            Built {
                json: Some(body),
                headers: vec![("kg-tid".to_owned(), "220".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ARTIST_AUDIOS_NEW, |p: &ArtistAudiosParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("author_id", p.id.value());
            query.set("area_code", "all");
            query.set(
                "sort",
                if p.sort.as_deref() == Some("hot") {
                    1
                } else {
                    2
                },
            );
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("replace_api_version", 1);
            query.set("mvdata_need", 1);
            query.set("show_audio_honor", 1);
            query.set("show_audio_tag", 1);
            query.set("replace_need", 1);
            Built {
                headers: vec![("kg-tid".to_owned(), "36".to_owned())],
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ARTIST_DETAIL, |p: &ArtistDetailParams, _| {
            let body = json!({ "author_id": p.id.value() });
            Built {
                json: Some(body),
                headers: vec![("kg-tid".to_owned(), "36".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_ARTIST_FOLLOW_NEWSONGS,
            |p: &ArtistFollowNewsongsParams, _| {
                let last_album_id = js::or_flex(&p.last_album_id, 0);
                let mut query = QueryRecord::new();
                query.set("last_album_id", last_album_id);
                query.set("page_size", js::or_flex(&p.page_size, 30));
                query.set(
                    "opt_sort",
                    if js::unwrap_or_flex(&p.opt_sort, 0) == 2 {
                        2
                    } else {
                        1
                    },
                );
                let body = json!({ "last_album_id": last_album_id });
                Built {
                    json: Some(body),
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_ARTIST_HONOUR, |p: &ArtistHonourParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("singer_id", p.id.value());
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ARTIST_LISTS, |p: &ArtistListsParams, _| {
            let mut query = QueryRecord::new();
            query.set("musician", js::or_flex(&p.musician, 0));
            query.set("sextype", js::or_flex(&p.sextypes, 0));
            query.set("showtype", 2);
            query.set("type", js::or_flex(&p.list_type, 0));
            query.set("hotsize", js::or_flex(&p.hotsize, 30));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ARTIST_VIDEOS, |p: &ArtistVideosParams, _| {
            let tag_idx = match p.tag.as_deref() {
                Some("official") => "18",
                Some("live") => "20",
                Some("fan") => "23",
                Some("artist") => "42419",
                _ => "",
            };
            let mut query = QueryRecord::new();
            query.set_opt("author_id", p.id.value());
            query.set("is_fanmade", "");
            query.set("tag_idx", tag_idx);
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SINGER_LIST, |p: &SingerListParams, _| {
            let mut query = QueryRecord::new();
            query.set("hotsize", js::unwrap_or_flex(&p.hotsize, 200));
            query.set("musician", 0);
            query.set("sextype", js::unwrap_or_flex(&p.sextype, 0));
            query.set("showtype", 2);
            query.set("type", js::unwrap_or_flex(&p.list_type, 0));
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::ArtistApi<'_> {
    /// Artist albums (`artist.albums`).
    pub async fn albums(&self, params: ArtistAlbumsParams) -> Result<Value> {
        self.client.call_value("artist.albums", params).await
    }

    /// Artist audios (`artist.audios`).
    pub async fn audios(&self, params: ArtistAudiosParams) -> Result<Value> {
        self.client.call_value("artist.audios", params).await
    }

    /// Artist audios, new protocol (`artist.audios.new`).
    pub async fn audios_new(&self, params: ArtistAudiosParams) -> Result<Value> {
        self.client.call_value("artist.audios.new", params).await
    }

    /// Artist detail (`artist.detail`).
    pub async fn detail(&self, params: ArtistDetailParams) -> Result<Value> {
        self.client.call_value("artist.detail", params).await
    }

    /// Followed artists' new songs (`artist.follow.newsongs`).
    pub async fn follow_newsongs(&self, params: ArtistFollowNewsongsParams) -> Result<Value> {
        self.client
            .call_value("artist.follow.newsongs", params)
            .await
    }

    /// Artist honours (`artist.honour`).
    pub async fn honour(&self, params: ArtistHonourParams) -> Result<Value> {
        self.client.call_value("artist.honour", params).await
    }

    /// Artist lists (`artist.lists`).
    pub async fn lists(&self, params: ArtistListsParams) -> Result<Value> {
        self.client.call_value("artist.lists", params).await
    }

    /// Artist videos (`artist.videos`).
    pub async fn videos(&self, params: ArtistVideosParams) -> Result<Value> {
        self.client.call_value("artist.videos", params).await
    }

    /// Singer list (`singer.list`).
    pub async fn singer_list(&self, params: SingerListParams) -> Result<Value> {
        self.client.call_value("singer.list", params).await
    }
}
