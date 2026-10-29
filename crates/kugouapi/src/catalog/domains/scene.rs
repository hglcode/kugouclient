//! Scene / effects / theme-domain endpoints.
//!
//! Ported from `domains/scene.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `effects.artist` / `effects.car.brand.lists`.
    pub EffectsListParams {
        required {}
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Related id (car sound).
            rel_id: FlexI64 => "rel_id",
        }
    }
}

kugou_params! {
    /// Parameters for `effects.brand` / `effects.brand.detail`.
    pub EffectsBrandParams {
        required {}
        optional {
            /// Brand id.
            brand_id: FlexI64 => "brand_id",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `scene.audio.list`.
    pub SceneAudioListParams {
        required {
            /// Scene id.
            id: FlexI64 => "id",
        }
        optional {
            /// Module id.
            module_id: FlexI64 => "module_id",
            /// Tag filter.
            tag: String => "tag",
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `scene.collection.list` / `scene.video.list`.
    pub SceneTagListParams {
        required {
            /// Tag id.
            tag_id: FlexI64 => "tag_id",
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
    /// Parameters for `scene.lists.v2` / `scene.music`.
    pub SceneListParams {
        required {
            /// Scene id.
            id: FlexI64 => "id",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Sort: rec (default) / hot / new.
            sort: String => "sort",
        }
    }
}

kugou_params! {
    /// Parameters for `scene.module` / `scene.module.info`.
    pub SceneModuleParams {
        required {
            /// Scene id.
            id: FlexI64 => "id",
        }
        optional {
            /// Module id.
            module_id: FlexI64 => "module_id",
        }
    }
}

kugou_params! {
    /// Parameters for `theme.music`.
    pub ThemeMusicParams {
        required {
            /// Comma-separated theme category ids.
            ids: String => "ids",
        }
        optional {
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

kugou_params! {
    /// Parameters for `theme.music.detail`.
    pub ThemeMusicDetailParams {
        required {
            /// Theme category id.
            id: FlexI64 => "id",
        }
        optional {
            /// User id override.
            userid: FlexI64 => "userid",
        }
    }
}

kugou_params! {
    /// Parameters for `theme.playlist` / `theme.playlist.track`.
    pub ThemePlaylistParams {
        required {}
        optional {
            /// User id override.
            userid: FlexI64 => "userid",
            /// Theme id (`theme.playlist.track`).
            theme_id: FlexI64 => "theme_id",
        }
    }
}

macro_rules! effects_spec {
    ($id:literal, $module:literal, $method:expr, $path:literal) => {
        EndpointSpec {
            origin: Some("http://mobilecdngz.kugou.com"),
            clear_params: true,
            ..spec($id, $module, $method, $path)
        }
    };
}

const EP_EFFECTS_ARTIST: EndpointSpec = effects_spec!(
    "effects.artist",
    "effects_artist",
    crate::http::Method::Get,
    "/api/v3/sound/list"
);
const EP_EFFECTS_BRAND: EndpointSpec = effects_spec!(
    "effects.brand",
    "effects_brand",
    crate::http::Method::Get,
    "/api/v5/earphone/get_brand"
);
const EP_EFFECTS_BRAND_DETAIL: EndpointSpec = effects_spec!(
    "effects.brand.detail",
    "effects_brand_detail",
    crate::http::Method::Get,
    "/api/v5/earphone/get_model"
);
const EP_EFFECTS_CAR_BRAND: EndpointSpec = effects_spec!(
    "effects.car.brand",
    "effects_car_brand",
    crate::http::Method::Get,
    "/api/v5/car_sound/get_brand"
);
const EP_EFFECTS_CAR_BRAND_LISTS: EndpointSpec = effects_spec!(
    "effects.car.brand.lists",
    "effects_car_brand_lists",
    crate::http::Method::Get,
    "/api/v3/sound/list"
);
const EP_EFFECTS_MATCH: EndpointSpec = effects_spec!(
    "effects.match",
    "effects_match",
    crate::http::Method::Get,
    "/api/v5/earphone/match"
);
const EP_SCENE_AUDIO_LIST: EndpointSpec = spec(
    "scene.audio.list",
    "scene_audio_list",
    crate::http::Method::Post,
    "/scene/v1/scene/audio_list",
);
const EP_SCENE_COLLECTION_LIST: EndpointSpec = spec(
    "scene.collection.list",
    "scene_collection_list",
    crate::http::Method::Post,
    "/scene/v1/distribution/collection_list",
);
const EP_SCENE_LISTS: EndpointSpec = spec(
    "scene.lists",
    "scene_lists",
    crate::http::Method::Get,
    "/scene/v1/scene/list",
);
const EP_SCENE_LISTS_V2: EndpointSpec = spec(
    "scene.lists.v2",
    "scene_lists_v2",
    crate::http::Method::Post,
    "/scene/v1/scene/list_v2",
);
const EP_SCENE_MODULE: EndpointSpec = spec(
    "scene.module",
    "scene_module",
    crate::http::Method::Post,
    "/scene/v1/scene/module",
);
const EP_SCENE_MODULE_INFO: EndpointSpec = spec(
    "scene.module.info",
    "scene_module_info",
    crate::http::Method::Get,
    "/scene/v1/scene/module_info",
);
const EP_SCENE_MUSIC: EndpointSpec = spec(
    "scene.music",
    "scene_music",
    crate::http::Method::Post,
    "/genesisapi/v1/scene_music/rec_music",
);
const EP_SCENE_VIDEO_LIST: EndpointSpec = spec(
    "scene.video.list",
    "scene_video_list",
    crate::http::Method::Post,
    "/scene/v1/distribution/video_list",
);
const EP_THEME_MUSIC: EndpointSpec = spec(
    "theme.music",
    "theme_music",
    crate::http::Method::Post,
    "/everydayrec.service/v1/mul_theme_category_recommend",
);
const EP_THEME_MUSIC_DETAIL: EndpointSpec = spec(
    "theme.music.detail",
    "theme_music_detail",
    crate::http::Method::Post,
    "/everydayrec.service/v1/theme_category_recommend",
);
const EP_THEME_PLAYLIST: EndpointSpec = EndpointSpec {
    router: Some("everydayrec.service.kugou.com"),
    ..spec(
        "theme.playlist",
        "theme_playlist",
        crate::http::Method::Post,
        "/v2/getthemelist",
    )
};
const EP_THEME_PLAYLIST_TRACK: EndpointSpec = EndpointSpec {
    router: Some("everydayrec.service.kugou.com"),
    ..spec(
        "theme.playlist.track",
        "theme_playlist_track",
        crate::http::Method::Post,
        "/v2/gettheme_songidlist",
    )
};

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_EFFECTS_ARTIST, |p: &EffectsListParams, _| {
            let mut query = QueryRecord::new();
            query.set("plat", 2);
            query.set("version", 12460);
            query.set("apiver", 2);
            query.set("sort", 1);
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            query.set("classify", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_EFFECTS_BRAND, |p: &EffectsBrandParams, _| {
            let mut query = QueryRecord::new();
            query.set("sort", 1);
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_EFFECTS_BRAND_DETAIL, |p: &EffectsBrandParams, _| {
            let mut query = QueryRecord::new();
            query.set("brand_id", js::or_flex(&p.brand_id, 0));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_EFFECTS_CAR_BRAND, |_: &NoParams, _| Built::default()),
        simple_endpoint(&EP_EFFECTS_CAR_BRAND_LISTS, |p: &EffectsListParams, _| {
            let mut query = QueryRecord::new();
            query.set("plat", 2);
            query.set("version", 12460);
            query.set("apiver", 2);
            query.set("sort", 1);
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("page", js::or_flex(&p.page, 1));
            query.set("classify", 5);
            query.set("rel_id", js::or_flex(&p.rel_id, 0));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_EFFECTS_MATCH, |_: &NoParams, _| {
            let mut query = QueryRecord::new();
            query.set("plat", 2);
            query.set("version", 12460);
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SCENE_AUDIO_LIST, |p: &SceneAudioListParams, ctx| {
            let mut query = QueryRecord::new();
            query.set_opt("scene_id", p.id.value());
            query.set_opt("module_id", p.module_id.as_ref().and_then(FlexI64::value));
            query.set_opt("tag", p.tag.clone());
            query.set("page", js::or_flex(&p.page, 1));
            query.set("page_size", js::or_flex(&p.page_size, 30));
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "token": ctx.token,
                "userid": ctx.userid,
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SCENE_COLLECTION_LIST, |p: &SceneTagListParams, ctx| {
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "token": ctx.token,
                "userid": ctx.userid,
                "tag_id": p.tag_id.value(),
                "page": js::or_flex(&p.page, 1),
                "page_size": js::or_flex(&p.page_size, 30),
                "exposed_data": [],
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SCENE_LISTS, |_: &NoParams, _| Built::default()),
        simple_endpoint(&EP_SCENE_LISTS_V2, |p: &SceneListParams, ctx| {
            let sort_type = match p.sort.as_deref() {
                Some("hot") => 2,
                Some("new") => 3,
                _ => 1,
            };
            let mut query = QueryRecord::new();
            query.set_opt("scene_id", p.id.value());
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("sort_type", sort_type);
            query.set("kugouid", ctx.userid);
            let body = json!({ "exposure": [] });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SCENE_MODULE, |p: &SceneModuleParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("scene_id", p.id.value());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SCENE_MODULE_INFO, |p: &SceneModuleParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("scene_id", p.id.value());
            query.set_opt("module_id", p.module_id.as_ref().and_then(FlexI64::value));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SCENE_MUSIC, |p: &SceneListParams, _| {
            let mut query = QueryRecord::new();
            query.set_opt("scene_id", p.id.value());
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            let body = json!({ "exposure": [] });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_SCENE_VIDEO_LIST, |p: &SceneTagListParams, ctx| {
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "token": ctx.token,
                "userid": ctx.userid,
                "tag_id": p.tag_id.value(),
                "page": js::or_flex(&p.page, 1),
                "page_size": js::or_flex(&p.page_size, 30),
                "exposed_data": [],
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_THEME_MUSIC, |p: &ThemeMusicParams, ctx| {
            let body = json!({
                "platform": "android",
                "clienttime": ctx.clienttime,
                "show_theme_category_ids": p.ids,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "module_id": 508,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_THEME_MUSIC_DETAIL, |p: &ThemeMusicDetailParams, ctx| {
            let body = json!({
                "platform": "android",
                "clienttime": ctx.clienttime,
                "theme_category_id": p.id.value(),
                "show_theme_category_id": 0,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "module_id": 508,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_THEME_PLAYLIST, |p: &ThemePlaylistParams, ctx| {
            let body = json!({
                "platform": "android",
                "clientver": ctx.clientver,
                "clienttime": crate::crypto::now_ms(),
                "area_code": 1,
                "module_id": 1,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_THEME_PLAYLIST_TRACK, |p: &ThemePlaylistParams, ctx| {
            let body = json!({
                "platform": "android",
                "clientver": ctx.clientver,
                "clienttime": crate::crypto::now_ms(),
                "area_code": 1,
                "module_id": 1,
                "userid": js::or_chain(
                    &[p.userid.as_ref().and_then(FlexI64::value), Some(ctx.userid as i64)],
                    0,
                ),
                "theme_id": p.theme_id.as_ref().and_then(FlexI64::value),
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::SceneApi<'_> {
    /// Sound-effect artists (`effects.artist`).
    pub async fn effects_artist(&self, params: EffectsListParams) -> Result<Value> {
        self.client.call_value("effects.artist", params).await
    }

    /// Earphone brands (`effects.brand`).
    pub async fn effects_brand(&self, params: EffectsBrandParams) -> Result<Value> {
        self.client.call_value("effects.brand", params).await
    }

    /// Earphone models (`effects.brand.detail`).
    pub async fn effects_brand_detail(&self, params: EffectsBrandParams) -> Result<Value> {
        self.client.call_value("effects.brand.detail", params).await
    }

    /// Car-sound brands (`effects.car.brand`).
    pub async fn effects_car_brand(&self) -> Result<Value> {
        self.client
            .call_value("effects.car.brand", NoParams::default())
            .await
    }

    /// Car-sound lists (`effects.car.brand.lists`).
    pub async fn effects_car_brand_lists(&self, params: EffectsListParams) -> Result<Value> {
        self.client
            .call_value("effects.car.brand.lists", params)
            .await
    }

    /// Earphone match (`effects.match`).
    pub async fn effects_match(&self) -> Result<Value> {
        self.client
            .call_value("effects.match", NoParams::default())
            .await
    }

    /// Scene audio list (`scene.audio.list`).
    pub async fn scene_audio_list(&self, params: SceneAudioListParams) -> Result<Value> {
        self.client.call_value("scene.audio.list", params).await
    }

    /// Scene collection list (`scene.collection.list`).
    pub async fn scene_collection_list(&self, params: SceneTagListParams) -> Result<Value> {
        self.client
            .call_value("scene.collection.list", params)
            .await
    }

    /// Scene list (`scene.lists`).
    pub async fn scene_lists(&self) -> Result<Value> {
        self.client
            .call_value("scene.lists", NoParams::default())
            .await
    }

    /// Scene list v2 (`scene.lists.v2`).
    pub async fn scene_lists_v2(&self, params: SceneListParams) -> Result<Value> {
        self.client.call_value("scene.lists.v2", params).await
    }

    /// Scene module (`scene.module`).
    pub async fn scene_module(&self, params: SceneModuleParams) -> Result<Value> {
        self.client.call_value("scene.module", params).await
    }

    /// Scene module info (`scene.module.info`).
    pub async fn scene_module_info(&self, params: SceneModuleParams) -> Result<Value> {
        self.client.call_value("scene.module.info", params).await
    }

    /// Scene music (`scene.music`).
    pub async fn scene_music(&self, params: SceneListParams) -> Result<Value> {
        self.client.call_value("scene.music", params).await
    }

    /// Scene video list (`scene.video.list`).
    pub async fn scene_video_list(&self, params: SceneTagListParams) -> Result<Value> {
        self.client.call_value("scene.video.list", params).await
    }

    /// Theme music (`theme.music`).
    pub async fn theme_music(&self, params: ThemeMusicParams) -> Result<Value> {
        self.client.call_value("theme.music", params).await
    }

    /// Theme music detail (`theme.music.detail`).
    pub async fn theme_music_detail(&self, params: ThemeMusicDetailParams) -> Result<Value> {
        self.client.call_value("theme.music.detail", params).await
    }

    /// Theme playlist (`theme.playlist`).
    pub async fn theme_playlist(&self, params: ThemePlaylistParams) -> Result<Value> {
        self.client.call_value("theme.playlist", params).await
    }

    /// Theme playlist tracks (`theme.playlist.track`).
    pub async fn theme_playlist_track(&self, params: ThemePlaylistParams) -> Result<Value> {
        self.client.call_value("theme.playlist.track", params).await
    }
}
