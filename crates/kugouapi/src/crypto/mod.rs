//! `KuGou` protocol crypto primitives.
//!
//! Every primitive aligns bit-for-bit with the community reference
//! implementation (KuGouMusicApi):
//!
//! - `session_cipher` (`playlistAes`): random 6-char lowercase-alnum key,
//!   AES key = `md5(key)[0..16]`, IV = `md5(key)[16..32]`, ciphertext base64.
//!   Used by `register_dev` / cloud / favourites / playlist-delete and the rest
//!   of the "encrypted POST" family.
//! - `rsa_encrypt_hex` (`rsaEncrypt2`): RSA PKCS#1 v1.5, hex output.
//! - `aes_hex` (`cryptoAesEncrypt`): random 16-char key, AES-256-CBC with
//!   key = `md5(key)[0..32]`, IV = `md5(key)[16..32]` (or an explicit IV), hex
//!   output. Used by the login family.
//! - `rsa_encrypt_raw` (`cryptoRSAEncrypt`): padding-less RSA (zero-padded
//!   plaintext then raw modPow), hex output. Used for the login `pk` param.
//!
//! The [`CryptoProvider`] trait lets callers replace the register-device
//! pipeline wholesale.

mod cipher;
mod krc;
mod rsa;
mod security;

pub use cipher::{
    AesHexResult, SessionCipherResult, aes_hex_decrypt, aes_hex_encrypt, session_cipher_decrypt,
    session_cipher_encrypt,
};
pub use krc::decode_krc;
pub use rsa::{
    LITE_RSA_PUBLIC_KEY, STANDARD_RSA_PUBLIC_KEY, load_public_key, md5_of, random_string,
    random_thash, raw_modpow_public, rsa_encrypt_hex, rsa_encrypt_raw,
};
pub use security::{SimulateInput, SimulateResult, generate_simulate, generate_webgl_hash};

pub(crate) use security::now_ms;

use serde_json::Value;

use crate::config::Platform;
use crate::error::{Error, Result};

/// Payload accepted by the crypto primitives: either a raw string or a JSON
/// value (which is serialized compactly, exactly like `JSON.stringify`).
#[derive(Debug, Clone)]
pub enum CipherInput {
    /// Plain text payload.
    Text(String),
    /// JSON payload (`null` serializes to `"null"` like in JS).
    Json(Value),
}

impl CipherInput {
    /// Serialize to the plaintext bytes fed into the cipher.
    #[must_use]
    pub fn to_plain(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Json(v) => serde_json::to_string(v).unwrap_or_else(|_| "null".to_owned()),
        }
    }
}

impl From<&str> for CipherInput {
    fn from(v: &str) -> Self {
        Self::Text(v.to_owned())
    }
}

impl From<String> for CipherInput {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}

impl From<Value> for CipherInput {
    fn from(v: Value) -> Self {
        Self::Json(v)
    }
}

impl From<&Value> for CipherInput {
    fn from(v: &Value) -> Self {
        Self::Json(v.clone())
    }
}

/// Decrypted payload: JSON when the plaintext parses as JSON, text otherwise
/// (mirrors the TS `JSON.parse` try/catch).
#[derive(Debug, Clone, PartialEq)]
pub enum CipherOutput {
    /// Plain text payload (not valid JSON).
    Text(String),
    /// Parsed JSON payload.
    Json(Value),
}

impl CipherOutput {
    /// Parse decrypted text: JSON when possible, text otherwise.
    #[must_use]
    pub fn parse(text: String) -> Self {
        match serde_json::from_str(&text) {
            Ok(v) => Self::Json(v),
            Err(_) => Self::Text(text),
        }
    }

    /// Convert into a [`Value`] (text becomes a JSON string).
    #[must_use]
    pub fn into_value(self) -> Value {
        match self {
            Self::Text(s) => Value::String(s),
            Self::Json(v) => v,
        }
    }
}

/// Result of [`session_cipher_encrypt`] / mirrors of `encryptRequest`.
#[derive(Debug, Clone)]
pub struct EncryptedRequest {
    /// AES ciphertext (base64), sent directly as the POST body.
    pub body: String,
    /// 6-char session key (used by `encrypt_key`; also decrypts the response).
    pub key: String,
}

/// Replaceable crypto provider for the register-device pipeline.
///
/// The default implementation ([`DefaultCryptoProvider`]) aligns with the
/// reference protocol:
///
/// - `encrypt_request(data)` — session cipher, returns `{ body, key }`
/// - `decrypt_response(payload, key)` — `payload` is the base64 of the response
///   bytes
/// - `encrypt_key(key, userid, token)` — produces the `p` query parameter:
///   `rsaEncryptHex({aes: key, uid: userid, token}).toUpperCase()`
pub trait CryptoProvider: Send + Sync {
    /// Encrypt a request body.
    fn encrypt_request(&self, data: &CipherInput) -> EncryptedRequest;

    /// Decrypt a response payload.
    fn decrypt_response(&self, payload: &str, key: &str) -> Result<CipherOutput>;

    /// Produce the `p` query parameter.
    fn encrypt_key(&self, key: &str, userid: u64, token: &str) -> Result<String>;
}

/// Built-in provider with the reference protocol and bundled public keys.
#[derive(Debug, Clone)]
pub struct DefaultCryptoProvider {
    platform: Platform,
    rsa_public_key: Option<String>,
}

impl DefaultCryptoProvider {
    /// Create a provider for a platform, optionally overriding the RSA public
    /// key (PEM or DER base64, SPKI / PKCS#1).
    #[must_use]
    pub fn new(platform: Platform, rsa_public_key: Option<String>) -> Self {
        Self {
            platform,
            rsa_public_key,
        }
    }
}

impl Default for DefaultCryptoProvider {
    fn default() -> Self {
        Self::new(Platform::Standard, None)
    }
}

impl CryptoProvider for DefaultCryptoProvider {
    fn encrypt_request(&self, data: &CipherInput) -> EncryptedRequest {
        let result = session_cipher_encrypt(data);
        EncryptedRequest {
            body: result.body,
            key: result.key,
        }
    }

    fn decrypt_response(&self, payload: &str, key: &str) -> Result<CipherOutput> {
        session_cipher_decrypt(payload, key)
    }

    fn encrypt_key(&self, key: &str, userid: u64, token: &str) -> Result<String> {
        // Plaintext is always {"aes":…,"uid":…,"token":…} in this exact order.
        #[derive(serde::Serialize)]
        struct P<'a> {
            aes: &'a str,
            uid: u64,
            token: &'a str,
        }
        let payload = serde_json::to_string(&P {
            aes: key,
            uid: userid,
            token,
        })
        .map_err(|e| Error::Crypto(e.to_string()))?;

        let hex_out = match &self.rsa_public_key {
            None => rsa_encrypt_hex(&CipherInput::Text(payload), self.platform),
            Some(pem) => {
                let public = load_public_key(pem)?;
                rsa::pkcs1v15_hex(&public, payload.as_bytes())?
            }
        };
        Ok(hex_out.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn session_cipher_roundtrip() {
        let payload = json!({ "brand": "Redmi", "imei": "abc123", "batteryLevel": 88 });
        let encrypted = session_cipher_encrypt(&CipherInput::Json(payload.clone()));
        assert_eq!(encrypted.key.len(), 6);
        assert!(
            encrypted
                .key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );

        let decrypted = session_cipher_decrypt(&encrypted.body, &encrypted.key).unwrap();
        assert_eq!(decrypted, CipherOutput::Json(payload));
    }

    #[test]
    fn session_cipher_keys_are_random() {
        let a = session_cipher_encrypt(&CipherInput::Json(json!({})));
        let b = session_cipher_encrypt(&CipherInput::Json(json!({})));
        assert_ne!(a.key, b.key);
    }

    #[test]
    fn session_cipher_non_json_roundtrip() {
        let encrypted = session_cipher_encrypt(&CipherInput::Text("不是 JSON 的纯文本".to_owned()));
        let decrypted = session_cipher_decrypt(&encrypted.body, &encrypted.key).unwrap();
        assert_eq!(
            decrypted,
            CipherOutput::Text("不是 JSON 的纯文本".to_owned())
        );
    }

    #[test]
    fn aes_hex_roundtrip() {
        let payload = json!({ "pwd": "x", "code": "y" });
        let encrypted = aes_hex_encrypt(&CipherInput::Json(payload.clone()), None, None);
        assert_eq!(encrypted.key.len(), 16);
        assert!(encrypted.str.chars().all(|c| c.is_ascii_hexdigit()));

        let decrypted = aes_hex_decrypt(&encrypted.str, &encrypted.key, None).unwrap();
        assert_eq!(decrypted, CipherOutput::Json(payload));
    }

    #[test]
    fn aes_hex_explicit_key_iv() {
        let out = aes_hex_encrypt(
            &CipherInput::Text("hello".to_owned()),
            Some("0123456789abcdef"),
            Some("fedcba9876543210"),
        );
        assert_ne!(out.str, "");
        // roundtrip with the same explicit material
        let decrypted =
            aes_hex_decrypt(&out.str, "0123456789abcdef", Some("fedcba9876543210")).unwrap();
        assert_eq!(decrypted, CipherOutput::Text("hello".to_owned()));
    }

    #[test]
    fn rsa_hex_shape() {
        let hex_out = rsa_encrypt_hex(
            &CipherInput::Json(json!({"aes":"abcdef","uid":7,"token":"T"})),
            Platform::Standard,
        );
        // 1024-bit key → 128 bytes → 256 hex chars, lowercase, PKCS1 random →
        // two invocations differ.
        assert_eq!(hex_out.len(), 256);
        assert!(hex_out.chars().all(|c| c.is_ascii_hexdigit()));
        let other = rsa_encrypt_hex(
            &CipherInput::Json(json!({"aes":"abcdef","uid":7,"token":"T"})),
            Platform::Standard,
        );
        assert_ne!(hex_out, other);
    }

    #[test]
    fn rsa_raw_shape() {
        let payload = json!({ "clienttime_ms": 1717171717171u64, "key": "0123456789abcdef" });
        let hex_out =
            rsa_encrypt_raw(&CipherInput::Json(payload), None, Platform::Standard).unwrap();
        assert_eq!(hex_out.len(), 256);
        assert!(hex_out.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn default_provider_encrypt_key_shape() {
        let provider = DefaultCryptoProvider::default();
        let p = provider.encrypt_key("abcdef", 42, "TOKEN").unwrap();
        assert_eq!(p.len(), 256);
        assert!(
            p.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase())
        );
    }

    #[test]
    fn default_provider_roundtrip() {
        let provider = DefaultCryptoProvider::default();
        let payload = json!({ "dfid": "DFID", "token": "T" });
        let encrypted = provider.encrypt_request(&CipherInput::Json(payload.clone()));
        assert_eq!(
            provider
                .decrypt_response(&encrypted.body, &encrypted.key)
                .unwrap(),
            CipherOutput::Json(payload)
        );
    }

    #[test]
    fn random_string_shape() {
        let s = random_string(24);
        assert_eq!(s.len(), 24);
        assert!(
            s.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        );
    }

    #[test]
    fn md5_of_json_vs_scalar() {
        use crate::sign::md5;
        assert_eq!(md5_of(&CipherInput::Json(json!({"a":1}))), md5("{\"a\":1}"));
        assert_eq!(md5_of(&CipherInput::Text("abc".into())), md5("abc"));
    }
}
