//! Listen-together endpoints (`listen.together.*`).
//!
//! The reference implementation dispatches by `operation` to different URLs;
//! here each operation is an independent endpoint id (e.g.
//! `listen.together.music.list`) for a more direct call surface.
//!
//! Ported from `special/listenTogether.ts`.

use serde_json::{Map, Value, json};

use crate::catalog::def::{Built, EndpointSpec, NoParams, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64, FlexString};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

const YOUTH_BASE: &str = "https://youth.kugou.com";
const SELF_STUDY_BIZ: &str = "1000";
const MUSIC_ROOM_BIZ: &str = "1009";
const DEFAULT_MUSIC_ROOM_BG: &str =
    "https://youthimgbssdl.kugou.com/6e9cdcef8d163d06225d8cbeaa2f1ece.JPEG";
const PAGE_ID: u64 = 711_586_122;
const PARENT_PAGE_ID: &str = "356753938";

/// Shared listen-together parameters (all optional; handlers apply the
/// reference fallback chains).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ListenTogetherParams {
    /// User id override (falls back to the session).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userid: Option<FlexI64>,
    /// Token override (falls back to the session).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Room biz (`1000` self-study, `1009` music room).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub biz: Option<String>,
    /// Room id (`groupid` / `room_id` chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groupid: Option<String>,
    /// Room id alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// Room id alias (`roomid`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roomid: Option<String>,
    /// Chat message type (default 801).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msgtype: Option<FlexI64>,
    /// Nickname for chat messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// Image for chat messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub img: Option<String>,
    /// Chat alert text (`alert` / `message` chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert: Option<String>,
    /// Chat message alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// History max id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maxid: Option<FlexString>,
    /// Page size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagesize: Option<FlexI64>,
    /// Page number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<FlexI64>,
    /// Search keyword.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    /// Search position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<FlexI64>,
    /// Square order type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_type: Option<FlexI64>,
    /// Geo longitude.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<FlexI64>,
    /// Geo latitude.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<FlexI64>,
    /// Privilege event type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<FlexI64>,
    /// Genting room biz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_biz: Option<FlexI64>,
    /// History cursor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_id: Option<FlexI64>,
    /// Loop-pick flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_pick: Option<FlexI64>,
    /// Room tags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    /// Mixsong id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixsongid: Option<FlexI64>,
    /// Recommend behaviors (array / JSON-array string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behaviors: Option<Value>,
    /// Sync source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frm: Option<FlexI64>,
    /// Audio hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Send-to-all flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sendall: Option<FlexI64>,
    /// Room audios (array / JSON-array string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audios: Option<Value>,
    /// Progress info (object / JSON string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_info: Option<Value>,
    /// Switch action type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act_type: Option<FlexI64>,
    /// List version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_version: Option<FlexString>,
    /// Auto-play flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_auto: Option<FlexI64>,
    /// Player operation action (1 mode / 2 progress / 3 pause).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<FlexI64>,
    /// Play mode (action 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play_mode: Option<FlexI64>,
    /// Progress (action 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<FlexI64>,
    /// Pause flag (action 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause: Option<FlexI64>,
    /// Playlist audio filter (object / JSON string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<Value>,
    /// Song order user id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_userid: Option<FlexI64>,
    /// Song source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FlexI64>,
    /// Room privacy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_privacy: Option<FlexI64>,
    /// Pass-through data (object / JSON string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_through_data: Option<Value>,
    /// Room introduction / name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub introduction: Option<String>,
    /// Room name alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_name: Option<String>,
    /// Room background URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_url: Option<String>,
    /// Room background type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_bg_type: Option<FlexI64>,
    /// Bound playlist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_collection_id: Option<String>,
    /// Room capacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<FlexI64>,
    /// Biz-defined data (array / JSON-array string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub biz_defined_data: Option<Value>,
    /// Chat switch (1 on, else off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat: Option<FlexI64>,
    /// Study channel id alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    /// Study list sort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<FlexI64>,
    /// Page id override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_id: Option<FlexI64>,
    /// Parent page id override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ppage_id: Option<String>,
    /// Tag id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<String>,
    /// Room notice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_notice: Option<String>,
    /// Allow chat flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_chat: Option<FlexI64>,
    /// Room tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_tag: Option<String>,
    /// Music type (1 / 2 audios, 3 white noise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub music_type: Option<FlexI64>,
    /// Music style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub music_style: Option<Value>,
    /// White-noise type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white_noise_type: Option<FlexI64>,
    /// Study room type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_type: Option<FlexI64>,
    /// Member type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_type: Option<FlexI64>,
}

fn opt_str(values: &[&Option<String>]) -> Option<String> {
    values.iter().find_map(|v| match v {
        Some(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    })
}

fn room_id(p: &ListenTogetherParams) -> String {
    opt_str(&[&p.roomid, &p.room_id, &p.groupid]).unwrap_or_default()
}

fn group_id(p: &ListenTogetherParams) -> String {
    opt_str(&[&p.groupid, &p.room_id]).unwrap_or_default()
}

fn biz_of(p: &ListenTogetherParams) -> String {
    opt_str(&[&p.biz]).unwrap_or_else(|| SELF_STUDY_BIZ.to_owned())
}

/// `(userid, token)` from params or session.
fn auth_pair(p: &ListenTogetherParams, ctx: &crate::catalog::ctx::CallCtx<'_>) -> (u64, String) {
    let userid = p
        .userid
        .as_ref()
        .and_then(FlexI64::value)
        .filter(|u| *u != 0)
        .map_or(ctx.userid, |u| u as u64);
    let token = p
        .token
        .clone()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| ctx.token.clone());
    (userid, token)
}

fn parse_array(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::String(text)) if text.starts_with('[') => serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn parse_object(value: Option<&Value>) -> Map<String, Value> {
    let parsed = match value {
        Some(Value::String(text)) => serde_json::from_str::<Value>(text).unwrap_or(Value::Null),
        Some(other) => other.clone(),
        None => Value::Null,
    };
    parsed.as_object().cloned().unwrap_or_default()
}

fn music_room_audios(p: &ListenTogetherParams) -> Vec<Value> {
    parse_array(p.audios.as_ref())
        .into_iter()
        .take(50)
        .filter_map(|audio| {
            let hash = audio
                .get("hash")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if hash.is_empty() {
                return None;
            }
            Some(json!({
                "hash": hash,
                "mixsongid": audio
                    .get("mixsongid")
                    .cloned()
                    .or_else(|| audio.get("mixSongId").cloned())
                    .unwrap_or(json!("")),
                "fid": audio.get("fid").cloned().unwrap_or(json!(0)),
            }))
        })
        .collect()
}

macro_rules! lt_spec {
    ($id:literal, $module:literal, $method:expr, $path:literal, $origin:expr) => {
        EndpointSpec {
            origin: Some($origin),
            ..spec($id, $module, $method, $path)
        }
    };
}

const EP_CHAT_SEND: EndpointSpec = spec(
    "listen.together.chat.send",
    "listen_together_chat",
    crate::http::Method::Post,
    "/rmservice/v1/group/chat",
);
const EP_CHAT_HISTORY: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.chat.history",
        "listen_together_chat",
        crate::http::Method::Post,
        "/rmservice/v1/group/msg_history",
    )
};
const EP_DISCOVERY_CHANNEL_SEARCH: EndpointSpec = lt_spec!(
    "listen.together.discovery.channel_search",
    "listen_together_discovery",
    crate::http::Method::Get,
    "/v1/search/channel",
    YOUTH_BASE
);
const EP_DISCOVERY_KUGROUP_SQUARE: EndpointSpec = lt_spec!(
    "listen.together.discovery.kugroup_square",
    "listen_together_discovery",
    crate::http::Method::Get,
    "/v1/kugroup/square",
    YOUTH_BASE
);
const EP_DISCOVERY_GENTING_SQUARE: EndpointSpec = lt_spec!(
    "listen.together.discovery.genting_square",
    "listen_together_discovery",
    crate::http::Method::Get,
    "/v1/genting/square",
    YOUTH_BASE
);
const EP_DISCOVERY_KUGROUP_STREAMERS: EndpointSpec = lt_spec!(
    "listen.together.discovery.kugroup_streamers",
    "listen_together_discovery",
    crate::http::Method::Get,
    "/v1/kugroup/get_streamer_list",
    YOUTH_BASE
);
const EP_DISCOVERY_GENTING_STREAMERS: EndpointSpec = lt_spec!(
    "listen.together.discovery.genting_streamers",
    "listen_together_discovery",
    crate::http::Method::Get,
    "/v1/genting/get_streamer_list",
    YOUTH_BASE
);
const EP_DISCOVERY_RECENT_ROOMS: EndpointSpec = lt_spec!(
    "listen.together.discovery.recent_rooms",
    "listen_together_discovery",
    crate::http::Method::Get,
    "/v3/user/recent_room_dynamic",
    YOUTH_BASE
);
const EP_DISCOVERY_PRIVILEGE: EndpointSpec = lt_spec!(
    "listen.together.discovery.privilege",
    "listen_together_discovery",
    crate::http::Method::Get,
    "/v1/privilege/operate",
    YOUTH_BASE
);
const EP_DISCOVERY_GENTING_RECOMMEND: EndpointSpec = EndpointSpec {
    origin: Some("https://concepts.kugou.com"),
    ..spec(
        "listen.together.discovery.genting_recommend",
        "listen_together_discovery",
        crate::http::Method::Post,
        "/v1/genting/recommend",
    )
};
const EP_MUSIC_HISTORY: EndpointSpec = spec(
    "listen.together.music.history",
    "listen_together_music",
    crate::http::Method::Get,
    "/youth/v1/genting/history",
);
const EP_MUSIC_LIST: EndpointSpec = spec(
    "listen.together.music.list",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v2/genting/recommend",
);
const EP_MUSIC_DETAIL: EndpointSpec = spec(
    "listen.together.music.detail",
    "listen_together_music",
    crate::http::Method::Get,
    "/youth/v1/genting/get_musicroom_info",
);
const EP_MUSIC_MEMBERS: EndpointSpec = spec(
    "listen.together.music.members",
    "listen_together_music",
    crate::http::Method::Get,
    "/youth/v1/genting/get_musicroom_member",
);
const EP_MUSIC_INITIALIZE: EndpointSpec = spec(
    "listen.together.music.initialize",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/init_musicroom",
);
const EP_MUSIC_SYNC_PLAYER: EndpointSpec = spec(
    "listen.together.music.sync_player",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/music_sync_player",
);
const EP_MUSIC_PLAYBACK_URL: EndpointSpec = spec(
    "listen.together.music.playback_url",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/music_reqcmd",
);
const EP_MUSIC_SWITCH_SONG: EndpointSpec = spec(
    "listen.together.music.switch_song",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/music_sw",
);
const EP_MUSIC_PLAYER_OPERATION: EndpointSpec = spec(
    "listen.together.music.player_operation",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/music_player_opr",
);
const EP_MUSIC_PLAYLIST: EndpointSpec = spec(
    "listen.together.music.playlist",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/music_fetch_list",
);
const EP_MUSIC_RECENT_PLAYLIST: EndpointSpec = spec(
    "listen.together.music.recent_playlist",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/music_recent_list",
);
const EP_MUSIC_ORDER_SONG: EndpointSpec = spec(
    "listen.together.music.order_song",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/order_song",
);
const EP_MUSIC_SONG_ORDER_LIST: EndpointSpec = spec(
    "listen.together.music.song_order_list",
    "listen_together_music",
    crate::http::Method::Get,
    "/youth/v1/genting/song_order_list",
);
const EP_MUSIC_REMOVE_SONG: EndpointSpec = spec(
    "listen.together.music.remove_song",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/remove_song",
);
const EP_MUSIC_ADD: EndpointSpec = spec(
    "listen.together.music.music_add",
    "listen_together_music",
    crate::http::Method::Post,
    "/youth/v1/genting/music_add",
);
const EP_ROOM_CREATE: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.create",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/group/create",
    )
};
const EP_ROOM_JOIN: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.join",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/group/join",
    )
};
const EP_ROOM_STATE: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.state",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/group/info",
    )
};
const EP_ROOM_HEARTBEAT: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.heartbeat",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/group/heartbeat",
    )
};
const EP_ROOM_STATUS: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.status",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/user/get_status",
    )
};
const EP_ROOM_LEAVE: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.leave",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/group/leave",
    )
};
const EP_ROOM_DISMISS: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.dismiss",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/group/dismiss",
    )
};
const EP_ROOM_UPDATE_CHAT: EndpointSpec = EndpointSpec {
    auth: true,
    ..spec(
        "listen.together.room.update_chat",
        "listen_together_room",
        crate::http::Method::Post,
        "/rmservice/v1/group/update_info",
    )
};
const EP_ROOM_CHECK_MINOR: EndpointSpec = lt_spec!(
    "listen.together.room.check_minor",
    "listen_together_room",
    crate::http::Method::Get,
    "/v1/risk/check_minor",
    YOUTH_BASE
);
const EP_STUDY_CREATED_ROOMS: EndpointSpec = lt_spec!(
    "listen.together.study.created_rooms",
    "listen_together_study",
    crate::http::Method::Get,
    "/v1/study/user_create_room_list",
    YOUTH_BASE
);
const EP_STUDY_DELETE_CREATED_ROOM: EndpointSpec = spec(
    "listen.together.study.delete_created_room",
    "listen_together_study",
    crate::http::Method::Delete,
    "/youth/v1/room/delete_room",
);
const EP_STUDY_LIST: EndpointSpec = lt_spec!(
    "listen.together.study.list",
    "listen_together_study",
    crate::http::Method::Get,
    "/v1/room/get_room_list_by_tag",
    YOUTH_BASE
);
const EP_STUDY_DETAIL: EndpointSpec = lt_spec!(
    "listen.together.study.detail",
    "listen_together_study",
    crate::http::Method::Get,
    "/v1/room/get_room_detail",
    YOUTH_BASE
);
const EP_STUDY_MEMBERS: EndpointSpec = lt_spec!(
    "listen.together.study.members",
    "listen_together_study",
    crate::http::Method::Get,
    "/v1/room/get_member_list",
    YOUTH_BASE
);
const EP_STUDY_CONFIGURE: EndpointSpec = EndpointSpec {
    origin: Some(YOUTH_BASE),
    auth: true,
    ..spec(
        "listen.together.study.configure",
        "listen_together_study",
        crate::http::Method::Post,
        "/v1/user/make_room",
    )
};
const EP_STUDY_SYNC_PLAYER: EndpointSpec = EndpointSpec {
    origin: Some(YOUTH_BASE),
    auth: true,
    ..spec(
        "listen.together.study.sync_player",
        "listen_together_study",
        crate::http::Method::Post,
        "/v1/music/sync_player",
    )
};
const EP_STUDY_PLAYLIST: EndpointSpec = EndpointSpec {
    origin: Some(YOUTH_BASE),
    auth: true,
    ..spec(
        "listen.together.study.playlist",
        "listen_together_study",
        crate::http::Method::Post,
        "/v1/music/fetch_list",
    )
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        // ── chat ──
        simple_endpoint(&EP_CHAT_SEND, |p: &ListenTogetherParams, ctx| {
            let (userid, token) = auth_pair(p, ctx);
            let body = json!({
                "userid": userid,
                "token": token,
                "biz": biz_of(p),
                "groupid": group_id(p),
                "message": {
                    "msgtype": js::or_flex(&p.msgtype, 801),
                    "nickname": p.nickname.clone().unwrap_or_default(),
                    "img": p.img.clone().unwrap_or_default(),
                    "alert": opt_str(&[&p.alert, &p.message]).unwrap_or_default(),
                },
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_CHAT_HISTORY, |p: &ListenTogetherParams, ctx| {
            let (userid, token) = auth_pair(p, ctx);
            let body = json!({
                "userid": userid,
                "token": token,
                "biz": biz_of(p),
                "groupid": group_id(p),
                "maxid": p.maxid.clone().map_or_else(|| "0".to_owned(), |v| v.or("0")),
                "pagesize": p.pagesize.clone().map_or_else(|| "20".to_owned(), |v| v.or(20).to_string()),
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        // ── discovery ──
        simple_endpoint(
            &EP_DISCOVERY_CHANNEL_SEARCH,
            |p: &ListenTogetherParams, _| {
                let mut query = QueryRecord::new();
                query.set("keyword", p.keyword.clone().unwrap_or_default());
                query.set("page", js::or_flex(&p.page, 1));
                query.set("position", js::or_flex(&p.position, 1));
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_DISCOVERY_KUGROUP_SQUARE,
            |p: &ListenTogetherParams, _| discovery_query(p),
        ),
        simple_endpoint(
            &EP_DISCOVERY_GENTING_SQUARE,
            |p: &ListenTogetherParams, _| discovery_query(p),
        ),
        simple_endpoint(
            &EP_DISCOVERY_KUGROUP_STREAMERS,
            |p: &ListenTogetherParams, _| {
                let mut query = QueryRecord::new();
                query.set("longitude", js::or_flex(&p.longitude, 0));
                query.set("latitude", js::or_flex(&p.latitude, 0));
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(
            &EP_DISCOVERY_GENTING_STREAMERS,
            |p: &ListenTogetherParams, _| {
                let mut query = QueryRecord::new();
                query.set("page", js::or_flex(&p.page, 1));
                query.set("pagesize", js::or_flex(&p.pagesize, 20));
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_DISCOVERY_RECENT_ROOMS, |_: &NoParams, _| {
            Built::default()
        }),
        simple_endpoint(&EP_DISCOVERY_PRIVILEGE, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("event_type", js::or_flex(&p.event_type, 4));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(
            &EP_DISCOVERY_GENTING_RECOMMEND,
            |p: &ListenTogetherParams, _| {
                let mut query = QueryRecord::new();
                query.set("page", js::or_flex(&p.page, 1));
                query.set("pagesize", js::or_flex(&p.pagesize, 20));
                query.set("room_biz", js::or_flex(&p.room_biz, 1006));
                Built {
                    json: Some(json!({})),
                    query,
                    ..Built::default()
                }
            },
        ),
        // ── music rooms ──
        simple_endpoint(&EP_MUSIC_HISTORY, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("last_id", js::or_flex(&p.last_id, 0));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_LIST, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("page", (js::or_flex(&p.page, 1) - 1).max(0));
            query.set("pagesize", js::or_flex(&p.pagesize, 20));
            query.set("loop_pick", js::or_flex(&p.loop_pick, 0));
            query.set("tags", p.tags.clone().unwrap_or_default());
            query.set("room_biz", MUSIC_ROOM_BIZ);
            if let Some(mixsongid) = p.mixsongid.as_ref().and_then(FlexI64::value) {
                query.set("mixsongid", mixsongid);
            }
            let body = json!({ "data": parse_array(p.behaviors.as_ref()) });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_DETAIL, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            query.set(
                "biz",
                opt_str(&[&p.biz]).unwrap_or_else(|| MUSIC_ROOM_BIZ.to_owned()),
            );
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_MEMBERS, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            query.set("page", js::or_flex(&p.page, 1).max(1));
            query.set("pagesize", js::or_flex(&p.pagesize, 100));
            query.set("apiver", "3");
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_INITIALIZE, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            let mut body = Map::new();
            body.insert(
                "sendall".to_owned(),
                json!(js::unwrap_or_flex(&p.sendall, 1)),
            );
            body.insert("audios".to_owned(), json!(music_room_audios(p)));
            let progress = parse_object(p.progress_info.as_ref());
            if !progress.is_empty() {
                body.insert("progress_info".to_owned(), Value::Object(progress));
            }
            Built {
                json: Some(Value::Object(body)),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_SYNC_PLAYER, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            query.set("frm", js::or_flex(&p.frm, 2));
            Built {
                json: Some(json!({})),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_PLAYBACK_URL, |p: &ListenTogetherParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("page_id", 147_780_134);
            query.set("ppage_id", "356753938");
            let (userid, token) = auth_pair(p, ctx);
            let body = json!({
                "userid": userid,
                "token": token,
                "roomid": room_id(p),
                "audio": {
                    "hash": p.hash.clone().unwrap_or_default(),
                    "mixsongid": p.mixsongid.clone().and_then(|v| v.value()),
                },
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_SWITCH_SONG, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            let body = json!({
                "act_type": js::or_flex(&p.act_type, 1),
                "list_version": p.list_version.clone().map(|v| v.or("")).unwrap_or_default(),
                "is_auto": if js::or_flex(&p.is_auto, 0) != 0 { "1" } else { "0" },
                "audio": {
                    "hash": p.hash.clone().unwrap_or_default(),
                    "mixsongid": p.mixsongid.clone().and_then(|v| v.value()),
                },
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_PLAYER_OPERATION, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            let action = js::or_flex(&p.action, 3);
            let mut body = Map::new();
            body.insert("action".to_owned(), json!(action));
            if action == 1 {
                body.insert("play_mode".to_owned(), json!(js::or_flex(&p.play_mode, 1)));
            }
            if action == 2 {
                body.insert(
                    "progress".to_owned(),
                    json!((js::or_flex(&p.progress, 0) as f64).floor().max(0.0) as i64),
                );
            }
            if action == 3 {
                body.insert(
                    "pause".to_owned(),
                    json!(if js::or_flex(&p.pause, 2) == 1 {
                        "1"
                    } else {
                        "2"
                    }),
                );
            }
            Built {
                json: Some(Value::Object(body)),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_PLAYLIST, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            let mut body = Map::new();
            body.insert("pagesize".to_owned(), json!(js::or_flex(&p.pagesize, 50)));
            let audio = parse_object(p.audio.as_ref());
            if let Some(hash) = audio.get("hash").and_then(Value::as_str)
                && !hash.is_empty()
            {
                body.insert(
                    "audio".to_owned(),
                    json!({
                        "hash": hash,
                        "mixsongid": audio.get("mixsongid").cloned()
                            .or_else(|| audio.get("mixSongId").cloned())
                            .unwrap_or(json!("")),
                    }),
                );
            }
            Built {
                json: Some(Value::Object(body)),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_RECENT_PLAYLIST, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            Built {
                text: Some(String::new()),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_ORDER_SONG, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            let body = json!({
                "mixsongid": p.mixsongid.clone().and_then(|v| v.value()),
                "hash": p.hash.clone().unwrap_or_default(),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_SONG_ORDER_LIST, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_REMOVE_SONG, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            let body = json!({
                "mixsongid": p.mixsongid.clone().and_then(|v| v.value()),
                "hash": p.hash.clone().unwrap_or_default(),
                "order_userid": js::or_flex(&p.order_userid, 0).to_string(),
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_MUSIC_ADD, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("roomid", room_id(p));
            if let Some(order_userid) = p.order_userid.as_ref().and_then(FlexI64::value)
                && order_userid != 0
            {
                query.set("order_userid", order_userid.to_string());
                query.set("source", js::or_flex(&p.source, 1).to_string());
            }
            let mut body = Map::new();
            body.insert("action".to_owned(), json!(js::or_flex(&p.action, 4)));
            body.insert(
                "list_version".to_owned(),
                json!(p.list_version.clone().map(|v| v.or("")).unwrap_or_default()),
            );
            body.insert(
                "sendall".to_owned(),
                json!(js::unwrap_or_flex(&p.sendall, 1)),
            );
            body.insert("audios".to_owned(), json!(music_room_audios(p)));
            let progress = parse_object(p.progress_info.as_ref());
            if !progress.is_empty() {
                body.insert("progress_info".to_owned(), Value::Object(progress));
            }
            Built {
                json: Some(Value::Object(body)),
                query,
                ..Built::default()
            }
        }),
        // ── rooms ──
        simple_endpoint(&EP_ROOM_CREATE, |p: &ListenTogetherParams, ctx| {
            let (userid, token) = auth_pair(p, ctx);
            let biz = biz_of(p);
            let room_privacy = p
                .room_privacy
                .as_ref()
                .and_then(FlexI64::value)
                .unwrap_or(if biz == MUSIC_ROOM_BIZ { 1 } else { 3 });
            let mut pass_through = parse_object(p.pass_through_data.as_ref());
            pass_through.insert("room_privacy".to_owned(), json!(room_privacy));
            pass_through.insert("cp_notice".to_owned(), json!(1));

            let mut body = Map::new();
            body.insert("userid".to_owned(), json!(userid));
            body.insert("token".to_owned(), json!(token));
            body.insert("biz".to_owned(), json!(biz));
            if biz == MUSIC_ROOM_BIZ {
                body.insert(
                    "introduction".to_owned(),
                    json!(opt_str(&[&p.introduction, &p.room_name]).unwrap_or_default()),
                );
                pass_through.insert(
                        "room_bg_content".to_owned(),
                        json!(serde_json::to_string(&json!({
                            "bg_img": p.background_url.clone().unwrap_or_else(|| DEFAULT_MUSIC_ROOM_BG.to_owned()),
                            "room_bg_type": js::or_flex(&p.room_bg_type, 2).to_string(),
                        }))
                        .unwrap_or_default()),
                    );
                if room_privacy == 1 {
                    pass_through.insert(
                        "global_collection_id".to_owned(),
                        json!(p.global_collection_id.clone().unwrap_or_default()),
                    );
                } else {
                    body.insert("capacity".to_owned(), json!(js::or_flex(&p.capacity, 5)));
                }
            } else {
                let biz_defined = parse_array(p.biz_defined_data.as_ref());
                body.insert(
                    "biz_defined_data".to_owned(),
                    if biz_defined.is_empty() {
                        json!([{ "key": "lyric_switch", "value": 1 }])
                    } else {
                        json!(biz_defined)
                    },
                );
            }
            body.insert("pass_through_data".to_owned(), Value::Object(pass_through));
            Built {
                json: Some(Value::Object(body)),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ROOM_JOIN, |p: &ListenTogetherParams, ctx| {
            let (userid, token) = auth_pair(p, ctx);
            let pass_through = p
                .pass_through_data
                .clone()
                .filter(serde_json::Value::is_object)
                .unwrap_or_else(|| json!({ "cp_notice": 1 }));
            let body = json!({
                "userid": userid,
                "token": token,
                "biz": biz_of(p),
                "groupid": group_id(p),
                "pass_through_data": pass_through,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ROOM_STATE, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("biz", biz_of(p));
            let body = json!({ "groupid": group_id(p) });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ROOM_HEARTBEAT, |p: &ListenTogetherParams, ctx| {
            group_operation_built(p, ctx)
        }),
        simple_endpoint(&EP_ROOM_STATUS, |p: &ListenTogetherParams, ctx| {
            group_operation_built(p, ctx)
        }),
        simple_endpoint(&EP_ROOM_LEAVE, |p: &ListenTogetherParams, ctx| {
            group_operation_built(p, ctx)
        }),
        simple_endpoint(&EP_ROOM_DISMISS, |p: &ListenTogetherParams, ctx| {
            group_operation_built(p, ctx)
        }),
        simple_endpoint(&EP_ROOM_UPDATE_CHAT, |p: &ListenTogetherParams, _| {
            let body = json!({
                "groupid": group_id(p),
                "biz": MUSIC_ROOM_BIZ,
                "switch": { "chat": if js::unwrap_or_flex(&p.chat, 0) == 1 { 1 } else { 2 } },
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_ROOM_CHECK_MINOR, |_: &NoParams, _| Built::default()),
        // ── study rooms ──
        simple_endpoint(&EP_STUDY_CREATED_ROOMS, |_: &NoParams, _| Built::default()),
        simple_endpoint(
            &EP_STUDY_DELETE_CREATED_ROOM,
            |p: &ListenTogetherParams, _| {
                let mut query = QueryRecord::new();
                query.set(
                    "global_collection_id",
                    opt_str(&[&p.global_collection_id, &p.channel_id]).unwrap_or_default(),
                );
                query.set("roomid", room_id(p));
                Built {
                    query,
                    ..Built::default()
                }
            },
        ),
        simple_endpoint(&EP_STUDY_LIST, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.pagesize, 20));
            query.set("sort", js::or_flex(&p.sort, 0));
            query.set("page_id", js::or_flex(&p.page_id, 191_708_212));
            query.set(
                "ppage_id",
                p.ppage_id
                    .clone()
                    .unwrap_or_else(|| PARENT_PAGE_ID.to_owned()),
            );
            query.set("tag_id", p.tag_id.clone().unwrap_or_default());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_STUDY_DETAIL, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("room_id", p.room_id.clone().unwrap_or_default());
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_STUDY_MEMBERS, |p: &ListenTogetherParams, _| {
            let mut query = QueryRecord::new();
            query.set("room_id", p.room_id.clone().unwrap_or_default());
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.pagesize, 20));
            query.set("member_type", js::unwrap_or_flex(&p.member_type, 1));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_STUDY_CONFIGURE, |p: &ListenTogetherParams, _| {
            let music_type = js::or_flex(&p.music_type, 1);
            let page_id_default = match music_type {
                2 => 971_343_961,
                3 => 711_357_575,
                _ => PAGE_ID as i64,
            };
            let mut query = QueryRecord::new();
            query.set("page_id", js::or_flex(&p.page_id, page_id_default));
            query.set(
                "ppage_id",
                p.ppage_id
                    .clone()
                    .unwrap_or_else(|| PARENT_PAGE_ID.to_owned()),
            );
            query.set("type", js::unwrap_or_flex(&p.list_type, 1));

            let mut body = Map::new();
            body.insert("room_id".to_owned(), json!(room_id(p)));
            body.insert(
                "room_name".to_owned(),
                json!(p.room_name.clone().unwrap_or_default()),
            );
            body.insert(
                "global_collection_id".to_owned(),
                json!(p.global_collection_id.clone().unwrap_or_default()),
            );
            body.insert(
                "room_notice".to_owned(),
                json!(p.room_notice.clone().unwrap_or_default()),
            );
            body.insert(
                "allow_chat".to_owned(),
                json!(js::unwrap_or_flex(&p.allow_chat, 1)),
            );
            body.insert(
                "room_tag".to_owned(),
                json!(p.room_tag.clone().unwrap_or_else(|| "2".to_owned())),
            );
            body.insert("music_type".to_owned(), json!(music_type));
            if (music_type == 1 || music_type == 2) && music_type != 3 {
                if let Some(style) = &p.music_style
                    && !style.is_null()
                {
                    body.insert("music_style".to_owned(), style.clone());
                }
                let audios = parse_array(p.audios.as_ref());
                if !audios.is_empty() {
                    body.insert("audios".to_owned(), json!(audios));
                }
            } else if music_type == 3 {
                body.insert(
                    "white_noise_type".to_owned(),
                    json!(js::or_flex(&p.white_noise_type, 1)),
                );
            }
            Built {
                json: Some(Value::Object(body)),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_STUDY_SYNC_PLAYER, |p: &ListenTogetherParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("page_id", js::or_flex(&p.page_id, PAGE_ID as i64));
            query.set(
                "ppage_id",
                p.ppage_id
                    .clone()
                    .unwrap_or_else(|| PARENT_PAGE_ID.to_owned()),
            );
            let (userid, token) = auth_pair(p, ctx);
            let body = json!({
                "roomid": room_id(p),
                "frm": js::or_flex(&p.frm, 2),
                "userid": userid,
                "token": token,
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_STUDY_PLAYLIST, |p: &ListenTogetherParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("page_id", js::or_flex(&p.page_id, PAGE_ID as i64));
            query.set(
                "ppage_id",
                p.ppage_id
                    .clone()
                    .unwrap_or_else(|| PARENT_PAGE_ID.to_owned()),
            );
            let (userid, token) = auth_pair(p, ctx);
            let body = json!({
                "roomid": room_id(p),
                "frm": js::or_flex(&p.frm, 2),
                "pagesize": js::or_flex(&p.pagesize, 50),
                "userid": userid,
                "token": token,
            });
            Built {
                json: Some(body),
                query,
                ..Built::default()
            }
        }),
    ]
}

fn discovery_query(p: &ListenTogetherParams) -> Built {
    let mut query = QueryRecord::new();
    query.set("page", js::or_flex(&p.page, 1));
    query.set("pagesize", js::or_flex(&p.pagesize, 20));
    query.set("order_type", js::or_flex(&p.order_type, 1));
    Built {
        query,
        ..Built::default()
    }
}

fn group_operation_built(
    p: &ListenTogetherParams,
    ctx: &crate::catalog::ctx::CallCtx<'_>,
) -> Built {
    let (userid, token) = auth_pair(p, ctx);
    let body = json!({
        "userid": userid,
        "token": token,
        "biz": biz_of(p),
        "groupid": group_id(p),
    });
    Built {
        json: Some(body),
        ..Built::default()
    }
}

impl crate::api::ListenTogetherApi<'_> {
    /// Send a room chat message (`listen.together.chat.send`).
    pub async fn chat_send(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.chat.send", params)
            .await
    }

    /// Room chat history (`listen.together.chat.history`).
    pub async fn chat_history(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.chat.history", params)
            .await
    }

    /// Channel search (`listen.together.discovery.channel_search`).
    pub async fn discovery_channel_search(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.discovery.channel_search", params)
            .await
    }

    /// Ku-group square (`listen.together.discovery.kugroup_square`).
    pub async fn discovery_kugroup_square(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.discovery.kugroup_square", params)
            .await
    }

    /// Genting square (`listen.together.discovery.genting_square`).
    pub async fn discovery_genting_square(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.discovery.genting_square", params)
            .await
    }

    /// Ku-group streamers (`listen.together.discovery.kugroup_streamers`).
    pub async fn discovery_kugroup_streamers(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.discovery.kugroup_streamers", params)
            .await
    }

    /// Genting streamers (`listen.together.discovery.genting_streamers`).
    pub async fn discovery_genting_streamers(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.discovery.genting_streamers", params)
            .await
    }

    /// Recent rooms (`listen.together.discovery.recent_rooms`).
    pub async fn discovery_recent_rooms(&self) -> Result<Value> {
        self.client
            .call_value(
                "listen.together.discovery.recent_rooms",
                NoParams::default(),
            )
            .await
    }

    /// Discovery privilege (`listen.together.discovery.privilege`).
    pub async fn discovery_privilege(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.discovery.privilege", params)
            .await
    }

    /// Genting recommend (`listen.together.discovery.genting_recommend`).
    pub async fn discovery_genting_recommend(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.discovery.genting_recommend", params)
            .await
    }

    /// Music-room history (`listen.together.music.history`).
    pub async fn music_history(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.history", params)
            .await
    }

    /// Music-room list (`listen.together.music.list`).
    pub async fn music_list(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.list", params)
            .await
    }

    /// Music-room detail (`listen.together.music.detail`).
    pub async fn music_detail(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.detail", params)
            .await
    }

    /// Music-room members (`listen.together.music.members`).
    pub async fn music_members(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.members", params)
            .await
    }

    /// Initialize a music room (`listen.together.music.initialize`).
    pub async fn music_initialize(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.initialize", params)
            .await
    }

    /// Sync the room player (`listen.together.music.sync_player`).
    pub async fn music_sync_player(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.sync_player", params)
            .await
    }

    /// Room playback URL (`listen.together.music.playback_url`).
    pub async fn music_playback_url(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.playback_url", params)
            .await
    }

    /// Switch the room song (`listen.together.music.switch_song`).
    pub async fn music_switch_song(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.switch_song", params)
            .await
    }

    /// Player operation (`listen.together.music.player_operation`).
    pub async fn music_player_operation(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.player_operation", params)
            .await
    }

    /// Room playlist (`listen.together.music.playlist`).
    pub async fn music_playlist(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.playlist", params)
            .await
    }

    /// Recent room playlist (`listen.together.music.recent_playlist`).
    pub async fn music_recent_playlist(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.recent_playlist", params)
            .await
    }

    /// Order a song (`listen.together.music.order_song`).
    pub async fn music_order_song(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.order_song", params)
            .await
    }

    /// Song order list (`listen.together.music.song_order_list`).
    pub async fn music_song_order_list(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.song_order_list", params)
            .await
    }

    /// Remove a song (`listen.together.music.remove_song`).
    pub async fn music_remove_song(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.remove_song", params)
            .await
    }

    /// Add songs to the room (`listen.together.music.music_add`).
    pub async fn music_add(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.music.music_add", params)
            .await
    }

    /// Create a room (`listen.together.room.create`).
    pub async fn room_create(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.create", params)
            .await
    }

    /// Join a room (`listen.together.room.join`).
    pub async fn room_join(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.join", params)
            .await
    }

    /// Room state (`listen.together.room.state`).
    pub async fn room_state(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.state", params)
            .await
    }

    /// Room heartbeat (`listen.together.room.heartbeat`).
    pub async fn room_heartbeat(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.heartbeat", params)
            .await
    }

    /// User room status (`listen.together.room.status`).
    pub async fn room_status(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.status", params)
            .await
    }

    /// Leave a room (`listen.together.room.leave`).
    pub async fn room_leave(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.leave", params)
            .await
    }

    /// Dismiss a room (`listen.together.room.dismiss`).
    pub async fn room_dismiss(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.dismiss", params)
            .await
    }

    /// Toggle room chat (`listen.together.room.update_chat`).
    pub async fn room_update_chat(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.room.update_chat", params)
            .await
    }

    /// Minor risk check (`listen.together.room.check_minor`).
    pub async fn room_check_minor(&self) -> Result<Value> {
        self.client
            .call_value("listen.together.room.check_minor", NoParams::default())
            .await
    }

    /// Created study rooms (`listen.together.study.created_rooms`).
    pub async fn study_created_rooms(&self) -> Result<Value> {
        self.client
            .call_value("listen.together.study.created_rooms", NoParams::default())
            .await
    }

    /// Delete a created room (`listen.together.study.delete_created_room`).
    pub async fn study_delete_created_room(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.study.delete_created_room", params)
            .await
    }

    /// Study room list (`listen.together.study.list`).
    pub async fn study_list(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.study.list", params)
            .await
    }

    /// Study room detail (`listen.together.study.detail`).
    pub async fn study_detail(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.study.detail", params)
            .await
    }

    /// Study room members (`listen.together.study.members`).
    pub async fn study_members(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.study.members", params)
            .await
    }

    /// Configure a study room (`listen.together.study.configure`).
    pub async fn study_configure(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.study.configure", params)
            .await
    }

    /// Sync the study player (`listen.together.study.sync_player`).
    pub async fn study_sync_player(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.study.sync_player", params)
            .await
    }

    /// Study playlist (`listen.together.study.playlist`).
    pub async fn study_playlist(&self, params: ListenTogetherParams) -> Result<Value> {
        self.client
            .call_value("listen.together.study.playlist", params)
            .await
    }
}
