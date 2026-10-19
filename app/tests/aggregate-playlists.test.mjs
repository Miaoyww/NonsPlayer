import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { createRuntimeCache } from './helpers/runtime-cache.mjs'

function load(file, modules) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(file, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  return exports
}

test('derivation needs selected ownership and read capability, never platform write capability', () => {
  let profile = { userId: '42', reference: { source: 'qqmusic', id: '42' } }
  let enabled = true
  const { useCanDerive } = load('../src/features/aggregate/derive-action.tsx', {
    'react/jsx-runtime': {},
    './sync-control': {},
    '@/features/account/account': { useAccount: () => ({ profile }) },
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({
        statuses: [
          { enabled, descriptor: { source: 'qqmusic', capabilities: ['browse', 'userLibrary'] } }
        ]
      })
    }
  })
  const item = {
    kind: 'playlist',
    reference: { source: 'qqmusic', kind: 'playlist', id: 'a' },
    creatorId: '42'
  }
  assert.equal(useCanDerive(item), true)
  assert.equal(useCanDerive({ ...item, creatorId: '43' }), false)
  assert.equal(useCanDerive({ ...item, liked: true }), false)
  assert.equal(useCanDerive({ ...item, localId: '1' }), false)
  enabled = false
  assert.equal(useCanDerive(item), false)
  enabled = true
  profile = null
  assert.equal(useCanDerive(item), false)
})

test('aggregate changes invalidate only their member pages, including pending reads', async () => {
  const { requestCache, requestKey, invalidateAggregateCache } = createRuntimeCache()
  let reads = 0
  const read = (id) =>
    requestCache.get(requestKey('aggregate_tracks', { id, offset: 0 }), async () => ++reads)
  assert.equal(await read('one'), 1)
  assert.equal(await read('two'), 2)
  let release
  const stale = requestCache.get(
    requestKey('aggregate_tracks', { id: 'one', offset: 100 }),
    () =>
      new Promise((resolve) => {
        release = resolve
      })
  )
  invalidateAggregateCache('one')
  release([])
  await assert.rejects(stale, { code: 'cacheInvalidated' })
  assert.equal(await read('two'), 2)
  assert.equal(await read('one'), 3)
})
