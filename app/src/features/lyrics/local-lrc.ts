import type { LyricLine } from '@applemusic-like-lyrics/core'
import { parseLrc } from '@applemusic-like-lyrics/lyric'

interface InlineTranslation {
  originalEnd: number
  translation: string
}

const inlineDelimiter = /\s+[/／]\s+|\s*\/\/\s*/
const slashDelimiter = /^(.*?)\s*[/／]\s*(.*?)$/
const cjk = /[\u1100-\u11ff\u3040-\u31ff\u3400-\u4dbf\u4e00-\u9fff\uac00-\ud7af]/

function text(line: LyricLine) {
  return line.words.map((word) => word.word).join('')
}

function splitInline(value: string): InlineTranslation | null {
  const match = inlineDelimiter.exec(value)
  if (match) {
    const translation = value.slice(match.index + match[0].length).trim()
    if (value.slice(0, match.index).trim() && translation)
      return { originalEnd: match.index, translation }
  }

  const slash = slashDelimiter.exec(value)
  if (!slash) return null
  const original = slash[1].trim()
  const translation = slash[2].trim()
  if (!original || !translation || !(cjk.test(original) || cjk.test(translation))) return null
  return { originalEnd: slash[1].length, translation }
}

function withoutTranslation(line: LyricLine, end: number) {
  const words: LyricLine['words'] = []
  let offset = 0
  for (const word of line.words) {
    if (offset >= end) break
    const remaining = end - offset
    if (word.word.length <= remaining) {
      words.push(word)
    } else {
      const kept = word.word.slice(0, remaining).trimEnd()
      if (kept) words.push({ ...word, word: kept, endTime: word.startTime })
      break
    }
    offset += word.word.length
  }
  return words.length ? words : line.words
}

function inlineTranslations(lines: LyricLine[]) {
  const matches = lines.map((line) => splitInline(text(line)))
  const count = matches.filter(Boolean).length
  const detected =
    lines.length <= 2
      ? count === lines.length
      : count >= 2 && (count / lines.length >= 0.25 || count >= 6)
  if (!detected) return null
  return lines.map((line, index) => {
    const match = matches[index]
    return match
      ? {
          ...line,
          words: withoutTranslation(line, match.originalEnd),
          translatedLyric: match.translation
        }
      : line
  })
}

function sameTimeTranslations(lines: LyricLine[]) {
  const groups = new Map<number, LyricLine[]>()
  for (const line of lines) {
    const group = groups.get(line.startTime)
    if (group) group.push(line)
    else groups.set(line.startTime, [line])
  }
  const found = [...groups.values()].some((group) => {
    const original = text(group[0]).trim()
    return group.length > 1 && group.slice(1).some((line) => text(line).trim() !== original)
  })
  if (!found) return lines

  return [...groups.entries()]
    .sort(([left], [right]) => left - right)
    .map(([, group]) => {
      const original = text(group[0]).trim()
      const translation = group
        .slice(1)
        .map((line) => text(line).trim())
        .filter((line) => line && line !== original)
        .join(' / ')
      return translation ? { ...group[0], translatedLyric: translation } : group[0]
    })
}

export function parseLocalLrc(content: string) {
  const lines = parseLrc(content)
  if (!lines.length) return lines
  return inlineTranslations(lines) ?? sameTimeTranslations(lines)
}
