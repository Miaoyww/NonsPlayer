import type { LyricLine } from '@applemusic-like-lyrics/core'

export function mediaLyricLines(lines: LyricLine[]) {
  return lines
    .filter((line) => !line.isBG)
    .map((line) => ({
      startTime: Math.floor(line.startTime),
      endTime: Math.floor(line.endTime),
      text: line.words
        .map((word) => word.word)
        .join('')
        .trim(),
      translation: line.translatedLyric.trim(),
      romanization: line.romanLyric.trim()
    }))
    .filter((line) => line.text)
    .sort((a, b) => a.startTime - b.startTime)
}
