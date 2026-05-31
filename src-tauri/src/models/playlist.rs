use serde::{Deserialize, Serialize};

use super::song::Song;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub md5: String,
    pub name: String,
    pub share_url: String,
    pub avatar_url: String,
    pub small_avatar_url: String,
    pub middle_avatar_url: String,
    pub title: String,
    pub create_time: String,
    pub creator: String,
    pub description: String,
    pub music_track_ids: Vec<String>,
    pub tags: Vec<String>,
    pub musics: Vec<Song>,
    pub is_initialized: bool,
    pub adapter_slug: String,
    pub play_count: u32,
    pub musics_count: u32,
}

impl Playlist {
    pub fn empty() -> Self {
        Self {
            id: String::new(),
            md5: String::new(),
            name: String::new(),
            share_url: String::new(),
            avatar_url: String::new(),
            small_avatar_url: String::new(),
            middle_avatar_url: String::new(),
            title: String::new(),
            create_time: String::new(),
            creator: String::new(),
            description: String::new(),
            music_track_ids: vec![],
            tags: vec![],
            musics: vec![],
            is_initialized: false,
            adapter_slug: String::new(),
            play_count: 0,
            musics_count: 0,
        }
    }
}
