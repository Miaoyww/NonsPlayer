import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInThisContext } from 'node:vm'
import ts from 'typescript'
import { createRuntimeCache } from './helpers/runtime-cache.mjs'

const code = ts.transpileModule(
  readFileSync(new URL('../src/features/settings/cache-management.ts', import.meta.url), 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } }
).outputText

function harness(failNative = false) {
  const cache = createRuntimeCache()
  const calls = []
  let previews = 1
  let continuations = 1
  const exports = {}
  const nativeStatus = Object.fromEntries(
    ['covers', 'collections', 'metadata', 'compiled', 'logs'].map((kind) => [
      kind,
      { usedBytes: 100, entries: 1 }
    ])
  )
  const modules = {
    '@/lib/runtime-cache': cache,
    '@/lib/player': {
      nativeCall: async (command, args) => {
        calls.push({ command, args })
        if (failNative) throw new Error('disk unavailable')
        return nativeStatus
      }
    },
    '@/features/library/library-preview': {
      clearLibraryPreviews: () => (previews = 0),
      libraryPreviewCacheStatus: () => ({ usedBytes: previews * 40, entries: previews })
    },
    '@/features/music/queries': { clearContinuations: () => (continuations = 0) }
  }
  runInThisContext(`(function(exports, require) { ${code}\n })`)(exports, (name) => {
    assert.ok(name in modules, name)
    return modules[name]
  })
  return { ...exports, ...cache, calls, nativeStatus, remaining: () => [previews, continuations] }
}

test('other cache statistics report disk cover payloads without adding frontend path metadata', async () => {
  const app = harness()
  app.requestCache.set('track_lyrics:one', 'lyrics')
  app.requestCache.set('music_query:one', 'metadata')
  app.coverCache.set('runtime_cover:one', 'cover')
  const queries = app.requestCache.stats(app.isOtherQueryCacheKey)
  const status = await app.otherCacheStatus()
  assert.equal(status.queries.entries, 2)
  assert.equal(status.queries.usedBytes, queries.usedBytes + 40)
  assert.equal(status.covers.usedBytes, 100)
  assert.equal(status.covers.entries, 1)
  assert.equal(app.requestCache.peek('track_lyrics:one'), 'lyrics')
})

test('clearing queries rejects old in-flight results while preserving lyrics and covers', async () => {
  const app = harness()
  app.requestCache.set('track_lyrics:one', 'lyrics')
  app.coverCache.set('runtime_cover:one', 'cover')
  let finish
  const pending = app.requestCache.get(
    'music_query:pending',
    () => new Promise((r) => (finish = r))
  )
  const rejection = assert.rejects(pending, app.CacheInvalidatedError)
  await app.clearOtherCache('queries')
  await rejection
  finish('late')
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(app.requestCache.peek('music_query:pending'), undefined)
  assert.equal(app.requestCache.peek('track_lyrics:one'), 'lyrics')
  assert.equal(app.coverCache.peek('runtime_cover:one'), 'cover')
  assert.deepEqual(app.remaining(), [0, 0])
  assert.equal(app.calls.length, 0)
})

test('clearing playlist snapshots also drops derived query pages without deleting lyrics', async () => {
  const app = harness()
  app.requestCache.set('track_lyrics:one', 'lyrics')
  app.requestCache.set('music_query:one', 'old page')
  await app.clearOtherCache('collections')
  assert.deepEqual(app.calls, [{ command: 'clear_other_cache', args: { kind: 'collections' } }])
  assert.equal(app.requestCache.peek('music_query:one'), undefined)
  assert.equal(app.requestCache.peek('track_lyrics:one'), 'lyrics')
  assert.deepEqual(app.remaining(), [0, 0])
})

test('logs clear independently and a failed cover cleanup preserves the frontend cache', async () => {
  const app = harness()
  app.coverCache.set('runtime_cover:one', 'cover')
  app.requestCache.set('music_query:one', 'page')
  await app.clearOtherCache('logs')
  assert.equal(app.coverCache.peek('runtime_cover:one'), 'cover')
  assert.equal(app.requestCache.peek('music_query:one'), 'page')
  assert.deepEqual(app.remaining(), [1, 1])
  const failed = harness(true)
  failed.coverCache.set('runtime_cover:one', 'cover')
  await assert.rejects(failed.clearOtherCache('covers'), /disk unavailable/)
  assert.equal(failed.coverCache.peek('runtime_cover:one'), 'cover')
})
