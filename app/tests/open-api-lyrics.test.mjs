import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import * as amll from '@applemusic-like-lyrics/lyric'
import FastPriorityQueue from 'fastpriorityqueue'
import { JSDOM } from 'jsdom'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function load(path, require) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }).outputText,
    { exports, require }
  )
  return exports
}

const matching = load('../src/features/lyrics/auxiliary-matching.ts', () => FastPriorityQueue)
const localLrc = load('../src/features/lyrics/local-lrc.ts', () => amll)
const metadata = load('../src/features/lyrics/netease-metadata.ts', () => ({}))
const parser = load('../src/features/lyrics/parse-lyrics.ts', (name) => {
  if (name === './auxiliary-matching') return matching
  if (name === './local-lrc') return localLrc
  if (name === './netease-metadata') return metadata
  return amll
})
const fixtures = JSON.parse(
  readFileSync(new URL('./fixtures/open-api-lyrics.json', import.meta.url), 'utf8')
)

for (const fixture of fixtures) {
  test(`shared native TTML fixture matches the displayed lyrics: ${fixture.name}`, () => {
    const lines = parser.parseLyrics(fixture.lyrics, fixture.durationMs, fixture.cue ?? undefined)
    const actual = Array.from(lines, (line) => ({
      startTime: line.startTime,
      endTime: line.endTime,
      text: line.words.map((word) => word.word).join(''),
      translation: line.translatedLyric,
      romanization: line.romanLyric,
      words: Array.from(line.words, (word) => ({
        text: word.word,
        startTime: word.startTime,
        endTime: word.endTime
      }))
    }))
    assert.deepEqual(actual, fixture.expected)
  })
}

// Native tests can export generated documents for a cross-library round trip.
if (process.env.NONS_TTML_FIXTURE_OUTPUT) {
  const dom = new JSDOM('')
  globalThis.DOMParser = dom.window.DOMParser
  const generated = JSON.parse(readFileSync(process.env.NONS_TTML_FIXTURE_OUTPUT, 'utf8'))
  for (const fixture of fixtures) {
    test(`AMLL reads native TTML without losing lyric content: ${fixture.name}`, () => {
      const document = generated.find((value) => value.name === fixture.name)
      assert.ok(document, `missing native output: ${fixture.name}`)
      const lines = amll.parseTTML(document.content).lines
      const actual = Array.from(lines, (line) => ({
        startTime: line.startTime,
        endTime: line.endTime,
        text: line.words.map((word) => word.word).join(''),
        translation: line.translatedLyric,
        romanization: line.romanLyric
      }))
      const expected = fixture.expected.map(({ words: _words, ...line }) => line)
      assert.deepEqual(actual, expected)
      if (fixture.lyrics.format === 'qrc' || fixture.lyrics.format === 'yrc') {
        assert.deepEqual(
          Array.from(lines, (line) =>
            Array.from(line.words, (word) => ({
              text: word.word,
              startTime: word.startTime,
              endTime: word.endTime
            }))
          ),
          fixture.expected.map((line) => line.words)
        )
      }
    })
  }
}
