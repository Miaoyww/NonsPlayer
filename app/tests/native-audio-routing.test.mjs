import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

const sources = Object.fromEntries(
  ['lib/player.ts', 'lib/platform.ts', 'features/playback/player-store.ts'].map((path) => [
    path,
    ts.transpileModule(readFileSync(new URL(`../src/${path}`, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }).outputText
  ])
)

function harness({ platform = 'Windows', tauri = true, readyError } = {}) {
  const navigator = { userAgent: platform }
  const calls = []
  const required = []
  const registered = []
  const unlistened = []
  const listeners = new Map()
  const applied = []
  const errors = []
  let executors = 0
  let disposed = 0
  let sendFeedback
  const fm = { created: 0, updated: 0, disposed: 0 }
  const state = {
    sequence: 1,
    queueRevision: 1,
    revision: 1,
    index: null,
    status: 'paused',
    volume: platform === 'Android' ? 1 : 0.8,
    privateFmSession: null,
    shuffle: false,
    repeatMode: 'off',
    playbackOptions: { fades: true, crossfade: true, crossfadeDurationMs: 6000 },
    deviceId: 'native-output-id',
    actualQuality: null,
    error: null,
    mediaError: null
  }
  const progress = {
    sequence: 1,
    revision: 1,
    positionMs: 12000,
    durationMs: 180000,
    status: 'paused'
  }
  const bootstrap = {
    state,
    queue: { sequence: 1, queueRevision: 1, queue: [], shuffleOrder: [] },
    progress
  }
  const control = {
    serial: 1,
    source: null,
    seekId: 0,
    positionMs: 12000,
    startMs: 0,
    endMs: null,
    playing: false,
    volume: 1,
    fades: true,
    crossfadeMs: 6000,
    deviceId: null,
    next: null
  }
  function compile(path, modules = {}) {
    const exports = {}
    runInNewContext(sources[path], {
      exports,
      navigator,
      performance: { now: () => 100 },
      require(name) {
        required.push(name)
        assert.ok(Object.hasOwn(modules, name), `unexpected dependency: ${name}`)
        return modules[name]
      }
    })
    return exports
  }
  const modules = {
    '@/lib/platform': compile('lib/platform.ts'),
    '@/features/playback/player-store': compile('features/playback/player-store.ts'),
    '@/features/music/scoped-query': { ScopedMusicQueries: class {} },
    '@/features/music/cover-image': { coverOriginal: (value) => value },
    '@/features/library/library-preview': { clearLibraryPreviews() {} },
    '@/features/discovery/private-fm': {
      createPrivateFm() {
        fm.created++
        return {
          update: () => fm.updated++,
          dispose: () => fm.disposed++
        }
      }
    },
    '@/features/playback/web-audio': {
      createWebAudio(send) {
        executors++
        sendFeedback = send
        return {
          apply: (value) => applied.push(value),
          dispose: () => disposed++
        }
      }
    },
    '@tauri-apps/api/core': { isTauri: () => tauri },
    '@tauri-apps/api/event': {
      async listen(name, listener) {
        registered.push(name)
        listeners.set(name, listener)
        return () => {
          unlistened.push(name)
          listeners.delete(name)
        }
      }
    },
    '@/lib/diagnostics': {
      reportCaughtError: (operation, cause) => errors.push({ operation, cause }),
      async diagnosticInvoke(command, args) {
        calls.push([command, args])
        if (command === 'player_snapshot') return bootstrap
        if (command === 'web_audio_ready') {
          if (readyError) throw readyError
          return control
        }
        assert.equal(command, 'web_audio_feedback')
      }
    },
    react: {},
    '@/lib/runtime-cache': { cachedCommands: new Set() }
  }
  return {
    player: compile('lib/player.ts', modules),
    calls,
    required,
    registered,
    unlistened,
    listeners,
    applied,
    errors,
    fm,
    state,
    progress,
    control,
    executors: () => executors,
    disposed: () => disposed,
    feedback: (value) => sendFeedback(value),
    emit(name, payload) {
      assert.ok(listeners.has(name), `missing listener: ${name}`)
      listeners.get(name)({ payload })
    }
  }
}

for (const platform of ['Windows', 'Macintosh', 'Linux'])
  test(`${platform} connects native playback state without loading a WebAudio executor`, async () => {
    const app = harness({ platform })
    const disconnect = await app.player.connectPlayer()
    try {
      assert.equal(app.executors(), 0)
      assert.equal(app.required.includes('@/features/playback/web-audio'), false)
      assert.equal(app.registered.includes('web-audio-control'), false)
      assert.deepEqual(
        app.calls.map(([command]) => command),
        ['player_snapshot']
      )
      assert.equal(app.player.getPlayer().deviceId, app.state.deviceId)
      assert.equal(app.player.getPlayer().playbackOptions, app.state.playbackOptions)
      assert.equal(app.player.getProgress().positionMs, 12000)
      app.emit('player-state', { ...app.state, sequence: 2, volume: 0.3 })
      app.emit('player-progress', { ...app.progress, sequence: 2, positionMs: 24000 })
      assert.equal(app.player.getPlayer().volume, 0.3)
      assert.equal(app.player.getProgress().positionMs, 24000)
      assert.deepEqual(app.errors, [])
    } finally {
      disconnect()
    }
    assert.equal(app.listeners.size, 0)
    assert.deepEqual(app.unlistened, app.registered)
    assert.equal(app.fm.disposed, 1)
    assert.equal(app.disposed(), 0)
  })

test('Android initializes WebAudio, forwards controls and feedback, and disposes on disconnect', async () => {
  const app = harness({ platform: 'Android' })
  const disconnect = await app.player.connectPlayer()
  let listener
  try {
    assert.equal(app.executors(), 1)
    assert.equal(app.required.includes('@/features/playback/web-audio'), true)
    assert.deepEqual(
      app.calls.map(([command]) => command),
      ['web_audio_ready', 'player_snapshot']
    )
    assert.deepEqual(app.applied, [app.control])
    const next = { ...app.control, serial: 2, playing: true, fades: false, crossfadeMs: 3000 }
    listener = app.listeners.get('web-audio-control')
    app.emit('web-audio-control', next)
    assert.equal(app.applied.at(-1), next)
    const feedback = {
      serial: 2,
      sequence: 1,
      kind: 'playing',
      positionMs: 12000,
      durationMs: 180000
    }
    await app.feedback(feedback)
    assert.equal(app.calls.at(-1)[0], 'web_audio_feedback')
    assert.equal(app.calls.at(-1)[1].feedback, feedback)
    assert.equal(app.player.getPlayer().playbackOptions.crossfadeDurationMs, 6000)
    assert.deepEqual(app.errors, [])
  } finally {
    disconnect()
  }
  assert.equal(app.disposed(), 1)
  assert.equal(app.listeners.size, 0)
  assert.deepEqual(app.unlistened, app.registered)
  listener({ payload: { ...app.control, serial: 3 } })
  assert.equal(app.applied.length, 2)
})

for (const platform of ['Windows', 'Android'])
  test(`${platform} observer windows synchronize without owning an audio executor`, async () => {
    const app = harness({ platform })
    const disconnect = await app.player.connectPlayer({ observerOnly: true })
    try {
      assert.equal(app.executors(), 0)
      assert.equal(app.required.includes('@/features/playback/web-audio'), false)
      assert.equal(app.listeners.has('web-audio-control'), false)
      assert.deepEqual(
        app.calls.map(([command]) => command),
        ['player_snapshot']
      )
      assert.equal(app.player.getProgress().positionMs, 12000)
      assert.equal(app.fm.created, 0)
    } finally {
      disconnect()
    }
    assert.equal(app.listeners.size, 0)
  })

test('Android releases the executor and its listener when WebAudio initialization fails', async () => {
  const error = Error('WebAudio initialization failed')
  const app = harness({ platform: 'Android', readyError: error })
  await assert.rejects(app.player.connectPlayer(), (cause) => cause === error)
  assert.equal(app.executors(), 1)
  assert.equal(app.disposed(), 1)
  assert.equal(app.listeners.size, 0)
  assert.deepEqual(app.unlistened, ['web-audio-control'])
  assert.deepEqual(
    app.calls.map(([command]) => command),
    ['web_audio_ready']
  )
  assert.equal(app.fm.disposed, 1)
  assert.equal(app.errors.at(-1).cause, error)
})

test('a browser preview does not connect either audio executor', async () => {
  const app = harness({ platform: 'Android', tauri: false })
  const disconnect = await app.player.connectPlayer()
  disconnect()
  assert.equal(app.executors(), 0)
  assert.equal(app.fm.created, 0)
  assert.deepEqual(app.calls, [])
  assert.deepEqual(app.registered, [])
})
