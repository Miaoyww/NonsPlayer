import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'
function harness() {
  let id = 0
  const frames = new Map(),
    timers = new Map(),
    exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/lib/after-first-paint.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    {
      exports,
      window: {
        requestAnimationFrame(fn) {
          frames.set(++id, fn)
          return id
        },
        cancelAnimationFrame(id) {
          frames.delete(id)
        },
        setTimeout(fn, delay) {
          assert.equal(delay, 200)
          timers.set(++id, fn)
          return id
        },
        clearTimeout(id) {
          timers.delete(id)
        }
      }
    }
  )
  return {
    ...exports,
    frames,
    timers,
    paint() {
      const batch = [...frames.values()]
      frames.clear()
      batch.forEach((fn) => fn())
    }
  }
}
test('optional work waits for a painted frame, runs once and subsequent interactions run immediately', () => {
  const app = harness()
  let calls = 0
  app.afterFirstPaint(() => calls++)
  assert.equal(calls, 0)
  app.paint()
  assert.equal(calls, 0)
  app.paint()
  assert.equal(calls, 1)
  assert.equal(app.timers.size, 0)
  app.afterFirstPaint(() => calls++)
  assert.equal(calls, 2)
})
test('unmount cancels queued work and hidden windows have a bounded fallback', () => {
  const app = harness()
  let calls = 0
  const cancel = app.afterFirstPaint(() => calls++)
  app.paint()
  cancel()
  app.paint()
  assert.equal(app.frames.size, 0)
  assert.equal(app.timers.size, 0)
  assert.equal(calls, 0)
  app.afterFirstPaint(() => calls++)
  const timeout = [...app.timers.values()][0]
  timeout()
  timeout()
  app.paint()
  assert.equal(calls, 1)
  assert.equal(app.frames.size, 0)
})
