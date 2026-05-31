import { invoke } from "@tauri-apps/api/core";

// ── Dynamic port resolution ──────────────────────────────────────────

let _apiPort: number | null = null;

async function getApiPort(): Promise<number> {
  if (_apiPort !== null) return _apiPort;
  try {
    _apiPort = await invoke<number>("get_api_port");
    return _apiPort;
  } catch {
    _apiPort = 37562;
    return _apiPort;
  }
}

async function getApiBase(): Promise<string> {
  const port = await getApiPort();
  return `http://127.0.0.1:${port}/api`;
}

// ── Types ────────────────────────────────────────────────────────────

export interface NeteaseLyricResult {
  lrc: string;
  yrc: string;
  /** Translated line-level lyrics (LRC format) */
  tlyric: string;
  /** Romanized line-level lyrics (LRC format) */
  romalrc: string;
  /** Translated word-level lyrics (YRC format) */
  ytlrc: string;
  /** Romanized word-level lyrics (YRC format) */
  yromalrc: string;
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
    const baseUrl = await getApiBase();
    const resp = await fetch(`${baseUrl}/lyric?id=${encodeURIComponent(songId)}`);
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
    const baseUrl = await getApiBase();
    const resp = await fetch(
      `${baseUrl}/search?keywords=${encodeURIComponent(keywords)}&limit=${limit}`,
    );
    if (!resp.ok) return [];
    const data = await resp.json();
    return data.songs ?? [];
  } catch {
    return [];
  }
}
