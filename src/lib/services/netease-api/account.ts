/**
 * Account module — user profile, playlists, favorites.
 *
 * Mirror of api-enhanced modules:
 *   login_status, user_playlist, playlist_detail (for favorites)
 */

import type { Account } from "$lib/types/account";
import type { Playlist } from "$lib/types/playlist";
import { mapAccount, mapPlaylist, mapPlaylistsFromList } from "../netease-mapper";
import { neteaseRequest } from "./request";
import { isLoggedIn, getCookie } from "./cookies";

// ── Account ───────────────────────────────────────────────────────────

export async function getAccount(): Promise<Account> {
  if (!isLoggedIn()) throw new Error("Not logged in");

  const data = await neteaseRequest("api", "/api/w/nuser/account/get", {});
  console.log("[netease-api] getAccount | response code:", data.code, "has profile:", !!data.profile);
  if (data.code !== 200) throw new Error("Not logged in: code " + data.code);

  const profile = data.profile ?? {};
  const uid = data.account?.id ?? data.profile?.userId ?? "";
  if (!uid) throw new Error("Not logged in: no uid");

  return mapAccount(profile, uid, getCookie());
}

// ── User playlists ────────────────────────────────────────────────────

export async function getUserPlaylists(uid: string): Promise<Playlist[]> {
  const rawId = uid.replace(/^netease_user_/, "");
  const data = await neteaseRequest("api", "/api/user/playlist", {
    uid: rawId,
    limit: "50",
    offset: "0",
    includeVideo: "true",
  });
  return mapPlaylistsFromList(data.playlist ?? []);
}

/** Get the user's "favorite" playlist (❤️ 我喜欢的音乐, specialType === 5). */
export async function getFavoritePlaylist(uid: string): Promise<Playlist | null> {
  const rawId = uid.replace(/^netease_user_/, "");
  const data = await neteaseRequest("api", "/api/user/playlist", {
    uid: rawId,
    limit: "50",
    offset: "0",
    includeVideo: "true",
  });
  const rawList: any[] = data.playlist ?? [];

  const favRaw = rawList.find((p: any) => p.specialType === 5);
  if (!favRaw) return null;

  const detail = await neteaseRequest("api", "/api/v6/playlist/detail", {
    id: String(favRaw.id),
    n: "100000",
    s: "8",
  });
  return mapPlaylist(detail.playlist ?? detail);
}
