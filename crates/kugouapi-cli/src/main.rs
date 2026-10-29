//! `kugou` — the command-line client for the `kugouapi` crate.

mod commands;
mod session_store;

use anyhow::{Context, Result};
use clap::Parser;

use kugouapi::{KuGouClient, Platform};

use crate::session_store::session_path;

#[derive(Debug, Parser)]
#[command(
    name = "kugou",
    version,
    about = "Unofficial KuGou music API client (271 endpoints)",
    after_help = "Sessions persist across invocations in ~/.kugou/session.json."
)]
struct Cli {
    /// Use the concept-edition (lite) platform preset.
    #[arg(long, global = true)]
    lite: bool,

    /// Gateway base URL override.
    #[arg(long, global = true)]
    base_url: Option<String>,

    /// Per-attempt timeout in seconds (default 10).
    #[arg(long, global = true, default_value_t = 10)]
    timeout: u64,

    /// GET retry count (default 1).
    #[arg(long, global = true, default_value_t = 1)]
    retries: u32,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Search songs.
    Search {
        /// Search keyword.
        keyword: String,
        /// Page number.
        #[arg(long, default_value_t = 1)]
        page: u64,
        /// Page size.
        #[arg(long, default_value_t = 30)]
        size: u64,
    },
    /// Resolve a playback URL for a song hash.
    Play {
        /// Song hash.
        hash: String,
        /// Quality: 128 / 320 / flac / high / super / `viper_atmos` / …
        #[arg(short, long, default_value = "128")]
        quality: String,
    },
    /// Download the lyric for a song (use `lyric-search` to find ids).
    Lyric {
        /// Lyric id.
        id: i64,
        /// Lyric access key.
        accesskey: String,
        /// Decode KRC / base64 content.
        #[arg(long)]
        decode: bool,
    },
    /// Search lyrics by keyword and print candidates.
    LyricSearch { keyword: String },
    /// Show rank lists (or one rank's info).
    Rank {
        /// Rank id (omit for the list of ranks).
        rankid: Option<i64>,
    },
    /// Register the device and persist the session.
    Register,
    /// Login flows: password, SMS code, or token refresh.
    Login {
        /// Account username (password login).
        #[arg(long, requires = "password")]
        username: Option<String>,
        /// Account password (password login).
        #[arg(long)]
        password: Option<String>,
        /// Mobile number (SMS login).
        #[arg(long)]
        mobile: Option<String>,
        /// SMS code (SMS login).
        #[arg(long)]
        code: Option<String>,
    },
    /// Call any of the 271 catalog endpoints with dynamic JSON params.
    Call {
        /// Endpoint id, e.g. `rank.list` (see `endpoints`).
        id: String,
        /// JSON params (`{}` when omitted).
        params: Option<String>,
    },
    /// List catalog endpoints (optionally filtered / as markdown).
    Endpoints {
        /// Substring filter.
        #[arg(long)]
        filter: Option<String>,
        /// Emit the full markdown catalog table.
        #[arg(long)]
        markdown: bool,
    },
    /// Show the persisted session.
    Session,
}

fn build_client(cli: &Cli) -> Result<KuGouClient> {
    let mut builder = KuGouClient::builder()
        .timeout(std::time::Duration::from_secs(cli.timeout))
        .retries(cli.retries);
    if cli.lite {
        builder = builder.platform(Platform::Lite);
    }
    if let Some(base_url) = &cli.base_url {
        builder = builder.base_url(base_url.clone());
    }
    if let Some(session) = session_store::load().context("loading the saved session")? {
        builder = builder.session(session);
    }
    builder.build().context("building the client")
}

fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = build_client(&cli)?;

    match cli.command {
        Command::Search {
            keyword,
            page,
            size,
        } => commands::search(&client, &keyword, page, size).await?,
        Command::Play { hash, quality } => commands::play(&client, &hash, &quality).await?,
        Command::Lyric {
            id,
            accesskey,
            decode,
        } => commands::lyric(&client, id, &accesskey, decode).await?,
        Command::LyricSearch { keyword } => commands::lyric_search(&client, &keyword).await?,
        Command::Rank { rankid } => commands::rank(&client, rankid).await?,
        Command::Register => {
            let data = commands::register(&client).await?;
            session_store::save_from_client(&client)?;
            print_json(&data);
        }
        Command::Login {
            username,
            password,
            mobile,
            code,
        } => {
            let data = commands::login(&client, username, password, mobile, code).await?;
            session_store::save_from_client(&client)?;
            print_json(&data);
        }
        Command::Call { id, params } => {
            let params: serde_json::Value = match params {
                Some(text) => serde_json::from_str(&text)
                    .with_context(|| format!("parsing params for `{id}`"))?,
                None => serde_json::json!({}),
            };
            let data = client.call(&id, params).await?;
            print_json(&data);
        }
        Command::Endpoints { filter, markdown } => {
            commands::endpoints(filter.as_deref(), markdown);
        }
        Command::Session => match session_store::load()? {
            Some(session) => print_json(&serde_json::to_value(&session)?),
            None => {
                println!(
                    "no session saved yet ({} does not exist)",
                    session_path().display()
                );
            }
        },
    }
    Ok(())
}
