import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

const track = (key) => ({
  key: String(key),
  source: {
    kind: 'online',
    track: { reference: { source: 'qqmusic', kind: 'track', id: String(key) } }
  }
})
const settle = async () => {
  for (let i = 0; i < 12; i++) await Promise.resolve()
}
function harness() {
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/discovery/private-fm.ts', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS }
      }
    ).outputText,
    {
      exports,
      require: (name) => (name === '@/features/music/references' ? musicReference : {})
    }
  )
  let state = {
    privateFmSession: 7,
    queue: [1, 2, 3].map(track),
    index: 0,
    status: 'playing',
    shuffle: false
  }
  const calls = [],
    errors = []
  const fm = exports.createPrivateFm(
    () => state,
    (command, args) =>
      new Promise((resolve, reject) => calls.push({ command, args, resolve, reject })),
    (error) => errors.push(error),
    () =>
      new Promise((resolve, reject) =>
        calls.push({
          command: 'music_query',
          args: {
            source: 'qqmusic',
            refresh: true,
            request: { operation: 'recommendedTracks', kind: 'fm' }
          },
          resolve,
          reject
        })
      )
  )
  return {
    fm,
    calls,
    errors,
    state: () => state,
    update(patch) {
      state = { ...state, ...patch }
      fm.update()
    }
  }
}

test('FM replenishes at the penultimate song, deduplicates batches and continues across batches', async () => {
  const app = harness()
  app.fm.update()
  assert.equal(app.calls.length, 0)
  app.update({ index: 1 })
  app.fm.update()
  app.update({ status: 'buffering' })
  assert.equal(app.calls.length, 1)
  assert.equal(app.calls[0].args.refresh, true)
  app.calls[0].resolve([3, 4, 4, 5, 6].map(track))
  await settle()
  assert.equal(app.calls[1].command, 'append_private_fm')
  assert.deepEqual(Array.from(app.calls[1].args.keys), ['4', '5', '6'])
  assert.equal(app.calls[1].args.session, 7)
  assert.equal(app.calls[1].args.queueLen, 3)
  app.update({ queue: [1, 2, 3, 4, 5, 6].map(track), status: 'playing' })
  app.calls[1].resolve()
  await settle()
  app.update({ index: 4 })
  assert.equal(app.calls[2].command, 'music_query')
})

test('queue edits discard in-flight batches and do not request more songs', async () => {
  for (const queue of [[1, 2, 3, 9], [1, 3], [], [8, 9, 10]]) {
    const app = harness()
    app.update({ index: 1 })
    app.update({ queue: queue.map(track), privateFmSession: null })
    app.calls[0].resolve([4, 5, 6].map(track))
    await settle()
    app.update({ index: 0 })
    assert.equal(app.calls.length, 1)
  }
})

test('a restarted FM session ignores the previous session response', async () => {
  const app = harness()
  app.update({ index: 1 })
  app.update({ privateFmSession: 8, queue: [7, 8, 9].map(track) })
  app.calls[0].resolve([4, 5, 6].map(track))
  await settle()
  assert.equal(app.calls.length, 2)
  app.calls[1].resolve([10, 11, 12].map(track))
  await settle()
  assert.equal(app.calls[2].args.session, 8)
})

test('failed and duplicate-only batches do not loop and can be retried', async () => {
  for (const duplicateOnly of [false, true]) {
    const app = harness()
    app.update({ index: 1 })
    if (duplicateOnly) app.calls[0].resolve([1, 2, 3].map(track))
    else app.calls[0].reject(new Error('offline'))
    await settle()
    app.fm.update()
    assert.equal(app.calls.length, 1)
    assert.ok(app.errors.at(-1))
    app.fm.retry()
    assert.equal(app.calls.length, 2)
  }
})

test('a fast skip stays serial and retries at the last song after a failure', async () => {
  const app = harness()
  app.update({ index: 1 })
  app.update({ index: 2 })
  assert.equal(app.calls.length, 1)
  app.calls[0].reject(new Error('offline'))
  await settle()
  assert.equal(app.calls.length, 2)
})

test('bounded queues still replenish after the historical prefix is trimmed', async () => {
  const app = harness()
  app.update({ queue: Array.from({ length: 1000 }, (_, i) => track(i)), index: 998 })
  app.calls[0].resolve([1000, 1001, 1002].map(track))
  await settle()
  app.update({ queue: Array.from({ length: 1000 }, (_, i) => track(i + 3)), index: 995 })
  app.calls[1].resolve()
  await settle()
  app.update({ index: 998 })
  assert.equal(app.calls[2].command, 'music_query')
})

test('the global player connection replenishes without mounting the discovery page', async () => {
  const load = (path, modules = {}) => {
    const exports = {}
    runInNewContext(
      ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS }
      }).outputText,
      {
        exports,
        performance: { now: () => 0 },
        require: (name) =>
          name === '@/lib/diagnostics'
            ? { diagnosticInvoke: modules['@tauri-apps/api/core']?.invoke }
            : (modules[name] ?? {})
      }
    )
    return exports
  }
  const listeners = new Map(),
    calls = []
  const state = {
    revision: 1,
    playbackOptions: { fades: false, crossfade: false, crossfadeDurationMs: 3000 },
    privateFmSession: 7,
    queue: [1, 2, 3].map(track),
    index: 0,
    status: 'playing',
    shuffle: false
  }
  const player = load('../src/lib/player.ts', {
    '@/lib/platform': { isAndroid: () => false },
    '@/features/playback/player-store': load('../src/features/playback/player-store.ts'),
    '@/features/music/scoped-query': { ScopedMusicQueries: class {} },
    '@/features/discovery/private-fm': load('../src/features/discovery/private-fm.ts', {
      '@/features/music/references': musicReference,
      './discovery-api': {
        discoveryTracks: async () => {
          calls.push({
            command: 'music_query',
            args: { source: 'qqmusic', request: { operation: 'recommendedTracks', kind: 'fm' } }
          })
          return [4, 5, 6].map(track)
        }
      }
    }),
    '@/lib/runtime-cache': { cachedCommands: new Set() },
    '@tauri-apps/api/event': {
      listen: async (event, callback) => {
        listeners.set(event, callback)
        return () => {}
      }
    },
    '@tauri-apps/api/core': {
      isTauri: () => true,
      invoke: async (command, args) => {
        calls.push({ command, args })
        if (command === 'player_snapshot')
          return {
            state: { ...state, sequence: 0, queueRevision: 0 },
            queue: { queue: state.queue, shuffleOrder: [], queueRevision: 0, sequence: 0 },
            progress: {
              revision: 1,
              sequence: 0,
              positionMs: 0,
              durationMs: 180000,
              status: 'playing'
            }
          }
        if (command === 'music_query') return [4, 5, 6].map(track)
      }
    }
  })
  const disconnect = await player.connectPlayer()
  listeners.get('player-state')({
    payload: { ...state, sequence: 1, queueRevision: 0, revision: 2, index: 1 }
  })
  await settle()
  assert.deepEqual(
    calls.map((call) => call.command),
    ['player_snapshot', 'music_query', 'append_private_fm']
  )
  disconnect()
})
