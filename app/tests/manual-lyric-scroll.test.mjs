import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

function loadLock() {
  const source = readFileSync(
    new URL('../src/features/lyrics/manual-lyric-scroll.ts', import.meta.url),
    'utf8'
  )
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS }
  })
  const exports = {}
  runInNewContext(`${outputText}\nexports.testLock = ManualScrollLock;`, {
    exports,
    setTimeout,
    clearTimeout
  })
  return exports.testLock
}

test('manual lyric scrolling keeps auto-alignment suspended until the idle delay expires', () => {
  const ManualScrollLock = loadLock()
  const scheduled = []
  const cancelled = []
  const resumed = []
  const lock = new ManualScrollLock(
    () => resumed.push('resume'),
    3500,
    (callback, delay) => {
      const timer = { callback, delay }
      scheduled.push(timer)
      return timer
    },
    (timer) => cancelled.push(timer)
  )

  lock.suspend()
  assert.equal(lock.isLocked, true)
  assert.equal(scheduled[0].delay, 3500)

  lock.suspend()
  assert.equal(cancelled.length, 1)
  assert.equal(lock.isLocked, true)
  assert.deepEqual(resumed, [])

  scheduled[1].callback()
  assert.equal(lock.isLocked, false)
  assert.deepEqual(resumed, ['resume'])
})

test('releasing manual lyric scrolling cancels pending recovery and resumes immediately', () => {
  const ManualScrollLock = loadLock()
  const scheduled = []
  const cancelled = []
  let resumed = 0
  const lock = new ManualScrollLock(
    () => resumed++,
    3500,
    (callback) => {
      const timer = { callback }
      scheduled.push(timer)
      return timer
    },
    (timer) => cancelled.push(timer)
  )

  lock.suspend()
  lock.release()

  assert.equal(lock.isLocked, false)
  assert.equal(cancelled.length, 1)
  assert.equal(resumed, 0)
})
