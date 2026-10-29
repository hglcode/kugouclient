//! Protocol constants and platform presets.
//!
//! Every "magic number" lives here. Values come from public `KuGou` protocol
//! reverse-engineering material and Android client captures, and can be
//! overridden per client via [`crate::client::KuGouClientBuilder`].

/// Client platform identifier.
///
/// The `Lite` preset (the "concept" / Young edition) switches `appid`,
/// `clientver`, signature salts, RSA keys and several fixed parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Platform {
    /// Standard Android client (appid 1005).
    #[default]
    Standard,
    /// Concept / Young edition (appid 3116).
    Lite,
}

impl Platform {
    /// Platform preset: `(appid, clientver)`.
    #[must_use]
    pub fn config(self) -> PlatformConfig {
        match self {
            Self::Standard => PlatformConfig {
                appid: 1005,
                clientver: 20489,
            },
            Self::Lite => PlatformConfig {
                appid: 3116,
                clientver: 11440,
            },
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Standard => "standard",
            Self::Lite => "lite",
        })
    }
}

/// Platform base configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformConfig {
    /// Gateway `appid`.
    pub appid: u64,
    /// Gateway `clientver`.
    pub clientver: u64,
}

/// Default API gateway.
pub const DEFAULT_BASE_URL: &str = "https://gateway.kugou.com";

/// User service base URL (used by `register_dev`).
pub const REGISTER_BASE_URL: &str = "https://userservice.kugou.com";

/// Third-party login `srcappid` (login family endpoints).
pub const SRCAPPID: u64 = 2919;

/// `WeChat` open-platform parameters (`login.wx.*`).
pub const WX_APPIDS: [(Platform, WxAppId); 2] = [
    (
        Platform::Standard,
        WxAppId {
            appid: "wx79f2c4418704b4f8",
            secret: "4efcab88b700769e376e3f6087b8abc9",
        },
    ),
    (
        Platform::Lite,
        WxAppId {
            appid: "wx72b795aca60ad321",
            secret: "33e486041e5e25729a4e3d2da7502f9a",
        },
    ),
];

/// `WeChat` open-platform app credentials.
#[derive(Debug, Clone, Copy)]
pub struct WxAppId {
    /// `WeChat` app id.
    pub appid: &'static str,
    /// `WeChat` app secret.
    pub secret: &'static str,
}

/// QQ open-platform app ids (`login.qq.*`).
pub fn qq_appid(platform: Platform) -> &'static str {
    match platform {
        Platform::Standard => "205141",
        Platform::Lite => "101706348",
    }
}

/// `WeChat` open-platform parameters for a platform.
#[must_use]
pub fn wx_appid(platform: Platform) -> &'static WxAppId {
    WX_APPIDS
        .iter()
        .find(|(p, _)| *p == platform)
        .map(|(_, w)| w)
        .expect("every platform has a `WeChat` preset")
}

/// Fixed trackercdn (`/v5/url`) parameters per platform, from Android captures.
#[derive(Debug, Clone, Copy)]
pub struct TrackerParams {
    /// `page_id` query parameter.
    pub page_id: u64,
    /// `ppage_id` query parameter (comma separated).
    pub p_page_ids: &'static str,
    /// `pid` query parameter.
    pub pid: u64,
    /// `version` query parameter.
    pub version: u64,
    /// `clientver` query parameter.
    pub clientver: u64,
}

/// trackercdn parameters for a platform.
#[must_use]
pub fn tracker_params(platform: Platform) -> TrackerParams {
    match platform {
        Platform::Standard => TrackerParams {
            page_id: 151_369_488,
            p_page_ids: "463467626,350369493,788954147",
            pid: 2,
            version: 11430,
            clientver: 11430,
        },
        Platform::Lite => TrackerParams {
            page_id: 967_177_915,
            p_page_ids: "356753938,823673182,967485191",
            pid: 411,
            version: 11430,
            clientver: 11430,
        },
    }
}

/// Default `mid` used before registration (constant from the reference
/// implementation; superseded by the server-provided mid once registered).
pub const DEFAULT_MID: &str = "31460504985089440976998453790954693734";

/// Client identification headers (the server uses these to fingerprint the
/// request origin).
pub const CLIENT_HEADERS: [(&str, &str); 4] = [
    ("kg-rc", "1"),
    ("kg-thash", "5d816a0"),
    ("kg-rec", "1"),
    ("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F"),
];

/// User-Agent used by `register_dev`.
pub const DEFAULT_USER_AGENT: &str = "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi";

/// Login family `support-calm` User-Agent (`login.cellphone`).
pub const LOGIN_UA: &str = "Android16-1070-11440-130-0-LOGIN-wifi";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_configs() {
        assert_eq!(Platform::Standard.config().appid, 1005);
        assert_eq!(Platform::Standard.config().clientver, 20489);
        assert_eq!(Platform::Lite.config().appid, 3116);
        assert_eq!(Platform::Lite.config().clientver, 11440);
    }

    #[test]
    fn tracker_params_differ_per_platform() {
        assert_ne!(
            tracker_params(Platform::Standard).pid,
            tracker_params(Platform::Lite).pid
        );
    }

    #[test]
    fn wx_qq_presets() {
        assert_eq!(wx_appid(Platform::Lite).appid, "wx72b795aca60ad321");
        assert_eq!(qq_appid(Platform::Standard), "205141");
    }
}
