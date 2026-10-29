//! Device registration: prints and persists nothing itself — demonstrates the
//! built-in session flow.
//!
//! ```text
//! cargo run -p kugouapi --example register_device
//! ```

#[tokio::main]
async fn main() -> Result<(), kugouapi::Error> {
    let client = kugouapi::KuGouClient::builder()
        .device(kugouapi::random_device())
        .build()?;

    let data = client
        .register_device(kugouapi::RegisterDeviceParams::new())
        .await?;
    println!("registered:");
    println!("  dfid   = {:?}", data.dfid);
    println!("  mid    = {:?}", data.mid);
    println!("  userid = {:?}", data.userid);

    // The client now carries the credentials automatically:
    println!("session (client view):");
    if let Some(session) = client.session() {
        println!("  dfid={}, userid={}", session.dfid, session.userid);
    }

    // A second client can restore the snapshot:
    let restored = kugouapi::KuGouClient::builder()
        .session(client.session().unwrap())
        .build()?;
    println!("restored client dfid={}", restored.session().unwrap().dfid);

    Ok(())
}
