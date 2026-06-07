/**
 * Song module — detail, URL, lyric, like.
 *
 * Mirror of api-enhanced modules:
 *   song_detail, song_url_v1, lyric_new, like
 */

import type { Song } from "$lib/types/song";
import { mapSong } from "../netease-mapper";
import { neteaseRequest } from "./request";

// ── Song detail ───────────────────────────────────────────────────────

export async function getSong(id: string): Promise<Song> {
  const rawId = id.replace(/^netease_song_/, "");
  const data = await neteaseRequest("api", "/api/v3/song/detail", {
    c: JSON.stringify([{ id: rawId }]),
  });
  const songs: any[] = data.songs ?? [];
  if (songs.length === 0) throw new Error(`Song not found: ${id}`);
  return mapSong(songs[0]);
}

export async function getSongs(ids: string[]): Promise<Song[]> {
  if (ids.length === 0) return [];
  const c = JSON.stringify(
    ids.map((id) => ({ id: Number(id.replace(/^netease_song_/, "")) })),
  );
  const data = await neteaseRequest("api", "/api/v3/song/detail", { c });
  return (data.songs ?? []).map(mapSong);
}

// ── Song URL ──────────────────────────────────────────────────────────

export async function getSongUrl(
  id: string,
  br = 999000,
): Promise<string> {
  const rawId = id.replace(/^netease_song_/, "");

  try {
    const data = await neteaseRequest(
      "api",
      "/api/song/enhance/download/url",
      {
        id: rawId,
        br: String(br),
      },
    );
    console.log("Netease song URL data:", data);
    return data?.data?.url ?? "";
  } catch {
    return "";
  }
}

// ── Lyric ─────────────────────────────────────────────────────────────

export async function getLyric(id: string): Promise<string> {
  const rawId = id.replace(/^netease_song_/, "");
  const data = await neteaseRequest("api", "/api/song/lyric/v1", {
    id: rawId,
    cp: "false",
    tv: "0",
    lv: "0",
    rv: "0",
    kv: "0",
    yv: "0",
    ytv: "0",
    yrv: "0",
  });
  const yrc = data.yrc?.lyric ?? "";
  if (yrc) return yrc;
  return data.lrc?.lyric ?? "";
}

// ── Like ──────────────────────────────────────────────────────────────

export async function toggleLike(id: string, like: boolean): Promise<boolean> {
  const rawId = id.replace(/^netease_song_/, "");
  const data = await neteaseRequest("api", "/api/radio/like", {
    alg: "itembased",
    trackId: rawId,
    like: like ? "true" : "false",
    time: "3",
  });
  return data.code === 200;
}
