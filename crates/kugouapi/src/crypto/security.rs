//! SSA secondary-verification simulation (`sid` / `edt` generation).
//!
//! Port of the reference `generateSimulate`:
//!
//! - a random mouse trajectory (cubic bezier with jitter) is assembled into an
//!   event stream,
//! - `edt` = AES-128-CBC(`md5(rand16)[0..16]`, fixed IV) over the event string,
//! - `sid` = RSA-OAEP-SHA256 over the AES key, base64.

use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use rand::{Rng, RngExt};
use rsa::Oaep;
use rsa::pkcs8::DecodePublicKey;
// NOTE: rsa 0.9 internally speaks digest 0.10 / rand_core 0.6, so the OAEP
// hash type must come from rsa's own `sha2` re-export, not this crate's sha2 0.11.
use rsa::sha2::Sha256;

use crate::error::{Error, Result};
use crate::sign::md5;

const SECURITY_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoW2+Ylo8ALePSQTP0xBF\nlFmEOHvBD9tS+s7DBlfKEu3RzzvZTaX1JtYbX4+AVUqj6ARz8IM+CKByqGFvbHN/\nW64XxNI+q7z36ajCL3VTJ2W5G9MCJitc6oGbire4NQfhaEq0nC+hxBWQvCbIFflA\n2ItrLUbSU7z1bHA/a+jlQm4OWvY+IKnTryOJTPuT1yNOVjbJ8wBLKy2DgQr9pPqW\nPmEQtGpR5IM9V8Kao6PaSdKYOWGbX3i2+RzIKhvZUxxtJwdVbqPlDPlW9h4/xIBc\n56Lgvr4aIl8nFtwbj4UJVUTFuGrs0tY9H/tXvZ22dUCKuGxW/gW7ZF+gXz6vHtYa\nrQIDAQAB\n-----END PUBLIC KEY-----";

const SECURITY_IV: &str = "kugousecurity123";

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

/// WebGL fingerprint simulation for non-browser environments: a random uint64
/// as a decimal string.
#[must_use]
pub fn generate_webgl_hash() -> String {
    let mut rng = rand::rng();
    let hi: u32 = rng.random();
    let lo: u32 = rng.random();
    (u64::from(hi) * 0x1_0000_0000 + u64::from(lo)).to_string()
}

fn ri(rng: &mut (impl Rng + ?Sized), min: i64, max: i64) -> i64 {
    rng.random_range(min..=max)
}

#[derive(Debug, Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

/// Cubic bezier trajectory with random jitter.
///
/// Single-letter parameters mirror the reference math (`t` / `u` …).
fn bezier_path(
    rng: &mut (impl Rng + ?Sized),
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    point_count: usize,
) -> Vec<Point> {
    let c1x = start_x + (end_x - start_x) * 0.3 + ri(rng, -80, 80) as f64;
    let c1y = start_y + (end_y - start_y) * 0.2 + ri(rng, -60, 60) as f64;
    let c2x = start_x + (end_x - start_x) * 0.7 + ri(rng, -60, 60) as f64;
    let c2y = start_y + (end_y - start_y) * 0.8 + ri(rng, -40, 40) as f64;

    let mut pts = Vec::with_capacity(point_count + 1);
    for i in 0..=point_count {
        let t = i as f64 / point_count as f64;
        let u = 1.0 - t;
        let x =
            u * u * u * start_x + 3.0 * u * u * t * c1x + 3.0 * u * t * t * c2x + t * t * t * end_x;
        let y =
            u * u * u * start_y + 3.0 * u * u * t * c1y + 3.0 * u * t * t * c2y + t * t * t * end_y;
        let jitter = (3.0 - t * 2.5).max(0.5);
        pts.push(Point {
            x: x + (rng.random::<f64>() - 0.5) * jitter,
            y: y + (rng.random::<f64>() - 0.5) * jitter,
        });
    }
    pts
}

/// Simulated event stream (mouse / scroll / window events + sentinel records).
///
/// Single-letter parameters mirror the reference math (`t` / `i` / `x` / `y`).
fn generate_edt_data(
    rng: &mut (impl Rng + ?Sized),
    start: (f64, f64),
    end: (f64, f64),
    mouse_points: usize,
    sentinel: u64,
) -> String {
    let mut entries: Vec<String> = Vec::new();
    let mut ts: i64 = 0;
    let mut ei: i64 = 0;

    let f3 = |ts: i64, idx: i64, x: i64, y: i64| format!("3,{ts},{idx},{x},{y}");
    let f5 = |ts: i64, idx: i64| format!("5,{ts},{idx}");
    let f6 = |ts: i64, idx: i64, x: i64, y: i64| format!("6,{ts},{idx},{x},{y}");
    let fs3 = |idx: i64, x: i64, y: i64| format!("3,{sentinel},{idx},{x},{y}");
    let fs5 = |idx: i64| format!("5,{sentinel},{idx}");
    let fs6 = |idx: i64, x: i64, y: i64| format!("6,{sentinel},{idx},{x},{y}");

    entries.push(f5(0, 0));
    entries.push(fs5(0));
    entries.push(f5(0, 0));
    entries.push(fs5(0));

    ts += ri(rng, 5, 20);
    entries.push(f6(ts, ei, 750, 500));
    entries.push(fs6(ei, 750, 500));
    ei += 1;

    for _ in 0..3 {
        ts += ri(rng, 80, 600);
        entries.push(f5(ts, ei));
        entries.push(fs5(ei));
        ei += 1;
    }

    let path = bezier_path(rng, start.0, start.1, end.0, end.1, mouse_points);
    let mut si: i64 = 0;
    for (i, point) in path.iter().enumerate() {
        let (x, y) = (point.x, point.y);
        ts += ri(rng, 8, 50);
        entries.push(f3(ts, si, x.round() as i64, y.round() as i64));
        entries.push(fs3(si, x.round() as i64, y.round() as i64));
        if i > 0 && i % 12 == 0 {
            ts += ri(rng, 20, 60);
            entries.push(f5(ts, ei));
            entries.push(fs5(ei));
            ei += 1;
        }
        si = (si + 1) % 2;
    }

    ts += ri(rng, 5, 30);
    entries.push(f3(
        ts,
        1,
        (end.0 + ri(rng, -5, 5) as f64).round() as i64,
        (end.1 + ri(rng, -5, 5) as f64).round() as i64,
    ));
    entries.push(fs3(1, end.0.round() as i64, end.1.round() as i64));

    entries.join(":")
}

/// Input for [`generate_simulate`].
#[derive(Debug, Clone, Default)]
pub struct SimulateInput {
    /// Device mid (defaults to `0` like the reference).
    pub mid: Option<String>,
    /// User id (defaults to `0`).
    pub userid: Option<u64>,
    /// Device dfid (defaults to `0`).
    pub dfid: Option<String>,
    /// Pre-computed WebGL hash; generated when absent.
    pub webgl_hash: Option<String>,
}

/// Result of [`generate_simulate`].
#[derive(Debug, Clone)]
pub struct SimulateResult {
    /// AES-128-CBC encrypted event stream (base64).
    pub edt: String,
    /// RSA-OAEP-SHA256 encrypted AES key (base64).
    pub sid: String,
}

/// Generate `sid` / `edt` (aligned with the reference `generateSimulate`).
pub fn generate_simulate(input: SimulateInput) -> Result<SimulateResult> {
    let mut rng = rand::rng();

    let sentinel: u64 = 0xffff_ffff - rng.random_range(0..20);
    let key = md5(&super::random_string(16))[0..16].to_owned();
    let points = ri(&mut rng, 30, 60) as usize;
    let start_x = ri(&mut rng, 200, 600) as f64;
    let start_y = ri(&mut rng, 200, 500) as f64;
    let end_x = ri(&mut rng, 500, 700) as f64;
    let end_y = ri(&mut rng, 80, 150) as f64;
    let mid = input.mid.clone().unwrap_or_else(|| "0".to_owned());
    let userid = input.userid.unwrap_or(0);
    let dfid = input.dfid.clone().unwrap_or_else(|| "0".to_owned());
    let webgl_hash = input.webgl_hash.unwrap_or_else(generate_webgl_hash);
    let ts = now_ms();

    let data = generate_edt_data(
        &mut rng,
        (start_x, start_y),
        (end_x, end_y),
        points,
        sentinel,
    );
    let sid_plaintext = format!(
        "mid={mid};userid={userid};dfid={dfid};webgl={webgl_hash};webdriver=0;ts={ts};data={data}"
    );

    let cipher = Aes128CbcEnc::new_from_slices(key.as_bytes(), SECURITY_IV.as_bytes())
        .expect("security key/iv are fixed 16 bytes");
    let edt = BASE64.encode(cipher.encrypt_padded_vec::<Pkcs7>(sid_plaintext.as_bytes()));

    let public_key = rsa::RsaPublicKey::from_public_key_pem(SECURITY_PUBLIC_KEY)
        .map_err(|e| Error::Crypto(format!("security public key: {e}")))?;
    // rsa 0.9 bounds its RNG on rand_core 0.6 traits; rand 0.10's ThreadRng
    // cannot satisfy them, so the OAEP seed comes from rsa's re-exported OsRng
    // (the trajectory above keeps using rand 0.10).
    let mut os_rng = rsa::rand_core::OsRng;
    let sid = BASE64.encode(
        public_key
            .encrypt(&mut os_rng, Oaep::new::<Sha256>(), key.as_bytes())
            .map_err(|e| Error::Crypto(format!("security sid: {e}")))?,
    );

    Ok(SimulateResult { edt, sid })
}

/// Current UNIX time in milliseconds.
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulate_shape() {
        let result = generate_simulate(SimulateInput::default()).unwrap();
        // 2048-bit OAEP-SHA256 → 256 bytes → base64 length 344
        assert_eq!(result.sid.len(), 344);
        assert_ne!(result.edt, "");
        assert!(
            result
                .edt
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        );
    }

    #[test]
    fn webgl_hash_is_decimal() {
        let h = generate_webgl_hash();
        assert!(h.chars().all(|c| c.is_ascii_digit()));
    }
}
