/**
 * Maps raw api-enhanced HTTP responses to NonsPlayer frontend types.
 *
 * All IDs are prefixed with "netease_" + type tag to avoid collisions
 * across adapters.  The prefix convention matches what the old Rust
 * mapper produced.
 */

import type { Song } from "$lib/types/song";
import type { Album } from "$lib/types/album";
import type { Artist } from "$lib/types/artist";
import type { Playlist } from "$lib/types/playlist";
import type { Account } from "$lib/types/account";
import type { SearchResult } from "$lib/types/adapter";
import type { TopPlaylistGroup } from "$lib/types/top-playlist";
import type { PlaylistCategory } from "$lib/types/playlist-category";

// ── ID helpers ────────────────────────────────────────────────────────

const SONG_PREFIX = "netease_song_";
const ALBUM_PREFIX = "netease_album_";
const ARTIST_PREFIX = "netease_artist_";
const PLAYLIST_PREFIX = "netease_playlist_";
const USER_PREFIX = "netease_user_";

function songId(raw: number | string): string {
  return `${SONG_PREFIX}${raw}`;
}
function albumId(raw: number | string): string {
  return `${ALBUM_PREFIX}${raw}`;
}
function artistId(raw: number | string): string {
  return `${ARTIST_PREFIX}${raw}`;
}
function playlistId(raw: number | string): string {
  return `${PLAYLIST_PREFIX}${raw}`;
}
function userId(raw: number | string): string {
  return `${USER_PREFIX}${raw}`;
}

function avatarUrl(url: string | undefined | null): string {
  return url ?? "";
}
function smallAvatar(url: string | undefined | null): string {
  const u = url ?? "";
  return u ? `${u}?param=50y50` : "";
}
function middleAvatar(url: string | undefined | null): string {
  const u = url ?? "";
  return u ? `${u}?param=200y200` : "";
}

// ── Empty / placeholder factories ─────────────────────────────────────

function emptySong(): Song {
  return {
    id: "",
    md5: "",
    name: "",
    shareUrl: "",
    avatarUrl: "",
    smallAvatarUrl: "",
    middleAvatarUrl: "",
    album: emptyAlbum(),
    artists: [],
    isEmpty: true,
    duration: 0,
    url: "",
    lyric: null,
    available: false,
    isLiked: false,
    trans: null,
    albumName: "",
    artistsName: "",
    durationText: "0:00",
    adapterSlug: "netease",
  };
}

function emptyAlbum(): Album {
  return {
    id: "",
    md5: "",
    name: "",
    shareUrl: "",
    avatarUrl: "",
    smallAvatarUrl: "",
    middleAvatarUrl: "",
    createDate: "",
    description: "",
    songs: [],
    artists: [],
    artistsName: "",
    collectionCount: 0,
    trackCount: 0,
    adapterSlug: "netease",
  };
}

function emptyArtist(): Artist {
  return {
    id: "",
    md5: "",
    name: "",
    shareUrl: "",
    avatarUrl: "",
    smallAvatarUrl: "",
    middleAvatarUrl: "",
    description: "",
    songs: [],
    musicCount: 0,
    trans: "",
    adapterSlug: "netease",
  };
}

function emptyPlaylist(): Playlist {
  return {
    id: "",
    md5: "",
    name: "",
    shareUrl: "",
    avatarUrl: "",
    smallAvatarUrl: "",
    middleAvatarUrl: "",
    title: "",
    createTime: "",
    creator: "",
    description: "",
    musicTrackIds: [],
    tags: [],
    musics: [],
    isInitialized: false,
    playCount: 0,
    musicsCount: 0,
    adapterSlug: "netease",
  };
}

// ── Duration ──────────────────────────────────────────────────────────

function formatDuration(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

// ── Song mapping ──────────────────────────────────────────────────────

/** Map a raw song item from api-enhanced (/song/detail, /cloudsearch, etc.) */
export function mapSong(raw: any): Song {
  if (!raw) return emptySong();
  const a = raw.al ?? raw.album ?? {};
  const ars: any[] = raw.ar ?? raw.artists ?? [];
  const artists: Artist[] = ars.map((ar: any) => mapArtist(ar));
  const dur = raw.dt ?? raw.duration ?? 0;
  return {
    id: songId(raw.id),
    md5: String(raw.id ?? ""),
    name: raw.name ?? "",
    shareUrl: "",
    avatarUrl: avatarUrl(a.picUrl ?? a.pic_url),
    smallAvatarUrl: smallAvatar(a.picUrl ?? a.pic_url),
    middleAvatarUrl: middleAvatar(a.picUrl ?? a.pic_url),
    album: {
      id: albumId(a.id ?? ""),
      md5: String(a.id ?? ""),
      name: a.name ?? "",
      shareUrl: "",
      avatarUrl: avatarUrl(a.picUrl ?? a.pic_url),
      smallAvatarUrl: smallAvatar(a.picUrl ?? a.pic_url),
      middleAvatarUrl: middleAvatar(a.picUrl ?? a.pic_url),
      createDate: "",
      description: "",
      songs: [],
      artists: [...artists],
      artistsName: artists.map((ar) => ar.name).join("/"),
      collectionCount: 0,
      trackCount: 0,
      adapterSlug: "netease",
    },
    artists,
    isEmpty: false,
    duration: typeof dur === "number" ? dur / 1000 : 0,
    url: "",
    lyric: null,
    available: true,
    isLiked: false,
    trans: raw.tns?.[0] ?? null,
    albumName: a.name ?? "",
    artistsName: artists.map((ar) => ar.name).join("/"),
    durationText: formatDuration(typeof dur === "number" ? dur : 0),
    adapterSlug: "netease",
  };
}

// ── Album mapping ─────────────────────────────────────────────────────

export function mapAlbum(raw: any): Album {
  if (!raw) return emptyAlbum();
  const ars: any[] = raw.artists ?? raw.ar ?? [];
  const artists: Artist[] = ars.map((ar: any) => mapArtist(ar));
  const songs: Song[] = (raw.songs ?? []).map(mapSong);
  const coverUrl = raw.picUrl ?? raw.pic_url ?? raw.coverImgUrl ?? "";
  return {
    id: albumId(raw.id),
    md5: String(raw.id ?? ""),
    name: raw.name ?? "",
    shareUrl: "",
    avatarUrl: avatarUrl(coverUrl),
    smallAvatarUrl: smallAvatar(coverUrl),
    middleAvatarUrl: middleAvatar(coverUrl),
    createDate: raw.publishTime ? String(raw.publishTime) : (raw.createDate ?? ""),
    description: raw.description ?? raw.desc ?? "",
    songs,
    artists,
    artistsName: artists.map((a) => a.name).join("/"),
    collectionCount: raw.info?.commentCount ?? raw.collectionCount ?? 0,
    trackCount: raw.size ?? songs.length,
    adapterSlug: "netease",
  };
}

// ── Artist mapping ────────────────────────────────────────────────────

export function mapArtist(raw: any): Artist {
  if (!raw) return emptyArtist();
  const picUrl = raw.picUrl ?? raw.pic_url ?? raw.cover ?? raw.img1v1Url ?? "";
  return {
    id: artistId(raw.id),
    md5: String(raw.id ?? ""),
    name: raw.name ?? "",
    shareUrl: "",
    avatarUrl: avatarUrl(picUrl),
    smallAvatarUrl: smallAvatar(picUrl),
    middleAvatarUrl: middleAvatar(picUrl),
    description: raw.briefDesc ?? raw.description ?? "",
    songs: [],
    musicCount: raw.musicSize ?? raw.albumSize ?? 0,
    trans: raw.trans ?? raw.tns?.[0] ?? "",
    adapterSlug: "netease",
  };
}

// ── Playlist mapping ──────────────────────────────────────────────────

export function mapPlaylist(raw: any): Playlist {
  if (!raw) return emptyPlaylist();
  const tracks: any[] = raw.tracks ?? [];
  const trackIds: string[] = (raw.trackIds ?? []).map((t: any) =>
    String(t.id ?? t),
  );
  const songs: Song[] = tracks.map(mapSong);
  const cover = raw.coverImgUrl ?? raw.picUrl ?? "";
  return {
    id: playlistId(raw.id),
    md5: String(raw.id ?? ""),
    name: raw.name ?? "",
    shareUrl: raw.shareUrl ?? "",
    avatarUrl: avatarUrl(cover),
    smallAvatarUrl: smallAvatar(cover),
    middleAvatarUrl: middleAvatar(cover),
    title: raw.name ?? "",
    createTime: raw.createTime ? String(raw.createTime) : "",
    creator: raw.creator?.nickname ?? raw.creator ?? "",
    description: raw.description ?? raw.desc ?? "",
    musicTrackIds: trackIds,
    tags: raw.tags ?? [],
    musics: songs,
    isInitialized: songs.length > 0,
    playCount: raw.playCount ?? raw.playcount ?? 0,
    musicsCount: trackIds.length || songs.length,
    adapterSlug: "netease",
  };
}

// ── Account mapping ───────────────────────────────────────────────────

export function mapAccount(profile: any, uid: number | string, cookie: string): Account {
  return {
    id: userId(uid),
    md5: String(uid),
    name: profile?.nickname ?? "网易云用户",
    token: cookie,
    avatarUrl: avatarUrl(profile?.avatarUrl),
    isLoggedIn: true,
    key: String(uid),
  };
}

// ── Search result mapping ─────────────────────────────────────────────

export function mapSearchResult(raw: any): SearchResult {
  const result = raw.result ?? raw;
  return {
    songs: (result.songs ?? []).map(mapSong),
    albums: (result.albums ?? []).map(mapAlbum),
    artists: (result.artists ?? []).map(mapArtist),
    playlists: (result.playlists ?? []).map(mapPlaylist),
  };
}

// ── Toplist / playlist square ─────────────────────────────────────────

export function mapTopPlaylistGroups(list: any[]): TopPlaylistGroup[] {
  if (!Array.isArray(list)) return [];

  const official: Playlist[] = [];
  const featured: Playlist[] = [];

  for (const item of list) {
    const pl = mapPlaylist(item);
    // Items with ToplistType are official rankings
    if (item.ToplistType || item.toplistType) {
      official.push(pl);
    } else {
      featured.push(pl);
    }
  }

  const groups: TopPlaylistGroup[] = [];
  if (official.length > 0) groups.push({ name: "Official", playlists: official });
  if (featured.length > 0) groups.push({ name: "Featured", playlists: featured });
  return groups;
}

export function mapPlaylistCategories(raw: any): PlaylistCategory[] {
  const categories = raw.categories ?? {};
  const sub: any[] = raw.sub ?? [];
  const result: PlaylistCategory[] = [];

  for (const [key, name] of Object.entries(categories)) {
    const tags = sub
      .filter((c: any) => String(c.category) === key)
      .map((c: any) => c.name);
    if (tags.length > 0) {
      result.push({ name: name as string, tags });
    }
  }
  return result;
}

export function mapPlaylistsFromList(list: any[]): Playlist[] {
  if (!Array.isArray(list)) return [];
  return list.map(mapPlaylist);
}
