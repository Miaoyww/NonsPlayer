import type { LyricLine } from '@applemusic-like-lyrics/core'

export interface LyricCandidate {
  source: 'qq' | 'netease'
  id: number | null
  mid: string
  title: string
  artist: string
  album: string
  durationMs: number
}
export interface LyricCorrection {
  candidate: LyricCandidate | null
  offsetMs: number | null
}
export const automaticLyrics: LyricCorrection = { candidate: null, offsetMs: null }

/** Shift lines and word timestamps together; never change the playback clock. */
export function offsetLyrics(lines: LyricLine[], offsetMs: number): LyricLine[] {
  if (!offsetMs) return lines
  return lines.map((line) => ({
    ...line,
    startTime: Math.max(0, line.startTime + offsetMs),
    endTime: Math.max(0, line.endTime + offsetMs),
    words: line.words.map((word) => ({
      ...word,
      startTime: Math.max(0, word.startTime + offsetMs),
      endTime: Math.max(0, word.endTime + offsetMs)
    }))
  }))
}
