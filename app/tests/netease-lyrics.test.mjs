import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'
import * as amll from '@applemusic-like-lyrics/lyric'
import FastPriorityQueue from 'fastpriorityqueue'

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

const lyrics = (content) => ({
  source: 'netease',
  format: 'lrc',
  content,
  translation: null,
  romanization: null
})
const text = (line) => line.words.map((word) => word.word).join('')

test('NetEase metadata JSON lines are removed before parsing', () => {
  const content = [
    '{"t":0,"c":[{"tx":"作词: "},{"tx":"陈可心"}]}',
    '[00:01.741]',
    '[00:02.422]天雷惊吼 风涌 竹影抖',
    '[00:07.171]长缨擎空 剑舞 宿命咒',
    '{"t":183811,"c":[{"tx":"录音师 Recording Engineer: "},{"tx":"徐志明"}]}',
    '{"t":184882,"c":[{"tx":"平面设计Graphic Design: "},{"tx":"候e"}]}'
  ].join('\n')

  const lines = exports.parseLyrics(lyrics(content), 200_000)
  assert.deepEqual(lines.map(text), ['天雷惊吼 风涌 竹影抖', '长缨擎空 剑舞 宿命咒'])
  assert.ok(lines.every((line) => !text(line).includes('{"t"')))
})

test('timestamped NetEase metadata lines do not become lyric or translation text', () => {
  const content = [
    '[00:02.422]天雷惊吼',
    '[00:02.422]{"t":184882,"c":[{"tx":"平面设计Graphic Design: "},{"tx":"候e"}]}',
    '[00:07.171]长缨擎空'
  ].join('\n')

  const lines = exports.parseLyrics(lyrics(content), 200_000)
  assert.deepEqual(lines.map(text), ['天雷惊吼', '长缨擎空'])
  assert.ok(lines.every((line) => !line.translatedLyric.includes('{"t"')))
})

test('word and line ends are repaired after parsing', () => {
  const content = [
    '[03:03.678]<03:03.679>a<03:03.679>b<03:03.679>c<03:03.679>d<03:03.679>e[03:03.678]',
    '[03:03.678]{"t":184882,"c":[{"tx":"平面设计Graphic Design: "},{"tx":"候e"}]}',
    '[03:05.000]next'
  ].join('\n')

  const lines = exports.parseLyrics(lyrics(content), 200_000)
  for (const line of lines) {
    assert.ok(line.endTime >= line.startTime)
    for (const word of line.words) assert.ok(word.endTime >= word.startTime)
  }
  assert.equal(lines[0].endTime, 185_000)
  assert.equal(lines[0].words[4].endTime, 185_000)
})

test('ordinary JSON-like lyric text is preserved', () => {
  const line = '{"foo":"bar"}'
  assert.deepEqual(
    exports
      .parseLyricsContent(lyrics(`[00:01.000]${line}`))
      .map((value) => value.words.map((word) => word.word).join('')),
    [line]
  )
})
