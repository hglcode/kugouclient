//! Playlist-domain endpoints.
//!
//! Ported from `domains/playlist.ts`.

use serde_json::{Value, json};

use crate::catalog::ctx::CallCtx;
use crate::catalog::def::{
    Built, EndpointSpec, NoParams, Salt, handler_endpoint, simple_endpoint, spec,
};
use crate::crypto::CipherInput;
use crate::error::{Error, Result};
use crate::js::{self, FlexI64, FlexString};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `playlist.detail` / `playlist.similar`.
    pub PlaylistDetailParams {
        required {
            /// Comma-separated `global_collection_id` values.
            ids: String => "ids",
        }
        optional {
            /// User id override.
            userid: FlexI64 => "userid",
            /// Token override.
            token: String => "token",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.effect`.
    pub PlaylistEffectParams {
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
    /// Parameters for `playlist.track.all`.
    pub PlaylistTrackAllParams {
        required {
            /// `global_collection_id`.
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
    /// Parameters for `playlist.track.all.new`.
    pub PlaylistTrackAllNewParams {
        required {
            /// List id.
            listid: FlexString => "listid",
        }
        optional {
            /// List type.
            list_type: FlexI64 => "type",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size (default 300).
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.tracks.del`.
    pub PlaylistTracksDelParams {
        required {
            /// List id.
            listid: FlexString => "listid",
        }
        optional {
            /// Comma-separated file ids.
            fileids: String => "fileids",
        }
    }
}

kugou_params! {
    /// Parameters for `sheet.collection`.
    pub SheetCollectionParams {
        required {}
        optional {
            /// Square position (default 2).
            position: FlexI64 => "position",
        }
    }
}

kugou_params! {
    /// Parameters for `sheet.detail`.
    pub SheetDetailParams {
        required {
            /// Opern id.
            id: FlexString => "id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `sheet.explore` / `sheet.rank`.
    pub SheetExploreParams {
        required {}
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Opern level.
            level: FlexI64 => "level",
            /// Instrument filter.
            instruments: FlexI64 => "instruments",
            /// Tag id.
            tagid: FlexI64 => "tagid",
        }
    }
}

kugou_params! {
    /// Parameters for `sheet.song`.
    pub SheetSongParams {
        required {
            /// Mixsong id.
            album_audio_id: FlexI64 => "album_audio_id",
        }
        optional {
            /// Instrument filter (default 1).
            instruments: FlexI64 => "instruments",
            /// Opern level (default 0).
            level: FlexI64 => "level",
        }
    }
}

kugou_params! {
    /// Parameters for `playlist.share.official`.
    pub PlaylistShareOfficialParams {
        required {
            /// Playlist id (`global_collection_id`).
            id: FlexString => "id",
        }
        optional {
            /// Share type (1 = own playlist, default).
            share_type: FlexI64 => "type",
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

const EP_PLAYLIST_DETAIL: EndpointSpec = EndpointSpec {
    router: Some("pubsongs.kugou.com"),
    ..spec(
        "playlist.detail",
        "playlist_detail",
        crate::http::Method::Post,
        "/v3/get_list_info",
    )
};
const EP_PLAYLIST_EFFECT: EndpointSpec = spec(
    "playlist.effect",
    "playlist_effect",
    crate::http::Method::Post,
    "/pubsongs/v1/get_sound_effect_list",
);
const EP_PLAYLIST_SIMILAR: EndpointSpec = spec(
    "playlist.similar",
    "playlist_similar",
    crate::http::Method::Post,
    "/pubsongs/v1/kmr_get_similar_lists",
);
const EP_PLAYLIST_TAGS: EndpointSpec = spec(
    "playlist.tags",
    "playlist_tags",
    crate::http::Method::Post,
    "/pubsongs/v1/get_tags_by_type",
);
const EP_PLAYLIST_TRACK_ALL: EndpointSpec = spec(
    "playlist.track.all",
    "playlist_track_all",
    crate::http::Method::Get,
    "/pubsongs/v2/get_other_list_file_nofilt",
);
const EP_PLAYLIST_TRACK_ALL_NEW: EndpointSpec = EndpointSpec {
    router: Some("cloudlist.service.kugou.com"),
    ..spec(
        "playlist.track.all.new",
        "playlist_track_all_new",
        crate::http::Method::Post,
        "/v4/get_list_all_file_v3",
    )
};
const EP_PLAYLIST_TRACKS_DEL: EndpointSpec = EndpointSpec {
    router: Some("cloudlist.service.kugou.com"),
    ..spec(
        "playlist.tracks.del",
        "playlist_tracks_del",
        crate::http::Method::Post,
        "/v4/delete_songs",
    )
};
const EP_SHEET_COLLECTION: EndpointSpec = EndpointSpec {
    salt: Salt::Web,
    ..spec(
        "sheet.collection",
        "sheet_collection",
        crate::http::Method::Get,
        "/miniyueku/v1/opern_square/get_home_module_config",
    )
};
const EP_SHEET_DETAIL: EndpointSpec = spec(
    "sheet.detail",
    "sheet_detail",
    crate::http::Method::Get,
    "/opern/v1/detail/info",
);
const EP_SHEET_EXPLORE: EndpointSpec = spec(
    "sheet.explore",
    "sheet_explore",
    crate::http::Method::Post,
    "/opern/v1/home/get_rec_opern",
);
const EP_SHEET_RANK: EndpointSpec = spec(
    "sheet.rank",
    "sheet_rank",
    crate::http::Method::Post,
    "/opern/v1/home/get_rank_opern",
);
const EP_SHEET_SONG: EndpointSpec = spec(
    "sheet.song",
    "sheet_song",
    crate::http::Method::Get,
    "/opern/v1/detail/song_info",
);
const EP_SHEET_TAGS: EndpointSpec = spec(
    "sheet.tags",
    "sheet_tags",
    crate::http::Method::Get,
    "/opern/v1/home/get_tags",
);
const EP_PLAYLIST_SHARE_OFFICIAL: EndpointSpec = EndpointSpec {
    origin: Some("http://t.service.kugou.com"),
    salt: Salt::None,
    clear_params: true,
    clear_headers: true,
    ..spec(
        "playlist.share.official",
        "playlist_share_official",
        crate::http::Method::Get,
        "/app",
    )
};

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_PLAYLIST_DETAIL, |p: &PlaylistDetailParams, ctx| {
            let data: Vec<Value> = p
                .ids
                .split(',')
                .map(|s| json!({ "'global_collection_id'": s }))
                .collect();
            let body = json!({
                "data": data,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "token": p
                    .token
                    .clone()
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| ctx.token.clone()),
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_PLAYLIST_EFFECT, |p: &PlaylistEffectParams, _| {
            let body = json!({
                "page": js::or_flex(&p.page, 1),
                "pagesize": js::or_flex(&p.page_size, 30),
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_PLAYLIST_SIMILAR, |p: &PlaylistDetailParams, ctx| {
            let data: Vec<Value> = p
                .ids
                .split(',')
                .map(|s| json!({ "'global_collection_id'": s }))
                .collect();
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "clienttime": ctx.clienttime,
                "key": ctx.sign_params_key(ctx.clienttime),
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "ugc": 1,
                "show_list": 1,
                "need_songs": 1,
                "data": data,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_PLAYLIST_TAGS, |_: &NoParams, _| {
            let body = json!({
                "tag_type": "collection",
                "tag_id": 0,
                "source": 3,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_PLAYLIST_TRACK_ALL, |p: &PlaylistTrackAllParams, _| {
            let page_size = js::or_flex(&p.page_size, 30);
            let mut query = QueryRecord::new();
            query.set("area_code", 1);
            query.set("begin_idx", (js::or_flex(&p.page, 1) - 1) * page_size);
            query.set("plat", 1);
            query.set("type", 1);
            query.set("mode", 1);
            query.set("personal_switch", 1);
            query.set("extend_fields", "abtags,hot_cmt,popularization");
            query.set("pagesize", page_size);
            query.set(
                "global_collection_id",
                p.id.value()
                    .map_or_else(String::new, std::string::ToString::to_string),
            );
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_PLAYLIST_TRACK_ALL_NEW,
            |p: &PlaylistTrackAllNewParams, ctx| {
                let body = json!({
                    "listid": p.listid.value().map(std::string::ToString::to_string),
                    "userid": ctx.userid,
                    "type": js::or_flex(&p.list_type, 0),
                    "page": js::or_flex(&p.page, 1),
                    "pagesize": js::or_flex(&p.page_size, 300),
                    "area_code": 1,
                    "allplatform": 1,
                    "show_cover": 1,
                    "token": ctx.token,
                });
                Built {
                    json: Some(body),
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_PLAYLIST_TRACKS_DEL,
            |p: &PlaylistTracksDelParams, ctx| {
                let data: Vec<Value> = p
                    .fileids
                    .clone()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i64>().ok().map(|n| json!({ "fileid": n })))
                    .collect();
                let body = json!({
                    "listid": p.listid.value().map(std::string::ToString::to_string),
                    "userid": ctx.userid,
                    "data": data,
                    "type": 0,
                    "token": ctx.token,
                    "list_ver": 0,
                });
                Built {
                    json: Some(body),
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_SHEET_COLLECTION, |p: &SheetCollectionParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("srcappid", ctx.srcappid);
            query.set("position", js::unwrap_or_flex(&p.position, 2));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SHEET_DETAIL, |p: &SheetDetailParams, _| {
            let mut query = QueryRecord::new();
            query.set(
                "opern_id",
                p.id.value()
                    .map_or_else(String::new, std::string::ToString::to_string),
            );
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SHEET_EXPLORE, |p: &SheetExploreParams, _| {
            let query = sheet_explore_query(p);
            let body = json!({ "exposure_mixids": "" });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SHEET_RANK, |p: &SheetExploreParams, _| {
            let query = sheet_explore_query(p);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SHEET_SONG, |p: &SheetSongParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("mixsongid", p.album_audio_id.value());
            query.set("instruments", js::unwrap_or_flex(&p.instruments, 1));
            query.set("opern_level", js::unwrap_or_flex(&p.level, 0));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SHEET_TAGS, |_: &NoParams, _| Built::default()),
        // ── official share-link report (upstream #293, t.service short links) ──
        handler_endpoint(&EP_PLAYLIST_SHARE_OFFICIAL, playlist_share_official_handler),
    ]
}

fn sheet_explore_query(p: &SheetExploreParams) -> QueryRecord {
    let mut query = QueryRecord::new();
    query.set("pagesize", js::or_flex(&p.page_size, 30));
    query.set("page", js::or_flex(&p.page, 1));
    query.set("opern_level", js::or_flex(&p.level, 0));
    query.set("instruments", js::or_flex(&p.instruments, 1));
    query.set("tagid", js::or_flex(&p.tagid, 0));
    query
}

fn playlist_share_official_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a PlaylistShareOfficialParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let id =
            p.id.value()
                .map(std::string::ToString::to_string)
                .unwrap_or_default();
        if id.is_empty() {
            return Err(Error::Api {
                code: 0,
                message: "playlist id must not be empty".to_owned(),
                payload: None,
            });
        }
        let is_type1 = js::or_flex(&p.share_type, 1) == 1;
        let userid = js::or_chain(
            &[
                p.userid.as_ref().and_then(FlexI64::value),
                Some(ctx.userid as i64),
            ],
            0,
        );
        let link = format!(
            "https://activity.kugou.com/share/v-a00a45b0/index.html?u={userid}\
             &h1=143298444507010930495471534764048014549\
             &h2=406dd14b2efd5e51affd6da64c84d0f6\
             &specialid={}&global_specialid={id}&cType={}",
            if is_type1 { "-2147483648" } else { "0" },
            if is_type1 { '0' } else { '1' },
        );

        let mut query = QueryRecord::new();
        query.set("caller", "mlist");
        query.set("url", link.clone());
        query.set("cmid", 5);
        let link_md5 = ctx.md5(&CipherInput::Text(link.clone()));
        query.set(
            "md5",
            ctx.md5(&CipherInput::Text(format!("kgclientshare{link_md5}"))),
        );

        let built = Built {
            query,
            headers: vec![
                (
                    "User-Agent".to_owned(),
                    "Android9-1070-10597-18-0-Share-wifi".to_owned(),
                ),
                ("KG-THash".to_owned(), crate::crypto::random_thash()),
            ],
            ..Built::default()
        };
        ctx.dispatch_regular(&EP_PLAYLIST_SHARE_OFFICIAL, built)
            .await
    })
}

impl crate::api::PlaylistApi<'_> {
    /// Playlist detail (`playlist.detail`).
    pub async fn detail(&self, params: PlaylistDetailParams) -> Result<Value> {
        self.client.call_value("playlist.detail", params).await
    }

    /// Sound-effect list (`playlist.effect`).
    pub async fn effect(&self, params: PlaylistEffectParams) -> Result<Value> {
        self.client.call_value("playlist.effect", params).await
    }

    /// Similar playlists (`playlist.similar`).
    pub async fn similar(&self, params: PlaylistDetailParams) -> Result<Value> {
        self.client.call_value("playlist.similar", params).await
    }

    /// Playlist tags (`playlist.tags`).
    pub async fn tags(&self) -> Result<Value> {
        self.client
            .call_value("playlist.tags", NoParams::default())
            .await
    }

    /// Playlist tracks (`playlist.track.all`).
    pub async fn track_all(&self, params: PlaylistTrackAllParams) -> Result<Value> {
        self.client.call_value("playlist.track.all", params).await
    }

    /// Playlist tracks, new protocol (`playlist.track.all.new`).
    pub async fn track_all_new(&self, params: PlaylistTrackAllNewParams) -> Result<Value> {
        self.client
            .call_value("playlist.track.all.new", params)
            .await
    }

    /// Delete playlist tracks (`playlist.tracks.del`).
    pub async fn tracks_del(&self, params: PlaylistTracksDelParams) -> Result<Value> {
        self.client.call_value("playlist.tracks.del", params).await
    }

    /// Sheet collection (`sheet.collection`).
    pub async fn sheet_collection(&self, params: SheetCollectionParams) -> Result<Value> {
        self.client.call_value("sheet.collection", params).await
    }

    /// Sheet detail (`sheet.detail`).
    pub async fn sheet_detail(&self, params: SheetDetailParams) -> Result<Value> {
        self.client.call_value("sheet.detail", params).await
    }

    /// Sheet explore (`sheet.explore`).
    pub async fn sheet_explore(&self, params: SheetExploreParams) -> Result<Value> {
        self.client.call_value("sheet.explore", params).await
    }

    /// Sheet rank (`sheet.rank`).
    pub async fn sheet_rank(&self, params: SheetExploreParams) -> Result<Value> {
        self.client.call_value("sheet.rank", params).await
    }

    /// Sheet song (`sheet.song`).
    pub async fn sheet_song(&self, params: SheetSongParams) -> Result<Value> {
        self.client.call_value("sheet.song", params).await
    }

    /// Sheet tags (`sheet.tags`).
    pub async fn sheet_tags(&self) -> Result<Value> {
        self.client
            .call_value("sheet.tags", NoParams::default())
            .await
    }

    /// Official share-link report (`playlist.share.official`).
    pub async fn share_official(&self, params: PlaylistShareOfficialParams) -> Result<Value> {
        self.client
            .call_value("playlist.share.official", params)
            .await
    }
}
