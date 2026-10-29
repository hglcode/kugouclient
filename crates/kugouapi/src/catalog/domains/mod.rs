//! Regular gateway endpoint domains (static metadata + typed projections).

pub mod account;
pub mod album;
pub mod artist;
pub mod comment;
pub mod discover;
pub mod media;
pub mod playlist;
pub mod radio;
pub mod scene;
pub mod search;
pub mod song;
pub mod user;
pub mod verticals;

use super::def::RegisteredEndpoint;

/// All regular-domain endpoints.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    let mut all = Vec::new();
    all.extend(account::endpoints());
    all.extend(search::endpoints());
    all.extend(song::endpoints());
    all.extend(album::endpoints());
    all.extend(artist::endpoints());
    all.extend(comment::endpoints());
    all.extend(media::endpoints());
    all.extend(discover::endpoints());
    all.extend(radio::endpoints());
    all.extend(scene::endpoints());
    all.extend(verticals::endpoints());
    all.extend(user::endpoints());
    all.extend(playlist::endpoints());
    all
}
