//! `KuGou` request signatures.
//!
//! Algorithm: `md5(salt + sorted "k=v" pairs + [body] + salt)`.
//! Salts come from public client reverse-engineering material and vary per
//! signature kind and platform.

use md5::{Digest, Md5};

use crate::config::Platform;
use crate::query::{QueryRecord, QueryValue};

/// Signature kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignKind {
    /// Android client signature (default). Salt varies per platform.
    #[default]
    Android,
    /// `register_dev` signature.
    Register,
    /// Web (h5) signature.
    Web,
}

/// Signature salts per kind; the android salt varies per platform.
pub fn sign_salt(kind: SignKind, platform: Platform) -> &'static str {
    match (kind, platform) {
        (SignKind::Android, Platform::Standard) => "OIlwieks28dk2k092lksi2UIkp",
        (SignKind::Android, Platform::Lite) => "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA",
        (SignKind::Register, _) => "1014",
        (SignKind::Web, _) => "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt",
    }
}

/// trackercdn (`/v5/url`) `key` parameter salts.
pub fn tracker_key_salt(platform: Platform) -> &'static str {
    match platform {
        Platform::Standard => "57ae12eb6890223e355ccfcb74edf70d",
        Platform::Lite => "185672dd44712f60bb1736df5a377e82",
    }
}

/// `signParamsKey` salts (same values as the android signature salt).
fn params_key_salt(platform: Platform) -> &'static str {
    sign_salt(SignKind::Android, platform)
}

/// Cloud playback (`user_cloud_url`) key salt.
pub const CLOUD_KEY_SALT: &str = "ebd1ac3134c880bda6a2194537843caa0162e2e7";

/// Lowercase hex MD5 of a UTF-8 string.
#[must_use]
pub fn md5(data: &str) -> String {
    let digest = Md5::digest(data.as_bytes());
    hex::encode(digest)
}

/// Options for [`signature`].
#[derive(Debug, Clone, Default)]
pub struct SignOptions<'a> {
    /// Signature kind, defaults to android.
    pub kind: SignKind,
    /// Platform for the android salt, defaults to standard.
    pub platform: Platform,
    /// Request body participating in the signature (only when non-empty).
    pub data: Option<&'a str>,
}

/// Generate a request signature.
///
/// - query pairs are sorted by key and concatenated as `k1=v1k2=v2...`
///   (raw values, no URL encoding),
/// - `data` participates only when non-empty,
/// - the salt wraps both ends.
#[must_use]
pub fn signature(query: &QueryRecord, options: SignOptions<'_>) -> String {
    let SignOptions {
        kind,
        platform,
        data,
    } = options;
    let salt = sign_salt(kind, platform);

    let mut hasher = Md5::new();
    hasher.update(salt.as_bytes());
    hasher.update(canonical_query(query).as_bytes());
    if let Some(data) = data.filter(|d| !d.is_empty()) {
        hasher.update(data.as_bytes());
    }
    hasher.update(salt.as_bytes());
    hex::encode(hasher.finalize())
}

/// Canonicalised query string for signing (objects become `JSON.stringify`).
fn canonical_query(query: &QueryRecord) -> String {
    let mut pairs: Vec<(&str, &QueryValue)> = query.iter().collect();
    pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut out = String::new();
    for (k, v) in pairs {
        out.push_str(k);
        out.push('=');
        out.push_str(&v.to_wire_string());
    }
    out
}

/// Input for [`tracker_key`].
#[derive(Debug, Clone, Copy)]
pub struct TrackerKeyInput<'a> {
    /// Music file hash (lowercase).
    pub hash: &'a str,
    /// Device MID (must match the `mid` sent in the request).
    pub mid: &'a str,
    /// User id.
    pub userid: u64,
    /// App id.
    pub appid: u64,
    /// Platform, defaults to standard.
    pub platform: Platform,
}

/// Generate the trackercdn `key` parameter:
/// `md5(hash + salt + appid + mid + userid)`.
///
/// Note: after the key joins the query, the `signature` must be recomputed over
/// the full parameter set including the key.
#[must_use]
pub fn tracker_key(input: TrackerKeyInput<'_>) -> String {
    let salt = tracker_key_salt(input.platform);
    md5(&format!(
        "{}{}{}{}{}",
        input.hash, salt, input.appid, input.mid, input.userid
    ))
}

/// Options for [`sign_params_key`].
#[derive(Debug, Clone, Default)]
pub struct ParamsKeyOptions {
    /// Platform, defaults to standard.
    pub platform: Platform,
    /// Explicit appid override.
    pub appid: Option<u64>,
    /// Explicit clientver override.
    pub clientver: Option<u64>,
}

/// `signParamsKey`: `md5(appid + salt + clientver + data)`.
///
/// Used as the `key` query parameter by many gateway endpoints (comments,
/// cloud, favourites …).
#[must_use]
pub fn sign_params_key(data: impl std::fmt::Display, options: ParamsKeyOptions) -> String {
    let platform = options.platform;
    let appid = options.appid.unwrap_or_else(|| platform.config().appid);
    let clientver = options
        .clientver
        .unwrap_or_else(|| platform.config().clientver);
    let salt = params_key_salt(platform);
    md5(&format!("{appid}{salt}{clientver}{data}"))
}

/// Cloud playback (`user_cloud_url`) key:
/// `md5("musicclound" + hash + pid + salt)`.
#[must_use]
pub fn cloud_key(hash: &str, pid: impl std::fmt::Display) -> String {
    md5(&format!("musicclound{hash}{pid}{CLOUD_KEY_SALT}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values were computed independently with GNU coreutils md5sum
    // (tests/vectors.md in the TS repo); they cross-check this implementation.

    fn q(pairs: &[(&str, crate::query::QueryValue)]) -> QueryRecord {
        let mut record = QueryRecord::new();
        for (k, v) in pairs {
            record.set(*k, v.clone());
        }
        record
    }

    #[test]
    fn md5_vectors() {
        assert_eq!(md5("-"), "336d5ebc5436534e61d16e63ddfca327");
        assert_eq!(md5(""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn register_salt_vector() {
        let query = q(&[("a", 1i64.into()), ("b", 2i64.into())]);
        assert_eq!(
            signature(&query, SignOptions::kind(SignKind::Register)),
            "47e07c526393f9ba10275796777b85d8"
        );
    }

    #[test]
    fn web_salt_utf8_vector() {
        let query = q(&[("keyword", "周杰伦".into()), ("page", 1i64.into())]);
        assert_eq!(
            signature(&query, SignOptions::kind(SignKind::Web)),
            "b55b6d374a2ec6840000b8247afd237d"
        );
    }

    #[test]
    fn android_vectors() {
        let query = q(&[("a", 1i64.into())]);
        assert_eq!(
            signature(&query, SignOptions::default()),
            "1e327cbf961f35919716aeabf27f8701"
        );
        assert_eq!(
            signature(
                &query,
                SignOptions {
                    platform: Platform::Lite,
                    ..SignOptions::default()
                }
            ),
            "4b319095ce4ff6cac9023c646fb8e0eb"
        );
    }

    #[test]
    fn data_participates_when_non_empty() {
        let query = q(&[("a", 1i64.into()), ("b", 2i64.into())]);
        assert_eq!(
            signature(
                &query,
                SignOptions {
                    kind: SignKind::Register,
                    data: Some("x"),
                    ..SignOptions::default()
                }
            ),
            "292a0589917dc08ddd6750880c691944"
        );
        // Empty data is equivalent to no data
        assert_eq!(
            signature(
                &query,
                SignOptions {
                    kind: SignKind::Register,
                    data: Some(""),
                    ..SignOptions::default()
                }
            ),
            signature(&query, SignOptions::kind(SignKind::Register))
        );
    }

    #[test]
    fn key_order_irrelevant() {
        let a = q(&[("a", 1i64.into()), ("b", 2i64.into())]);
        let b = q(&[("b", 2i64.into()), ("a", 1i64.into())]);
        assert_eq!(
            signature(&a, SignOptions::kind(SignKind::Register)),
            signature(&b, SignOptions::kind(SignKind::Register))
        );
    }

    #[test]
    fn tracker_key_vectors() {
        assert_eq!(
            tracker_key(TrackerKeyInput {
                hash: "abc",
                mid: "m",
                userid: 0,
                appid: 1005,
                platform: Platform::Standard,
            }),
            "3044a4b4a532665d2a0859411f8f35ed"
        );
        assert_eq!(
            tracker_key(TrackerKeyInput {
                hash: "abc",
                mid: "m",
                userid: 7,
                appid: 3116,
                platform: Platform::Lite,
            }),
            "e3fc84a69ee6bc675cf64c48b587784e"
        );
    }

    #[test]
    fn params_key_and_cloud_key() {
        let k = sign_params_key(
            1717171717171u64,
            ParamsKeyOptions {
                platform: Platform::Standard,
                ..Default::default()
            },
        );
        assert_eq!(k.len(), 32);
        assert!(
            k.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        let c = cloud_key("HASH", 2);
        assert_eq!(c.len(), 32);
    }

    impl SignOptions<'_> {
        fn kind(kind: SignKind) -> Self {
            Self {
                kind,
                ..Self::default()
            }
        }
    }
}
