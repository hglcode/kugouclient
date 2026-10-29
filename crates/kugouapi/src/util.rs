//! Shared helpers for the catalog and special handlers.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

/// `_comment` family `firstValue`: the first value that is neither null nor an
/// empty string (stringified).
#[must_use]
pub fn first_value(values: &[Value]) -> Option<Value> {
    values.iter().find_map(|v| match v {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        other => Some(other.clone()),
    })
}

/// `compact`: drop `undefined` / `null` / empty-string fields.
#[must_use]
pub fn compact(object: &serde_json::Map<String, Value>) -> serde_json::Map<String, Value> {
    object
        .iter()
        .filter(|(k, v)| {
            let _ = k;
            !matches!(v, Value::Null) && !matches!(v, Value::String(s) if s.is_empty())
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Parse `"a,b,c"` / JSON array values into a `Vec<String>`
/// (simplified port of the reference `splitList`).
#[must_use]
pub fn split_list(value: &Value) -> Vec<String> {
    match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items
            .iter()
            .map(stringify_trimmed)
            .filter(|s| !s.is_empty())
            .collect(),
        Value::String(text) => {
            let text = text.trim();
            if text.is_empty() {
                return Vec::new();
            }
            if text.starts_with('[')
                && let Ok(Value::Array(items)) = serde_json::from_str::<Value>(text)
            {
                return items
                    .iter()
                    .map(stringify_trimmed)
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            // fall back to comma splitting
            text.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(std::borrow::ToOwned::to_owned)
                .collect()
        }
        other => {
            let s = stringify_trimmed(other);
            if s.is_empty() { Vec::new() } else { vec![s] }
        }
    }
}

fn stringify_trimmed(value: &Value) -> String {
    match value {
        Value::String(s) => s.trim().to_owned(),
        Value::Null => String::new(),
        other => other.to_string().trim().to_owned(),
    }
}

/// base64 / dataURL → bytes (`None` when invalid). Port of `normalizeBase64`:
/// the payload must be non-empty, multiple-of-4 length and consist only of
/// base64 alphabet with at most two trailing `=`.
#[must_use]
pub fn normalize_base64(input: &str) -> Option<Vec<u8>> {
    let base64 = if let Some(rest) = input.strip_prefix("data:") {
        let idx = rest.find(',')?;
        &rest[idx + 1..]
    } else {
        input
    };
    let base64: String = base64.chars().filter(|c| !c.is_whitespace()).collect();

    if base64.is_empty() || !base64.len().is_multiple_of(4) || !is_valid_base64(&base64) {
        return None;
    }
    BASE64.decode(base64).ok()
}

fn is_valid_base64(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1] == b'=' {
        end -= 1;
    }
    // at most two padding chars
    if bytes.len() - end > 2 {
        return false;
    }
    bytes[..end]
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'/' || b == b'+')
}

/// Image magic-number detection (JPEG / PNG / GIF / BMP / WEBP).
#[must_use]
pub fn is_image_buffer(buf: &[u8]) -> bool {
    if buf.len() < 12 {
        return false;
    }
    let is_jpeg = buf[0] == 0xff && buf[1] == 0xd8;
    let is_png = buf[0] == 0x89 && buf[1] == 0x50 && buf[2] == 0x4e && buf[3] == 0x47;
    let is_gif = buf[0] == 0x47 && buf[1] == 0x49 && buf[2] == 0x46;
    let is_bmp = buf[0] == 0x42 && buf[1] == 0x4d;
    let is_webp = &buf[0..4] == b"RIFF" && &buf[8..12] == b"WEBP";
    is_jpeg || is_png || is_gif || is_bmp || is_webp
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn first_value_semantics() {
        let none = first_value(&[json!(null), json!("")]);
        assert_eq!(none, None);
        let some = first_value(&[json!(null), json!(""), json!("v"), json!("w")]);
        assert_eq!(some, Some(json!("v")));
    }

    #[test]
    fn compact_drops_empty() {
        let mut map = serde_json::Map::new();
        map.insert("a".into(), json!(1));
        map.insert("b".into(), json!(null));
        map.insert("c".into(), json!(""));
        map.insert("d".into(), json!("x"));
        let compacted = compact(&map);
        assert_eq!(compacted.len(), 2);
        assert!(compacted.contains_key("a"));
        assert!(compacted.contains_key("d"));
    }

    #[test]
    fn split_list_forms() {
        assert_eq!(split_list(&json!("a,b, c")), vec!["a", "b", "c"]);
        assert_eq!(split_list(&json!(["x", " y "])), vec!["x", "y"]);
        assert_eq!(split_list(&json!("[\"p\",\"q\"]")), vec!["p", "q"]);
        assert_eq!(split_list(&json!(null)), [] as [std::string::String; 0]);
        assert_eq!(split_list(&json!("")), [] as [std::string::String; 0]);
        assert_eq!(split_list(&json!("   ")), [] as [std::string::String; 0]);
    }

    #[test]
    fn normalize_base64_forms() {
        assert!(normalize_base64("!!!").is_none());
        assert!(normalize_base64("").is_none());
        assert!(normalize_base64("abc").is_none()); // len % 4 != 0
        assert!(normalize_base64("ab=c").is_none()); // invalid char position
        assert!(normalize_base64("aGVsbG8=").is_some());
        assert_eq!(
            normalize_base64("data:image/png;base64,aGVsbG8=").as_deref(),
            Some(b"hello".as_slice())
        );
        assert!(normalize_base64("aGVs bG8=").is_some()); // whitespace stripped
        assert!(normalize_base64("aGVsbG8==").is_none()); // invalid padding count for length? 'hello' → 'aGVsbG8=' len 8; 'aGVsbG8==' len 9 not %4
    }

    #[test]
    fn image_detection() {
        assert!(is_image_buffer(&[
            0xff, 0xd8, 0xff, 0xe0, 0, 0, 0, 0, 0, 0, 0, 0
        ]));
        assert!(is_image_buffer(&[
            0x89, 0x50, 0x4e, 0x47, 0, 0, 0, 0, 0, 0, 0, 0
        ]));
        let mut webp = *b"RIFF0000WEBP";
        webp[4..8].copy_from_slice(&[0, 0, 0, 0]);
        assert!(is_image_buffer(&webp));
        assert!(!is_image_buffer(&[0u8; 12]));
        assert!(!is_image_buffer(&[1, 2, 3]));
    }
}
