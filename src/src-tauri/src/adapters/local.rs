use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use lofty::prelude::*;
use lofty::read_from_path;
use walkdir::WalkDir;

use super::{Adapter, AdapterMetadata, CapabilityType, SearchResult};
use crate::error::{Error, Result};
use crate::models::{album::Album, artist::Artist, playlist::Playlist, song::Song};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "m4a", "ogg", "wma", "ape", "aac", "aiff", "opus",
];

pub struct LocalAdapter {
    metadata: AdapterMetadata,
    songs: HashMap<String, Song>,
}

impl LocalAdapter {
    pub fn new(music_dirs: Vec<PathBuf>) -> Result<Self> {
        let metadata = AdapterMetadata {
            slug: "local".into(),
            platform: "local".into(),
            display_platform: "本地音乐".into(),
            author: "NonsPlayer".into(),
            description: "播放本地音乐文件".into(),
            version: "0.1.0".into(),
        };

        let mut adapter = Self {
            metadata,
            songs: HashMap::new(),
        };

        adapter.scan(&music_dirs)?;
        Ok(adapter)
    }

    pub fn reload(&mut self, music_dirs: &[PathBuf]) -> Result<()> {
        self.scan(music_dirs)
    }

    fn scan(&mut self, dirs: &[PathBuf]) -> Result<()> {
        self.songs.clear();

        for dir in dirs {
            if !dir.is_dir() {
                continue;
            }

            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();

                if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                    continue;
                }

                let song = Self::parse_song(path);
                match song {
                    Ok(s) => {
                        self.songs.insert(s.id.clone(), s);
                    }
                    Err(e) => {
                        eprintln!("Skipping {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_song(path: &std::path::Path) -> Result<Song> {
        let tagged_file =
            read_from_path(path).map_err(|e| Error::AudioParse(e.to_string()))?;

        let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
        let props = tagged_file.properties();
        let duration = props.duration().as_secs_f64();

        let name = tag
            .and_then(|t| t.title())
            .as_deref()
            .unwrap_or("")
            .to_string();
        let name = if name.is_empty() {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        } else {
            name
        };

        let artist_name = tag
            .and_then(|t| t.artist())
            .as_deref()
            .unwrap_or("未知艺术家")
            .to_string();

        let album_name = tag
            .and_then(|t| t.album())
            .as_deref()
            .unwrap_or("未知专辑")
            .to_string();

        let id = path.to_string_lossy().to_string();
        let minutes = (duration / 60.0) as u64;
        let seconds = (duration % 60.0) as u64;
        let duration_text = format!("{}:{:02}", minutes, seconds);

        let artist = Artist {
            id: format!("local_artist_{}", artist_name),
            name: artist_name.clone(),
            ..Artist::empty()
        };

        let album = Album {
            id: format!("local_album_{}", album_name),
            name: album_name.clone(),
            artists: vec![artist.clone()],
            artists_name: artist_name.clone(),
            ..Album::empty()
        };

        Ok(Song {
            id: id.clone(),
            name: name.clone(),
            album: album.clone(),
            artists: vec![artist],
            duration,
            duration_text: duration_text.clone(),
            available: true,
            url: id, // local path as playable URL
            album_name,
            artists_name: artist_name,
            adapter_slug: "local".into(),
            ..Song::empty()
        })
    }
}

#[async_trait]
impl Adapter for LocalAdapter {
    fn metadata(&self) -> &AdapterMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> Vec<CapabilityType> {
        vec![CapabilityType::Music, CapabilityType::Search]
    }

    async fn get_song(&self, id: &str) -> Result<Song> {
        self.songs
            .get(id)
            .cloned()
            .ok_or_else(|| Error::SongNotFound(id.into()))
    }

    async fn get_songs(&self, ids: &[String]) -> Result<Vec<Song>> {
        let mut songs = Vec::with_capacity(ids.len());
        for id in ids {
            songs.push(self.get_song(id).await?);
        }
        Ok(songs)
    }

    async fn get_song_url(&self, id: &str) -> Result<String> {
        // Local files: URL is the file path
        if self.songs.contains_key(id) {
            Ok(format!("file://{}", id))
        } else {
            Err(Error::SongNotFound(id.into()))
        }
    }

    async fn get_lyric(&self, _id: &str) -> Result<String> {
        // TODO: try to read .lrc file with same name
        Ok(String::new())
    }

    async fn toggle_like(&self, _id: &str, _like: bool) -> Result<bool> {
        Err(Error::NotImplemented("local files don't support like".into()))
    }

    async fn get_album(&self, _id: &str) -> Result<Album> {
        Err(Error::NotImplemented("local albums".into()))
    }

    async fn get_artist(&self, _id: &str) -> Result<Artist> {
        Err(Error::NotImplemented("local artists".into()))
    }

    async fn get_playlist(&self, _id: &str) -> Result<Playlist> {
        Err(Error::NotImplemented("local playlists".into()))
    }

    async fn search(&self, keyword: &str) -> Result<SearchResult> {
        let keyword = keyword.to_lowercase();
        let songs: Vec<Song> = self
            .songs
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&keyword)
                    || s.artists_name.to_lowercase().contains(&keyword)
                    || s.album_name.to_lowercase().contains(&keyword)
            })
            .cloned()
            .collect();

        Ok(SearchResult {
            songs,
            albums: vec![],
            artists: vec![],
            playlists: vec![],
        })
    }
}
