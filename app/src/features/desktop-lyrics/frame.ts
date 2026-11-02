import type { LyricLine } from '@applemusic-like-lyrics/core'
import { clamp01, type DesktopLyricsFrame } from './presentation'

export function lyricText(line: LyricLine | undefined) {
  return line?.words.map((word) => word.word).join('') ?? ''
}

export function lineIndexAt(lines: readonly LyricLine[], positionMs: number) {
  let low = 0
  let high = lines.length
  while (low < high) {
    const middle = (low + high) >> 1
    if (lines[middle].startTime <= positionMs) low = middle + 1
    else high = middle
  }
  return low - 1
}

export function wordScanProgress(line: LyricLine, positionMs: number) {
  const text = lyricText(line)
  if (!line.words.length || !text.length) return null

  let scannedTextLength = 0
  let textOffset = 0
  let hasUsableWord = false
  let lastUsableWordEnd = line.startTime

  for (const word of line.words) {
    const wordLength = word.word.length
    if (!wordLength) continue

    const start = word.startTime
    const end = word.endTime
    if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) {
      textOffset += wordLength
      continue
    }

    hasUsableWord = true
    lastUsableWordEnd = Math.max(lastUsableWordEnd, end)
    if (positionMs <= start) break

    if (end <= start) {
      scannedTextLength = textOffset + wordLength
      textOffset += wordLength
      continue
    }

    const progress = clamp01((positionMs - start) / (end - start))
    scannedTextLength = textOffset + wordLength * progress
    if (progress < 1) break
    textOffset += wordLength
  }

  if (!hasUsableWord) return null
  if (positionMs >= lastUsableWordEnd) scannedTextLength = text.length
  return clamp01(scannedTextLength / Math.max(1, text.length))
}

export function frameAt(
  lines: readonly LyricLine[],
  positionMs: number,
  trackId: string,
  isPlaying: boolean
): DesktopLyricsFrame {
  const currentLineIndex = lineIndexAt(lines, positionMs)
  const line = currentLineIndex >= 0 ? lines[currentLineIndex] : undefined
  if (!line) {
    return {
      scene: 'message',
      layout: 'single',
      translationMode: false,
      current: '',
      currentTranslation: '',
      currentLineIndex: -1,
      progress: 0,
      wordScanProgress: null,
      trackId,
      isPlaying
    }
  }

  const current = lyricText(line).trim() || ' '
  const currentTranslation = (line.translatedLyric ?? '').trim()
  const endTime = line.endTime > line.startTime ? line.endTime : line.startTime + 1
  return {
    scene: 'lyrics',
    layout: currentTranslation ? 'translationPair' : 'single',
    translationMode: Boolean(currentTranslation),
    current,
    currentTranslation,
    currentLineIndex,
    progress: clamp01((positionMs - line.startTime) / (endTime - line.startTime)),
    wordScanProgress: wordScanProgress(line, positionMs),
    trackId,
    isPlaying
  }
}
