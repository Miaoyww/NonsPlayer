import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

async function harness() {
  let now = 0
  const listeners = new Map()
  const initial = {
    revision: 1,
    queue: [],
    index: null,
    status: 'playing',
    positionMs: 30000,
    durationMs: 180000,
    volume: 0.8
  }
  const modules = {
    '@/features/music/scoped-query': { ScopedMusicQueries: class {} },
    '@/features/discovery/private-fm': {
      createPrivateFm: () => ({ update() {}, dispose() {} })
    },
    '@tauri-apps/api/core': { isTauri: () => true, invoke: async () => initial },
    '@tauri-apps/api/event': {
      listen: async (name, listener) => {
        listeners.set(name, listener)
        return () => listeners.delete(name)
      }
    },
    react: { useSyncExternalStore: (subscribe, getSnapshot) => getSnapshot() },
    '@/lib/runtime-cache': { cachedCommands: new Set() }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL('../src/lib/player.ts', import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }).outputText,
    {
      exports,
      performance: { now: () => now },
      require: (name) => {
        assert.ok(Object.hasOwn(modules, name), `unexpected dependency: ${name}`)
        return modules[name]
      }
    }
  )
  await exports.connectPlayer()
  return {
    player: exports,
    time(value) {
      now = value
    },
    state(patch) {
      listeners.get('player-state')({ payload: { ...exports.getPlayer(), ...patch } })
    },
    progress(patch) {
      listeners.get('player-progress')({ payload: { ...exports.getProgress(), ...patch } })
    }
  }
}

test('rapid volume state events preserve the progress clock between native samples', async () => {
  const app = await harness()
  for (let now = 20; now <= 200; now += 20) {
    app.time(now)
    const before = app.player.currentPosition()
    app.state({ volume: now / 200 })
    assert.equal(app.player.currentPosition(), before, 'volume must not rewind the timeline')
  }
  app.progress({ positionMs: 30200 })
  app.time(240)
  app.state({ volume: 0.5, positionMs: 30200 })
  assert.equal(app.player.currentPosition(), 30240)
})

test('seek, status transitions and track revisions still update progress', async () => {
  const app = await harness()
  app.time(100)
  app.state({ positionMs: 10000 })
  assert.equal(app.player.currentPosition(), 10000)
  app.time(150)
  app.state({ status: 'paused', positionMs: 10050 })
  app.time(200)
  assert.equal(app.player.currentPosition(), 10050)
  app.state({ status: 'playing' })
  app.time(250)
  assert.equal(app.player.currentPosition(), 10100)
  app.state({ revision: 2, positionMs: 0 })
  assert.equal(app.player.currentPosition(), 0)
  app.progress({ revision: 1, positionMs: 99000 })
  assert.equal(app.player.currentPosition(), 0)
})
