import type { LyricLine } from '@applemusic-like-lyrics/core'

export function findLyricLine(lines: LyricLine[], positionMs: number) {
  if (!Number.isFinite(positionMs)) return null
  const usable = lines.filter((line) => !line.isBG && line.words.some((word) => !!word.word.trim()))
  let low = 0
  let high = usable.length - 1
  let candidate: LyricLine | null = null
  while (low <= high) {
    const middle = (low + high) >> 1
    const line = usable[middle]
    if (line.startTime <= positionMs) {
      candidate = line
      low = middle + 1
    } else {
      high = middle - 1
    }
  }
  return candidate
}
