use serde::{Deserialize, Serialize};

use super::{album::Album, artist::Artist};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    pub md5: String,
    pub name: String,
    pub share_url: String,
    pub avatar_url: String,
    pub small_avatar_url: String,
    pub middle_avatar_url: String,
    pub album: Album,
    pub artists: Vec<Artist>,
    pub is_empty: bool,
    pub duration: f64,
    pub url: String,
    pub available: bool,
    pub is_liked: bool,
    pub trans: Option<String>,
    pub album_name: String,
    pub artists_name: String,
    pub duration_text: String,
    pub adapter_slug: String,
}

impl Song {
    pub fn empty() -> Self {
        Self {
            id: String::new(),
            md5: String::new(),
            name: String::new(),
            share_url: String::new(),
            avatar_url: String::new(),
            small_avatar_url: String::new(),
            middle_avatar_url: String::new(),
            album: Album::empty(),
            artists: vec![],
            is_empty: true,
            duration: 0.0,
            url: String::new(),
            available: false,
            is_liked: false,
            trans: None,
            album_name: String::new(),
            artists_name: String::new(),
            duration_text: String::new(),
            adapter_slug: String::new(),
        }
    }
}
