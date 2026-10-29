//! Comment-domain regular endpoints.
//!
//! Ported from `domains/comment.ts`.

use serde_json::Value;

use crate::catalog::def::{Built, EndpointSpec, NoParams, Salt, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `comment.album` / `comment.playlist`.
    pub CommentListParams {
        required {
            /// Target id (album id or playlist id).
            id: FlexI64 => "id",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Show classify flag.
            show_classify: FlexI64 => "show_classify",
            /// Show hotword list flag.
            show_hotword_list: FlexI64 => "show_hotword_list",
        }
    }
}

kugou_params! {
    /// Parameters for `comment.count`.
    pub CommentCountParams {
        required {}
        optional {}
    }
}

kugou_params! {
    /// Parameters for `comment.music`.
    pub CommentMusicParams {
        required {
            /// Mixsong id.
            mixsongid: FlexI64 => "mixsongid",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Show classify flag.
            show_classify: FlexI64 => "show_classify",
            /// Show hotword list flag.
            show_hotword_list: FlexI64 => "show_hotword_list",
        }
    }
}

kugou_params! {
    /// Parameters for `comment.music.classify`.
    pub CommentMusicClassifyParams {
        required {
            /// Mixsong id.
            mixsongid: FlexI64 => "mixsongid",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Classify type id.
            type_id: FlexI64 => "type_id",
            /// Sort method.
            sort: FlexI64 => "sort",
        }
    }
}

kugou_params! {
    /// Parameters for `comment.music.hotword`.
    pub CommentMusicHotwordParams {
        required {
            /// Mixsong id.
            mixsongid: FlexI64 => "mixsongid",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Hot word filter.
            hot_word: String => "hot_word",
        }
    }
}

const EP_COMMENT_ALBUM: EndpointSpec = spec(
    "comment.album",
    "comment_album",
    crate::http::Method::Post,
    "/m.comment.service/v1/cmtlist",
);
const EP_COMMENT_COUNT: EndpointSpec = EndpointSpec {
    router: Some("sum.comment.service.kugou.com"),
    salt: Salt::Web,
    ..spec(
        "comment.count",
        "comment_count",
        crate::http::Method::Get,
        "/index.php",
    )
};
const EP_COMMENT_MUSIC: EndpointSpec = spec(
    "comment.music",
    "comment_music",
    crate::http::Method::Post,
    "/mcomment/v1/cmtlist",
);
const EP_COMMENT_MUSIC_CLASSIFY: EndpointSpec = spec(
    "comment.music.classify",
    "comment_music_classify",
    crate::http::Method::Post,
    "/mcomment/v1/cmt_classify_list",
);
const EP_COMMENT_MUSIC_HOTWORD: EndpointSpec = spec(
    "comment.music.hotword",
    "comment_music_hotword",
    crate::http::Method::Post,
    "/mcomment/v1/get_hot_word",
);
const EP_COMMENT_PLAYLIST: EndpointSpec = spec(
    "comment.playlist",
    "comment_playlist",
    crate::http::Method::Post,
    "/m.comment.service/v1/cmtlist",
);

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_COMMENT_ALBUM, |p: &CommentListParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("childrenid", p.id.value());
            query.set("need_show_image", 1);
            query.set("p", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("show_classify", js::or_flex(&p.show_classify, 1));
            query.set("show_hotword_list", js::or_flex(&p.show_hotword_list, 1));
            query.set("code", "94f1792ced1df89aa68a7939eaf2efca");
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_COMMENT_COUNT, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("r", "comments/getcommentsnum");
            query.set("code", "fc4be23b4e972707f36b8a828a93ba8a");
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_COMMENT_MUSIC, |p: &CommentMusicParams, _| {
            let mut query = QueryRecord::new();
            query.set("ver", comment_ver());
            query.set_opt("mixsongid", p.mixsongid.value());
            query.set("need_show_image", 1);
            query.set("p", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("show_classify", js::or_flex(&p.show_classify, 1));
            query.set("show_hotword_list", js::or_flex(&p.show_hotword_list, 1));
            query.set("extdata", "0");
            query.set("code", "fc4be23b4e972707f36b8a828a93ba8a");
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_COMMENT_MUSIC_CLASSIFY,
            |p: &CommentMusicClassifyParams, _| {
                let mut query = QueryRecord::new();
                query.set_opt("mixsongid", p.mixsongid.value());
                query.set("need_show_image", 1);
                query.set("page", js::or_flex(&p.page, 1));
                query.set("pagesize", js::or_flex(&p.page_size, 30));
                query.set_opt("type_id", p.type_id.as_ref().and_then(FlexI64::value));
                query.set("extdata", "0");
                query.set("code", "fc4be23b4e972707f36b8a828a93ba8a");
                query.set(
                    "sort_method",
                    if js::unwrap_or_flex(&p.sort, 0) == 2 {
                        js::unwrap_or_flex(&p.sort, 1)
                    } else {
                        1
                    },
                );
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_COMMENT_MUSIC_HOTWORD,
            |p: &CommentMusicHotwordParams, _| {
                let mut query = QueryRecord::new();
                query.set_opt("mixsongid", p.mixsongid.value());
                query.set("need_show_image", 1);
                query.set("p", js::or_flex(&p.page, 1));
                query.set("pagesize", js::or_flex(&p.page_size, 30));
                query.set_opt("hot_word", p.hot_word.clone());
                query.set("extdata", "0");
                query.set("code", "fc4be23b4e972707f36b8a828a93ba8a");
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_COMMENT_PLAYLIST, |p: &CommentListParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("childrenid", p.id.value());
            query.set("need_show_image", 1);
            query.set("p", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("show_classify", js::or_flex(&p.show_classify, 1));
            query.set("show_hotword_list", js::or_flex(&p.show_hotword_list, 1));
            query.set("code", "ca53b96fe5a1d9c22d71c8f522ef7c4f");
            query.set("content_type", 0);
            query.set("tag", 5);
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

/// The reference reads `process.env.CMT_VER || 6`.
fn comment_ver() -> i64 {
    std::env::var("CMT_VER")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6)
}

impl crate::api::CommentApi<'_> {
    /// Album comments (`comment.album`).
    pub async fn album(&self, params: CommentListParams) -> Result<Value> {
        self.client.call_value("comment.album", params).await
    }

    /// Comment counts (`comment.count`).
    pub async fn count(&self) -> Result<Value> {
        self.client
            .call_value("comment.count", CommentCountParams::default())
            .await
    }

    /// Music comments (`comment.music`).
    pub async fn music(&self, params: CommentMusicParams) -> Result<Value> {
        self.client.call_value("comment.music", params).await
    }

    /// Music comment classifications (`comment.music.classify`).
    pub async fn music_classify(&self, params: CommentMusicClassifyParams) -> Result<Value> {
        self.client
            .call_value("comment.music.classify", params)
            .await
    }

    /// Music comment hot words (`comment.music.hotword`).
    pub async fn music_hotword(&self, params: CommentMusicHotwordParams) -> Result<Value> {
        self.client
            .call_value("comment.music.hotword", params)
            .await
    }

    /// Playlist comments (`comment.playlist`).
    pub async fn playlist(&self, params: CommentListParams) -> Result<Value> {
        self.client.call_value("comment.playlist", params).await
    }
}
