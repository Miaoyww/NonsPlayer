import { invoke } from "@tauri-apps/api/core";
import { parseTTML, type LyricLine as RawLyricLine } from "@applemusic-like-lyrics/lyric";
import type { LyricLine } from "$lib/types/lyric";

// ── Constants ────────────────────────────────────────────────────────

const TAG = "[amll-db]";
const BASE_URL = "https://raw.githubusercontent.com/Steve-xmh/amll-ttml-db/refs/heads/main";

export interface CacheStats {
  count: number;
  sizeBytes: number;
}

// ── Tauri invoke wrappers ────────────────────────────────────────────

async function saveTtmlCache(songId: string, content: string): Promise<void> {
  return invoke("save_ttml_cache", { songId, content });
}

async function getTtmlCache(songId: string): Promise<string | null> {
  return invoke("get_ttml_cache", { songId });
}

async function deleteTtmlCache(songId: string): Promise<void> {
  return invoke("delete_ttml_cache", { songId });
}

// ── Public API ───────────────────────────────────────────────────────

/**
 * Fetch TTML from local cache or AMLL DB.
 *
 * @param songId   Numeric song ID (e.g. "3346496228")
 * @param platform Adapter platform slug (e.g. "netease", "qq")
 */
export async function getTtml(songId: string, platform: string): Promise<string | null> {
  // 1. Try local cache (keyed by songId alone — platform doesn't change the content)
  try {
    const cached = await getTtmlCache(songId);
    if (cached) {
      // Validate that cached content is parseable TTML
      try {
        parseTTML(cached);
        console.log(`${TAG} cache HIT for ${songId} (${cached.length} bytes)`);
        return cached;
      } catch {
        console.warn(`${TAG} corrupt cache for ${songId}, removing and re-fetching`);
        await deleteTtmlCache(songId).catch(() => {});
      }
    } else {
      console.log(`${TAG} cache MISS for ${songId}`);
    }
  } catch (err) {
    console.warn(`${TAG} cache read error for ${songId}:`, err);
  }

  // 2. Fetch from AMLL DB: {base}/{platform}-lyrics/{songId}.ttml
  const url = `${BASE_URL}/${encodeURIComponent(platform)}-lyrics/${encodeURIComponent(songId)}.ttml`;
  console.log(`${TAG} fetching: ${url}`);

  try {
    const resp = await fetch(url);

    if (resp.status === 404) {
      console.log(`${TAG} 404 — ${songId} not in AMLL DB`);
      return null;
    }

    if (!resp.ok) {
      console.warn(`${TAG} HTTP ${resp.status} for ${songId}`);
      return null;
    }

    const ttml = await resp.text();
    if (!ttml || ttml.trim().length === 0) {
      console.log(`${TAG} empty response body for ${songId}`);
      return null;
    }
    console.log(`${TAG} fetched ${ttml.length} bytes for ${songId}`);

    // 3. Validate before caching
    try {
      parseTTML(ttml);
    } catch (err) {
      console.warn(`${TAG} unparseable TTML for ${songId}:`, err);
      return null;
    }

    // 4. Save to local cache
    saveTtmlCache(songId, ttml).then(
      () => console.log(`${TAG} cached ${songId} (${ttml.length} bytes)`),
      (err) => console.warn(`${TAG} failed to cache ${songId}:`, err),
    );

    return ttml;
  } catch (err) {
    console.warn(`${TAG} fetch error for ${songId}:`, err);
    return null;
  }
}

/**
 * Parse a TTML content string into LyricLine[].
 * Maps the raw parseTTML output to our LyricLine type, adding default `obscene` field.
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

/**
 * Clear all cached TTML files.
 */
export async function clearTtmlCache(): Promise<void> {
  return invoke("clear_ttml_cache");
}

/**
 * Get TTML cache statistics (file count and total size).
 */
export async function getCacheStats(): Promise<CacheStats> {
  return invoke("get_ttml_cache_stats");
}

/**
 * Format bytes into a human-readable string.
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const size = bytes / Math.pow(1024, i);
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
