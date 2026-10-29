//! JavaScript coercion helpers.
//!
//! The reference implementation reads endpoint parameters as loose JS values and
//! coerces them on the fly (`Number(p.x) || 0`, `p.x || ''`, `String(p.x)` …).
//! The Rust port keeps typed parameter structs, but reproduces the exact
//! *fallback semantics* of those coercions through the helpers in this module so
//! that wire requests stay bit-for-bit identical.

use serde_json::Value;

/// A JSON scalar that deserializes from a number **or** a numeric string,
/// mirroring JS `Number(x)` coercion used all over the reference modules.
///
/// ```
/// use kugouapi::js::FlexI64;
/// use serde_json::json;
///
/// let a: FlexI64 = serde_json::from_value(json!(42)).unwrap();
/// let b: FlexI64 = serde_json::from_value(json!("42")).unwrap();
/// assert_eq!(a, b);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlexI64(pub Option<i64>);

impl FlexI64 {
    /// The numeric value, or `None` when absent or non-numeric (JS `NaN`).
    pub fn value(&self) -> Option<i64> {
        self.0
    }

    /// `Number(x) || default` — zero falls back too (JS falsy).
    #[must_use]
    pub fn or(self, default: i64) -> i64 {
        match self.0 {
            Some(v) if v != 0 => v,
            _ => default,
        }
    }

    /// `x ?? default` — only null/undefined fall back; zero survives.
    #[must_use]
    pub fn unwrap_or(self, default: i64) -> i64 {
        self.0.unwrap_or(default)
    }
}

impl serde::Serialize for FlexI64 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            Some(v) => serializer.serialize_i64(v),
            None => serializer.serialize_unit(),
        }
    }
}

impl<'de> serde::Deserialize<'de> for FlexI64 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Self(flex_number(&value)))
    }
}

/// Parse a JSON value the way JS `Number()` would.
fn flex_number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(i)
            } else {
                n.as_f64().map(|f| {
                    if f.fract() == 0.0 && f.is_finite() {
                        f as i64
                    } else {
                        // Non-integral floats coerce like JS parseInt-style
                        // truncation in these protocol sites; keep truncation.
                        f as i64
                    }
                })
            }
        }
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Some(0) // Number('') === 0
            } else {
                trimmed.parse::<f64>().ok().map(|f| f as i64)
            }
        }
        Value::Bool(b) => Some(i64::from(*b)), // Number(true) === 1
        Value::Null => None,
        _ => Some(0), // Number({}) / Number([]) edge cases → NaN→0-ish; unused in practice
    }
}

impl From<i64> for FlexI64 {
    fn from(v: i64) -> Self {
        Self(Some(v))
    }
}

impl From<u64> for FlexI64 {
    fn from(v: u64) -> Self {
        Self(Some(v as i64))
    }
}

impl From<i32> for FlexI64 {
    fn from(v: i32) -> Self {
        Self(Some(i64::from(v)))
    }
}

impl From<u32> for FlexI64 {
    fn from(v: u32) -> Self {
        Self(Some(i64::from(v)))
    }
}

impl From<usize> for FlexI64 {
    fn from(v: usize) -> Self {
        Self(Some(v as i64))
    }
}

impl From<&str> for FlexI64 {
    fn from(v: &str) -> Self {
        Self(flex_number(&Value::String(v.to_owned())))
    }
}

/// A JSON scalar that deserializes from a string **or** a number, mirroring
/// `String(p.x)` / `p.x || ''` patterns.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlexString(pub Option<String>);

impl FlexString {
    /// The string value, or `None` when absent.
    pub fn value(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// `String(x) || default` — empty string falls back too (JS falsy).
    #[must_use]
    pub fn or<'a>(self, default: &'a str) -> String
    where
        'a: 'static,
    {
        match &self.0 {
            Some(s) if !s.is_empty() => s.clone(),
            _ => default.to_owned(),
        }
    }

    /// `x ?? default`.
    #[must_use]
    pub fn unwrap_or(self, default: &str) -> String {
        self.0.unwrap_or_else(|| default.to_owned())
    }

    /// Unwrap or yield an empty string.
    #[must_use]
    pub fn or_empty(self) -> String {
        self.0.unwrap_or_default()
    }
}

impl serde::Serialize for FlexString {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.0 {
            Some(s) => serializer.serialize_str(s),
            None => serializer.serialize_unit(),
        }
    }
}

impl<'de> serde::Deserialize<'de> for FlexString {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Self(match value {
            Value::String(s) => Some(s),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }))
    }
}

impl From<&str> for FlexString {
    fn from(v: &str) -> Self {
        Self(Some(v.to_owned()))
    }
}

impl From<String> for FlexString {
    fn from(v: String) -> Self {
        Self(Some(v))
    }
}

impl FlexString {
    /// Build from a string, treating empty as absent (JS `p.x || …` semantics
    /// are then applied by the caller via [`FlexString::or`]).
    pub fn from_non_empty(v: impl Into<String>) -> Self {
        let s = v.into();
        if s.is_empty() {
            Self(None)
        } else {
            Self(Some(s))
        }
    }
}

/// JS truthiness for an optional string: `Some(non-empty)` is truthy.
#[must_use]
pub fn truthy_str(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|s| !s.is_empty())
}

/// `p.flag ?? default` for optional booleans.
#[must_use]
pub fn bool_or(value: Option<bool>, default: bool) -> bool {
    value.unwrap_or(default)
}

/// `Number(p.x) === 1` — the reference modules gate branches on this a lot.
#[must_use]
pub fn is_one(value: &FlexI64) -> bool {
    value.value() == Some(1)
}

/// `p.x || default` for an optional flex-number field (None / zero fall back).
#[must_use]
pub fn or_flex(value: &Option<FlexI64>, default: i64) -> i64 {
    value
        .as_ref()
        .and_then(FlexI64::value)
        .filter(|v| *v != 0)
        .unwrap_or(default)
}

/// `p.x ?? default` for an optional flex-number field (only None falls back).
#[must_use]
pub fn unwrap_or_flex(value: &Option<FlexI64>, default: i64) -> i64 {
    value.as_ref().and_then(FlexI64::value).unwrap_or(default)
}

/// `Number(p.x) === 1` for an optional flex-number field.
#[must_use]
pub fn is_one_opt(value: &Option<FlexI64>) -> bool {
    value.as_ref().and_then(FlexI64::value) == Some(1)
}

/// JS `a || b || c` chain over optional numbers: the first non-zero,
/// non-missing value wins, otherwise `default`.
#[must_use]
pub fn or_chain(values: &[Option<i64>], default: i64) -> i64 {
    for value in values.iter().flatten() {
        if *value != 0 {
            return *value;
        }
    }
    default
}

/// JS `substring`: clamp and swap indices, operating on **chars** (JS operates
/// on UTF-16 code units; for the phone-masking use case ASCII is assumed).
#[must_use]
pub fn js_substring(s: &str, start: usize, end: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let (mut a, mut b) = (start.min(len), end.min(len));
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    chars[a..b].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flex_i64_coercion() {
        assert_eq!(
            serde_json::from_value::<FlexI64>(serde_json::json!(7))
                .unwrap()
                .value(),
            Some(7)
        );
        assert_eq!(
            serde_json::from_value::<FlexI64>(serde_json::json!("7"))
                .unwrap()
                .value(),
            Some(7)
        );
        assert_eq!(
            serde_json::from_value::<FlexI64>(serde_json::json!(" 8 "))
                .unwrap()
                .value(),
            Some(8)
        );
        assert_eq!(
            serde_json::from_value::<FlexI64>(serde_json::json!(""))
                .unwrap()
                .value(),
            Some(0)
        );
        assert_eq!(
            serde_json::from_value::<FlexI64>(serde_json::json!(null))
                .unwrap()
                .value(),
            None
        );
        assert_eq!(
            serde_json::from_value::<FlexI64>(serde_json::json!(true))
                .unwrap()
                .value(),
            Some(1)
        );
    }

    #[test]
    fn falsy_fallbacks() {
        // Number(p.x) || 5 with x = 0 → 5 (JS falsy)
        assert_eq!(FlexI64(Some(0)).or(5), 5);
        // p.x ?? 5 with x = 0 → 0 (nullish keeps zero)
        assert_eq!(FlexI64(Some(0)).unwrap_or(5), 0);
        assert_eq!(FlexI64(None).unwrap_or(5), 5);
        // p.x || 'd' with x = '' → 'd'
        assert_eq!(FlexString(Some(String::new())).or("d"), "d");
        assert_eq!(FlexString(None).or("d"), "d");
        assert_eq!(FlexString(Some("v".into())).or("d"), "v");
    }

    #[test]
    fn substring_semantics() {
        // login.cellphone masks: substring(0,2) + '*****' + substring(10,11)
        let mobile = "13800000000";
        assert_eq!(js_substring(mobile, 0, 2), "13");
        assert_eq!(js_substring(mobile, 10, 11), "0");
        // out-of-range clamps
        assert_eq!(js_substring("abc", 2, 99), "c");
        assert_eq!(js_substring("abc", 5, 9), "");
    }
}
