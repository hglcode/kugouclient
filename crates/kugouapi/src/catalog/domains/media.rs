//! Media-domain endpoints (mv / audio aggregates / video).
//!
//! Ported from `domains/media.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, simple_endpoint, spec};
use crate::error::Result;
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `kmr.audio.mv`.
    pub KmrAudioMvParams {
        required {
            /// Comma-separated album audio ids.
            album_audio_id: String => "album_audio_id",
        }
        optional {
            /// Field selection.
            fields: String => "fields",
        }
    }
}

kugou_params! {
    /// Parameters for `krm.audio`.
    pub KrmAudioParams {
        required {
            /// Comma-separated album audio ids.
            album_audio_id: String => "album_audio_id",
        }
        optional {
            /// Field selection (default `base`).
            fields: String => "fields",
        }
    }
}

kugou_params! {
    /// Parameters for `video.detail`.
    pub VideoDetailParams {
        required {
            /// Comma-separated video ids.
            id: String => "id",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `video.privilege`.
    pub VideoPrivilegeParams {
        required {
            /// Comma-separated file hashes.
            hash: String => "hash",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `video.url`.
    pub VideoUrlParams {
        required {
            /// Video hash.
            hash: String => "hash",
        }
        optional {}
    }
}

const EP_KMR_AUDIO_MV: EndpointSpec = EndpointSpec {
    router: Some("openapi.kugou.com"),
    ..spec(
        "kmr.audio.mv",
        "kmr_audio_mv",
        crate::http::Method::Post,
        "/kmr/v1/audio/mv",
    )
};
const EP_KRM_AUDIO: EndpointSpec = EndpointSpec {
    router: Some("openapi.kugou.com"),
    ..spec(
        "krm.audio",
        "krm_audio",
        crate::http::Method::Post,
        "/kmr/v2/audio",
    )
};
const EP_VIDEO_DETAIL: EndpointSpec = EndpointSpec {
    router: Some("kmr.service.kugou.com"),
    clear_params: true,
    ..spec(
        "video.detail",
        "video_detail",
        crate::http::Method::Post,
        "/v1/video",
    )
};
const EP_VIDEO_PRIVILEGE: EndpointSpec = EndpointSpec {
    router: Some("media.store.kugou.com"),
    ..spec(
        "video.privilege",
        "video_privilege",
        crate::http::Method::Post,
        "/v1/get_video_privilege",
    )
};
const EP_VIDEO_URL: EndpointSpec = EndpointSpec {
    router: Some("trackermv.kugou.com"),
    tracker_key: true,
    ..spec(
        "video.url",
        "video_url",
        crate::http::Method::Get,
        "/v2/interface/index",
    )
};

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_KMR_AUDIO_MV, |p: &KmrAudioMvParams, _| {
            let data: Vec<Value> = p
                .album_audio_id
                .split(',')
                .map(|s| json!({ "album_audio_id": s }))
                .collect();
            let body = json!({
                "data": data,
                "fields": p.fields.clone().unwrap_or_default(),
            });
            Built {
                json: Some(body),
                headers: vec![("KG-TID".to_owned(), "38".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_KRM_AUDIO, |p: &KrmAudioParams, _| {
            let data: Vec<Value> = p
                .album_audio_id
                .split(',')
                .filter_map(|s| {
                    s.trim()
                        .parse::<i64>()
                        .ok()
                        .map(|n| json!({ "entity_id": n }))
                })
                .collect();
            let body = json!({
                "data": data,
                "fields": p.fields.clone().filter(|f| !f.is_empty()).unwrap_or_else(|| "base".to_owned()),
            });
            Built {
                json: Some(body),
                headers: vec![("KG-TID".to_owned(), "238".to_owned())],
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_VIDEO_DETAIL, |p: &VideoDetailParams, ctx| {
            let data: Vec<Value> = p.id.split(',').map(|s| json!({ "video_id": s })).collect();
            let body = json!({
                "appid": ctx.appid,
                "clientver": ctx.clientver,
                "clienttime": ctx.clienttime,
                "mid": ctx.mid,
                "uuid": ctx.uuid,
                "dfid": ctx.dfid,
                "token": ctx.token,
                "key": ctx.sign_params_key(ctx.clienttime.to_string()),
                "show_resolution": 1,
                "data": data,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_VIDEO_PRIVILEGE, |p: &VideoPrivilegeParams, ctx| {
            let resource: Vec<Value> = p
                .hash
                .split(',')
                .map(|s| json!({ "hash": s, "id": 0, "name": "" }))
                .collect();
            let body = json!({
                "appid": ctx.appid,
                "area_code": 1,
                "behavior": "play",
                "clientver": ctx.clientver,
                "dfid": ctx.dfid,
                "mid": ctx.mid,
                "resource": resource,
                "token": ctx.token,
                "userid": ctx.userid,
                "vip": ctx.vip_type,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_VIDEO_URL, |p: &VideoUrlParams, _| {
            let mut query = QueryRecord::new();
            query.set("backupdomain", 1);
            query.set("cmd", 123);
            query.set("ext", "mp4");
            query.set("ismp3", 0);
            query.set("hash", p.hash.clone());
            query.set("pid", 1);
            query.set("type", 1);
            Built {
                query,
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::MediaApi<'_> {
    /// Audio MV aggregate (`kmr.audio.mv`).
    pub async fn kmr_audio_mv(&self, params: KmrAudioMvParams) -> Result<Value> {
        self.client.call_value("kmr.audio.mv", params).await
    }

    /// Audio aggregate (`krm.audio`).
    pub async fn krm_audio(&self, params: KrmAudioParams) -> Result<Value> {
        self.client.call_value("krm.audio", params).await
    }

    /// Video detail (`video.detail`).
    pub async fn video_detail(&self, params: VideoDetailParams) -> Result<Value> {
        self.client.call_value("video.detail", params).await
    }

    /// Video privilege (`video.privilege`).
    pub async fn video_privilege(&self, params: VideoPrivilegeParams) -> Result<Value> {
        self.client.call_value("video.privilege", params).await
    }

    /// Video URL (`video.url`).
    pub async fn video_url(&self, params: VideoUrlParams) -> Result<Value> {
        self.client.call_value("video.url", params).await
    }
}
