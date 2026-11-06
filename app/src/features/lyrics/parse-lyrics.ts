import { parseLrc, parseQrc, parseTTML, parseYrc } from '@applemusic-like-lyrics/lyric'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import type { Lyrics } from '@/lib/player'
import { matchAuxiliary } from './auxiliary-matching'
import { parseLocalLrc } from './local-lrc'
import { stripNeteaseMetadata } from './netease-metadata'

export class EmptyLyricsError extends Error {
  readonly code = 'emptyLyrics'
}

export function parseLyricsContent(value: Lyrics): LyricLine[] {
  switch (value.format) {
    case 'ttml':
      return parseTTML(value.content).lines
    case 'yrc':
      return parseYrc(stripNeteaseMetadata(value.content))
    case 'qrc':
      return parseQrc(value.content)
    case 'lrc':
      return value.source === 'local'
        ? parseLocalLrc(stripNeteaseMetadata(value.content))
        : parseLrc(stripNeteaseMetadata(value.content))
  }
}

function cleanAuxiliary(text: string) {
  return text.trim() === '//' ? '' : text
}

function auxiliary(text: string) {
  // Only identify the format here; AMLL parses all timestamps and lyric text.
  const lines = /^\[\d+,\d+\]\(/m.test(text)
    ? parseYrc(text)
    : /^\[\d+,\d+\]/m.test(text)
      ? parseQrc(text)
      : parseLrc(text)
  return lines
    .map((line) => ({
      start: line.startTime,
      text: line.words
        .map((word) => word.word)
        .join('')
        .trim()
    }))
    .filter((line) => Number.isFinite(line.start) && line.text)
    .sort((left, right) => left.start - right.start)
}

export function parseLyrics(
  value: Lyrics,
  duration: number,
  cue?: { startMs: number; endMs: number }
): LyricLine[] {
  const result = parseLyricsContent(value).filter((line) =>
    line.words.some((word) => word.word.trim())
  )
  const starts = result.map((line) => line.startTime)
  const translations = matchAuxiliary(starts, value.translation ? auxiliary(value.translation) : [])
  const romans = matchAuxiliary(starts, value.romanization ? auxiliary(value.romanization) : [])
  if (!result.length) throw new EmptyLyricsError()
  const nextDistinct = new Array<number | undefined>(result.length)
  let greaterStart: number | undefined
  for (let index = result.length - 1; index >= 0; index--) {
    const start = result[index].startTime
    if (greaterStart !== undefined && greaterStart > start) nextDistinct[index] = greaterStart
    if (greaterStart === undefined || start < greaterStart) greaterStart = start
  }
  const parsed = result.map((line, index) => {
    const end =
      Number.isFinite(line.endTime) && line.endTime > line.startTime
        ? line.endTime
        : (nextDistinct[index] ?? Math.max(duration, line.startTime + 5000))
    if (!Number.isFinite(line.startTime) || line.startTime < 0 || end < line.startTime)
      throw new Error('歌词时间轴无效。')
    if (line.words.some((word) => !Number.isFinite(word.startTime) || word.startTime < 0))
      throw new Error('歌词逐词时间轴无效。')
    const lineEnd = Math.max(end, ...line.words.map((word) => word.startTime))
    return {
      ...line,
      endTime: lineEnd,
      words: line.words.map((word, wordIndex) => {
        const nextStart = line.words[wordIndex + 1]?.startTime
        const wordEnd =
          Number.isFinite(word.endTime) && word.endTime > word.startTime ? word.endTime : lineEnd
        const endTime =
          Number.isFinite(nextStart) && nextStart >= word.startTime && wordEnd > nextStart
            ? nextStart
            : wordEnd
        return {
          ...word,
          endTime: Math.max(word.startTime, endTime)
        }
      }),
      translatedLyric: cleanAuxiliary(line.translatedLyric).trim()
        ? line.translatedLyric
        : cleanAuxiliary(translations[index] ?? ''),
      romanLyric: cleanAuxiliary(line.romanLyric).trim()
        ? line.romanLyric
        : cleanAuxiliary(romans[index] ?? '')
    }
  })
  if (!cue || value.source !== 'local') return parsed
  const shifted = parsed
    .filter((line) => line.startTime >= cue.startMs && line.startTime < cue.endMs)
    .map((line) => ({
      ...line,
      startTime: line.startTime - cue.startMs,
      endTime: Math.min(line.endTime, cue.endMs) - cue.startMs,
      words: line.words.map((word) => ({
        ...word,
        startTime: Math.max(0, Math.min(word.startTime, cue.endMs) - cue.startMs),
        endTime: Math.max(0, Math.min(word.endTime, cue.endMs) - cue.startMs)
      }))
    }))
  if (!shifted.length) throw new EmptyLyricsError()
  return shifted
}
