import type { LyricLine } from '@applemusic-like-lyrics/core'

/** Main text plus visible auxiliary text, each present part on its own line. */
export function lyricLineText(line: LyricLine) {
  return [line.words.map((word) => word.word).join(''), line.translatedLyric, line.romanLyric]
    .map((part) => part.trim())
    .filter(Boolean)
    .join('\n')
}

export function lyricsText(lines: LyricLine[]) {
  return lines.map(lyricLineText).filter(Boolean).join('\n')
}
