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

async function hookHarness(
  stored,
  picked,
  resolved = { source: 'http://127.0.0.1/mv', local: false }
) {
  const calls = []
  const slots = []
  const effects = []
  const listeners = new Map()
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
  const storedBinding =
    stored && stored.source
      ? stored
      : stored && {
          source: 'local',
          id: '',
          title: stored.name,
          offsetMs: stored.offsetMs,
          local: stored
        }
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
        if (name === './local-mv')
          return {
            localMvSource: (mv) => `asset://${mv.path}`,
            resolvedMvSource: (mv) => (mv.local ? `asset://${mv.source}` : mv.source)
          }
        if (name === '@tauri-apps/api/event')
          return {
            listen: async (event, handler) => {
              const handlers = listeners.get(event) ?? []
              handlers.push(handler)
              listeners.set(event, handlers)
              return () => {
                listeners.set(
                  event,
                  (listeners.get(event) ?? []).filter((value) => value !== handler)
                )
              }
            }
          }
        assert.equal(name, '@/lib/player')
        return {
          errorText: String,
          nativeCall: async (command, args) => {
            calls.push({ command, ...args })
            if (command === 'track_mv') return storedBinding
            if (command === 'pick_local_mv') return picked
            if (command === 'link_local_mv') return picked
            if (command === 'unlink_mv_binding') return
            if (command === 'resolve_track_mv') return resolved
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
    exports,
    calls,
    get current() {
      return hook
    },
    async toggle() {
      const result = await hook.toggle()
      hook = render()
      return result
    },
    async choose() {
      await hook.choose()
      hook = render()
    },
    async apply() {
      const result = await hook.apply()
      hook = render()
      return result
    },
    async refreshBinding() {
      await hook.refreshBinding()
      hook = render()
    },
    async unlink() {
      const result = await hook.unlink()
      hook = render()
      return result
    },
    async setOffset(value) {
      await hook.setOffset(value)
      hook = render()
    },
    async emit(event, payload) {
      await Promise.resolve()
      for (const handler of listeners.get(event) ?? []) handler({ payload })
      hook = render()
    }
  }
}

test('the MV button returns to lyrics from the selection page and toggles elsewhere', async () => {
  const harness = await hookHarness(null, null)
  assert.equal(harness.exports.mvButtonTarget('lyrics', 'lyrics'), 'toggle')
  assert.equal(harness.exports.mvButtonTarget('lyrics', 'mv'), 'lyrics')
  assert.equal(harness.exports.mvButtonTarget('mv', 'mv'), 'toggle')
})

test('an unlinked MV click opens selection without leaving lyrics mode', async () => {
  const mv = { name: 'live.mp4', path: 'C:/Videos/live.mp4', available: true, offsetMs: 0 }
  const harness = await hookHarness(null, mv)
  assert.equal(await harness.toggle(), 'select')
  assert.equal(harness.current.mode, 'lyrics')
  assert.deepEqual(
    harness.calls.map((call) => call.command),
    ['track_mv']
  )
})

test('choosing a video stays pending until apply links the current track', async () => {
  const mv = { name: 'live.mp4', path: 'C:/Videos/live.mp4', available: true, offsetMs: 0 }
  const harness = await hookHarness(null, mv)
  await harness.toggle()
  await harness.choose()
  assert.equal(harness.current.mode, 'lyrics')
  assert.equal(harness.current.selection.title, 'live.mp4')
  assert.equal(harness.current.linked, false)
  assert.equal(await harness.apply(), true)
  assert.equal(harness.current.mode, 'mv')
  assert.equal(harness.current.linked, true)
  assert.deepEqual(
    harness.calls.map((call) => call.command),
    ['track_mv', 'pick_local_mv', 'link_local_mv']
  )
  assert.deepEqual(harness.calls.at(-1), {
    command: 'link_local_mv',
    key: 'track-a',
    path: 'C:/Videos/live.mp4'
  })
})

test('an existing association switches immediately and cancelling a picker keeps lyrics', async () => {
  const existing = {
    name: 'live.mp4',
    path: 'C:/Videos/live.mp4',
    available: true,
    offsetMs: 250
  }
  const linked = await hookHarness(existing, existing)
  assert.equal(await linked.toggle(), 'mv')
  assert.equal(linked.current.mode, 'mv')
  assert.deepEqual(
    linked.calls.map((call) => call.command),
    ['track_mv']
  )
  await linked.setOffset(1_000)
  assert.equal(linked.current.selection.offsetMs, 1_000)
  assert.deepEqual(linked.calls.at(-1), {
    command: 'set_mv_offset',
    key: 'track-a',
    offsetMs: 1_000
  })

  const cancelled = await hookHarness(null, null)
  assert.equal(await cancelled.toggle(), 'select')
  await cancelled.choose()
  assert.equal(await cancelled.apply(), false)
  assert.equal(cancelled.current.mode, 'lyrics')
  assert.deepEqual(
    cancelled.calls.map((call) => call.command),
    ['track_mv', 'pick_local_mv']
  )
})

test('an unavailable association opens selection instead of the missing MV', async () => {
  const missing = {
    name: 'missing.mp4',
    path: 'C:/Videos/missing.mp4',
    available: false,
    offsetMs: 0
  }
  const harness = await hookHarness(missing, missing)
  assert.equal(await harness.toggle(), 'select')
  assert.equal(harness.current.mode, 'lyrics')
  assert.equal(harness.current.linked, true)
  assert.equal(harness.current.selection, undefined)
  assert.deepEqual(
    harness.calls.map((call) => call.command),
    ['track_mv']
  )
})

test('menu lookup finds an existing binding and unlinking resets the MV state', async () => {
  const existing = {
    name: 'live.mp4',
    path: 'C:/Videos/live.mp4',
    available: true,
    offsetMs: 250
  }
  const harness = await hookHarness(existing, existing)
  await harness.refreshBinding()
  assert.equal(harness.current.linked, true)
  assert.equal(harness.current.mode, 'lyrics')
  assert.equal(harness.current.selection.offsetMs, 250)
  assert.equal(await harness.apply(), true)
  assert.equal(harness.current.mode, 'mv')
  assert.equal(await harness.unlink(), true)
  assert.equal(harness.current.linked, false)
  assert.equal(harness.current.selection, undefined)
  assert.equal(harness.current.mode, 'lyrics')
  assert.deepEqual(
    harness.calls.map((call) => call.command),
    ['track_mv', 'unlink_mv_binding']
  )
})

test('adapter MV resolves through the host proxy and uses the unified offset command', async () => {
  const binding = {
    source: 'bilibili',
    id: 'BV1xx411c7mD',
    title: 'Official MV',
    offsetMs: 0
  }
  const harness = await hookHarness(binding, null)
  assert.equal(await harness.toggle(), 'mv')
  assert.equal(harness.current.mode, 'mv')
  assert.equal(harness.current.selection.url, 'http://127.0.0.1/mv')
  assert.equal(harness.current.selection.local, false)
  await harness.setOffset(200)
  assert.deepEqual(harness.calls.at(-1), {
    command: 'set_mv_offset',
    key: 'track-a',
    offsetMs: 200
  })
})

test('a saved adapter MV uses the asset protocol without becoming a local fallback', async () => {
  const binding = {
    source: 'bilibili',
    id: 'BV1xx411c7mD',
    title: 'Official MV',
    offsetMs: 0
  }
  const harness = await hookHarness(binding, null, {
    source: 'C:/data/mvs/saved.mp4',
    local: true
  })
  assert.equal(await harness.toggle(), 'mv')
  assert.equal(harness.current.selection.url, 'asset://C:/data/mvs/saved.mp4')
  assert.equal(harness.current.selection.local, false)
})

test('a completed save switches the playing adapter MV to the saved file', async () => {
  const binding = {
    source: 'bilibili',
    id: 'BV1xx411c7mD',
    title: 'Official MV',
    offsetMs: 250
  }
  const harness = await hookHarness(binding, null)
  assert.equal(await harness.toggle(), 'mv')
  await harness.emit('mv-saved', {
    source: 'bilibili',
    id: 'BV1xx411c7mD',
    path: 'C:/data/mvs/saved.mp4'
  })
  assert.equal(harness.current.selection.url, 'asset://C:/data/mvs/saved.mp4')
  assert.equal(harness.current.selection.localPath, 'C:/data/mvs/saved.mp4')
  assert.equal(harness.current.selection.offsetMs, 250)
  assert.equal(harness.current.selection.local, false)
  assert.equal(harness.current.mode, 'mv')
})

test('a save event for another MV keeps the current source', async () => {
  const binding = {
    source: 'bilibili',
    id: 'BV1xx411c7mD',
    title: 'Official MV',
    offsetMs: 0
  }
  const harness = await hookHarness(binding, null)
  assert.equal(await harness.toggle(), 'mv')
  await harness.emit('mv-saved', {
    source: 'bilibili',
    id: 'BV1other',
    path: 'C:/data/mvs/other.mp4'
  })
  assert.equal(harness.current.selection.url, 'http://127.0.0.1/mv')
})
