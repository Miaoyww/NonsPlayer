import { invoke } from "@tauri-apps/api/core";
import type {
  AdapterConfig,
  AdapterMetadata,
  LoginStatus,
  SearchResult,
} from "$lib/types/adapter";
import type { Playlist } from "$lib/types/playlist";
import type { Account } from "$lib/types/account";
import type { TopPlaylistGroup } from "$lib/types/top-playlist";
import type { PlaylistCategory } from "$lib/types/playlist-category";
import * as neteaseApi from "./netease-api";

// ── Frontend adapter slugs ────────────────────────────────────────────

/** Adapters that run on the frontend via Tauri IPC (netease proxy). */
const FRONTEND_ADAPTERS = new Set(["netease"]);

function isFrontend(adapter: string): boolean {
  return FRONTEND_ADAPTERS.has(adapter);
}

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

export function getSongUrl(adapter: string, id: string): Promise<string> {
  if (isFrontend(adapter)) return neteaseApi.getSongUrl(id);
  return invoke("get_song_url", { adapter, id });
}

export function getLyric(adapter: string, id: string): Promise<string> {
  if (isFrontend(adapter)) return neteaseApi.getLyric(id);
  return invoke("get_lyric", { adapter, id });
}

export function toggleLike(adapter: string, id: string, like: boolean): Promise<boolean> {
  if (isFrontend(adapter)) return neteaseApi.toggleLike(id, like);
  return invoke("toggle_like", { adapter, id, like });
}

// -- Album / Artist / Playlist --

export function getPlaylist(adapter: string, id: string): Promise<Playlist> {
  if (isFrontend(adapter)) return neteaseApi.getPlaylist(id);
  return invoke("get_playlist", { adapter, id });
}

// -- Search --

export function search(adapter: string, keyword: string): Promise<SearchResult> {
  if (isFrontend(adapter)) return neteaseApi.search(keyword);
  return invoke("search", { adapter, keyword });
}

// -- Account --

export async function loginQrUrl(adapter: string): Promise<[string, string]> {
  if (isFrontend(adapter)) {
    const { unikey: key } = await neteaseApi.loginQrKey();
    const { qrimg } = await neteaseApi.loginQrCreate(key);
    // qrimg is a base64 data:image/png;… URL — use directly as img src
    return [key, qrimg];
  }
  return invoke("login_qr_url", { adapter });
}

export async function checkLogin(adapter: string, key: string): Promise<LoginStatus> {
  if (isFrontend(adapter)) {
    const result = await neteaseApi.loginQrCheck(key);
    switch (result.code) {
      case 801:
        return { status: "waiting", qr_url: "" };
      case 802:
        return { status: "scanned" };
      case 803: {
        // Get account info after successful login
        let account: Account;
        try {
          account = await neteaseApi.getAccount();
        } catch {
          account = {
            id: "",
            md5: "",
            name: "网易云用户",
            token: neteaseApi.getCookie(),
            avatarUrl: "",
            isLoggedIn: true,
            key,
          };
        }
        return { status: "confirmed", account };
      }
      case 800:
        return { status: "timeout" };
      default:
        return { status: "cancelled" };
    }
  }
  return invoke("check_login", { adapter, key });
}

export function getAccount(adapter: string): Promise<Account> {
  if (isFrontend(adapter)) return neteaseApi.getAccount();
  return invoke("get_account", { adapter });
}

export function getUserPlaylists(adapter: string): Promise<Playlist[]> {
  if (isFrontend(adapter)) {
    // For netease, we need the user's numeric uid. Since we don't have it
    // readily available, fetch account first then get playlists.
    return neteaseApi.getAccount().then((acc) =>
      neteaseApi.getUserPlaylists(acc.id),
    );
  }
  return invoke("get_user_playlists", { adapter });
}

export function getFavoritePlaylist(adapter: string): Promise<Playlist | null> {
  if (isFrontend(adapter)) {
    console.log("[adapter-service] getFavoritePlaylist | frontend adapter:", adapter);
    return neteaseApi.getAccount().then((acc) => {
      console.log("[adapter-service] getFavoritePlaylist | account:", acc.name, acc.id);
      return neteaseApi.getFavoritePlaylist(acc.id);
    }).catch((e) => {
      console.log("[adapter-service] getFavoritePlaylist | error:", e);
      return null;
    });
  }
  return invoke("get_favorite_playlist", { adapter });
}

// -- Recommend --

export function getRecommendedPlaylists(
  adapter: string,
  count: number,
): Promise<Playlist[]> {
  if (isFrontend(adapter)) return neteaseApi.getRecommendedPlaylists(count);
  return invoke("get_recommended_playlists", { adapter, count });
}

// -- Discover --

export function getTopPlaylists(adapter: string): Promise<TopPlaylistGroup[]> {
  if (isFrontend(adapter)) return neteaseApi.getTopPlaylists();
  return invoke("get_top_playlists", { adapter });
}

export function getPlaylistCats(adapter: string): Promise<PlaylistCategory[]> {
  if (isFrontend(adapter)) return neteaseApi.getPlaylistCats();
  return invoke("get_playlist_cats", { adapter });
}

export async function getPlaylistSquare(
  adapter: string,
  cat: string,
  order: string,
  limit: number,
  offset: number,
  highQuality: boolean,
): Promise<[Playlist[], number]> {
  if (isFrontend(adapter)) {
    const result = await neteaseApi.getPlaylistSquare(cat, order, limit, offset, highQuality);
    return [result.playlists, result.total];
  }
  return invoke("get_playlist_square", {
    adapter,
    cat,
    order,
    limit,
    offset,
    highQuality,
  });
}

