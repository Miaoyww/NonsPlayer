import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

const settle = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve()
}
async function harness({ delayed = false } = {}) {
  let now = 0,
    sequence = 0,
    resolveInitial
  const listeners = new Map(),
    subscriptions = []
  const state = {
    sequence: 0,
    queueRevision: 0,
    revision: 1,
    index: 0,
    status: 'playing',
    volume: 0.8,
    privateFmSession: null,
    shuffle: false,
    repeatMode: 'off',
    playbackOptions: { fades: false, crossfade: false, crossfadeDurationMs: 3000 },
    deviceId: null,
    actualQuality: null,
    error: null,
    mediaError: null
  }
  const queue = { sequence: 0, queueRevision: 0, queue: [{ key: 'A' }], shuffleOrder: [] }
  const progress = {
    sequence: 0,
    revision: 1,
    positionMs: 30000,
    durationMs: 180000,
    status: 'playing'
  }
  const initial = { state, queue, progress }
  const deferred = new Promise((resolve) => {
    resolveInitial = resolve
  })
  const compile = (path, modules = {}) => {
    const exports = {}
    runInNewContext(
      ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS }
      }).outputText,
      {
        exports,
        performance: { now: () => now },
        require: (name) => {
          if (name === '@/lib/diagnostics')
            return { diagnosticInvoke: modules['@tauri-apps/api/core'].invoke }
          assert.ok(Object.hasOwn(modules, name), `unexpected dependency: ${name}`)
          return modules[name]
        }
      }
    )
    return exports
  }
  const modules = {
    '@/features/music/cover-image': coverImage,
    '@/features/library/library-preview': {
      clearLibraryPreviews() {},
      peekLibraryPreview() {},
      saveLibraryPreview() {}
    },
    './library-preview': {
      clearLibraryPreviews() {},
      peekLibraryPreview() {},
      saveLibraryPreview() {}
    },
    '@/lib/platform': { isAndroid: () => false },
    '@/features/playback/player-store': compile('../src/features/playback/player-store.ts'),
    '@/features/music/scoped-query': { ScopedMusicQueries: class {} },
    '@/features/discovery/private-fm': { createPrivateFm: () => ({ update() {}, dispose() {} }) },
    '@tauri-apps/api/core': {
      isTauri: () => true,
      invoke: async () => (delayed ? deferred : initial)
    },
    '@tauri-apps/api/event': {
      listen: async (name, listener) => {
        listeners.set(name, listener)
        return () => listeners.delete(name)
      }
    },
    react: {
      useMemo: (fn) => fn(),
      useSyncExternalStore(subscribe, getSnapshot) {
        const subscription = { value: getSnapshot(), renders: 0 }
        subscribe(() => {
          const next = getSnapshot()
          if (!Object.is(subscription.value, next)) {
            subscription.value = next
            subscription.renders++
          }
        })
        subscriptions.push(subscription)
        return subscription.value
      }
    },
    '@/lib/runtime-cache': { cachedCommands: new Set() }
  }
  const player = compile('../src/lib/player.ts', modules)
  const connecting = player.connectPlayer()
  await settle()
  if (!delayed) await connecting
  const emit = (name, payload) => listeners.get(name)({ payload })
  return {
    player,
    subscriptions,
    connecting,
    initial,
    finishBootstrap: (value = initial) => resolveInitial(value),
    time(value) {
      now = value
    },
    state(patch) {
      emit('player-state', { ...state, sequence: ++sequence, ...patch })
    },
    progress(patch) {
      emit('player-progress', { ...progress, sequence: ++sequence, ...patch })
    },
    queue(patch) {
      emit('player-queue', { ...queue, sequence: ++sequence, ...patch })
    },
    emit
  }
}

test('rapid volume events preserve progress and queue, lyrics and status selections', async () => {
  const app = await harness()
  app.player.usePlayer('queue', 'index')
  app.player.usePlayer('queue', 'index', 'revision', 'status')
  app.player.usePlayer('status')
  app.player.usePlayer('volume')
  app.player.useProgress()
  app.player.usePlayer('playbackOptions', 'deviceId')
  for (let now = 20; now <= 200; now += 20) {
    app.time(now)
    const before = app.player.currentPosition()
    app.state({
      volume: now / 200,
      playbackOptions: { fades: false, crossfade: false, crossfadeDurationMs: 3000 }
    })
    assert.equal(app.player.currentPosition(), before)
  }
  assert.deepEqual(
    app.subscriptions.map((s) => s.renders),
    [0, 0, 0, 10, 0, 0]
  )
  app.progress({ positionMs: 30200 })
  app.time(240)
  app.state({ volume: 0.5 })
  assert.equal(app.player.currentPosition(), 30240)
})

test('seek, pause/resume, buffering and track revisions consume progress events', async () => {
  const app = await harness()
  app.time(100)
  app.progress({ positionMs: 10000 })
  assert.equal(app.player.currentPosition(), 10000)
  app.time(150)
  app.state({ status: 'paused' })
  app.progress({ status: 'paused', positionMs: 10050 })
  app.time(200)
  assert.equal(app.player.currentPosition(), 10050)
  app.state({ status: 'playing' })
  app.progress({ positionMs: 10050 })
  app.time(250)
  assert.equal(app.player.currentPosition(), 10100)
  app.progress({ status: 'buffering', positionMs: 10100 })
  app.time(300)
  assert.equal(app.player.currentPosition(), 10100)
  app.state({ revision: 2 })
  app.progress({ revision: 2, positionMs: 0 })
  assert.equal(app.player.currentPosition(), 0)
  app.progress({ revision: 1, positionMs: 99000 })
  assert.equal(app.player.currentPosition(), 0)
})

test('queue and state pair atomically in either delivery order, including clear', async () => {
  for (const order of ['queue-first', 'state-first']) {
    const app = await harness()
    const previous = app.player.getPlayer()
    if (order === 'queue-first')
      app.queue({ queueRevision: 1, queue: [{ key: 'B' }], shuffleOrder: [0] })
    app.state({ queueRevision: 1, revision: 2 })
    if (order === 'state-first') {
      assert.equal(app.player.getPlayer(), previous)
      app.queue({ queueRevision: 1, queue: [{ key: 'B' }], shuffleOrder: [0] })
    }
    assert.equal(app.player.getPlayer().queue[0].key, 'B')
    assert.equal(previous.queue[0].key, 'A')
    app.queue({ queueRevision: 2, queue: [], shuffleOrder: [] })
    assert.equal(app.player.getPlayer().queue[0].key, 'B')
    app.state({ queueRevision: 2, revision: 3, index: null, status: 'stopped' })
    assert.equal(app.player.getPlayer().queue.length, 0)
    assert.equal(app.player.getPlayer().index, null)
  }
})

test('bootstrap retains the baseline queue when a volume event overtakes IPC', async () => {
  const app = await harness({ delayed: true })
  app.state({ volume: 0.3 })
  app.finishBootstrap()
  await app.connecting
  assert.equal(app.player.getPlayer().queue[0].key, 'A')
  assert.equal(app.player.getPlayer().volume, 0.3)
  assert.equal(app.player.currentPosition(), 30000)
})

test('progress before replacement state and bootstrap is retained; stale events ignored', async () => {
  const app = await harness({ delayed: true })
  app.progress({ revision: 2, positionMs: 500 })
  app.queue({ queueRevision: 1, queue: [{ key: 'B' }] })
  app.state({ queueRevision: 1, revision: 2 })
  app.finishBootstrap()
  await app.connecting
  assert.equal(app.player.getPlayer().queue[0].key, 'B')
  assert.equal(app.player.currentPosition(), 500)
  app.emit('player-state', app.initial.state)
  app.emit('player-queue', app.initial.queue)
  app.emit('player-progress', app.initial.progress)
  assert.equal(app.player.getPlayer().revision, 2)
  assert.equal(app.player.currentPosition(), 500)
})

test('reconnection gets a complete snapshot without queue changes', async () => {
  const app = await harness()
  const cleanup = await app.connecting
  cleanup()
  const stop = await app.player.connectPlayer()
  assert.equal(app.player.getPlayer().queue[0].key, 'A')
  assert.equal(app.player.currentPosition(), 30000)
  stop()
})
