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
  /** Adapter's native lyric API (typically LRC or YRC) */
  platform = "platform",
}

export interface LyricSource {
  source: LyricSourceType;
  /** Which adapter provides this song */
  adapterSlug?: string;
  /** Song ID for adapter platform */
  adapterSongId?: string;
}
