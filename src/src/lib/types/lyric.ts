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
