//! # kugouapi
//!
//! Unofficial [KuGou](https://www.kugou.com/) music API client for Rust.
//!
//! This crate is a Rust rendition of the TypeScript
//! [`kugou-api`](https://github.com/hglcode/kugouapi) design, which itself covers the
//! full module surface of the community reference implementation
//! [KuGouMusicApi](https://github.com/MakcRe/KuGouMusicApi) (main\@5d6c9f7, v1.6.2).
//!
//! ## Highlights
//!
//! - **271-endpoint catalog** — every reference module is callable through a single
//!   consistent dispatch (`[``KuGouClient::call``]`), plus idiomatic typed methods with
//!   strongly-typed parameter structs for compile-time checking.
//! - **Built-in session** — after device registration or login, `dfid` / `mid` /
//!   `token` / `userid` / VIP info are carried automatically and can be exported
//!   and restored across processes.
//! - **Complete protocol coverage** — android / web / register signatures,
//!   trackercdn keys, encrypted POST bodies (session cipher), cloudlist,
//!   the login family (AES + RSA), the comment family, and KRC lyric decoding,
//!   all aligned bit-for-bit with the reference implementation.
//! - **Production HTTP layer** — per-attempt timeouts, idempotent GET retries with
//!   exponential backoff, half-jitter and `Retry-After` support, a sliding-window
//!   circuit breaker and an optional token-bucket rate limiter.
//! - **Injectable** — the HTTP transport, crypto provider and RSA public keys are
//!   all replaceable, which keeps the test suite fully offline.
//!
//! ## Quick start
//!
//! ```no_run
//! use kugouapi::{KuGouClient, SearchSongParams};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), kugouapi::Error> {
//! let client = KuGouClient::builder().build()?;
//!
//! // Typed high-frequency endpoints
//! let result = client
//!     .search()
//!     .song(SearchSongParams::new("周杰伦").page_size(30))
//!     .await?;
//! for song in &result.lists {
//!     let url = client
//!         .song()
//!         .url(kugouapi::SongUrlParams::new(&song.file_hash))
//!         .await?;
//!     if let Some(play_url) = url.url.as_ref().and_then(|u| u.first()) {
//!         println!("{} -> {play_url}", song.file_name);
//!     }
//! }
//!
//! // Catalog dispatch for everything else (271 ids)
//! let rank = client.call("rank.list", serde_json::json!({})).await?;
//! # let _ = rank;
//! # Ok(())
//! # }
//! ```
//!
//! Endpoint ids are derived from the reference module names with underscores
//! converted to dots: `rank_list` → `rank.list`, `song_url_new` → `song.url.new`.
//! The full mapping is listed in `docs/ENDPOINTS.md` and can be regenerated with
//! `cargo run -p kugouapi-cli -- endpoints --markdown`.
//!
//! ## Session management
//!
//! The `KuGou` gateway API does **not** rely on HTTP cookies: identity credentials
//! (`dfid` / `mid` / `token` / `userid`) travel as query parameters and headers and
//! are managed explicitly through [`SessionSnapshot`]. They are written
//! automatically after successful registration / login and can be exported and
//! restored at any time.
//!
//! ```no_run
//! use kugouapi::{KuGouClient, random_device};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), kugouapi::Error> {
//! let client = KuGouClient::builder().device(random_device()).build()?;
//!
//! // Device registration (protocol crypto & public keys are built in)
//! client.account().register_device(Default::default()).await?;
//!
//! // Login endpoints persist credentials on success
//! client.call(
//!     "login.cellphone",
//!     serde_json::json!({ "mobile": "13800000000", "code": "1234" }),
//! ).await?;
//!
//! // Export / restore across processes
//! if let Some(session) = client.session() {
//!     let restored = KuGouClient::builder().session(session).build()?;
//! #   let _ = restored;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Platform presets
//!
//! ```no_run
//! # fn main() -> Result<(), kugouapi::Error> {
//! let lite = kugouapi::KuGouClient::builder()
//!     .platform(kugouapi::Platform::Lite) // appid / clientver / salts switch
//!     .build()?;
//! # let _ = lite;
//! # Ok(())
//! # }
//! ```
//!
//! ## crate layout
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`client`] entry | [`KuGouClient`] builder, base query / headers, `call` / `request` dispatch |
//! | [`catalog`] | endpoint registry (271 entries) and call context |
//! | `api` | typed parameter structs and response models |
//! | `crypto` | protocol primitives: session cipher, AES-hex, RSA, KRC, SSA simulation |
//! | `sign`] | request signature, tracker key, params key, cloud key |
//! | `http`] | transport abstraction, retry, circuit breaker, rate limiting |
//! | `device`] | device info, random generator, mid derivation |
//!
//! ## Offline testing
//!
//! The crate never touches the network in tests: the HTTP layer is exercised
//! through a mock [`http::Transport`] implementation and the crypto / signature
//! layers are pinned against externally computed vectors.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[macro_use]
mod macros;

pub mod api;
pub mod catalog;
pub mod client;
pub mod config;
pub mod crypto;
pub mod device;
pub mod error;
pub mod http;
pub mod js;
pub mod query;
pub mod session;
pub mod sign;
pub mod util;

#[doc(inline)]
pub use client::{KuGouClient, KuGouClientBuilder, RequestOptions};
#[doc(inline)]
pub use config::{DEFAULT_BASE_URL, Platform, REGISTER_BASE_URL};
#[doc(inline)]
pub use device::{DEFAULT_DEVICE, Device, calculate_mid, derive_mid, random_device};
#[doc(inline)]
pub use error::{Error, Result};
#[doc(inline)]
pub use query::{QueryRecord, QueryValue};
#[doc(inline)]
pub use session::SessionSnapshot;

pub use api::*;
pub use catalog::{ENDPOINT_COUNT, endpoint_ids};

/// Convenience re-export of [`serde_json::Value`] for dynamic payloads.
pub type Json = serde_json::Value;
