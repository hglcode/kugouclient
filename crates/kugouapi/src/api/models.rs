//! Typed response models for high-frequency endpoints.
//!
//! Field names match the `KuGou` payloads exactly; unlisted fields are preserved
//! through `#[serde(flatten)]` extra maps, mirroring the TS `[key: string]:
//! unknown` index signatures.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A song entry inside [`SearchData::lists`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchSong {
    /// Display name, e.g. `"周杰伦 - 晴天"`.
    #[serde(default, rename = "FileName")]
    pub file_name: String,
    /// File hash.
    #[serde(default, rename = "FileHash")]
    pub file_hash: String,
    /// HQ hash.
    #[serde(
        default,
        rename = "HQFileHash",
        skip_serializing_if = "Option::is_none"
    )]
    pub hq_file_hash: Option<String>,
    /// SQ (lossless) hash.
    #[serde(
        default,
        rename = "SQFileHash",
        skip_serializing_if = "Option::is_none"
    )]
    pub sq_file_hash: Option<String>,
    /// Album id.
    #[serde(default, rename = "AlbumID", skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
    /// Duration in seconds.
    #[serde(default, rename = "Duration", skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    /// Singer ids.
    #[serde(default, rename = "SingerId", skip_serializing_if = "Option::is_none")]
    pub singer_id: Option<Vec<u64>>,
    /// Remaining payload fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Response of `search.song`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchData {
    /// Song entries.
    #[serde(default)]
    pub lists: Vec<SearchSong>,
    /// Total match count.
    #[serde(default)]
    pub total: u64,
    /// Remaining payload fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// VIP trial-segment information (usually `end_ms = 60000`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HashOffset {
    /// Start byte of the segment.
    #[serde(
        default,
        rename = "start_byte",
        skip_serializing_if = "Option::is_none"
    )]
    pub start_byte: Option<u64>,
    /// End byte of the segment.
    #[serde(default, rename = "end_byte", skip_serializing_if = "Option::is_none")]
    pub end_byte: Option<u64>,
    /// Start time in milliseconds.
    #[serde(default, rename = "start_ms", skip_serializing_if = "Option::is_none")]
    pub start_ms: Option<u64>,
    /// End time in milliseconds.
    #[serde(default, rename = "end_ms", skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<u64>,
    /// Segment file type.
    #[serde(default, rename = "file_type", skip_serializing_if = "Option::is_none")]
    pub file_type: Option<i64>,
    /// Hash of the segment.
    #[serde(
        default,
        rename = "offset_hash",
        skip_serializing_if = "Option::is_none"
    )]
    pub offset_hash: Option<String>,
    /// Remaining payload fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Response of `song.url`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SongUrlData {
    /// Playback URLs (free songs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<Vec<String>>,
    /// Backup URLs.
    #[serde(
        default,
        rename = "backup_url",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_url: Option<Vec<String>>,
    /// Duration in milliseconds.
    #[serde(
        default,
        rename = "timelength",
        skip_serializing_if = "Option::is_none"
    )]
    pub timelength: Option<u64>,
    /// File size in bytes.
    #[serde(default, rename = "file_size", skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    /// Actual quality served.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    /// VIP trial-segment info.
    #[serde(
        default,
        rename = "hash_offset",
        skip_serializing_if = "Option::is_none"
    )]
    pub hash_offset: Option<HashOffset>,
    /// Entitlement status (0 = requires copyright / paid judgement).
    #[serde(
        default,
        rename = "priv_status",
        skip_serializing_if = "Option::is_none"
    )]
    pub priv_status: Option<i64>,
    /// Reasons the song cannot play: `pkg` / `buy` / `copyright` …
    #[serde(
        default,
        rename = "fail_process",
        skip_serializing_if = "Option::is_none"
    )]
    pub fail_process: Option<Vec<String>>,
    /// Remaining payload fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Response of `register.dev`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegisterData {
    /// Device fingerprint id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dfid: Option<String>,
    /// Device mid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid: Option<String>,
    /// Login token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// User id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userid: Option<u64>,
    /// Remaining payload fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn search_data_decodes_with_extras() {
        let data: SearchData = serde_json::from_value(json!({
            "lists": [{
                "FileName": "周杰伦 - 晴天",
                "FileHash": "HASH",
                "HQFileHash": "HQ",
                "AlbumID": "123",
                "Duration": 269,
                "SingerId": [7220, 3521],
                "OriMData": "whatever"
            }],
            "total": 480,
            "wikidata": "x"
        }))
        .unwrap();
        assert_eq!(data.total, 480);
        assert_eq!(data.lists.len(), 1);
        let song = &data.lists[0];
        assert_eq!(song.file_name, "周杰伦 - 晴天");
        assert_eq!(song.file_hash, "HASH");
        assert_eq!(song.duration, Some(269));
        assert_eq!(song.singer_id.as_deref(), Some(&[7220, 3521][..]));
        assert_eq!(song.extra["OriMData"], "whatever");
        assert_eq!(data.extra["wikidata"], "x");
    }

    #[test]
    fn song_url_data_decodes() {
        let data: SongUrlData = serde_json::from_value(json!({
            "url": ["http://a/x.mp3"],
            "backup_url": ["http://b/x.mp3"],
            "timelength": 269000,
            "file_size": 4300000,
            "hash_offset": { "start_ms": 0, "end_ms": 60000, "offset_hash": "OH" },
            "priv_status": 0,
            "fail_process": ["pkg"]
        }))
        .unwrap();
        assert_eq!(data.url, Some(vec!["http://a/x.mp3".to_owned()]));
        assert_eq!(data.hash_offset.as_ref().unwrap().end_ms, Some(60000));
        assert_eq!(data.fail_process.as_deref(), Some(&["pkg".to_owned()][..]));
    }

    #[test]
    fn register_data_decodes() {
        let data: RegisterData = serde_json::from_value(
            json!({"dfid": "D", "mid": "M", "userid": 9, "token": "T", "extra1": 1}),
        )
        .unwrap();
        assert_eq!(data.dfid.as_deref(), Some("D"));
        assert_eq!(data.userid, Some(9));
        assert_eq!(data.extra["extra1"], 1);
    }
}
