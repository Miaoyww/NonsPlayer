import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function load(path, modules = {}) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }).outputText,
    { exports, require: (name) => modules[name] }
  )
  return exports
}

test('single-song offset shifts words and lines together, clamps at zero and leaves source data intact', () => {
  const { offsetLyrics } = load('../src/features/lyrics/correction.ts')
  const lines = [
    {
      startTime: 500,
      endTime: 2000,
      words: [{ word: 'hello', startTime: 600, endTime: 1200 }],
      translatedLyric: '你好'
    }
  ]
  assert.equal(offsetLyrics(lines, 0), lines)
  const advanced = offsetLyrics(lines, -800)
  assert.equal(advanced[0].startTime, 0)
  assert.equal(advanced[0].endTime, 1200)
  assert.equal(advanced[0].words[0].startTime, 0)
  assert.equal(advanced[0].words[0].endTime, 400)
  assert.equal(lines[0].words[0].startTime, 600)
  assert.equal(offsetLyrics(lines, 300)[0].startTime, 800)
})

test('a manually selected online lyric overrides local mode and only retries QRC as LRC', async () => {
  const calls = []
  const { loadLyrics } = load('../src/features/lyrics/load-lyrics.ts', {
    '@/lib/diagnostics': { reportCaughtError() {} },
    '@/lib/player': {
      nativeCall: async (command, args) => {
        calls.push({ command, ...args })
        return args.skipQrc
          ? { source: 'qq', format: 'lrc', content: 'valid' }
          : { source: 'qq', format: 'qrc', content: 'bad' }
      }
    }
  })
  const value = await loadLyrics(
    { key: 'local:1', source: { kind: 'local' } },
    false,
    { localSource: 'local', amll: true, qq: true },
    (value) => {
      if (value.format === 'qrc') throw new Error('invalid QRC')
      return value.content
    },
    () => true,
    true
  )
  assert.equal(value.parsed, 'valid')
  assert.equal(calls.length, 2)
  assert.ok(calls.every((call) => call.command === 'track_lyrics'))
  assert.equal(calls[1].skipQrc, true)
})

test('invalid selected lyrics do not silently fall back to another source', async () => {
  let calls = 0
  const { loadLyrics } = load('../src/features/lyrics/load-lyrics.ts', {
    '@/lib/diagnostics': { reportCaughtError() {} },
    '@/lib/player': {
      nativeCall: async () => {
        calls++
        return { source: 'netease', format: 'lrc', content: 'bad' }
      }
    }
  })
  await assert.rejects(
    loadLyrics(
      { key: 'online:1', source: { kind: 'online' } },
      false,
      { amll: true, qq: true },
      () => {
        throw new Error('bad timeline')
      },
      () => true,
      true
    ),
    /bad timeline/
  )
  assert.equal(calls, 1)
})
