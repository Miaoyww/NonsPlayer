import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function load(storage = {}) {
  const values = new Map(Object.entries(storage))
  const exports = {}
  const source = readFileSync(
    new URL('../src/features/lyrics/use-lyrics-settings.ts', import.meta.url),
    'utf8'
  )
  runInNewContext(
    ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    {
      exports,
      localStorage: {
        getItem: (key) => values.get(key) ?? null,
        setItem: (key, value) => values.set(key, value)
      },
      require: (name) =>
        name === 'react'
          ? { useSyncExternalStore: (_subscribe, getSnapshot) => getSnapshot() }
          : undefined
    }
  )
  return { ...exports, values }
}

test('global lyric offset defaults to -200ms, persists valid values and ignores invalid ones', () => {
  const settings = load()
  assert.equal(settings.defaultLyricsOffsetMs, -200)
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, -200)
  settings.useLyricsSettings().setLyricsOffsetMs(350)
  assert.equal(settings.values.get('nons-lyrics-offset-ms'), '350')
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 350)
  settings.useLyricsSettings().setLyricsOffsetMs(60001)
  settings.useLyricsSettings().setLyricsOffsetMs(100.5)
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 350)
})

test('global lyric offset restores a valid saved value and falls back for invalid storage', () => {
  assert.equal(load({ 'nons-lyrics-offset-ms': '-450' }).useLyricsSettings().lyricsOffsetMs, -450)
  assert.equal(load({ 'nons-lyrics-offset-ms': '60001' }).useLyricsSettings().lyricsOffsetMs, -200)
})
