//! Session credentials.
//!
//! Written automatically after successful device registration / login, carried
//! on every subsequent request, and exportable / restorable across processes.
//! The JSON shape is field-compatible with the TypeScript library's
//! `SessionSnapshot`, so snapshots can be exchanged between the two.

use serde::{Deserialize, Serialize};

/// Registration / login session snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Device fingerprint id.
    pub dfid: String,
    /// Device mid.
    pub mid: String,
    /// Device UUID.
    pub uuid: String,
    /// Login token.
    pub token: String,
    /// User id.
    pub userid: u64,
    /// VIP type (optional, after login).
    #[serde(rename = "vipType", skip_serializing_if = "Option::is_none", default)]
    pub vip_type: Option<i64>,
    /// VIP token (optional, after login).
    #[serde(rename = "vipToken", skip_serializing_if = "Option::is_none", default)]
    pub vip_token: Option<String>,
    /// Registration time (UNIX seconds), local bookkeeping only.
    #[serde(
        rename = "registeredAt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub registered_at: Option<u64>,
}

impl SessionSnapshot {
    /// An "anonymous" snapshot: `dfid = "-"`, `mid = mdid("-")`.
    #[must_use]
    pub fn anonymous() -> Self {
        Self {
            dfid: "-".to_owned(),
            mid: crate::device::derive_mid("-"),
            uuid: "-".to_owned(),
            token: String::new(),
            userid: 0,
            vip_type: None,
            vip_token: None,
            registered_at: None,
        }
    }
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self::anonymous()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anonymous_snapshot() {
        let s = SessionSnapshot::anonymous();
        assert_eq!(s.dfid, "-");
        assert_eq!(s.mid, "336d5ebc5436534e61d16e63ddfca327");
        assert_eq!(s.userid, 0);
    }

    #[test]
    fn json_interop_with_ts_shape() {
        let s = SessionSnapshot {
            dfid: "D".into(),
            mid: "M".into(),
            uuid: "U".into(),
            token: "T".into(),
            userid: 7,
            vip_type: Some(3),
            vip_token: Some("VT".into()),
            registered_at: Some(1_700_000_000),
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["vipType"], 3);
        assert_eq!(json["vipToken"], "VT");
        assert_eq!(json["registeredAt"], 1_700_000_000);

        let back: SessionSnapshot = serde_json::from_value(json).unwrap();
        assert_eq!(back, s);
    }
}
