//! Special protocol families (login, encrypted POST, cloudlist, comments,
//! listen-together, composite flows).

pub mod account;
pub mod cloud;
pub mod comment;
pub mod listen_together;
pub mod media;
pub mod search;
pub mod song;
pub mod team;
pub mod user;

use super::def::RegisteredEndpoint;

/// All special-family endpoints.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    let mut all = Vec::new();
    all.extend(song::endpoints());
    all.extend(account::endpoints());
    all.extend(cloud::endpoints());
    all.extend(comment::endpoints());
    all.extend(listen_together::endpoints());
    all.extend(user::endpoints());
    all.extend(media::endpoints());
    all.extend(search::endpoints());
    all.extend(team::endpoints());
    all
}
