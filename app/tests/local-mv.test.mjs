import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

const exports = {}
runInNewContext(
  ts.transpileModule(readFileSync(new URL('../src/features/mv/sync.ts', import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS }
  }).outputText,
  { exports }
)

test('local MV time follows playback and clamps at the end of the video', () => {
  assert.equal(exports.localMvOffsetStepMs, 100)
  assert.equal(exports.localMvTargetSeconds(0, 120), 0)
  assert.equal(exports.localMvTargetSeconds(30_500, 120), 30.5)
  assert.equal(exports.localMvTargetSeconds(130_000, 120), 120)
  assert.equal(exports.localMvTargetSeconds(-500, 120), 0)
  assert.equal(exports.localMvTargetSeconds(30_000, Number.NaN), 30)
  assert.equal(exports.localMvTargetSeconds(3_000, 120, 5_000), 0)
  assert.equal(exports.localMvTargetSeconds(8_000, 120, 5_000), 3)
  assert.equal(exports.localMvTargetSeconds(1_000, 120, -2_000), 3)
  assert.equal(exports.localMvWaiting(4_999, 5_000), true)
  assert.equal(exports.localMvWaiting(5_000, 5_000), false)
  assert.equal(exports.localMvWaiting(0, -1_000), false)
  assert.equal(exports.clampLocalMvOffset(900_000), 600_000)
  assert.equal(exports.clampLocalMvOffset(-900_000), -600_000)
})

test('local MV only seeks after drift exceeds the playback tolerance', () => {
  assert.equal(exports.localMvNeedsSync(30, 30.2), false)
  assert.equal(exports.localMvNeedsSync(30, 30.36), true)
  assert.equal(exports.localMvNeedsSync(30, 29.64), true)
  assert.equal(exports.localMvNeedsSync(Number.NaN, 30), false)
})

async function hookHarness(stored, picked) {
  const calls = []
  const slots = []
  const effects = []
  let cursor = 0
  const React = {
    useState(initial) {
      const index = cursor++
      slots[index] ??= { value: typeof initial === 'function' ? initial() : initial }
      return [
        slots[index].value,
        (value) => {
          slots[index].value = typeof value === 'function' ? value(slots[index].value) : value
        }
      ]
    },
    useRef(value) {
      const index = cursor++
      slots[index] ??= { current: value }
      return slots[index]
    },
    useEffect(fn) {
      effects.push(fn)
    },
    useCallback(fn) {
      return fn
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/mv/use-local-mv.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    {
      exports,
      require(name) {
        if (name === 'react') return React
        if (name === './sync')
          return {
            clampLocalMvOffset: (value) => Math.max(-600_000, Math.min(600_000, Math.round(value)))
          }
        assert.equal(name, '@/lib/player')
        return {
          errorText: String,
          nativeCall: async (command, args) => {
            calls.push({ command, ...args })
            if (command === 'local_mv') return stored
            if (command === 'pick_local_mv') return picked
          }
        }
      }
    }
  )
  const render = () => {
    cursor = 0
    return exports.useLocalMv('track-a', () => {})
  }
  render()
  effects.splice(0).forEach((fn) => fn())
  let hook = render()
  return {
    calls,
    get current() {
      return hook
    },
    async toggle() {
      await hook.toggle()
      hook = render()
    },
    async setOffset(value) {
      await hook.setOffset(value)
      hook = render()
    }
  }
}

test('the first MV click selects a file, later clicks only switch modes', async () => {
  const mv = { name: 'live.mp4', path: 'C:/Videos/live.mp4', available: true, offsetMs: 0 }
  const harness = await hookHarness(null, mv)
  await harness.toggle()
  assert.equal(harness.current.mode, 'mv')
  assert.deepEqual(
    harness.calls.map((call) => call.command),
    ['local_mv', 'pick_local_mv']
  )
  await harness.toggle()
  assert.equal(harness.current.mode, 'lyrics')
  assert.equal(harness.calls.length, 2)
})

test('an existing association switches immediately and cancelling a picker keeps lyrics', async () => {
  const existing = {
    name: 'live.mp4',
    path: 'C:/Videos/live.mp4',
    available: true,
    offsetMs: 250
  }
  const linked = await hookHarness(existing, existing)
  await linked.toggle()
  assert.equal(linked.current.mode, 'mv')
  assert.deepEqual(
    linked.calls.map((call) => call.command),
    ['local_mv']
  )
  await linked.setOffset(1_000)
  assert.equal(linked.current.selection.offsetMs, 1_000)
  assert.deepEqual(linked.calls.at(-1), {
    command: 'set_local_mv_offset',
    key: 'track-a',
    offsetMs: 1_000
  })

  const cancelled = await hookHarness(null, null)
  await cancelled.toggle()
  assert.equal(cancelled.current.mode, 'lyrics')
  assert.deepEqual(
    cancelled.calls.map((call) => call.command),
    ['local_mv', 'pick_local_mv']
  )
})
