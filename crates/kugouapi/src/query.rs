//! Ordered query parameter record.
//!
//! Mirrors the TS `QueryRecord` (`Record<string, QueryValue>`):
//!
//! - insertion order is preserved (it determines URL parameter order),
//! - keys are unique (later assignment replaces the value in place),
//! - `null`-ish entries are stripped before signing/sending,
//! - objects and arrays serialize to compact JSON text, both in the URL and in
//!   the signature canonicalisation (matching `JSON.stringify` / `String(v)`).

use serde_json::Value;

/// A query parameter value.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryValue {
    /// Plain string value.
    Str(String),
    /// Integer value.
    Int(i64),
    /// Unsigned integer value.
    UInt(u64),
    /// Floating point value.
    Float(f64),
    /// Boolean value.
    Bool(bool),
    /// Object / array value — serialized to compact JSON on the wire.
    Json(Value),
}

impl QueryValue {
    /// Wire representation of this value:
    /// `String(v)` for scalars, `JSON.stringify(v)` for objects/arrays.
    #[must_use]
    pub fn to_wire_string(&self) -> String {
        match self {
            Self::Str(s) => s.clone(),
            Self::Int(i) => i.to_string(),
            Self::UInt(u) => u.to_string(),
            Self::Float(f) => format_js_float(*f),
            Self::Bool(b) => b.to_string(),
            Self::Json(v) => serde_json::to_string(v).unwrap_or_default(),
        }
    }
}

/// Format a float the way JavaScript `String(number)` does for the values the
/// protocol actually carries (integral values lose the trailing `.0`).
fn format_js_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

impl From<&str> for QueryValue {
    fn from(v: &str) -> Self {
        Self::Str(v.to_owned())
    }
}

impl From<String> for QueryValue {
    fn from(v: String) -> Self {
        Self::Str(v)
    }
}

impl From<i64> for QueryValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<u64> for QueryValue {
    fn from(v: u64) -> Self {
        Self::UInt(v)
    }
}

impl From<u32> for QueryValue {
    fn from(v: u32) -> Self {
        Self::UInt(u64::from(v))
    }
}

impl From<i32> for QueryValue {
    fn from(v: i32) -> Self {
        Self::Int(i64::from(v))
    }
}

impl From<usize> for QueryValue {
    fn from(v: usize) -> Self {
        Self::UInt(v as u64)
    }
}

impl From<bool> for QueryValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<f64> for QueryValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<Value> for QueryValue {
    fn from(v: Value) -> Self {
        match v {
            Value::String(s) => Self::Str(s),
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Self::UInt(u)
                } else if let Some(i) = n.as_i64() {
                    Self::Int(i)
                } else {
                    Self::Float(n.as_f64().unwrap_or_default())
                }
            }
            Value::Bool(b) => Self::Bool(b),
            other => Self::Json(other),
        }
    }
}

/// An insertion-ordered map of query parameters with unique keys.
///
/// This is the Rust counterpart of the TS `QueryRecord` object literal:
/// spreading (`{ ...base, ...endpoint }`) maps to [`QueryRecord::merge`],
/// which replaces existing keys in place while keeping their original slot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueryRecord {
    entries: Vec<(String, QueryValue)>,
}

impl QueryRecord {
    /// Create an empty record.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a record with the given capacity hint.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            entries: Vec::with_capacity(cap),
        }
    }

    /// Insert (or replace in place) a key.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<QueryValue>) -> &mut Self {
        let key = key.into();
        let value = value.into();
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.entries.push((key, value));
        }
        self
    }

    /// Insert only when `value` is `Some` (TS `...(cond ? { k: v } : {})`).
    pub fn set_opt(
        &mut self,
        key: impl Into<String>,
        value: Option<impl Into<QueryValue>>,
    ) -> &mut Self {
        if let Some(v) = value {
            self.set(key, v);
        }
        self
    }

    /// Insert a JSON object / array value directly.
    pub fn set_json(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        self.set(key, QueryValue::Json(value));
        self
    }

    /// Look up a key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&QueryValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Look up a key's wire string.
    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<String> {
        self.get(key).map(QueryValue::to_wire_string)
    }

    /// Remove a key.
    pub fn remove(&mut self, key: &str) -> Option<QueryValue> {
        let idx = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(idx).1)
    }

    /// Whether a key exists.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Merge another record into this one (`{ ...self, ...other }`):
    /// existing keys keep their position but take the incoming value.
    pub fn merge(&mut self, other: &QueryRecord) -> &mut Self {
        for (k, v) in &other.entries {
            self.set(k, v.clone());
        }
        self
    }

    /// Iterate entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &QueryValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the record is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Build a `serde_json::Map` view of the record (values widened).
    #[must_use]
    pub fn to_json_map(&self) -> serde_json::Map<String, Value> {
        let mut map = serde_json::Map::new();
        for (k, v) in &self.entries {
            let value = match v {
                QueryValue::Str(s) => Value::String(s.clone()),
                QueryValue::Int(i) => Value::from(*i),
                QueryValue::UInt(u) => Value::from(*u),
                QueryValue::Float(f) => match serde_json::Number::from_f64(*f) {
                    Some(n) => Value::Number(n),
                    // JSON.stringify(NaN) === null
                    None => Value::Null,
                },
                QueryValue::Bool(b) => Value::from(*b),
                QueryValue::Json(v) => v.clone(),
            };
            map.insert(k.clone(), value);
        }
        map
    }

    /// Builder-style `set`.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<QueryValue>) -> Self {
        self.set(key, value);
        self
    }
}

impl<'a> IntoIterator for &'a QueryRecord {
    type Item = (&'a str, &'a QueryValue);
    type IntoIter = Box<dyn Iterator<Item = (&'a str, &'a QueryValue)> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_replaces_in_place() {
        let mut q = QueryRecord::new();
        q.set("a", 1).set("b", 2).set("a", 3);
        assert_eq!(q.len(), 2);
        let keys: Vec<&str> = q.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["a", "b"]);
        assert_eq!(q.get_str("a"), Some("3".to_owned()));
    }

    #[test]
    fn merge_replaces_keeps_position() {
        let mut base = QueryRecord::new();
        base.set("appid", 1005).set("dfid", "-");
        let mut over = QueryRecord::new();
        over.set("dfid", "NEW");
        over.set("keyword", "kw");
        base.merge(&over);
        let keys: Vec<&str> = base.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["appid", "dfid", "keyword"]);
        assert_eq!(base.get_str("dfid"), Some("NEW".to_owned()));
    }

    #[test]
    fn wire_strings() {
        assert_eq!(QueryValue::UInt(30).to_wire_string(), "30");
        assert_eq!(QueryValue::Bool(true).to_wire_string(), "true");
        assert_eq!(QueryValue::Float(1.0).to_wire_string(), "1");
        assert_eq!(QueryValue::Float(1.5).to_wire_string(), "1.5");
        assert_eq!(QueryValue::Str("周杰伦".into()).to_wire_string(), "周杰伦");
        let json = serde_json::json!(["a", "b"]);
        assert_eq!(QueryValue::Json(json).to_wire_string(), "[\"a\",\"b\"]");
    }

    #[test]
    fn set_opt_skips_none() {
        let mut q = QueryRecord::new();
        q.set_opt("x", None::<u64>).set_opt("y", Some(5u64));
        assert!(!q.contains_key("x"));
        assert_eq!(q.get_str("y"), Some("5".to_owned()));
    }
}
