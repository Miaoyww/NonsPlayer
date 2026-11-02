import type { LyricLine } from '@applemusic-like-lyrics/core'
import { reportDiagnostic } from '@/lib/diagnostics'
import type { Lyrics } from '@/lib/player'

const MAX_TEXT_CHARS = 180
const MAX_WORD_CHARS = 80
const MAX_WORDS_PER_LINE = 128

export interface CurrentLyricDiagnostic {
  operation: string
  trackKey: string
  trackLabel: string
  positionMs: number
  lyrics: Lyrics | null
  lines: readonly LyricLine[]
}

function compact(value: string, limit: number) {
  const text = value
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[{}[\]]/g, ' ')
    .trim()
  return text.length > limit ? `${text.slice(0, limit)}…` : text
}

function lineIndexAt(lines: readonly LyricLine[], positionMs: number) {
  let low = 0
  let high = lines.length
  while (low < high) {
    const middle = (low + high) >> 1
    if (lines[middle].startTime <= positionMs) low = middle + 1
    else high = middle
  }
  return low - 1
}

function lineText(line: LyricLine) {
  return line.words.map((word) => word.word).join('')
}

function lineSummary(index: number, line: LyricLine) {
  const words = line.words
    .slice(0, MAX_WORDS_PER_LINE)
    .map(
      (word) =>
        `${compact(word.word, MAX_WORD_CHARS)}@${Math.round(word.startTime)}-${Math.round(word.endTime)}`
    )
    .join('|')
  const omitted = line.words.length > MAX_WORDS_PER_LINE ? '|…' : ''
  return [
    `line=${index}`,
    `range=${Math.round(line.startTime)}-${Math.round(line.endTime)}`,
    `text=${compact(lineText(line), MAX_TEXT_CHARS)}`,
    `words=${words}${omitted}`,
    `translation=${compact(line.translatedLyric, MAX_TEXT_CHARS)}`,
    `romanization=${compact(line.romanLyric, MAX_TEXT_CHARS)}`
  ].join(';')
}

export function formatCurrentLyricDiagnostic(input: CurrentLyricDiagnostic) {
  const current = lineIndexAt(input.lines, input.positionMs)
  const fields = [
    `track=${compact(input.trackLabel, MAX_TEXT_CHARS)}`,
    `key=${compact(input.trackKey, MAX_TEXT_CHARS)}`,
    `source=${input.lyrics?.source ?? 'none'}`,
    `format=${input.lyrics?.format ?? 'none'}`,
    `positionMs=${Math.round(input.positionMs)}`,
    `lineCount=${input.lines.length}`,
    `currentLine=${current}`
  ]
  for (
    let index = Math.max(0, current - 1);
    index <= Math.min(current + 2, input.lines.length - 1);
    index++
  ) {
    fields.push(lineSummary(index, input.lines[index]))
  }
  return fields.join(';')
}

export function reportCurrentLyrics(input: CurrentLyricDiagnostic) {
  console.info('[NonsPlayer] 当前歌词数据', {
    operation: input.operation,
    trackKey: input.trackKey,
    trackLabel: input.trackLabel,
    positionMs: input.positionMs,
    lyrics: input.lyrics,
    lines: input.lines
  })
  reportDiagnostic({
    level: 'info',
    command: 'lyrics_data',
    operation: input.operation,
    source: input.lyrics?.source,
    stage: input.lyrics?.format,
    code: 'loaded',
    message: formatCurrentLyricDiagnostic(input)
  })
}
