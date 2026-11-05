import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/mv/mv-lyric-line.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports }
)

function line(text, startTime, endTime, extra = {}) {
  return {
    words: [{ word: text, startTime, endTime }],
    translatedLyric: '',
    romanLyric: '',
    startTime,
    endTime,
    isBG: false,
    isDuet: false,
    ...extra
  }
}

test('the MV lyric card keeps the latest line through gaps and switches at the next line', () => {
  const lines = [line('first', 1_000, 2_000), line('second', 2_000, 3_500)]
  assert.equal(exports.findLyricLine(lines, 500), null)
  assert.equal(exports.findLyricLine(lines, 1_000).words[0].word, 'first')
  assert.equal(exports.findLyricLine(lines, 1_999).words[0].word, 'first')
  assert.equal(exports.findLyricLine(lines, 2_000).words[0].word, 'second')
  assert.equal(exports.findLyricLine(lines, 3_500).words[0].word, 'second')
})

test('the MV lyric card retains the previous line when a line ends before the next begins', () => {
  const lines = [line('first', 1_000, 1_500), line('second', 3_000, 3_500)]
  assert.equal(exports.findLyricLine(lines, 2_200).words[0].word, 'first')
})

test('background and empty lines never replace the main lyric', () => {
  const lines = [
    line('main', 1_000, 3_000),
    line('background', 1_100, 3_000, { isBG: true }),
    line('   ', 1_200, 3_000)
  ]
  assert.equal(exports.findLyricLine(lines, 1_500).words[0].word, 'main')
})

test('invalid playback positions leave the card empty', () => {
  assert.equal(exports.findLyricLine([line('main', 1_000, 2_000)], Number.NaN), null)
  assert.equal(exports.findLyricLine([], 1_000), null)
})
