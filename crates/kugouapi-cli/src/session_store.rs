//! Session persistence for the CLI (`~/.kugou/session.json`).

use std::path::PathBuf;

use anyhow::{Context, Result};
use kugouapi::{KuGouClient, SessionSnapshot};

/// The session file location.
pub fn session_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".kugou")
        .join("session.json")
}

/// Load the saved session, if any.
pub fn load() -> Result<Option<SessionSnapshot>> {
    let path = session_path();
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text)
        .map(Some)
        .with_context(|| format!("parsing {}", path.display()))
}

/// Persist the client's current session.
pub fn save_from_client(client: &KuGouClient) -> Result<()> {
    if let Some(session) = client.session() {
        save(&session)
    } else {
        Ok(())
    }
}

/// Write a session to disk.
pub fn save(session: &SessionSnapshot) -> Result<()> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(session)?;
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
