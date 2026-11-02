import type { LyricLine } from '@applemusic-like-lyrics/core'

export interface DesktopLyricsAuxiliaryVisibility {
  translation: boolean
  pronunciation: boolean
}

export function lyricText(line: LyricLine | null | undefined) {
  return line?.words.map((word) => word.word).join('') ?? ''
}

export function lyricPronunciation(line: LyricLine | null | undefined) {
  const direct = line?.romanLyric?.trim()
  if (direct) return direct
  const words = line?.words ?? []
  const romanWords = words
    .map((word) => word.romanWord?.trim() ?? '')
    .filter((word) => word.length > 0)
  if (romanWords.length) return romanWords.join(' ')
  return words
    .flatMap((word) => word.ruby ?? [])
    .map((word) => word.word.trim())
    .join('')
}

export function auxiliaryLines(
  line: LyricLine | null | undefined,
  visibility: DesktopLyricsAuxiliaryVisibility
): string[] {
  const lines: string[] = []
  if (visibility.translation) {
    const translation = line?.translatedLyric.trim()
    if (translation) lines.push(translation)
  }
  if (visibility.pronunciation) {
    const pronunciation = lyricPronunciation(line).trim()
    if (pronunciation) lines.push(pronunciation)
  }
  return lines
}

export function hasVisibleAuxiliary(
  line: LyricLine | null | undefined,
  visibility: DesktopLyricsAuxiliaryVisibility
) {
  if (visibility.translation && line?.translatedLyric.trim()) return true
  return visibility.pronunciation && !!lyricPronunciation(line).trim()
}
