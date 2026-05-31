/**
 * Netease API service — HTTP calls to a locally-running api-enhanced server.
 *
 * All api-enhanced endpoints accept any HTTP method (GET / POST).
 * We use GET with query parameters for simplicity.
 *
 * Cookie handling:
 * - After a successful login (qr/check returns code 803), the response
 *   includes a "cookie" field.  We store it and send it as `?cookie=...`
 *   on every subsequent request.
 * - Anonymous requests (before login) do not need a cookie.
 */

import type { Song } from "$lib/types/song";
import type { Album } from "$lib/types/album";
import type { Artist } from "$lib/types/artist";
import type { Playlist } from "$lib/types/playlist";
import type { Account } from "$lib/types/account";
import type { SearchResult, LoginStatus } from "$lib/types/adapter";
import type { TopPlaylistGroup } from "$lib/types/top-playlist";
import type { PlaylistCategory } from "$lib/types/playlist-category";

import {
  mapSong,
  mapAlbum,
  mapArtist,
  mapPlaylist,
  mapAccount,
  mapSearchResult,
  mapTopPlaylistGroups,
  mapPlaylistCategories,
  mapPlaylistsFromList,
} from "./netease-mapper";

// ── Configuration ──────────────────────────────────────────────────────

/** Base URL of the local api-enhanced server. */
const API_BASE = "http://localhost:3000";

/** Current cookie string (set after login). */
let _cookie = "";

export function getCookie(): string {
  return _cookie;
}

export function setCookie(c: string): void {
  _cookie = c;
}

// ── Low-level fetch ───────────────────────────────────────────────────

async function get(path: string, params: Record<string, string> = {}): Promise<any> {
  const url = new URL(`${API_BASE}${path}`);

  // Always include cookie if we have one
  if (_cookie) {
    url.searchParams.set("cookie", _cookie);
  }
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null && v !== "") {
      url.searchParams.set(k, v);
    }
  }

  const resp = await fetch(url.toString());
  if (!resp.ok) {
    throw new Error(`api-enhanced HTTP ${resp.status}: ${resp.statusText}`);
  }
  return resp.json();
}

// ── Login ─────────────────────────────────────────────────────────────

export interface QrKeyResult {
  unikey: string;
}

export async function loginQrKey(): Promise<QrKeyResult> {
  const data = await get("/login/qr/key");
  // api-enhanced nests inside data
  return data.data ?? data;
}

export async function loginQrCreate(key: string): Promise<{ qrurl: string; qrimg: string }> {
  const data = await get("/login/qr/create", { key, qrimg: "true" });
  const inner = data.data ?? data;
  return { qrurl: inner.qrurl, qrimg: inner.qrimg };
}

/**
 * Poll login status.
 * Returns the raw api-enhanced response so the adapter-service can
 * interpret the code and build a LoginStatus.
 */
export async function loginQrCheck(key: string): Promise<{ code: number; cookie?: string; message?: string }> {
  const data = await get("/login/qr/check", { key });
  // api-enhanced wraps in body for login/qr/check
  const body = data.body ?? data;
  // code may be a number or string
  const code = typeof body.code === "number" ? body.code : Number(body.code ?? -1);

  const result: { code: number; cookie?: string; message?: string } = { code, message: body.message };

  // Login success — save cookie
  if (code === 803 && body.cookie) {
    _cookie = body.cookie;
    result.cookie = body.cookie;
  }

  return result;
}

export async function loginRefresh(): Promise<boolean> {
  const data = await get("/login/refresh");
  if (data.code === 200 && data.cookie) {
    _cookie = data.cookie;
    return true;
  }
  return false;
}

// ── Account ───────────────────────────────────────────────────────────

export async function getAccount(): Promise<Account> {
  // Require login cookie; otherwise the API returns an anonymous/error response
  if (!_cookie || !_cookie.includes("MUSIC_U")) {
    throw new Error("Not logged in");
  }
  const data = await get("/user/account");
  // api-enhanced forwards Netease's response. code !== 200 means not logged in.
  if (data.code !== 200) {
    throw new Error("Not logged in");
  }
  const profile = data.profile ?? {};
  const uid = data.account?.id ?? data.profile?.userId ?? "";
  if (!uid) {
    throw new Error("Not logged in");
  }
  return mapAccount(profile, uid, _cookie);
}

export async function getUserPlaylists(uid: string): Promise<Playlist[]> {
  // Strip the netease_user_ prefix for the API
  const rawId = uid.replace(/^netease_user_/, "");
  const data = await get("/user/playlist", { uid: rawId, limit: "50", offset: "0" });
  return mapPlaylistsFromList(data.playlist ?? []);
}

export async function getFavoritePlaylist(uid: string): Promise<Playlist | null> {
  const playlists = await getUserPlaylists(uid);
  // Favorite playlist has specialType === 5
  // Since we can't easily check specialType without the raw data,
  // we call the same endpoint and look for the "我喜欢的音乐" playlist
  // In practice api-enhanced returns specialType in raw, but mapper
  // doesn't pass it through. We check by name.
  const fav = playlists.find((p) => p.name === "我喜欢的音乐");
  return fav ?? null;
}

// ── Search ────────────────────────────────────────────────────────────

export async function search(
  keyword: string,
  type: number = 1,
  limit: number = 20,
  offset: number = 0,
): Promise<SearchResult> {
  const data = await get("/cloudsearch", {
    keywords: keyword,
    type: String(type),
    limit: String(limit),
    offset: String(offset),
  });
  return mapSearchResult(data);
}

// ── Song ──────────────────────────────────────────────────────────────

export async function getSong(id: string): Promise<Song> {
  const rawId = id.replace(/^netease_song_/, "");
  const data = await get("/song/detail", { ids: rawId });
  const songs: any[] = data.songs ?? [];
  if (songs.length === 0) throw new Error(`Song not found: ${id}`);
  return mapSong(songs[0]);
}

export async function getSongs(ids: string[]): Promise<Song[]> {
  if (ids.length === 0) return [];
  const rawIds = ids.map((id) => id.replace(/^netease_song_/, "")).join(",");
  const data = await get("/song/detail", { ids: rawIds });
  return (data.songs ?? []).map(mapSong);
}

export async function getSongUrl(id: string, level = "exhigh"): Promise<string> {
  const rawId = id.replace(/^netease_song_/, "");
  const data = await get("/song/url/v1", { id: rawId, level });
  const urls: any[] = data.data ?? [];
  if (urls.length === 0) {
    // Fallback to /song/url
    const fallback = await get("/song/url", { id: rawId, br: "999000" });
    const fallbackUrls: any[] = fallback.data ?? [];
    return fallbackUrls[0]?.url ?? "";
  }
  return urls[0]?.url ?? "";
}

export async function getLyric(id: string): Promise<string> {
  const rawId = id.replace(/^netease_song_/, "");
  try {
    const data = await get("/lyric/new", { id: rawId });
    // Return YRC (word-level) if available, otherwise LRC
    const yrc = data.yrc?.lyric ?? "";
    if (yrc) return yrc;
    const lrc = data.lrc?.lyric ?? "";
    return lrc;
  } catch {
    // Fallback to old /lyric endpoint
    const data = await get("/lyric", { id: rawId });
    const lrc = data.lrc?.lyric ?? "";
    return lrc;
  }
}

export async function toggleLike(id: string, like: boolean): Promise<boolean> {
  const rawId = id.replace(/^netease_song_/, "");
  const data = await get("/like", { id: rawId, like: like ? "true" : "false" });
  return data.code === 200;
}

// ── Album / Artist / Playlist ─────────────────────────────────────────

export async function getAlbum(id: string): Promise<Album> {
  const rawId = id.replace(/^netease_album_/, "");
  const data = await get("/album", { id: rawId });
  // /album returns { songs: [], album: { ... } }
  const album = data.album ?? data;
  const songs: any[] = data.songs ?? album.songs ?? [];
  return {
    ...mapAlbum(album),
    songs: songs.map(mapSong),
  };
}

export async function getArtist(id: string): Promise<Artist> {
  const rawId = id.replace(/^netease_artist_/, "");
  const data = await get("/artists", { id: rawId });
  const artist = data.artist ?? data;
  const hotSongs: any[] = data.hotSongs ?? artist.hotSongs ?? [];
  return {
    ...mapArtist(artist),
    songs: hotSongs.map(mapSong),
    musicCount: artist.musicSize ?? hotSongs.length,
  };
}

export async function getPlaylist(id: string): Promise<Playlist> {
  const rawId = id.replace(/^netease_playlist_/, "");
  const data = await get("/playlist/detail", { id: rawId, s: "50" });
  const pl = data.playlist ?? data;
  return mapPlaylist(pl);
}

// ── Recommend ─────────────────────────────────────────────────────────

export async function getRecommendedPlaylists(count: number = 30): Promise<Playlist[]> {
  const data = await get("/personalized", { limit: String(count) });
  return mapPlaylistsFromList(data.result ?? []);
}

export async function getDailyRecommended(): Promise<Song[]> {
  const data = await get("/recommend/songs");
  const dailySongs: any[] = data.data?.dailySongs ?? [];
  return dailySongs.map(mapSong);
}

// ── Discover ──────────────────────────────────────────────────────────

export async function getTopPlaylists(): Promise<TopPlaylistGroup[]> {
  const data = await get("/toplist/detail");
  return mapTopPlaylistGroups(data.list ?? []);
}

export async function getPlaylistCats(): Promise<PlaylistCategory[]> {
  const data = await get("/playlist/catlist");
  return mapPlaylistCategories(data);
}

export async function getPlaylistSquare(
  cat: string,
  order: string,
  limit: number,
  offset: number,
  highQuality: boolean,
): Promise<{ playlists: Playlist[]; total: number }> {
  if (highQuality) {
    const data = await get("/top/playlist/highquality", {
      cat,
      limit: String(limit),
      before: String(offset),
    });
    const playlists = mapPlaylistsFromList(data.playlists ?? []);
    return { playlists, total: data.total ?? 0 };
  } else {
    const data = await get("/top/playlist", {
      cat,
      order,
      limit: String(limit),
      offset: String(offset),
      total: "true",
    });
    const playlists = mapPlaylistsFromList(data.playlists ?? []);
    return { playlists, total: data.total ?? 0 };
  }
}

// ── Lyrics (for lyric-service) ────────────────────────────────────────

export interface NeteaseLyricResult {
  lrc: string;
  yrc: string;
  tlyric: string;
  romalrc: string;
  ytlrc: string;
  yromalrc: string;
}

/** Fetch full lyric data for the lyric-service's use. */
export async function fetchNeteaseLyric(songId: string): Promise<NeteaseLyricResult | null> {
  try {
    // Try /lyric/new first for YRC support
    const data = await get("/lyric/new", { id: songId });
    return {
      lrc: data.lrc?.lyric ?? "",
      yrc: data.yrc?.lyric ?? "",
      tlyric: data.tlyric?.lyric ?? "",
      romalrc: data.romalrc?.lyric ?? "",
      ytlrc: data.ytlrc?.lyric ?? "",
      yromalrc: data.yromalrc?.lyric ?? "",
    };
  } catch {
    return null;
  }
}

/** Search Netease songs by keyword for lyric matching. */
export async function searchNeteaseSong(
  keywords: string,
  limit = 5,
): Promise<{ id: string; name: string; artists: string; album: string }[]> {
  try {
    const data = await get("/search", { keywords, limit: String(limit) });
    return (data.result?.songs ?? data.songs ?? []).map((s: any) => ({
      id: String(s.id),
      name: s.name ?? "",
      artists: (s.artists ?? s.ar ?? []).map((a: any) => a.name).join("/"),
      album: s.album?.name ?? s.al?.name ?? "",
    }));
  } catch {
    return [];
  }
}
