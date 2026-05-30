// ── Configuration ────────────────────────────────────────────────────

const API_BASE = "http://127.0.0.1:25884/api";

// ── Types ────────────────────────────────────────────────────────────

export interface NeteaseLyricResult {
  lrc: string;
  yrc: string;
}

export interface NeteaseSearchSong {
  id: string;
  name: string;
  artists: string;
  album: string;
}

// ── Public API ───────────────────────────────────────────────────────

/** Fetch both LRC and YRC for a Netease song ID. Returns null on failure. */
export async function fetchNeteaseLyric(songId: string): Promise<NeteaseLyricResult | null> {
  try {
    const resp = await fetch(`${API_BASE}/lyric?id=${encodeURIComponent(songId)}`);
    if (!resp.ok) return null;
    return await resp.json();
  } catch {
    return null;
  }
}

/** Search Netease for songs matching a keyword string. */
export async function searchNeteaseSong(
  keywords: string,
  limit = 5,
): Promise<NeteaseSearchSong[]> {
  try {
    const resp = await fetch(
      `${API_BASE}/search?keywords=${encodeURIComponent(keywords)}&limit=${limit}`,
    );
    if (!resp.ok) return [];
    const data = await resp.json();
    return data.songs ?? [];
  } catch {
    return [];
  }
}
