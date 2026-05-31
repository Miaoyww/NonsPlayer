use serde::{Deserialize, Serialize};

use super::{artist::Artist, song::Song};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub md5: String,
    pub name: String,
    pub share_url: String,
    pub avatar_url: String,
    pub small_avatar_url: String,
    pub middle_avatar_url: String,
    pub create_date: String,
    pub description: String,
    pub songs: Vec<Song>,
    pub artists: Vec<Artist>,
    pub artists_name: String,
    pub adapter_slug: String,
    pub collection_count: u32,
    pub track_count: u32,
}

impl Album {
    pub fn empty() -> Self {
        Self {
            id: String::new(),
            md5: String::new(),
            name: String::new(),
            share_url: String::new(),
            avatar_url: String::new(),
            small_avatar_url: String::new(),
            middle_avatar_url: String::new(),
            create_date: String::new(),
            description: String::new(),
            songs: vec![],
            artists: vec![],
            artists_name: String::new(),
            adapter_slug: String::new(),
            collection_count: 0,
            track_count: 0,
        }
    }
}
