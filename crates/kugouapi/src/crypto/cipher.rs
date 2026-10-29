//! AES cipher family: `sessionCipher` (playlistAes) and `aesHex` (cryptoAes).

use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use super::{CipherInput, CipherOutput};
use crate::error::{Error, Result};
use crate::sign::md5;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

/// Result of [`session_cipher_encrypt`].
#[derive(Debug, Clone)]
pub struct SessionCipherResult {
    /// AES ciphertext (base64), used as the POST body.
    pub body: String,
    /// 6-char session key (returned to the server inside the RSA `p` param).
    pub key: String,
}

/// `playlistAes`: random 6-char key → md5-derived AES-128 key/IV → base64 body.
#[must_use]
pub fn session_cipher_encrypt(data: &CipherInput) -> SessionCipherResult {
    let key = super::random_string(6).to_lowercase();
    let body = session_cipher_raw(data, &key);
    SessionCipherResult { body, key }
}

fn session_cipher_raw(data: &CipherInput, key: &str) -> String {
    let digest = md5(key);
    // NOTE: the AES key/IV are the *ASCII bytes of the hex digest slices*,
    // not the raw digest bytes.
    let aes_key = digest.as_bytes()[0..16].to_vec();
    let iv = digest.as_bytes()[16..32].to_vec();

    let plaintext = data.to_plain();
    let cipher = Aes128CbcEnc::new_from_slices(&aes_key, &iv)
        .expect("sessionCipher key/iv are fixed 16 bytes");
    let ciphertext = cipher.encrypt_padded_vec::<Pkcs7>(plaintext.as_bytes());
    BASE64.encode(ciphertext)
}

/// Inverse of [`session_cipher_encrypt`]: base64 ciphertext → JSON (or the raw
/// text when the plaintext is not valid JSON). Mirrors `playlistAesDecrypt`.
pub fn session_cipher_decrypt(payload: &str, key: &str) -> Result<CipherOutput> {
    let digest = md5(key);
    let aes_key = digest.as_bytes()[0..16].to_vec();
    let iv = digest.as_bytes()[16..32].to_vec();

    let ciphertext = BASE64
        .decode(payload)
        .map_err(|e| Error::Crypto(format!("sessionCipher: invalid base64: {e}")))?;
    let decipher = Aes128CbcDec::new_from_slices(&aes_key, &iv)
        .expect("sessionCipher key/iv are fixed 16 bytes");
    let plaintext = decipher
        .decrypt_padded_vec::<Pkcs7>(&ciphertext)
        .map_err(|e| Error::Crypto(format!("sessionCipher: decrypt failed: {e}")))?;

    Ok(CipherOutput::parse(
        String::from_utf8_lossy(&plaintext).into_owned(),
    ))
}

/// Result of [`aes_hex_encrypt`].
#[derive(Debug, Clone)]
pub struct AesHexResult {
    /// AES-256-CBC ciphertext (hex).
    pub str: String,
    /// The key actually used (random when none was supplied).
    pub key: String,
}

/// `cryptoAesEncrypt`: random 16-char key → md5-derived AES-256 key / IV → hex
/// ciphertext. When an explicit `key` **and** `iv` are supplied the explicit IV
/// is used verbatim (the key is still md5-hashed) — this is exactly what the
/// login family's `t1` / `t2` parameters rely on.
#[must_use]
pub fn aes_hex_encrypt(data: &CipherInput, key: Option<&str>, iv: Option<&str>) -> AesHexResult {
    let key = key.filter(|k| !k.is_empty()).map_or_else(
        || super::random_string(16).to_lowercase(),
        std::borrow::ToOwned::to_owned,
    );
    let str = aes_hex_raw(data, &key, iv);
    AesHexResult { str, key }
}

fn aes_hex_raw(data: &CipherInput, key: &str, explicit_iv: Option<&str>) -> String {
    let digest = md5(key);
    let aes_key = digest.as_bytes()[0..32].to_vec();
    let iv = match explicit_iv {
        Some(iv) => iv.as_bytes().to_vec(),
        None => digest.as_bytes()[16..32].to_vec(),
    };

    let plaintext = data.to_plain();
    let cipher = Aes256CbcEnc::new_from_slices(&aes_key, &iv)
        .expect("cryptoAes key is 32 bytes and iv is 16 bytes");
    let ciphertext = cipher.encrypt_padded_vec::<Pkcs7>(plaintext.as_bytes());
    hex::encode(ciphertext)
}

/// `cryptoAesDecrypt`: hex ciphertext → JSON (or raw text).
pub fn aes_hex_decrypt(data: &str, key: &str, iv: Option<&str>) -> Result<CipherOutput> {
    let digest = md5(key);
    let aes_key = digest.as_bytes()[0..32].to_vec();
    let iv = match iv {
        Some(iv) => iv.as_bytes().to_vec(),
        None => digest.as_bytes()[16..32].to_vec(),
    };

    let ciphertext =
        hex::decode(data).map_err(|e| Error::Crypto(format!("cryptoAes: invalid hex: {e}")))?;
    let decipher = Aes256CbcDec::new_from_slices(&aes_key, &iv)
        .expect("cryptoAes key is 32 bytes and iv is 16 bytes");
    let plaintext = decipher
        .decrypt_padded_vec::<Pkcs7>(&ciphertext)
        .map_err(|e| Error::Crypto(format!("cryptoAes: decrypt failed: {e}")))?;

    Ok(CipherOutput::parse(
        String::from_utf8_lossy(&plaintext).into_owned(),
    ))
}
