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
      setTimeout: (fn) => {
        fn()
        return 0
      },
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  return exports
}

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

function targetsHarness({ query, write, native, aggregate, current } = {}) {
  const account = { source: 'netease', id: '42' }
  const calls = []
  const api = load('../src/features/aggregate/playlist-targets.ts', {
    '@/features/music/client': {
      musicClient: {
        currentAccount: current ?? (async () => ({ reference: account })),
        query:
          query ??
          (async () => {
            throw new Error('unexpected query')
          }),
        write: write ?? (async (...args) => calls.push(['remote', ...args]))
      }
    },
    '@/features/music/entities': {
      musicCollection: (entity) => ({
        ...entity,
        id: entity.reference.id,
        kind: 'playlist',
        creatorId: entity.creator?.id
      })
    },
    '@/features/music/legacy-reference': {
      collectionReference: (item) => item.reference,
      onlineReference: (track) => track.reference ?? null
    },
    '@/lib/player': {
      nativeCall:
        native ??
        (async (command) => {
          if (command === 'aggregate_playlists') return []
          calls.push(command)
          return { key: 'task', running: false, playlistId: 'local', error: null }
        })
    },
    './aggregate-library': { writeAggregate: aggregate ?? (async (request) => calls.push(request)) }
  })
  return {
    ...api,
    calls,
    account,
    target: {
      item: { id: 'p', reference: { source: 'netease', kind: 'playlist', id: 'p' } },
      account,
      writable: true
    }
  }
}

test('same source adds remotely; foreign or local tracks create a local version; converted targets never write back', async () => {
  const h = targetsHarness()
  const same = { key: 'n', reference: { source: 'netease', kind: 'track', id: 'n' } }
  await h.addToPlaylist(h.target, [same])
  assert.equal(h.calls[0][0], 'remote')
  h.calls.length = 0
  await h.addToPlaylist({ ...h.target, writable: false }, [
    { key: 'qq', reference: { source: 'qqmusic', kind: 'track', id: 'q' } }
  ])
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls)), [
    'aggregate_sync',
    { operation: 'add', id: 'local', keys: ['qq'] }
  ])
  h.calls.length = 0
  await h.addToPlaylist(h.target, [{ key: 'file' }])
  assert.equal(h.calls[0], 'aggregate_sync')
  h.calls.length = 0
  await h.addToPlaylist(h.target, [same], 'existing')
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls)), [
    { operation: 'add', id: 'existing', keys: ['n'] }
  ])
})

test('conversion waits for complete synchronization and rejects failed reads or changed accounts before adding', async () => {
  let added = 0,
    checks = 0
  const h = targetsHarness({
    native: async (command) =>
      command === 'aggregate_playlists'
        ? []
        : command === 'aggregate_sync'
          ? { running: true, key: 'task' }
          : { running: false, playlistId: 'local', error: null },
    aggregate: async () => added++,
    current: async () => ({ reference: { source: 'netease', id: ++checks === 1 ? '42' : 'other' } })
  })
  await assert.rejects(h.addToPlaylist(h.target, [{ key: 'file' }]), /账号已变化/)
  assert.equal(added, 0)
  const failed = targetsHarness({
    native: async (command) =>
      command === 'aggregate_playlists' ? [] : { running: false, error: '后页失败' },
    aggregate: async () => added++
  })
  await assert.rejects(failed.addToPlaylist(failed.target, [{ key: 'file' }]), /后页失败/)
  assert.equal(added, 0)
})

test('owned targets traverse 30-item library pages and reject other owners; library grid avoids duplicate local versions', async () => {
  const pages = []
  const h = targetsHarness({
    query: async (source, request) => {
      pages.push(request.page)
      return {
        type: 'entities',
        data: {
          items: [
            {
              name: 'Mine',
              reference: { source, kind: 'playlist', id: request.page.cursor ?? 'first' },
              creator: { source, id: request.page.cursor ? 'other' : '42' }
            }
          ],
          nextCursor: request.page.cursor ? null : 'next'
        }
      }
    }
  })
  const result = await h.ownedPlaylistTargets('netease', false)
  assert.equal(result.length, 1)
  assert.equal(result[0].writable, false)
  assert.deepEqual(JSON.parse(JSON.stringify(pages)), [
    { cursor: null, limit: 30 },
    { cursor: 'next', limit: 30 }
  ])
  const remote = [{ id: 'remote' }],
    local = [
      { id: 'local', localId: 'local' },
      { id: 'independent', localId: 'independent' }
    ]
  assert.deepEqual(
    JSON.parse(JSON.stringify(h.mergeLibraryPlaylists(remote, local, () => ({ id: 'local' })))),
    [remote[0], local[1]]
  )
  assert.equal(h.mergeLibraryPlaylists([], local, () => undefined).length, 2)
})

test('a persisted conversion overrides stale presentation state before a same-platform remote write', async () => {
  const h = targetsHarness({
    native: async (command) => {
      assert.equal(command, 'aggregate_playlists')
      return [
        {
          id: 'persisted',
          link: {
            reference: { source: 'netease', id: 'p' },
            account: { source: 'netease', id: '42' }
          }
        }
      ]
    }
  })
  await h.addToPlaylist(h.target, [{ key: 'n', reference: { source: 'netease', id: 'n' } }])
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls)), [
    { operation: 'add', id: 'persisted', keys: ['n'] }
  ])
})
