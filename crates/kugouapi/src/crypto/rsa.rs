//! RSA family: PKCS#1 v1.5 (`rsaEncrypt2`), raw no-padding (`cryptoRSAEncrypt`)
//! and public-key loading.

use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::{BoxedUint, Pkcs1v15Encrypt, PublicKey};

use super::CipherInput;
use crate::config::Platform;
use crate::error::{Error, Result};

/// Bundled standard-edition RSA public key (community-published material).
pub const STANDARD_RSA_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDIAG7QOELSYoIJvTFJhMpe1s/g\nbjDJX51HBNnEl5HXqTW6lQ7LC8jr9fWZTwusknp+sVGzwd40MwP6U5yDE27M/X1+\nUR4tvOGOqp94TJtQ1EPnWGWXngpeIW5GxoQGao1rmYWAu6oi1z9XkChrsUdC6DJE\n5E221wf/4WLFxwAtRQIDAQAB\n-----END PUBLIC KEY-----";

/// Bundled concept-edition (lite) RSA public key.
pub const LITE_RSA_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDECi0Np2UR87scwrvTr72L6oO0\n1rBbbBPriSDFPxr3Z5syug0O24QyQO8bg27+0+4kBzTBTBOZ/WWU0WryL1JSXRTX\nLgFVxtzIY41Pe7lPOgsfTCn5kZcvKhYKJesKnnJDNr5/abvTGf+rHG3YRwsCHcQ08\n/q6ifSioBszvb3QiwIDAQAB\n-----END PUBLIC KEY-----";

/// The bundled public key for a platform.
#[must_use]
pub fn platform_public_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Standard => STANDARD_RSA_PUBLIC_KEY,
        Platform::Lite => LITE_RSA_PUBLIC_KEY,
    }
}

/// Reference `randomString`: uppercase letters + digits.
#[must_use]
pub fn random_string(len: usize) -> String {
    const POOL: &[u8] = b"1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    use rand::RngExt;
    let mut rng = rand::rng();
    (0..len)
        .map(|_| POOL[rng.random_range(0..POOL.len())] as char)
        .collect()
}

/// Generate a random KG-THash (7 hex chars).
#[must_use]
pub fn random_thash() -> String {
    use rand::RngExt;
    let bytes: [u8; 4] = rand::rng().random();
    hex::encode(bytes).chars().take(7).collect()
}

/// `cryptoMd5`: md5 of `JSON.stringify(data)` for objects / `String(data)`
/// otherwise.
#[must_use]
pub fn md5_of(data: &CipherInput) -> String {
    crate::sign::md5(&data.to_plain())
}

/// `rsaEncrypt2`: RSA PKCS#1 v1.5 over the JSON payload, lowercase hex.
#[must_use]
pub fn rsa_encrypt_hex(data: &CipherInput, platform: Platform) -> String {
    let pem = platform_public_key(platform);
    let key = load_public_key(pem).expect("bundled public keys are valid");
    pkcs1v15_hex(&key, data.to_plain().as_bytes()).expect("bundled public keys are valid")
}

/// PKCS#1 v1.5 encrypt to lowercase hex with an explicit key.
pub fn pkcs1v15_hex(key: &PublicKey, plaintext: &[u8]) -> Result<String> {
    // rsa 0.9 bounds its RNG on rand_core 0.6 traits, which rand 0.10's
    // ThreadRng cannot satisfy; rsa's re-exported OsRng bridges the gap.
    let mut rng = rand::rng();
    let ciphertext = key
        .encrypt(&mut rng, Pkcs1v15Encrypt, plaintext)
        .map_err(|e| Error::Crypto(format!("rsaEncrypt2: {e}")))?;
    Ok(hex::encode(ciphertext))
}

/// `cryptoRSAEncrypt`: padding-less RSA — the plaintext is zero-padded to the
/// modulus length (little-endian copy at offset 0, i.e. left-aligned with
/// trailing zero bytes) and then raw modPow is applied. Lowercase hex output
/// padded to the modulus length.
///
/// Used for the login family `pk` parameter.
pub fn rsa_encrypt_raw(
    data: &CipherInput,
    public_key_pem: Option<&str>,
    platform: Platform,
) -> Result<String> {
    let key = match public_key_pem {
        Some(pem) => load_public_key(pem)?,
        None => {
            load_public_key(platform_public_key(platform)).expect("bundled public keys are valid")
        }
    };
    raw_modpow_hex(&key, data.to_plain().as_bytes())
}

/// Raw no-padding RSA over a pre-padded buffer, returning the raw ciphertext
/// bytes (used by the `login.device.kick` flow).
pub fn raw_modpow_public(key: &PublicKey, padded: &[u8]) -> Result<Vec<u8>> {
    let modulus_len = key.size();
    if padded.len() != modulus_len {
        return Err(Error::Crypto(format!(
            "raw RSA input length {} does not match modulus length {}",
            padded.len(),
            modulus_len
        )));
    }

    let m = BoxedUint::from_be_slice(padded, (modulus_len * 8) as u32)
        .map_err(|e| Error::Crypto(format!("raw RSA input: {e}")))?;
    let c = rsa::hazmat::rsa_encrypt(key, &m)
        .map_err(|e| Error::Crypto(format!("raw RSA encrypt: {e}")))?;

    let bytes = c.to_be_bytes(); // 固定长度，通常正好是 modulus_len
    Ok(bytes.to_vec())
}

/// Raw no-padding RSA: zero-pad plaintext to modulus length, modPow, hex output
/// left-padded to the modulus length (matching Node's `RSA_NO_PADDING`
/// fixed-length output).
pub fn raw_modpow_hex(key: &PublicKey, plaintext: &[u8]) -> Result<String> {
    let modulus_len = key.size();
    if plaintext.len() > modulus_len {
        return Err(Error::Crypto(format!(
            "raw RSA plaintext length {} exceeds modulus length {}",
            plaintext.len(),
            modulus_len
        )));
    }
    let mut padded = vec![0u8; modulus_len];
    padded[modulus_len - plaintext.len()..].copy_from_slice(plaintext);

    let bytes = raw_modpow_public(key, &padded)?;
    Ok(hex::encode(bytes).to_uppercase())
}

/// Load an RSA public key from PEM (SPKI or PKCS#1) or DER base64 (SPKI, then
/// PKCS#1).
pub fn load_public_key(key: &str) -> Result<PublicKey> {
    if key.contains("-----") {
        if let Ok(parsed) = PublicKey::from_public_key_pem(key) {
            return Ok(parsed);
        }
        if let Ok(parsed) = PublicKey::from_pkcs1_pem(key) {
            return Ok(parsed);
        }
        return Err(Error::Crypto(
            "invalid RSA public key: unreadable PEM".to_owned(),
        ));
    }

    let der = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key.trim())
        .map_err(|e| Error::Crypto(format!("invalid RSA public key (DER base64): {e}")))?;
    if let Ok(parsed) = PublicKey::from_public_key_der(&der) {
        return Ok(parsed);
    }
    if let Ok(parsed) = PublicKey::from_pkcs1_der(&der) {
        return Ok(parsed);
    }
    Err(Error::Crypto(
        "invalid RSA public key: neither PEM nor DER base64 (SPKI/PKCS#1)".to_owned(),
    ))
}
