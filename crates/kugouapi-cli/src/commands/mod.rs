//! CLI subcommand implementations.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use kugouapi::catalog::{endpoint, endpoint_ids};
use kugouapi::{KuGouClient, RegisterDeviceParams, SearchSongParams, SongUrlParams};

pub async fn search(client: &KuGouClient, keyword: &str, page: u64, size: u64) -> Result<()> {
    let data = client
        .search()
        .song(
            SearchSongParams::new(keyword)
                .page(page as i64)
                .page_size(size as i64),
        )
        .await
        .context("search failed")?;
    println!("total: {}", data.total);
    for (index, song) in data.lists.iter().enumerate() {
        println!(
            "{:>3}. {} [{}] hash={} album={:?} dur={:?}s",
            index + 1,
            song.file_name,
            song.extra
                .get("Privilege")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            song.file_hash,
            song.album_id,
            song.duration,
        );
    }
    Ok(())
}

pub async fn play(client: &KuGouClient, hash: &str, quality: &str) -> Result<()> {
    let quality: kugouapi::Quality = quality
        .parse()
        .with_context(|| format!("unknown quality `{quality}`"))?;
    let data = client
        .song()
        .url(SongUrlParams::new(hash).quality(quality))
        .await
        .context("resolving the playback url failed")?;
    if let Some(url) = data.url.as_ref().and_then(|urls| urls.first()) {
        println!("{url}");
        Ok(())
    } else {
        eprintln!("no direct url (privilege required); details:");
        println!("{}", serde_json::to_string_pretty(&data)?);
        Ok(())
    }
}

pub async fn lyric(client: &KuGouClient, id: i64, accesskey: &str, decode: bool) -> Result<()> {
    let data = client
        .call(
            "lyric",
            json!({ "id": id, "accesskey": accesskey, "decode": decode, "fmt": "krc" }),
        )
        .await
        .context("downloading the lyric failed")?;
    if let Some(text) = data.get("decodeContent").and_then(Value::as_str) {
        println!("{text}");
        Ok(())
    } else {
        println!("{}", serde_json::to_string_pretty(&data)?);
        Ok(())
    }
}

pub async fn lyric_search(client: &KuGouClient, keyword: &str) -> Result<()> {
    let data = client
        .call("search.lyric", json!({ "keywords": keyword, "man": "yes" }))
        .await
        .context("lyric search failed")?;
    let candidates = data
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if candidates.is_empty() {
        println!("no lyric candidates");
        return Ok(());
    }
    for candidate in &candidates {
        println!(
            "{:?} {:?}  id={:?} accesskey={:?}",
            candidate.get("song").and_then(Value::as_str).unwrap_or("?"),
            candidate
                .get("singer")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            candidate.get("id"),
            candidate.get("accesskey"),
        );
    }
    Ok(())
}

pub async fn rank(client: &KuGouClient, rankid: Option<i64>) -> Result<()> {
    let data = match rankid {
        Some(rankid) => client
            .call("rank.info", json!({ "rankid": rankid }))
            .await
            .context("rank info failed")?,
        None => client
            .call("rank.list", json!({}))
            .await
            .context("rank list failed")?,
    };
    println!("{}", serde_json::to_string_pretty(&data)?);
    Ok(())
}

pub async fn register(client: &KuGouClient) -> Result<Value> {
    let data = client
        .register_device(RegisterDeviceParams::new())
        .await
        .context("device registration failed")?;
    Ok(serde_json::to_value(&data)?)
}

pub async fn login(
    client: &KuGouClient,
    username: Option<String>,
    password: Option<String>,
    mobile: Option<String>,
    code: Option<String>,
) -> Result<Value> {
    let data = match (username, password, mobile, code) {
        (Some(username), Some(password), _, _) => client
            .call(
                "login",
                json!({ "username": username, "password": password }),
            )
            .await
            .context("password login failed")?,
        (_, _, Some(mobile), Some(code)) => client
            .call("login.cellphone", json!({ "mobile": mobile, "code": code }))
            .await
            .context("SMS login failed")?,
        _ => {
            bail!("pass --username/--password or --mobile/--code")
        }
    };
    Ok(data)
}

pub fn endpoints(filter: Option<&str>, markdown: bool) {
    if markdown {
        println!("# Endpoint catalog (ENDPOINTS)");
        println!();
        println!(
            "{} endpoints in total, covering every module of the reference",
            endpoint_ids().len()
        );
        println!("implementation (KuGouMusicApi main\\@5d6c9f7).");
        println!();
        println!("Regenerate with `cargo run -p kugouapi-cli -- endpoints --markdown`.");
        println!();
        println!("| endpoint id | module | method | path | origin / router | auth | note |");
        println!("|---|---|---|---|---|---|---|");
        for id in endpoint_ids() {
            let Some(entry) = endpoint(id) else {
                continue;
            };
            let spec = entry.spec;
            let extra = match (spec.origin, spec.router) {
                (Some(origin), Some(router)) => format!("@{origin} router:{router}"),
                (Some(origin), None) => format!("@{origin}"),
                (None, Some(router)) => format!("router:{router}"),
                (None, None) => String::new(),
            };
            let note = spec.note.unwrap_or("");
            println!(
                "| `{}` | {} | {} | {} | {} | {} | {} |",
                spec.id,
                spec.module,
                spec.method.as_str(),
                spec.path,
                extra,
                if spec.auth { "yes" } else { "" },
                note,
            );
        }
        return;
    }

    for id in endpoint_ids() {
        if let Some(filter) = filter
            && !id.contains(filter)
        {
            continue;
        }
        let extra = endpoint(id)
            .map(|entry| {
                let spec = entry.spec;
                format!(
                    "{} {}{}{}",
                    spec.method.as_str(),
                    spec.path,
                    spec.origin.map(|o| format!(" @{o}")).unwrap_or_default(),
                    spec.router
                        .map(|r| format!(" router:{r}"))
                        .unwrap_or_default(),
                )
            })
            .unwrap_or_default();
        println!("{id:<44} {extra}");
    }
}
