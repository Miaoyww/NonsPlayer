import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness() {
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/playback/volume-sender.ts', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS }
      }
    ).outputText,
    { exports }
  )
  let now = 0,
    confirmed = 0.8,
    nextTimer = 0
  const timers = new Map(),
    calls = [],
    errors = []
  const sender = exports.createVolumeSender(
    (volume) => new Promise((resolve, reject) => calls.push({ volume, resolve, reject })),
    () => confirmed,
    {
      now: () => now,
      setTimeout: (fn, delay) => {
        timers.set(++nextTimer, { fn, due: now + delay })
        return nextTimer
      },
      clearTimeout: (id) => timers.delete(id)
    }
  )
  return {
    sender,
    calls,
    timers,
    errors,
    change: (volume, immediate = false) =>
      sender.change(volume, (error) => errors.push(error), immediate),
    acknowledge(volume) {
      confirmed = volume
      sender.acknowledge(volume)
    },
    time(value) {
      now = value
      for (const [id, timer] of timers)
        if (timer.due <= now) {
          timers.delete(id)
          timer.fn()
        }
    }
  }
}
const settle = async () => {
  for (let i = 0; i < 12; i++) await Promise.resolve()
}

test('dense inputs retain one latest request; slow IPC and superseded failures cannot lose the final value', async () => {
  const app = harness()
  app.change(0.1)
  await settle()
  for (let i = 1; i <= 100; i++) {
    app.time(i * 2)
    app.change(i / 100)
  }
  app.sender.flush()
  assert.equal(app.calls.length, 1)
  assert.equal(app.timers.size, 0)
  app.calls[0].reject(new Error('superseded failure'))
  await settle()
  assert.equal(app.calls.length, 2)
  assert.equal(app.calls[1].volume, 1)
  assert.equal(app.sender.getSnapshot(), 1)
  assert.equal(app.errors.length, 0)
  app.acknowledge(0.1)
  app.calls[1].resolve()
  await settle()
  assert.equal(app.sender.getSnapshot(), 1, 'old volume echoes cannot replace optimistic feedback')
  app.acknowledge(1)
  assert.equal(app.sender.getSnapshot(), null)
})

test('continuous inputs deliver the latest value every 40ms without requiring release', async () => {
  const app = harness()
  app.change(0.1)
  await settle()
  app.calls[0].resolve()
  await settle()
  app.time(10)
  app.change(0.2)
  app.time(20)
  app.change(0.3)
  app.time(39)
  await settle()
  assert.equal(app.calls.length, 1)
  app.time(40)
  await settle()
  assert.equal(app.calls[1].volume, 0.3)
  app.calls[1].resolve()
  await settle()
})

test('latest failure releases optimistic feedback and immediate mute bypasses the timer', async () => {
  const app = harness()
  app.change(0.4)
  await settle()
  app.calls[0].resolve()
  await settle()
  app.time(10)
  app.change(0.3)
  assert.equal(app.timers.size, 1)
  app.change(0, true)
  await settle()
  assert.equal(app.calls[1].volume, 0)
  assert.equal(app.timers.size, 0)
  const failure = new Error('audio unavailable')
  app.calls[1].reject(failure)
  await settle()
  assert.equal(app.sender.getSnapshot(), null)
  assert.deepEqual(app.errors, [failure])
})
