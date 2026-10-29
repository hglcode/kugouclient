//! Live smoke test: search + playback URL (mirrors the TS `bun run smoke`).
//!
//! ```text
//! cargo run -p kugouapi --example smoke_search -- 周杰伦
//! ```

#[tokio::main]
async fn main() -> Result<(), kugouapi::Error> {
    let keyword = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "周杰伦".to_owned());

    let client = kugouapi::KuGouClient::builder()
        .device(kugouapi::random_device())
        .build()?;

    // 1. search
    let result = client
        .search()
        .song(kugouapi::SearchSongParams::new(&keyword).page_size(10))
        .await?;
    println!(
        "search `{keyword}`: {} results, showing first 5",
        result.total
    );
    for song in result.lists.iter().take(5) {
        println!("  {} (hash={})", song.file_name, song.file_hash);
    }

    // 2. playback url for the first free song
    if let Some(song) = result.lists.first() {
        match client
            .song()
            .url(kugouapi::SongUrlParams::new(&song.file_hash))
            .await
        {
            Ok(url) => {
                if let Some(direct) = url.url.as_ref().and_then(|u| u.first()) {
                    println!("play url: {direct}");
                } else {
                    println!("no direct url (privilege required): {:?}", url.fail_process);
                }
            }
            Err(err) => println!("play url failed: {err}"),
        }
    }

    // 3. catalog dispatch
    let hot = client.call("search.hot", serde_json::json!({})).await?;
    println!(
        "search.hot returned {} top-level fields",
        hot.as_object().map_or(0, serde_json::Map::len)
    );

    Ok(())
}
