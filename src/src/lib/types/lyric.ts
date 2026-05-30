// Aligns with @applemusic-like-lyrics/core LyricLine interface

export interface LyricWord {
  startTime: number;
  endTime: number;
  word: string;
  romanWord?: string;
  obscene?: boolean;
}

export interface LyricLine {
  words: LyricWord[];
  translatedLyric: string;
  romanLyric: string;
  startTime: number;
  endTime: number;
  isBG: boolean;
  isDuet: boolean;
}

// ── Lyric source types ──────────────────────────────────────────────

export enum LyricSourceType {
  /** Embedded or external .lrc file */
  local = "local",
  /** AMLL TTML DB (word-level synced lyrics) */
  amll = "amll",
  /** Adapter's native lyric API (fallback, typically LRC) */
  platform = "platform",
}

export interface LyricSource {
  source: LyricSourceType;
  /** Song ID used for AMLL TTML DB lookup */
  ttmlId?: string;
  /** Which adapter provides this song */
  adapterSlug?: string;
  /** Song ID for adapter platform fallback */
  adapterSongId?: string;
}

/** Maps `"adapterSlug:songId"` → LyricSource */
export type LyricMap = Record<string, LyricSource>;
