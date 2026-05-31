import { invoke } from "@tauri-apps/api/core";
import { parseTTML, type LyricLine as RawLyricLine } from "@applemusic-like-lyrics/lyric";
import type { LyricLine } from "$lib/types/lyric";

// ── Constants ────────────────────────────────────────────────────────

const BASE_URL = "https://raw.githubusercontent.com/Steve-xmh/amll-ttml-db/refs/heads/main";

// ── Tauri invoke wrappers ────────────────────────────────────────────

async function saveLyricCache(songId: string, format: string, content: string): Promise<void> {
  return invoke("save_lyric_cache", { songId, format, content });
}

async function getLyricCache(songId: string, format: string): Promise<string | null> {
  return invoke("get_lyric_cache", { songId, format });
}

// ── Public API ───────────────────────────────────────────────────────

/**
 * Fetch TTML from local cache or AMLL DB.
 *
 * @param songId   Numeric song ID (e.g. "3346496228")
 * @param platform Adapter platform slug (e.g. "ncm" for netease)
 */
export async function getTtml(songId: string, platform: string): Promise<string | null> {
  // 1. Try local cache
  try {
    const cached = await getLyricCache(songId, "ttml");
    if (cached) {
      try {
        parseTTML(cached);
        return cached;
      } catch {
        // Corrupt cache — will be overwritten on next fetch
      }
    }
  } catch { /* cache read error, proceed to fetch */ }

  // 2. Fetch from AMLL DB
  const url = `${BASE_URL}/${encodeURIComponent(platform)}-lyrics/${encodeURIComponent(songId)}.ttml`;

  try {
    const resp = await fetch(url);
    if (resp.status === 404 || !resp.ok) return null;

    const ttml = await resp.text();
    if (!ttml || ttml.trim().length === 0) return null;

    // 3. Validate before caching
    try {
      parseTTML(ttml);
    } catch {
      return null;
    }

    // 4. Save to local cache (fire and forget)
    saveLyricCache(songId, "ttml", ttml).catch(() => {});

    return ttml;
  } catch {
    return null;
  }
}

/**
 * Parse a TTML content string into LyricLine[].
 */
export function parseTtmlLyrics(ttml: string): LyricLine[] {
  const result = parseTTML(ttml);
  if (!Array.isArray(result.lines) || result.lines.length === 0) return [];

  return result.lines.map((line: RawLyricLine) => ({
    words: (line.words ?? []).map((word) => ({
      startTime: word.startTime ?? 0,
      endTime: word.endTime ?? 0,
      word: word.word ?? "",
      romanWord: word.romanWord,
      obscene: (word as any).obscene ?? false,
    })),
    translatedLyric: (line as any).translatedLyric ?? "",
    romanLyric: (line as any).romanLyric ?? "",
    startTime: line.startTime ?? 0,
    endTime: line.endTime ?? 0,
    isBG: (line as any).isBG ?? false,
    isDuet: (line as any).isDuet ?? false,
  }));
}
