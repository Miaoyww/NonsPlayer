import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import * as amll from '@applemusic-like-lyrics/lyric'
import FastPriorityQueue from 'fastpriorityqueue'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

const matching = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/auxiliary-matching.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports: matching, require: () => FastPriorityQueue }
)

const localLrc = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/local-lrc.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports: localLrc, require: () => amll }
)

const metadata = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/netease-metadata.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports: metadata, require: () => ({}) }
)

const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/parse-lyrics.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  {
    exports,
    require: (name) =>
      name === './auxiliary-matching'
        ? matching
        : name === './local-lrc'
          ? localLrc
          : name === './netease-metadata'
            ? metadata
            : amll
  }
)

function local(content, translation = null, romanization = null) {
  return { source: 'local', format: 'lrc', content, translation, romanization }
}

function text(line) {
  return line.words.map((word) => word.word).join('')
}

function plain(value) {
  return JSON.parse(JSON.stringify(value))
}

test('local LRC splits inline original and translation', () => {
  const lines = exports.parseLyrics(
    local('[00:10.00]First line / 第一句\n[00:15.00]Second line／第二句'),
    30000
  )
  assert.deepEqual(plain(Array.from(lines, (line) => [text(line), line.translatedLyric])), [
    ['First line', '第一句'],
    ['Second line', '第二句']
  ])
})

test('local LRC groups interleaved lines with the same timestamp', () => {
  const lines = exports.parseLyrics(
    local('[00:10.00]First line\n[00:10.00]第一句\n[00:15.00]Second line'),
    30000
  )
  assert.deepEqual(plain(Array.from(lines, (line) => [text(line), line.translatedLyric])), [
    ['First line', '第一句'],
    ['Second line', '']
  ])
})

test('local LRC groups a separate translation block by timestamp', () => {
  const lines = exports.parseLyrics(
    local('[00:10.00]First line\n[00:15.00]Second line\n\n[00:10.00]第一句\n[00:15.00]第二句'),
    30000
  )
  assert.deepEqual(plain(lines.map((line) => [text(line), line.translatedLyric])), [
    ['First line', '第一句'],
    ['Second line', '第二句']
  ])
})

test('local LRC does not split slash text without a translation signal', () => {
  const lines = exports.parseLyrics(local('[00:10.00]AC/DC\n[00:15.00]Rock and roll'), 30000)
  assert.deepEqual(plain(lines.map((line) => [text(line), line.translatedLyric])), [
    ['AC/DC', ''],
    ['Rock and roll', '']
  ])
})

test('local companion translation and romanization fill auxiliary layers', () => {
  const lines = exports.parseLyrics(
    local(
      '[00:10.00]First line\n[00:15.00]Second line',
      '[00:10.00]第一句\n[00:15.00]第二句',
      '[00:10.00]first roman\n[00:15.00]second roman'
    ),
    30000
  )
  assert.deepEqual(
    plain(lines.map((line) => [text(line), line.translatedLyric, line.romanLyric])),
    [
      ['First line', '第一句', 'first roman'],
      ['Second line', '第二句', 'second roman']
    ]
  )
})

test('local inline translation keeps enhanced LRC word timing', () => {
  const [line] = exports.parseLyrics(
    local('[00:10.00]<00:10.00>Hello <00:10.50>world<00:11.00> / 你好世界'),
    30000
  )
  assert.equal(text(line), 'Hello world')
  assert.equal(line.translatedLyric, '你好世界')
  assert.deepEqual(
    plain(Array.from(line.words, (word) => [word.word, word.startTime, word.endTime])),
    [
      ['Hello ', 10000, 10500],
      ['world', 10500, 11000]
    ]
  )
})

test('online LRC keeps duplicate timestamps as separate lines', () => {
  const lines = exports.parseLyrics(
    { ...local('[00:10.00]First line\n[00:10.00]第一句'), source: 'netease' },
    30000
  )
  assert.equal(lines.length, 2)
  assert.deepEqual(plain(lines.map((line) => text(line))), ['First line', '第一句'])
})
