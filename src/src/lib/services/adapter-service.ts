import { invoke } from "@tauri-apps/api/core";
import type {
  AdapterConfig,
  AdapterMetadata,
  LoginStatus,
  SearchResult,
} from "$lib/types/adapter";
import type { Song } from "$lib/types/song";
import type { Album } from "$lib/types/album";
import type { Artist } from "$lib/types/artist";
import type { Playlist } from "$lib/types/playlist";
import type { Account } from "$lib/types/account";
import type { TopPlaylistGroup } from "$lib/types/top-playlist";
import type { PlaylistCategory } from "$lib/types/playlist-category";

// -- Adapter management --

export function initAdapters(config: AdapterConfig): Promise<AdapterMetadata[]> {
  return invoke("init_adapters", { config });
}

export function scanLocal(musicDirs: string[]): Promise<AdapterMetadata[]> {
  return invoke("scan_local", { musicDirs });
}

export function listAdapters(): Promise<AdapterMetadata[]> {
  return invoke("list_adapters");
}

// -- Music --

export function getSong(adapter: string, id: string): Promise<Song> {
  return invoke("get_song", { adapter, id });
}

export function getSongs(adapter: string, ids: string[]): Promise<Song[]> {
  return invoke("get_songs", { adapter, ids });
}

export function getSongUrl(adapter: string, id: string): Promise<string> {
  return invoke("get_song_url", { adapter, id });
}

export function getLyric(adapter: string, id: string): Promise<string> {
  return invoke("get_lyric", { adapter, id });
}

export function toggleLike(adapter: string, id: string, like: boolean): Promise<boolean> {
  return invoke("toggle_like", { adapter, id, like });
}

// -- Album / Artist / Playlist --

export function getAlbum(adapter: string, id: string): Promise<Album> {
  return invoke("get_album", { adapter, id });
}

export function getArtist(adapter: string, id: string): Promise<Artist> {
  return invoke("get_artist", { adapter, id });
}

export function getPlaylist(adapter: string, id: string): Promise<Playlist> {
  return invoke("get_playlist", { adapter, id });
}

// -- Search --

export function search(adapter: string, keyword: string): Promise<SearchResult> {
  return invoke("search", { adapter, keyword });
}

// -- Account --

export function loginQrUrl(adapter: string): Promise<[string, string]> {
  return invoke("login_qr_url", { adapter });
}

export function checkLogin(adapter: string, key: string): Promise<LoginStatus> {
  return invoke("check_login", { adapter, key });
}

export function getAccount(adapter: string): Promise<Account> {
  return invoke("get_account", { adapter });
}

export function getUserPlaylists(adapter: string): Promise<Playlist[]> {
  return invoke("get_user_playlists", { adapter });
}

export function getFavoritePlaylist(adapter: string): Promise<Playlist | null> {
  return invoke("get_favorite_playlist", { adapter });
}

// -- Recommend --

export function getRecommendedPlaylists(
  adapter: string,
  count: number,
): Promise<Playlist[]> {
  return invoke("get_recommended_playlists", { adapter, count });
}

export function getDailyRecommended(adapter: string): Promise<Song[]> {
  return invoke("get_daily_recommended", { adapter });
}

// -- Discover --

export function getTopPlaylists(adapter: string): Promise<TopPlaylistGroup[]> {
  return invoke("get_top_playlists", { adapter });
}

export function getPlaylistCats(adapter: string): Promise<PlaylistCategory[]> {
  return invoke("get_playlist_cats", { adapter });
}

export function getPlaylistSquare(
  adapter: string,
  cat: string,
  order: string,
  limit: number,
  offset: number,
  highQuality: boolean,
): Promise<[Playlist[], number]> {
  return invoke("get_playlist_square", {
    adapter,
    cat,
    order,
    limit,
    offset,
    highQuality,
  });
}

