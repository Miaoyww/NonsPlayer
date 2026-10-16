import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness(options = { layout: 'edge', showTimeOnHover: true }) {
  const calls = [],
    values = [],
    exports = {},
    effects = []
  let progress = {
    revision: 1,
    positionMs: 30_000,
    durationMs: 180_000,
    status: 'playing',
    receivedAt: 0
  }
  let now = 0,
    nextFrame = 0
  const frames = new Map(),
    listeners = new Map()
  const document = {
    visibilityState: 'visible',
    addEventListener: (name, fn) => listeners.set(name, fn),
    removeEventListener: (name) => listeners.delete(name)
  }
  let cursor = 0
  const jsx = (type, props) => ({ type, props })
  const modules = {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    react: {
      useEffect: (effect) => effects.push(effect),
      useRef(initial) {
        const slot = cursor++
        return (values[slot] ??= { current: initial })
      },
      useState(initial) {
        const slot = cursor++
        if (!(slot in values)) values[slot] = initial
        return [
          values[slot],
          (next) => {
            values[slot] = next
          }
        ]
      }
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/features/playback/player-slider': { PlayerSlider: 'slider' },
    '@/lib/player': {
      useProgress: () => progress,
      getProgress: () => progress,
      currentPosition: (time = now) =>
        Math.min(
          progress.durationMs,
          progress.positionMs +
            (progress.status === 'playing'
              ? Math.min(500, Math.max(0, time - progress.receivedAt))
              : 0)
        ),
      formatTime: (ms) =>
        `${Math.floor(ms / 60_000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, '0')}`,
      nativeCall: async (command, args) => {
        calls.push({ command, ...args })
      }
    }
  }
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/playback/playback-timeline.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    {
      exports,
      document,
      performance: { now: () => now },
      requestAnimationFrame: (fn) => {
        frames.set(++nextFrame, fn)
        return nextFrame
      },
      cancelAnimationFrame: (id) => frames.delete(id),
      require: (name) => modules[name] ?? {}
    }
  )
  function render() {
    cursor = 0
    const tree = exports.PlaybackTimeline({
      ...options,
      onError: (error) => {
        throw error
      }
    })
    const children = tree.props.children.filter(Boolean)
    return {
      slider: children.find((child) => child.type === 'slider').props,
      tooltip: children.find((child) => child.props.className?.startsWith('timeline-tooltip'))
        ?.props,
      labels: children
        .filter((child) =>
          ['timeline-elapsed', 'timeline-duration'].includes(child.props.className)
        )
        .map((child) => child.props.children)
    }
  }
  return {
    render,
    calls,
    effects,
    get progress() {
      return progress
    },
    replaceProgress(next) {
      progress = { ...progress, ...next }
    },
    frames,
    document,
    listeners,
    frame(time) {
      now = time
      const callbacks = [...frames.values()]
      frames.clear()
      callbacks.forEach((fn) => fn(time))
    }
  }
}
test('edge timeline reveals hover and focus time without permanent time labels', () => {
  const app = harness()
  assert.equal(app.render().tooltip, undefined)
  app.render().slider.onPointerMove({
    clientX: 250,
    currentTarget: { getBoundingClientRect: () => ({ left: 100, width: 300 }) }
  })
  assert.equal(app.render().tooltip.children.join(''), '1:30 / 3:00')
  app.render().slider.onPointerLeave()
  assert.equal(app.render().tooltip, undefined)
  app.render().slider.onFocus()
  assert.equal(app.render().tooltip.children.join(''), '0:30 / 3:00')
  app.render().slider.onBlur()
  assert.equal(app.render().tooltip, undefined)
})
test('timeline commits pointer and keyboard seeks and cancels abandoned drags', () => {
  const app = harness()
  app.render().slider.onValueChange(90000)
  app.render().slider.onValueCommitted(90000)
  assert.equal(app.calls[0].positionMs, 90_000)
  app.render().slider.thumbProps.onKeyDown({
    key: 'ArrowRight',
    currentTarget: { value: '94000' },
    preventDefault() {}
  })
  assert.equal(app.calls[1].positionMs, 95_000)
  app.render().slider.onValueChange(99000)
  app.render().slider.onPointerCancel()
  assert.equal(app.render().slider.value, 95000)
  assert.equal(app.calls.length, 2)
})

test('hover time defaults off and full screen retains elapsed and remaining labels', () => {
  const app = harness({})
  const tree = app.render()
  assert.equal(tree.tooltip, undefined)
  assert.deepEqual(Array.from(tree.labels), ['0:30', '-2:30'])
  assert.equal(tree.slider.onPointerMove, undefined)
  assert.equal(tree.slider.step, 1)
})

test('animation updates controlled slider values and stops for hidden or stale playback', () => {
  const app = harness()
  app.render()
  const cleanup = app.effects.at(-1)()
  app.frame(125)
  assert.equal(app.render().slider.value, 30125)
  app.frame(250)
  assert.equal(app.render().slider.value, 30250)
  app.frame(700)
  assert.equal(app.render().slider.value, 30500, 'native clock extrapolation is bounded')
  assert.equal(app.frames.size, 0)
  app.progress.receivedAt = 700
  app.document.visibilityState = 'hidden'
  app.listeners.get('visibilitychange')()
  assert.equal(app.frames.size, 0)
  app.document.visibilityState = 'visible'
  app.listeners.get('visibilitychange')()
  assert.equal(app.frames.size, 1)
  cleanup()
  assert.equal(app.frames.size, 0)
  assert.equal(app.listeners.size, 0)
})

test('paused playback and drag previews do not run a frame loop; old revisions stop painting', () => {
  const app = harness()
  app.progress.status = 'paused'
  app.render()
  const stopPaused = app.effects.at(-1)()
  assert.equal(app.render().slider.value, 30000)
  assert.equal(app.frames.size, 0)
  stopPaused()
  app.progress.status = 'playing'
  app.render()
  const stopPlaying = app.effects.at(-1)()
  app.replaceProgress({ revision: app.progress.revision + 1 })
  app.frame(200)
  assert.equal(app.render().slider.value, 30000)
  assert.equal(app.frames.size, 0)
  stopPlaying()
  app.render().slider.onValueChange(90000)
  app.render()
  assert.equal(app.effects.at(-1)(), undefined)
  assert.equal(app.frames.size, 0)
})
