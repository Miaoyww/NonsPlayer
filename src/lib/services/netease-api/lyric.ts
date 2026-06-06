/**
 * Lyric module — full lyric fetch + song search for lyric matching.
 *
 * Mirror of api-enhanced modules:
 *   lyric_new, search (song-only)
 */

import type { NeteaseLyricResult } from "./types";
import { neteaseRequest } from "./request";

export type { NeteaseLyricResult } from "./types";

/** Fetch full lyric data (LRC, YRC, translations, romanization). */
export async function fetchNeteaseLyric(songId: string): Promise<NeteaseLyricResult | null> {
  try {
    const data = await neteaseRequest("api", "/api/song/lyric/v1", {
      id: songId,
      cp: "false",
      tv: "0",
      lv: "0",
      rv: "0",
      kv: "0",
      yv: "0",
      ytv: "0",
      yrv: "0",
    });
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

/** Search Netease songs by keyword (for lyric-service matching). */
export async function searchNeteaseSong(
  keywords: string,
  limit = 5,
): Promise<{ id: string; name: string; artists: string; album: string }[]> {
  try {
    const data = await neteaseRequest("api", "/api/search/get", {
      s: keywords,
      type: "1",
      limit: String(limit),
      offset: "0",
    });
    const songs = data.result?.songs ?? [];
    return songs.map((s: any) => ({
      id: String(s.id),
      name: s.name ?? "",
      artists: (s.artists ?? s.ar ?? []).map((a: any) => a.name).join("/"),
      album: s.album?.name ?? s.al?.name ?? "",
    }));
  } catch {
    return [];
  }
}
