use serde::{Deserialize, Serialize};

use super::song::Song;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub md5: String,
    pub name: String,
    pub share_url: String,
    pub avatar_url: String,
    pub small_avatar_url: String,
    pub middle_avatar_url: String,
    pub description: String,
    pub songs: Vec<Song>,
    pub adapter_slug: String,
    pub music_count: u32,
    pub trans: String,
}

impl Artist {
    pub fn empty() -> Self {
        Self {
            id: String::new(),
            md5: String::new(),
            name: String::new(),
            share_url: String::new(),
            avatar_url: String::new(),
            small_avatar_url: String::new(),
            middle_avatar_url: String::new(),
            description: String::new(),
            songs: vec![],
            adapter_slug: String::new(),
            music_count: 0,
            trans: String::new(),
        }
    }
}
