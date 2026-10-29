//! Typed API surface: domain builders, parameter structs and response models.
//!
//! Every catalog endpoint is reachable through three layers:
//!
//! 1. **typed methods** — `client.search().song(params).await` with
//!    strongly-typed parameter structs (compile-time field checking),
//! 2. **dynamic dispatch** — `client.call("search.song", json!({...}))`
//!    (parity with the TS `call(id, params)`),
//! 3. **generic escape hatch** — `client.request(path, opts)` for endpoints
//!    the catalog does not know.
//!
//! Responses are returned as [`serde_json::Value`] for the long tail (the `KuGou`
//! payloads are loose and version-drifting) and as typed models for the
//! high-frequency endpoints (`SearchData` / `SongUrlData` / `RegisterData`),
//! with unknown fields preserved through `#[serde(flatten)]`.

mod models;

pub use models::{RegisterData, SearchData, SearchSong, SongUrlData};

use serde_json::Value;

use crate::client::KuGouClient;
use crate::js::{FlexI64, FlexString};

/// Audio quality identifier.
///
/// `magic_`-prefixed special-effect channels (piano / acappella / …) are
/// handled internally by the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quality {
    /// Piano special-effect channel.
    Piano,
    /// A-cappella special-effect channel.
    Acappella,
    /// Subwoofer special-effect channel.
    Subwoofer,
    /// Ancient-style special-effect channel.
    Ancient,
    /// Surnay special-effect channel.
    Surnay,
    /// DJ special-effect channel.
    Dj,
    /// 128 kbps.
    Q128,
    /// 320 kbps.
    Q320,
    /// FLAC lossless.
    Flac,
    /// "High" quality.
    High,
    /// Viper Atmos.
    ViperAtmos,
    /// Viper Clear.
    ViperClear,
    /// Viper Tape.
    ViperTape,
    /// Super quality.
    Super,
}

impl Quality {
    /// Wire representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Piano => "piano",
            Self::Acappella => "acappella",
            Self::Subwoofer => "subwoofer",
            Self::Ancient => "ancient",
            Self::Surnay => "surnay",
            Self::Dj => "dj",
            Self::Q128 => "128",
            Self::Q320 => "320",
            Self::Flac => "flac",
            Self::High => "high",
            Self::ViperAtmos => "viper_atmos",
            Self::ViperClear => "viper_clear",
            Self::ViperTape => "viper_tape",
            Self::Super => "super",
        }
    }

    /// Normalized wire form (`magic_` prefix for special-effect channels).
    #[must_use]
    pub fn to_wire(self) -> String {
        const MAGIC: [Quality; 6] = [
            Quality::Piano,
            Quality::Acappella,
            Quality::Subwoofer,
            Quality::Ancient,
            Quality::Dj,
            Quality::Surnay,
        ];
        if MAGIC.contains(&self) {
            format!("magic_{}", self.as_str())
        } else {
            self.as_str().to_owned()
        }
    }
}

impl std::str::FromStr for Quality {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.trim().trim_start_matches("magic_");
        let quality = match normalized {
            "piano" => Self::Piano,
            "acappella" => Self::Acappella,
            "subwoofer" => Self::Subwoofer,
            "ancient" => Self::Ancient,
            "surnay" => Self::Surnay,
            "dj" => Self::Dj,
            "128" => Self::Q128,
            "320" => Self::Q320,
            "flac" => Self::Flac,
            "high" => Self::High,
            "viper_atmos" => Self::ViperAtmos,
            "viper_clear" => Self::ViperClear,
            "viper_tape" => Self::ViperTape,
            "super" => Self::Super,
            other => {
                return Err(crate::error::Error::InvalidParams {
                    endpoint: "quality".to_owned(),
                    message: format!("unknown quality `{other}`"),
                });
            }
        };
        Ok(quality)
    }
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for Quality {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for Quality {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(<D::Error as serde::de::Error>::custom)
    }
}

// ─────────────────────────────────────────────────────────────
// Core parameter structs
// ─────────────────────────────────────────────────────────────

kugou_params! {
    /// Parameters for `search.song` (typed [`crate::api::SearchApi::song`]).
    pub SearchSongParams {
        required {
            /// Search keyword.
            keyword: String => "keyword",
        }
        optional {
            /// Page number (default 1).
            page: FlexI64 => "page",
            /// Page size (default 30).
            page_size: FlexI64 => "pagesize",
            /// Search-error correction (default enabled).
            correction: bool => "iscorrection",
        }
    }
}

kugou_params! {
    /// Parameters for `song.url` (typed [`crate::api::SongApi::url`]).
    pub SongUrlParams {
        required {
            /// Music file hash.
            hash: String => "hash",
        }
        optional {
            /// Quality (default 128).
            quality: Quality => "quality",
            /// Album id (string or number).
            album_id: FlexString => "album_id",
            /// Album audio id.
            album_audio_id: FlexI64 => "album_audio_id",
            /// Return the trial segment (VIP songs).
            free_part: bool => "free_part",
        }
    }
}

/// Parameters for `register.dev`: dynamic device-field overrides plus an
/// optional user binding.
///
/// The overrides map accepts any `Device` key (`imei`, `uuid`,
/// `batteryLevel`, …); empty values are ignored exactly like the TS
/// `stripEmpty` semantics.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RegisterDeviceParams {
    /// Optional user id to bind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userid: Option<u64>,
    /// Optional login token to bind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Device field overrides (any `Device` key).
    #[serde(flatten)]
    pub overrides: serde_json::Map<String, Value>,
}

impl RegisterDeviceParams {
    /// Empty parameters (register with the client's configured device).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Override a device field.
    #[must_use]
    pub fn device_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.overrides.insert(key.into(), value.into());
        self
    }
}

// ─────────────────────────────────────────────────────────────
// Domain builders
// ─────────────────────────────────────────────────────────────

macro_rules! api_domain {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'a> {
            pub(crate) client: &'a KuGouClient,
        }

        impl<'a> $name<'a> {
            #[must_use]
            pub(crate) fn new(client: &'a KuGouClient) -> Self {
                Self { client }
            }
        }
    };
}

api_domain! {
    /// Search endpoints (`search.*`).
    SearchApi
}
api_domain! {
    /// Song endpoints (`song.*`, `lyric`, privileges).
    SongApi
}
api_domain! {
    /// Account / login / register family (`login.*`, `register.dev`, verify).
    AccountApi
}
api_domain! {
    /// User endpoints (`user.*`, blacklist).
    UserApi
}
api_domain! {
    /// Playlist endpoints (`playlist.*`).
    PlaylistApi
}
api_domain! {
    /// Artist endpoints (`artist.*`).
    ArtistApi
}
api_domain! {
    /// Album endpoints (`album.*`).
    AlbumApi
}
api_domain! {
    /// Comment endpoints (`comment.*`, barrages).
    CommentApi
}
api_domain! {
    /// Media endpoints (audio / video / mv / fm / images).
    MediaApi
}
api_domain! {
    /// Discovery endpoints (rank / top / everyday / home / ip / zone / style).
    DiscoverApi
}
api_domain! {
    /// Radio endpoints (`radio.*`).
    RadioApi
}
api_domain! {
    /// Scene endpoints (youth / scenario …).
    SceneApi
}
api_domain! {
    /// Vertical endpoints (sing / krc / effects / accompanies …).
    VerticalsApi
}
api_domain! {
    /// Cloud endpoints (`user.cloud.*`, `mv.collect.*`).
    CloudApi
}
api_domain! {
    /// Listen-together endpoints (`listen.together.*`).
    ListenTogetherApi
}
api_domain! {
    /// Team endpoints (`team.*`).
    TeamApi
}

impl KuGouClient {
    /// Search endpoints.
    #[must_use]
    pub fn search(&self) -> SearchApi<'_> {
        SearchApi::new(self)
    }

    /// Song endpoints (urls, lyrics, privileges).
    #[must_use]
    pub fn song(&self) -> SongApi<'_> {
        SongApi::new(self)
    }

    /// Account / login / register family.
    #[must_use]
    pub fn account(&self) -> AccountApi<'_> {
        AccountApi::new(self)
    }

    /// User endpoints.
    #[must_use]
    pub fn user(&self) -> UserApi<'_> {
        UserApi::new(self)
    }

    /// Playlist endpoints.
    #[must_use]
    pub fn playlist(&self) -> PlaylistApi<'_> {
        PlaylistApi::new(self)
    }

    /// Artist endpoints.
    #[must_use]
    pub fn artist(&self) -> ArtistApi<'_> {
        ArtistApi::new(self)
    }

    /// Album endpoints.
    #[must_use]
    pub fn album(&self) -> AlbumApi<'_> {
        AlbumApi::new(self)
    }

    /// Comment endpoints.
    #[must_use]
    pub fn comment(&self) -> CommentApi<'_> {
        CommentApi::new(self)
    }

    /// Media endpoints (audio / video / mv / fm / images).
    #[must_use]
    pub fn media(&self) -> MediaApi<'_> {
        MediaApi::new(self)
    }

    /// Discovery endpoints (rank / top / everyday / home / ip / zone …).
    #[must_use]
    pub fn discover(&self) -> DiscoverApi<'_> {
        DiscoverApi::new(self)
    }

    /// Radio endpoints.
    #[must_use]
    pub fn radio(&self) -> RadioApi<'_> {
        RadioApi::new(self)
    }

    /// Scene endpoints.
    #[must_use]
    pub fn scene(&self) -> SceneApi<'_> {
        SceneApi::new(self)
    }

    /// Vertical endpoints (sing / krc / effects …).
    #[must_use]
    pub fn verticals(&self) -> VerticalsApi<'_> {
        VerticalsApi::new(self)
    }

    /// Cloud endpoints (`user.cloud.*`).
    #[must_use]
    pub fn cloud(&self) -> CloudApi<'_> {
        CloudApi::new(self)
    }

    /// Listen-together endpoints.
    #[must_use]
    pub fn listen_together(&self) -> ListenTogetherApi<'_> {
        ListenTogetherApi::new(self)
    }

    /// Team endpoints.
    #[must_use]
    pub fn team(&self) -> TeamApi<'_> {
        TeamApi::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_wire_forms() {
        assert_eq!(Quality::Dj.to_wire(), "magic_dj");
        assert_eq!(Quality::Flac.to_wire(), "flac");
        assert_eq!(Quality::Q128.to_wire(), "128");
        assert_eq!(Quality::ViperAtmos.to_wire(), "viper_atmos");
        assert!("flac".parse::<Quality>().is_ok());
        assert_eq!("magic_dj".parse::<Quality>().unwrap(), Quality::Dj);
        assert!("nope".parse::<Quality>().is_err());
    }

    #[test]
    fn search_params_wire_roundtrip() {
        let params = SearchSongParams::new("周杰伦").page(2).page_size(10);
        let value = serde_json::to_value(&params).unwrap();
        assert_eq!(value["keyword"], "周杰伦");
        assert_eq!(value["page"], 2);
        assert_eq!(value["pagesize"], 10);
        assert!(value.get("iscorrection").is_none());

        let back: SearchSongParams = serde_json::from_value(value).unwrap();
        assert_eq!(back, params);

        // dynamic json path accepts numeric strings like the JS client
        let loose: SearchSongParams =
            serde_json::from_value(serde_json::json!({"keyword": "k", "page": "3"})).unwrap();
        assert_eq!(loose.page.unwrap().value(), Some(3));
    }

    #[test]
    fn song_url_params_wire() {
        let params = SongUrlParams::new("ABC").quality(Quality::Dj);
        let value = serde_json::to_value(&params).unwrap();
        assert_eq!(value["hash"], "ABC");
        assert_eq!(value["quality"], "dj");
    }

    #[test]
    fn register_device_params_flatten() {
        let params = RegisterDeviceParams::new().device_field("imei", "i".repeat(32));
        let value = serde_json::to_value(&params).unwrap();
        assert_eq!(value["imei"], "i".repeat(32));
        let back: RegisterDeviceParams =
            serde_json::from_value(serde_json::json!({"uuid": "U", "userid": 3})).unwrap();
        assert_eq!(back.userid, Some(3));
        assert_eq!(back.overrides["uuid"], "U");
    }
}
