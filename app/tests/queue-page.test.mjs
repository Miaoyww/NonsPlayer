import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

function harness() {
  const queue = [
    { key: 'a', title: 'First', artist: 'Alpha' },
    { key: 'b', title: '白夜にて', aliases: ['At the midnight sun'], artist: 'Albemuth' },
    { key: 'a', title: 'First', artist: 'Alpha' }
  ]
  const values = []
  let cursor = 0
  const calls = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    'react/jsx-runtime': { jsx, jsxs: jsx, Fragment: 'Fragment' },
    react: {
      useMemo: (fn) => fn(),
      useState(initial) {
        const index = cursor++
        values[index] ??= initial
        return [
          values[index],
          (next) => {
            values[index] = typeof next === 'function' ? next(values[index]) : next
          }
        ]
      }
    },
    'lucide-react': { Search: 'Search' },
    '@/lib/player': {
      usePlayer: () => ({ queue, index: 2 }),
      nativeCall: (command, args) => {
        calls.push({ command, args })
        return Promise.resolve()
      }
    },
    '@/features/queue/queue-controls': { QueueControls: 'QueueControls' },
    '@/components/music/track-list': { TrackList: 'TrackList' }
  }
  const source = readFileSync(
    new URL('../src/features/queue/queue-page.tsx', import.meta.url),
    'utf8'
  )
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  const exports = {}
  runInNewContext(outputText, {
    exports,
    require: (name) => {
      assert.ok(name in modules, name)
      return modules[name]
    }
  })
  function find(node, type) {
    if (!node || typeof node !== 'object') return
    if (node.type === type) return node.props
    for (const child of [node.props?.children].flat()) {
      const result = find(child, type)
      if (result) return result
    }
  }
  function render() {
    cursor = 0
    const tree = exports.QueuePage({
      onError: (error) => {
        throw error
      }
    })
    return {
      input: find(tree, 'input'),
      list: find(tree, 'TrackList'),
      controls: find(tree, 'QueueControls')
    }
  }
  return { render, calls }
}

test('queue search matches translations and artists and plays the original queue index', () => {
  const { render, calls } = harness()
  for (const keyword of [' midnight ', 'ALBEMUTH', '白夜']) {
    render().input.onChange({ target: { value: keyword } })
    const result = render()
    assert.equal(result.list.tracks.length, 1)
    assert.equal(result.list.tracks[0].key, 'b')
    result.list.onPlay(0)
    assert.equal(calls.at(-1).command, 'player_jump')
    assert.equal(calls.at(-1).args.index, 1)
  }
})

test('duplicate queue entries highlight only the current occurrence', () => {
  const { render, calls } = harness()
  render().input.onChange({ target: { value: 'First' } })
  const { list } = render()
  assert.equal(list.currentIndex, 1)
  list.onPlay(1)
  assert.equal(calls[0].args.index, 2)
})

test('locating the current track clears a filter that hides it and requests a new scroll', () => {
  const { render } = harness()
  render().input.onChange({ target: { value: 'no match' } })
  assert.equal(render().list, undefined)
  render().controls.onLocate()
  const result = render()
  assert.equal(result.input.value, '')
  assert.equal(result.list.currentIndex, 2)
  assert.equal(result.list.locateRequest, 1)
})

test('removing a search result targets its original queue occurrence', () => {
  const { render, calls } = harness()
  render().input.onChange({ target: { value: 'First' } })
  const { list } = render()
  list.onRemove(list.tracks[1], 1)
  assert.equal(calls[0].command, 'remove_queue_track')
  assert.equal(calls[0].args.index, 2)
  assert.equal(calls[0].args.key, 'a')
})
