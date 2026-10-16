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

const fixture = JSON.parse(readFileSync(new URL('./fixtures/qrc.json', import.meta.url), 'utf8'))
const exports = {}
const { outputText } = ts.transpileModule(
  readFileSync(new URL('../src/features/lyrics/parse-lyrics.ts', import.meta.url), 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } }
)
runInNewContext(outputText, {
  exports,
  require: (name) => (name === './auxiliary-matching' ? matching : amll)
})
const lyrics = {
  source: 'qq',
  format: 'qrc',
  content: fixture.main,
  translation: '[00:01.00]你好世界\n[00:03.00]再次',
  romanization: '[1000,1000]hello (1000,400)world(1400,600)'
}

test("QRC preserves each word's start and end through the actual AMLL parser", () => {
  const lines = exports.parseLyrics(lyrics, 4000)
  assert.equal(lines.length, 2)
  assert.deepEqual(
    Array.from(lines[0].words, (word) => [word.word, word.startTime, word.endTime]),
    [
      ['Hello ', 1000, 1400],
      ['world', 1400, 2000]
    ]
  )
  assert.equal(lines[0].startTime, 1000)
  assert.equal(lines[0].endTime, 2000)
  assert.equal(lines[0].translatedLyric, '你好世界')
  assert.equal(lines[0].romanLyric, 'hello world')
  assert.equal(lines[1].translatedLyric, '再次')
  assert.equal(
    exports
      .parseLyricsContent(lyrics)[0]
      .words.map((word) => word.word)
      .join(''),
    'Hello world'
  )
})

test('empty or malformed QRC triggers fallback; LRC and YRC still parse', () => {
  assert.throws(
    () => exports.parseLyrics({ ...lyrics, content: '<html>error</html>' }, 4000),
    exports.EmptyLyricsError
  )
  assert.throws(
    () => exports.parseLyrics({ ...lyrics, content: '[1000,1000]no word timestamps' }, 4000),
    exports.EmptyLyricsError
  )
  for (const [format, content] of [
    ['lrc', '[00:01.00]Hello'],
    ['yrc', '[1000,1000](1000,1000,0)Hello']
  ]) {
    assert.equal(
      exports.parseLyrics({ ...lyrics, format, content }, 4000)[0].words[0].word,
      'Hello'
    )
  }
})

test('translation and romanization tolerate timestamp drift through AMLL parsing', () => {
  for (const [format, content] of [
    ['qrc', '[1000,500]First(1000,500)\n[3000,500]Second(3000,500)'],
    ['yrc', '[1000,500](1000,500,0)First\n[3000,500](3000,500,0)Second'],
    ['lrc', '[00:01.00]First\n[00:03.00]Second']
  ]) {
    const result = exports.parseLyrics(
      {
        ...lyrics,
        format,
        content,
        translation: '[00:00.98]第一句\n[00:03.50]第二句',
        romanization: '[00:01.03]first\n[00:02.96]second'
      },
      4000
    )
    assert.deepEqual(
      Array.from(result, (line) => line.translatedLyric),
      ['第一句', '第二句']
    )
    assert.deepEqual(
      Array.from(result, (line) => line.romanLyric),
      ['first', 'second']
    )
  }
})

test('auxiliary matching prefers the nearest nonempty line and does not carry distant text', () => {
  const result = exports.parseLyrics(
    {
      ...lyrics,
      format: 'lrc',
      content: '[00:01.00]First\n[00:03.00]Second\n[00:05.00]Third',
      translation:
        '[00:00.60]较远\n[00:00.99]第一句\n[00:01.00]\n[00:03.00]第二句\n[00:03.01]较近\n[00:05.501]不应匹配',
      romanization: null
    },
    6000
  )
  assert.deepEqual(
    Array.from(result, (line) => line.translatedLyric),
    ['第一句', '第二句', '']
  )
})

test('word-level auxiliary text is parsed by the corresponding AMLL parser', () => {
  const result = exports.parseLyrics(
    {
      ...lyrics,
      translation: '[980,500](980,500,0)第一句\n[3010,500](3010,500,0)第二句',
      romanization: '[1020,500]first(1020,500)\n[2990,500]second(2990,500)'
    },
    4000
  )
  assert.deepEqual(
    Array.from(result, (line) => line.translatedLyric),
    ['第一句', '第二句']
  )
  assert.deepEqual(
    Array.from(result, (line) => line.romanLyric),
    ['first', 'second']
  )
})

test('slash placeholders disappear from translation and romanization without changing lyrics', () => {
  const result = exports.parseLyrics(
    {
      ...lyrics,
      format: 'lrc',
      content: '[00:01.00]Produced by: Example\n[00:03.00]A // B',
      translation: '[00:01.00] // \n[00:03.00]译文 // 内容',
      romanization: '[00:01.00]//\n[00:03.00]words / words'
    },
    4000
  )
  assert.equal(result[0].translatedLyric, '')
  assert.equal(result[0].romanLyric, '')
  assert.equal(result[1].translatedLyric, '译文 // 内容')
  assert.equal(result[1].words[0].word, 'A // B')
  assert.equal(result[1].romanLyric, 'words / words')
})

test('dense production credits never borrow the next vocal line translation', () => {
  const times = [23910, 23920, 23921, 23924, 23928, 23930, 29880]
  const result = exports.parseLyrics(
    {
      ...lyrics,
      content: times.map((time, index) => `[${time},100]Line ${index}(${time},100)`).join('\n'),
      translation:
        '[00:23.910]//\n[00:23.920]//\n[00:23.920]//\n[00:23.920]//\n[00:23.920]//\n[00:23.930]译文一\n[00:29.880]译文二',
      romanization: null
    },
    31000
  )
  assert.deepEqual(
    Array.from(result, (line) => line.translatedLyric),
    ['', '', '', '', '', '译文一', '译文二']
  )
})

test('equal timestamps pair translations and romanization in source order exactly once', () => {
  const result = exports.parseLyrics(
    {
      ...lyrics,
      content:
        '[1000,500]Voice A(1000,500)\n[1000,500]Voice B(1000,500)\n[1100,500]Untranslated(1100,500)',
      translation: '[00:01.00]甲\n[00:01.00]乙',
      romanization: '[00:01.00]A\n[00:01.00]B'
    },
    2000
  )
  assert.deepEqual(
    Array.from(result, (line) => line.translatedLyric),
    ['甲', '乙', '']
  )
  assert.deepEqual(
    Array.from(result, (line) => line.romanLyric),
    ['A', 'B', '']
  )
})

test('exact matches win over earlier nearby lines even without placeholder entries', () => {
  const result = exports.parseLyrics(
    {
      ...lyrics,
      format: 'lrc',
      content: '[00:01.00]Credit\n[00:01.10]Vocal',
      translation: '[00:01.10]译文',
      romanization: null
    },
    2000
  )
  assert.deepEqual(
    Array.from(result, (line) => line.translatedLyric),
    ['', '译文']
  )
})

test('bounded matching agrees with exhaustive nearest-pair ordering including ties and missing lines', () => {
  let seed = 12345
  const next = () => {
    seed = (seed * 1664525 + 1013904223) >>> 0
    return seed
  }
  for (let sample = 0; sample < 500; sample++) {
    const starts = Array.from({ length: next() % 15 }, () => (next() % 20) * 100)
    const entries = Array.from({ length: next() % 15 }, (_, i) => ({
      start: (next() % 20) * 100,
      text: `${i}`
    }))
    const pairs = starts
      .flatMap((start, line) =>
        entries.map((entry, index) => ({
          line,
          entry: index,
          distance: Math.abs(start - entry.start)
        }))
      )
      .filter((pair) => pair.distance <= 500)
      .sort((a, b) => a.distance - b.distance || a.line - b.line || a.entry - b.entry)
    const expected = Array(starts.length).fill(undefined)
    const used = new Set()
    for (const pair of pairs) {
      if (expected[pair.line] !== undefined || used.has(pair.entry)) continue
      expected[pair.line] = entries[pair.entry].text
      used.add(pair.entry)
    }
    assert.deepEqual(Array.from(matching.matchAuxiliary(starts, entries)), expected)
  }
})

test('large equal-time groups preserve order without a pair matrix', () => {
  const entries = Array.from({ length: 20000 }, (_, i) => ({ start: 1000, text: `${i}` }))
  const result = matching.matchAuxiliary(Array(20001).fill(1000), entries)
  assert.equal(result[0], '0')
  assert.equal(result[19999], '19999')
  assert.equal(result[20000], undefined)
})
