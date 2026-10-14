import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness(state) {
  const calls = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    'lucide-react': {},
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/platform': { isAndroid: () => false },
    '@/components/music/action-button': { ActionButton: 'button' },
    '@/features/playback/music-options': {},
    '@/features/playback/playback-timeline': {},
    '@/features/playback/player-slider': {},
    '@/features/playback/playback-transport': { PlaybackTransport: 'transport' },
    './use-volume-control': {
      useVolumeControl: () => ({ volume: state.volume, changeVolume() {}, commitVolume() {} })
    },
    react: { useEffect() {}, useState: (value) => [value, () => {}] },
    '@/lib/player': {
      usePlayer: () => state,
      adjacentIndex: () => null,
      statusLabels: { stopped: '已停止' },
      nativeCall: async (command, args) => calls.push({ command, ...args })
    }
  }
  function load(file) {
    const exports = {}
    runInNewContext(
      ts.transpileModule(
        readFileSync(new URL(`../src/features/playback/${file}.tsx`, import.meta.url), 'utf8'),
        {
          compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
        }
      ).outputText,
      { exports, require: (name) => modules[name] ?? {} }
    )
    return exports
  }
  return {
    calls,
    transport: load('playback-transport').PlaybackTransport,
    controls: load('now-playing-controls').NowPlayingControls,
    bar: load('playback-bar').PlaybackBar
  }
}

test('fullscreen controls expose shuffle and repeat through the shared transport', () => {
  const app = harness({ volume: 0.8 })
  const children = app.controls({ onError() {}, onQueue() {} }).props.children
  assert.equal(children.find((child) => child.type === 'transport').props.withModes, true)
})

test('transport exposes mode states and sends one playback command per click', () => {
  for (const repeatMode of ['off', 'all', 'one']) {
    const app = harness({ status: 'paused', index: 0, shuffle: repeatMode === 'off', repeatMode })
    const children = app.transport({ withModes: true, onError() {} }).props.children.filter(Boolean)
    const shuffle = children[0].props
    const repeat = children.at(-1).props
    assert.equal(shuffle['aria-pressed'], repeatMode === 'off')
    assert.equal(repeat['aria-pressed'], repeatMode !== 'off')
    assert.match(
      repeat['aria-label'],
      new RegExp({ off: '顺序播放', all: '列表循环', one: '单曲循环' }[repeatMode])
    )
    shuffle.onClick()
    repeat.onClick()
    assert.deepEqual(
      app.calls.map((call) => call.action),
      ['shuffle', 'repeat']
    )
    assert.ok(app.calls.every((call) => call.command === 'player_action'))
  }
})

test('shuffle leaves the repeat button unselected even with a legacy all-repeat snapshot', () => {
  const app = harness({ status: 'paused', index: 0, shuffle: true, repeatMode: 'all' })
  const children = app.transport({ withModes: true, onError() {} }).props.children.filter(Boolean)
  assert.equal(children[0].props['aria-pressed'], true)
  assert.equal(children.at(-1).props['aria-pressed'], false)
})

test('frontend shuffle navigation wraps both ends even when the repeat button is off', () => {
  const exports = {}
  const store = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/playback/player-store.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    { exports: store }
  )
  runInNewContext(
    ts.transpileModule(readFileSync(new URL('../src/lib/player.ts', import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }).outputText,
    {
      exports,
      require: (name) =>
        name === '@/lib/platform'
          ? { isAndroid: () => false }
          : name === '@/features/music/scoped-query'
            ? { ScopedMusicQueries: class {} }
            : name === '@/features/playback/player-store'
              ? store
              : {}
    }
  )
  const state = {
    queue: [{}, {}, {}],
    index: 1,
    shuffle: true,
    shuffleOrder: [1, 2, 0],
    repeatMode: 'off'
  }
  assert.equal(exports.adjacentIndex(state, 'previous'), 0)
  state.index = 0
  assert.equal(exports.adjacentIndex(state, 'next'), 1)
  state.shuffle = false
  assert.equal(exports.adjacentIndex(state, 'previous'), null)
  state.index = 2
  assert.equal(exports.adjacentIndex(state, 'next'), null)
  state.shuffle = true
  state.queue = [{}]
  state.shuffleOrder = [0]
  state.index = 0
  assert.equal(exports.adjacentIndex(state, 'next'), 0)
  assert.equal(exports.adjacentIndex(state, 'previous'), 0)
})

test('collapsible player places shuffle beside repeat and routes both through playback commands', () => {
  for (const shuffle of [false, true]) {
    const app = harness({ queue: [], index: null, status: 'stopped', shuffle, repeatMode: 'all' })
    const tree = app.bar({ onLyrics() {}, onQueue() {}, onError() {}, qualityControl: 'quality' })
    function find(node) {
      if (!node || typeof node !== 'object') return
      if (node.props?.className === 'capsule-playback-options') return node.props.children
      for (const child of [node.props?.children].flat()) {
        const found = find(child)
        if (found) return found
      }
    }
    const options = find(tree)
    assert.equal(options[0], 'quality')
    assert.equal(options[1].props['aria-label'], '随机播放')
    assert.equal(options[1].props['aria-pressed'], shuffle)
    assert.equal(options[2].props['aria-pressed'], !shuffle)
    options[1].props.onClick()
    options[2].props.onClick()
    assert.deepEqual(
      app.calls.map((call) => call.action),
      ['shuffle', 'repeat']
    )
  }
})
