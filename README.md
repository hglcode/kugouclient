# kugouapi-rs

[![CI](https://github.com/hglcode/kugouapi-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/hglcode/kugouapi-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Unofficial [KuGou](https://www.kugou.com/) music API client for Rust.

This crate is a Rust rendition of the TypeScript
[`kugou-api`](https://github.com/hglcode/kugouapi) design, which itself covers the
full module surface of the community reference implementation
[KuGouMusicApi](https://github.com/MakcRe/KuGouMusicApi) (main\@5d6c9f7, v1.6.2).

- **271-endpoint catalog** — every reference module is callable through a
  single consistent dispatch (`KuGouClient::call`), plus idiomatic typed
  methods with strongly-typed parameter structs for compile-time field
  checking (`client.search().song(...)`).
- **Built-in session** — after device registration or login, `dfid` / `mid` /
  `token` / `userid` / VIP info are carried automatically and can be exported
  and restored across processes.
- **Complete protocol coverage** — android / web / register signatures,
  trackercdn keys, encrypted POST bodies (session cipher), cloudlist, the
  login family (AES + RSA, QQ / WeChat / QR flows), the comment family, KRC
  lyric decoding and SSA simulation (`sid` / `edt`), all aligned bit-for-bit
  with the reference implementation.
- **Production HTTP layer** — per-attempt timeouts, idempotent GET retries
  with exponential backoff + half jitter + `Retry-After` support, a
  sliding-window circuit breaker and an optional token-bucket rate limiter,
  all behind a pluggable `Transport` trait (reqwest + rustls by default).
- **Injectable** — HTTP transport, crypto provider and RSA public keys are all
  replaceable, keeping the test suite fully offline.

## Workspace layout

| Crate | Description |
|---|---|
| `crates/kugouapi` | The library: client, protocol layers, endpoint catalog |
| `crates/kugouapi-cli` | The `kugou` binary: search / play / lyric / rank / register / login / call / endpoints |

## Install

Requires **Rust 1.89+** (MSRV is pinned by the 2025 RustCrypto stack:
`aes` 0.9 / `cipher` 0.5 / `sha2` 0.11).

```toml
[dependencies]
kugouapi = "0.1"
```

Or grab the CLI:

```bash
cargo install --path crates/kugouapi-cli
```

## Quick start

```rust,ignore
use kugouapi::{KuGouClient, SearchSongParams, SongUrlParams};

#[tokio::main]
async fn main() -> Result<(), kugouapi::Error> {
    let client = KuGouClient::builder().build()?;

    // Typed high-frequency endpoints
    let result = client
        .search()
        .song(SearchSongParams::new("周杰伦").page_size(30))
        .await?;
    for song in &result.lists {
        let url = client
            .song()
            .url(SongUrlParams::new(&song.file_hash))
            .await?;
        if let Some(play_url) = url.url.as_ref().and_then(|u| u.first()) {
            println!("{} -> {play_url}", song.file_name);
        }
    }

    // Catalog dispatch for everything else (271 ids)
    let rank = client.call("rank.list", serde_json::json!({})).await?;
    let lyric = client
        .call(
            "lyric",
            serde_json::json!({ "id": 652770770, "accesskey": "...", "decode": true }),
        )
        .await?;

    Ok(())
}
```

Endpoint ids derive from the reference module names with underscores converted
to dots: `rank_list` → `rank.list`, `song_url_new` → `song.url.new`. The full
mapping lives in [`docs/ENDPOINTS.md`](docs/ENDPOINTS.md) and can be
regenerated with `cargo run -p kugouapi-cli -- endpoints --markdown`.

## CLI

```bash
$ kugou search 周杰伦 --size 5
$ kugou play <hash> --quality flac
$ kugou lyric-search 晴天 && kugou lyric <id> <accesskey> --decode
$ kugou rank                     # rank list
$ kugou register                 # device registration (persists the session)
$ kugou login --mobile ... --code ...
$ kugou call rank.list '{}'      # raw catalog dispatch
$ kugou endpoints --filter song  # browse the 271 endpoints
```

Sessions persist in `~/.kugou/session.json` and are picked up automatically.

## Session management

The KuGou gateway API does **not** rely on HTTP cookies: identity credentials
(`dfid` / `mid` / `token` / `userid`) travel as query parameters and headers and
are managed explicitly through `SessionSnapshot`. They are written
automatically after successful registration / login, carried on every
subsequent request, and can be exported / restored at any time:

```rust,ignore
let client = KuGouClient::builder().build()?;
client.register_device(Default::default()).await?; // writes the session

// SMS login persists credentials too
client
    .call("login.cellphone", serde_json::json!({ "mobile": "...", "code": "..." }))
    .await?;

// Export / restore across processes (JSON-compatible with the TS library)
let snapshot = client.session().unwrap();
let restored = KuGouClient::builder().session(snapshot).build()?;
```

## Platform presets

```rust,ignore
// Concept edition (lite): appid / clientver / signature salts / RSA keys switch
let lite = KuGouClient::builder()
    .platform(kugouapi::Platform::Lite)
    .build()?;
```

## Customization

```rust,ignore
let client = KuGouClient::builder()
    .device(random_device())                 // random device fingerprint
    .timeout(Duration::from_secs(15))        // per-attempt timeout
    .retries(2)                              // GET retries
    .rate_limit(RateLimitOptions { requests_per_second: 10.0, burst: 20 })
    .circuit_breaker(None)                   // disable the breaker
    .ip("1.2.3.4")                           // forward the client IP
    .base_url("https://gateway.kugou.com")   // gateway override
    .appid(1005).clientver(20489)            // protocol overrides
    .crypto_provider(Box::new(MyCrypto))     // replace the register pipeline
    .transport(Arc::new(MyTransport))        // replace the HTTP layer
    .build()?;
```

## Protocol families

| Family | Endpoints (examples) | Notes |
|---|---|---|
| Gateway | `search.song`, `rank.list`, `user.detail` … | android signature + identity auto-injection |
| trackercdn | `song.url`, `song.url.auth`, `video.url` | auto `key`, included in the signature |
| Encrypted POST | `user.cloud`, `mv.collect`, `playlist.del` | session-cipher body, `p` = RSA({aes,uid,token}) |
| cloudlist | `playlist.update`, `playlist.sort`, `playlist.pic` | RSA session string + AES-CBC both ways |
| Login family | `login`, `login.cellphone`, `login.qr.*`, `login.qq.*`, `login.wx.*` | AES + RSA params, `secu_params` auto-decrypt |
| Comment family | `comment.*`, `song.barrage` | `m.comment.service` protocol, `key` = signParamsKey |
| Composite | `song.url.auth.merge`, `user.cloud.upload`, `user.update.avatar` | multi-step orchestration (auth → play / BSS multipart / image host) |
| Listen together | `listen.together.*` (43 operations) | one endpoint id per reference operation |
| CSCC | `user.listen.report` | device registration → session handshake → encrypted report |
| Team | `team.*` | h5 token envelope + web signature |

## Live verification status

| Capability | Status | Notes |
|---|---|---|
| `search()` / `search.song` | ✅ live-verified | 480 real results for `周杰伦` |
| `songUrl()` / `song.url` | ✅ live-verified | free songs return CDN urls; VIP songs return trial-segment info (`hash_offset`) |
| `rank.list` | ✅ live-verified | real TOP500 data |
| `search.lyric` + `lyric` | ✅ live-verified | KRC word-level timing decoded |
| Remaining 267 endpoints | 🟡 protocol-aligned | parameters and protocols ported bit-for-bit (60+ special families by hand), not individually live-verified |

Run the live smoke any time:

```bash
cargo run -p kugouapi --example smoke_search -- 周杰伦
cargo run -p kugouapi --example register_device
```

> The live `register.dev` endpoint currently answers with `error_code=20006`
> (signature rejected — the same behavior as the TypeScript implementation;
> the upstream protocol appears to require additional risk-control context).
> The full crypto pipeline remains covered by offline tests.

## Development

```bash
cargo fmt --all -- --check        # formatting
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace     # unit + integration + coverage gates
cargo test --workspace --doc      # doctests
cargo run -p kugouapi-cli -- endpoints --markdown > docs/ENDPOINTS.md
```

### Quality gates (CI-enforced)

- **fmt**: `cargo fmt --check` across the workspace
- **clippy**: `-D warnings` with `pedantic` lints
- **tests**: 106 tests — crypto/signature vectors computed externally with GNU
  coreutils `md5sum` (mirroring the TS vectors), mock-transport integration
  tests, and a catalog-coverage test embedding the official 271-id list so
  upstream additions fail the build
- **doctests** for every public example
- **docs**: `cargo doc` with `rustdoc::all`

The test suite never touches the network: the HTTP layer is exercised through
a mock `Transport`, and crypto layers are pinned against externally computed
vectors.

## Differences from the TypeScript library

- **Typed parameters**: Rust requires typed params per endpoint
  (`SearchSongParams::new("kw").page(2)`); the dynamic `call(id, json)` path
  deserializes into the same structs, so loose JSON payloads behave the same.
- **Binary payloads** (`user.cloud.upload`, `playlist.pic.upload`,
  `audio.match`, `user.update.avatar`, chat images) are passed as **base64
  strings** in the dynamic JSON surface — `serde_json` has no byte type.
- `login.device.kick` follows the original reference semantics (raw no-padding
  RSA over the zero-padded token with the lite public key); the TS variant
  contains a bug that JSON-stringifies the padded buffer.
- The tracker-style envelope (`errcode` / `status`) treats `status: 200` as
  success in addition to `status: 1` — several services answer with
  HTTP-style `{status: 200, errcode: 200, errmsg: "OK"}` success envelopes.
- The rate limiter waits for a token instead of exposing a bounded queue;
  caller timeouts play the role of the reference `queueTimeoutMs`.

## License

MIT
