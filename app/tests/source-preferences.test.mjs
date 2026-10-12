import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

test('changing current source preserves configured roles, order and revision without changing accounts', () => {
  const exports = {}
  const calls = []
  const code = ts.transpileModule(
    readFileSync(new URL('../src/features/music/source-preferences.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText
  runInNewContext(code, {
    exports,
    require: () => ({
      nativeCall: (...args) => {
        calls.push(args)
      }
    })
  })
  const saved = { current: 'netease', supplementary: ['third', 'qqmusic'], revision: 3 }
  const next = exports.selectCurrentSource(saved, 'qqmusic')
  assert.equal(next.current, 'qqmusic')
  assert.deepEqual(Array.from(next.supplementary), ['netease', 'third'])
  assert.equal(next.revision, 3)
  assert.deepEqual(saved.supplementary, ['third', 'qqmusic'])
  assert.equal(exports.selectCurrentSource(saved, 'netease'), saved)
  assert.deepEqual(
    Array.from(exports.selectCurrentSource(saved, 'new').supplementary),
    saved.supplementary
  )
  exports.sourcePreferences.set(next)
  assert.equal(calls[0][0], 'music_set_source_preferences')
  assert.equal(calls.length, 1)
})
