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

import { writable } from "svelte/store";
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

/** localStorage key for the login cookie. */
const COOKIE_STORAGE_KEY = "netease_cookie";

/** Current cookie string (set after login). Persisted across restarts. */
let _cookie = tryLoadCookie();

/** Reactive store that components can subscribe to for login/logout changes. */
export const neteaseAuth = writable({ loggedIn: _cookie.includes("MUSIC_U") });

function tryLoadCookie(): string {
  try {
    if (typeof localStorage !== "undefined") {
      return localStorage.getItem(COOKIE_STORAGE_KEY) ?? "";
    }
  } catch { /* not available in SSR */ }
  return "";
}

function persistCookie(c: string): void {
  try {
    if (typeof localStorage !== "undefined") {
      if (c) localStorage.setItem(COOKIE_STORAGE_KEY, c);
      else localStorage.removeItem(COOKIE_STORAGE_KEY);
    }
  } catch { /* not available in SSR */ }
}

export function getCookie(): string {
  return _cookie;
}

export function setCookie(c: string): void {
  const wasLoggedIn = _cookie.includes("MUSIC_U");
  _cookie = c;
  persistCookie(c);
  const nowLoggedIn = c.includes("MUSIC_U");
  console.log("[netease-api] setCookie | wasLoggedIn:", wasLoggedIn, "nowLoggedIn:", nowLoggedIn, "cookiePreview:", c.substring(0, 80));
  neteaseAuth.set({ loggedIn: nowLoggedIn });
}

/**
 * Try to restore the previous login session on app startup.
 * Returns an Account if the stored cookie is still valid, otherwise null.
 */
export async function tryAutoLogin(): Promise<Account | null> {
  console.log("[netease-api] tryAutoLogin | cookie exists:", !!_cookie, "has MUSIC_U:", _cookie.includes("MUSIC_U"));
  if (!_cookie) return null;

  // Try refreshing the token first
  try {
    await loginRefresh();
  } catch (e) {
    console.log("[netease-api] tryAutoLogin | refresh failed:", e);
    setCookie("");
    return null;
  }

  // Cookie is valid — fetch account info
  try {
    const acc = await getAccount();
    console.log("[netease-api] tryAutoLogin | account:", acc.name, acc.id);
    return acc;
  } catch (e) {
    console.log("[netease-api] tryAutoLogin | getAccount failed:", e);
    return null;
  }
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
  const data = await get("/login/qr/key", { timestamp: String(Date.now()) });
  return data.data ?? data;
}

export async function loginQrCreate(key: string): Promise<{ qrurl: string; qrimg: string }> {
  const data = await get("/login/qr/create", {
    key,
    qrimg: "true",
    platform: "pc",
    timestamp: String(Date.now()),
    ua: "pc",
  });
  const inner = data.data ?? data;
  return { qrurl: inner.qrurl, qrimg: inner.qrimg };
}

/**
 * Poll login status.  key is the unikey from loginQrKey.
 * Returns the raw api-enhanced response so adapter-service can interpret
 * the code and build a LoginStatus.
 *
 * Netease status codes:
 *   800 – QR code expired
 *   801 – waiting for scan
 *   802 – scanned, waiting for user confirmation on phone
 *   803 – login confirmed
 */
export async function loginQrCheck(key: string): Promise<{ code: number; cookie?: string; message?: string }> {
  const data = await get("/login/qr/check", {
    key,
    timestamp: String(Date.now()),
    ua: "pc",
  });
  const body = data.body ?? data;
  const code = typeof body.code === "number" ? body.code : Number(body.code ?? -1);

  const result: { code: number; cookie?: string; message?: string } = { code, message: body.message };

  // Login success — persist cookie to localStorage
  if (code === 803 && body.cookie) {
    setCookie(body.cookie);
    result.cookie = body.cookie;
  }

  return result;
}

export async function loginRefresh(): Promise<boolean> {
  const data = await get("/login/refresh", { timestamp: String(Date.now()) });
  if (data.code === 200 && data.cookie) {
    setCookie(data.cookie);
    return true;
  }
  return false;
}

// ── Account ───────────────────────────────────────────────────────────

export async function getAccount(): Promise<Account> {
  console.log("[netease-api] getAccount | cookie exists:", !!_cookie, "has MUSIC_U:", _cookie.includes("MUSIC_U"));
  if (!_cookie || !_cookie.includes("MUSIC_U")) {
    throw new Error("Not logged in");
  }
  const data = await get("/user/account");
  console.log("[netease-api] getAccount | response code:", data.code, "has profile:", !!data.profile);
  if (data.code !== 200) {
    throw new Error("Not logged in: code " + data.code);
  }
  const profile = data.profile ?? {};
  const uid = data.account?.id ?? data.profile?.userId ?? "";
  if (!uid) {
    throw new Error("Not logged in: no uid");
  }
  return mapAccount(profile, uid, _cookie);
}

export async function getUserPlaylists(uid: string): Promise<Playlist[]> {
  const rawId = uid.replace(/^netease_user_/, "");
  const data = await get("/user/playlist", { uid: rawId, limit: "50", offset: "0" });
  return mapPlaylistsFromList(data.playlist ?? []);
}

/**
 * Get the user's "favorite" playlist (❤️ 我喜欢的音乐).
 * Netease marks this with `specialType === 5` in the raw API response.
 * We need two API calls: /user/playlist to find it, then /playlist/detail for full data.
 */
export async function getFavoritePlaylist(uid: string): Promise<Playlist | null> {
  const rawId = uid.replace(/^netease_user_/, "");
  const data = await get("/user/playlist", { uid: rawId, limit: "50", offset: "0" });
  const rawList: any[] = data.playlist ?? [];

  // Find the playlist with specialType === 5
  const favRaw = rawList.find((p: any) => p.specialType === 5);
  if (!favRaw) return null;

  // Fetch full playlist detail (tracks are null in the summary response)
  const detail = await get("/playlist/detail", { id: String(favRaw.id), s: "0" });
  return mapPlaylist(detail.playlist ?? detail);
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
