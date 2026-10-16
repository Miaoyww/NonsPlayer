import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

// Exercise the actual component handlers without a WebView or native audio device.
// Hooks and IPC are stubbed; no separate copy of the volume handler is tested.
function volumeControl({ failure, desktop = true, fullscreen = false } = {}) {
  const calls = []
  const errors = []
  const slots = []
  let cursor = 0
  let now = 0,
    nextTimer = 0,
    confirmed = 0.8
  const timers = new Map()
  const jsx = (type, props) => ({ type, props })
  const modules = {
    '@/lib/platform': { isAndroid: () => false },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    react: {
      useSyncExternalStore: (_subscribe, getSnapshot) => getSnapshot(),
      useState: (initial) => {
        const i = cursor++
        if (!(i in slots)) slots[i] = initial
        return [
          slots[i],
          (value) => {
            slots[i] = value
          }
        ]
      },
      useEffect: () => {},
      useRef: (current) => {
        const i = cursor++
        return (slots[i] ??= { current })
      }
    },
    '@tauri-apps/api/core': { isTauri: () => desktop },
    '@/lib/player': {
      usePlayer: () => ({ volume: confirmed }),
      getPlayer: () => ({ volume: confirmed }),
      nativeCall: (command, args) => {
        calls.push({ command, ...args })
        if (!failure) confirmed = args.volume
        return failure ? Promise.reject(failure) : Promise.resolve()
      },
      statusLabels: { stopped: '已停止' }
    },
    'lucide-react': {},
    '@/components/music/action-button': {},
    '@/components/ui/slider': { Slider: 'slider' },
    '@/components/music/cover': {},
    '@/features/playback/music-options': {},
    '@/features/playback/playback-timeline': {},
    '@/features/playback/player-slider': { PlayerSlider: 'slider' },
    '@/features/playback/playback-transport': {}
  }
  function load(file) {
    const exports = {}
    const source = readFileSync(
      new URL(`../src/features/playback/${file}`, import.meta.url),
      'utf8'
    )
    runInNewContext(
      ts.transpileModule(source, {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        performance: { now: () => now },
        setTimeout: (fn, delay) => {
          timers.set(++nextTimer, { fn, due: now + delay })
          return nextTimer
        },
        clearTimeout: (id) => timers.delete(id),
        require: (name) => {
          assert.ok(Object.hasOwn(modules, name), `unexpected dependency: ${name}`)
          return modules[name]
        }
      }
    )
    return exports
  }
  modules['./volume-sender'] = load('volume-sender.ts')
  modules['./use-volume-control'] = load('use-volume-control.ts')
  const exports = load(fullscreen ? 'now-playing-controls.tsx' : 'volume-control.tsx')
  function render() {
    cursor = 0
    return (fullscreen ? exports.NowPlayingControls : exports.VolumeControl)({
      onError: (error) => errors.push(error),
      onQueue() {}
    })
  }
  const tree = render()
  function findVolume(node) {
    if (!node || typeof node !== 'object') return
    if (node.type === 'slider' && node.props['aria-label'] === '音量') return node.props
    for (const child of [node.props?.children].flat()) {
      const match = findVolume(child)
      if (match) return match
    }
  }
  const input = findVolume(tree)
  assert.ok(input, 'playback bar must expose a volume range')
  return {
    input,
    calls,
    errors,
    render,
    advance(time) {
      now = time
      for (const [id, timer] of timers)
        if (timer.due <= now) {
          timers.delete(id)
          timer.fn()
        }
    },
    get localVolume() {
      return findVolume(render()).value
    }
  }
}

const settle = async () => {
  for (let i = 0; i < 12; i++) await Promise.resolve()
}

test('mute button restores the volume selected before muting', async () => {
  const control = volumeControl()
  control.input.onValueChange(0.35)
  let button = control.render().props.children[0].props
  assert.equal(button['aria-label'], '静音')
  button.onClick()
  await settle()
  assert.equal(control.calls.at(-1).volume, 0)
  button = control.render().props.children[0].props
  assert.equal(button['aria-label'], '取消静音')
  button.onClick()
  await settle()
  assert.equal(control.calls.at(-1).volume, 0.35)
})

test('volume feedback is immediate while continuous commands coalesce and release flushes the final value', async () => {
  const control = volumeControl()
  control.input.onValueChange(0.5)
  assert.equal(control.localVolume, 0.5)
  await settle()
  assert.equal(control.calls[0].volume, 0.5, 'first input is sent before release')
  for (const [time, value] of [
    [10, 0.25],
    [20, 0],
    [30, 1]
  ]) {
    control.advance(time)
    control.input.onValueChange(value)
    assert.equal(control.localVolume, value)
    await settle()
    assert.equal(control.calls.length, 1)
  }
  control.input.onValueCommitted(1)
  await settle()
  assert.equal(control.calls.length, 2)
  assert.equal(control.calls.at(-1).volume, 1)
  control.advance(100)
  await settle()
  assert.equal(control.calls.length, 2, 'no stale value follows the committed value')
})

test('native volume failures reach the playback error handler', async () => {
  const failure = new Error('playback unavailable')
  const control = volumeControl({ failure })
  control.input.onValueChange(0.3)
  await settle()
  assert.deepEqual(control.errors, [failure])
})

test('volume is disabled in browser preview', () => {
  assert.equal(volumeControl({ desktop: false }).input.disabled, true)
})

test('fullscreen volume uses the same immediate feedback and final commit behavior', async () => {
  const control = volumeControl({ fullscreen: true })
  control.input.onValueChange(0.3)
  assert.equal(control.localVolume, 0.3)
  await settle()
  control.advance(10)
  control.input.onValueChange(0.6)
  control.input.onValueCommitted(0.6)
  await settle()
  assert.deepEqual(
    control.calls.map((call) => call.volume),
    [0.3, 0.6]
  )
})
