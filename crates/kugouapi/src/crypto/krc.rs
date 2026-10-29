//! KRC lyric decoding.
//!
//! Format: base64 payload → skip the 4-byte header → XOR every byte with a
//! 16-byte key (repeating) → zlib inflate. Returns an empty string when the
//! payload cannot be decoded (mirrors the reference behaviour).

use std::io::Read;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

const KRC_XOR_KEY: [u8; 16] = [
    64, 71, 97, 119, 94, 50, 116, 71, 81, 54, 49, 45, 206, 210, 110, 105,
];

/// Decode a KRC lyric payload (base64) into the plain text lyric.
#[must_use]
pub fn decode_krc(content: &str) -> String {
    let Ok(bytes) = BASE64.decode(content) else {
        return String::new();
    };
    if bytes.len() <= 4 {
        return String::new();
    }
    let mut body = bytes[4..].to_vec();
    for (i, byte) in body.iter_mut().enumerate() {
        *byte ^= KRC_XOR_KEY[i % KRC_XOR_KEY.len()];
    }

    let mut decoder = flate2::read::ZlibDecoder::new(&body[..]);
    let mut out = Vec::new();
    if decoder.read_to_end(&mut out).is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_known_payload() {
        // Constructed with the reference algorithm: zlib-compress a lyric body,
        // XOR with the key, prepend 4 header bytes, base64.
        let mut zipped = Vec::new();
        {
            let mut encoder =
                flate2::write::ZlibEncoder::new(&mut zipped, flate2::Compression::default());
            use std::io::Write;
            let _ = encoder.write_all(b"[offset:0][total:1000]line one\nline two");
            let _ = encoder.finish();
        }
        let mut obfuscated = vec![0u8; zipped.len()];
        for (i, byte) in zipped.iter().enumerate() {
            obfuscated[i] = byte ^ KRC_XOR_KEY[i % KRC_XOR_KEY.len()];
        }
        let mut framed = vec![0x4b, 0x52, 0x43, 0x01];
        framed.append(&mut obfuscated);
        let payload = BASE64.encode(framed);

        let decoded = decode_krc(&payload);
        assert!(decoded.contains("line one"));
        assert!(decoded.contains("line two"));
    }

    #[test]
    fn invalid_payload_yields_empty_string() {
        assert_eq!(decode_krc("!!!not-base64!!!"), "");
        assert_eq!(decode_krc(&BASE64.encode([1, 2, 3])), "");
    }
}
