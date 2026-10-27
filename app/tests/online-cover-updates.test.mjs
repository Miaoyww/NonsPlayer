import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { createRuntimeCache } from './helpers/runtime-cache.mjs'

function harness() {
  const registrations = [],
    deleted = []
  const exports = {}
  const modules = {
    '@tauri-apps/api/event': {
      listen(name, callback) {
        assert.equal(name, 'runtime-cover-updated')
        return new Promise((resolve) => registrations.push({ callback, resolve }))
      }
    },
    '@/lib/runtime-cache': {
      coverCache: { delete: (key) => deleted.push(key) },
      requestKey: (command, args) => `${command}:${args.url}`
    }
  }
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/music/online-cover-updates.ts', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  return { ...exports, registrations, deleted }
}

test('online covers share a ready listener and only notify the updated URL', async () => {
  const h = harness(),
    changes = []
  const a = h.subscribeOnlineCover('a', () => changes.push('a'))
  const alsoA = h.subscribeOnlineCover('a', () => changes.push('also-a'))
  const b = h.subscribeOnlineCover('b', () => changes.push('b'))
  assert.equal(h.registrations.length, 1)
  let stopped = 0,
    ready = false
  void a.ready.then(() => {
    ready = true
  })
  await Promise.resolve()
  assert.equal(ready, false)
  h.registrations[0].resolve(() => stopped++)
  await a.ready
  h.registrations[0].callback({ payload: { url: 'a' } })
  assert.deepEqual(changes, ['a', 'also-a'])
  assert.deepEqual(h.deleted, ['runtime_cover:a'])
  a.stop()
  alsoA.stop()
  assert.equal(stopped, 0)
  b.stop()
  assert.equal(stopped, 1)
  h.registrations[0].callback({ payload: { url: 'b' } })
  assert.deepEqual(changes, ['a', 'also-a'])
})

test('unmount before registration resolves cannot remove a newer shared listener', async () => {
  const h = harness(),
    changes = []
  const old = h.subscribeOnlineCover('cover', () => assert.fail('disposed callback'))
  old.stop()
  const active = h.subscribeOnlineCover('cover', () => changes.push('new'))
  let oldStopped = 0,
    activeStopped = 0
  h.registrations[0].resolve(() => oldStopped++)
  await old.ready
  assert.equal(oldStopped, 1)
  h.registrations[1].resolve(() => activeStopped++)
  await active.ready
  h.registrations[0].callback({ payload: { url: 'cover' } })
  h.registrations[1].callback({ payload: { url: 'cover' } })
  assert.deepEqual(changes, ['new'])
  active.stop()
  assert.equal(activeStopped, 1)
})

test('cover metadata weighting never serializes the cached entry', () => {
  const { coverCache } = createRuntimeCache()
  const key = 'runtime_cover:online'
  const value = {
    access: 'valid',
    data: 'asset://cache/image.jpg',
    toJSON() {
      assert.fail('cover serialization')
    }
  }
  coverCache.set(key, value)
  assert.equal(coverCache.peek(key), value)
  assert.equal(
    coverCache.stats().usedBytes,
    (key.length + value.access.length + value.data.length) * 2
  )
})
