import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'

const modules = {
  react: React,
  'react/jsx-runtime': jsx,
  'motion/react': {
    AnimatePresence: ({ children }) => children,
    useReducedMotion: () => true,
    motion: {
      div: ({ initial, animate, exit, transition, ...props }) => React.createElement('div', props)
    }
  },
  'lucide-react': Object.fromEntries(
    ['ChevronDown', 'ChevronUp', 'ListPlus', 'Play'].map((name) => [name, () => null])
  ),
  '@/components/music/cover': { Cover: () => React.createElement('span', { 'data-cover': true }) },
  '@/components/music/action-button': {
    ActionButton: ({ variant, size, ...props }) => React.createElement('button', props)
  },
  '@/components/music/playing-indicator': {
    PlayingIndicator: () => React.createElement('span', { 'data-playing': true })
  },
  '@/components/music/song-actions': {
    SongContextMenu: ({ render, children }) => React.cloneElement(render, {}, children)
  },
  '@/lib/player': { formatTime: (ms) => `${ms / 1000}s` },
  './cue-context-menu': {
    CueContextMenu: ({ render, children }) => React.cloneElement(render, {}, children)
  },
  '@/components/music/track-identity': {
    TrackIdentity: ({ track, cover }) =>
      React.createElement(
        'div',
        {},
        cover,
        React.createElement('span', { className: 'track-title' }, track.title),
        React.createElement('span', {}, track.artist)
      ),
    TrackSourceLabel: () => React.createElement('span', {}, '本地音乐')
  },
  './cue-card.css': {}
}
function load(path) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        jsx: ts.JsxEmit.ReactJSX,
        target: ts.ScriptTarget.ES2022
      }
    }).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  return exports
}
const { cueGroups } = load('../src/features/local/cue-groups.ts')
modules['./cue-track-card'] = load('../src/features/local/cue-track-card.tsx')
const { CueCard } = load('../src/features/local/cue-card.tsx')
const entry = (number, sheet = 'album.cue') => ({
  index: number - 1,
  rowKey: `${sheet}:${number}`,
  track: {
    key: `${sheet}:${number}`,
    title: `歌曲 ${number}`,
    artist: '歌手',
    album: '专辑',
    durationMs: 2000,
    cover: '',
    source: { kind: 'local', cue: { sheetPath: sheet, trackNumber: number } }
  }
})

test('CUE grouping retains separate sheets, sorts track numbers and only includes supplied matching members', () => {
  const ordinary = { track: { key: 'ordinary', source: { kind: 'local' } } }
  const input = [entry(4), ordinary, entry(1, 'other.cue'), entry(2), entry(1), entry(3)]
  const groups = cueGroups(input)
  assert.deepEqual(
    Array.from(groups, (group) => Array.from(group, ({ track }) => track.key)),
    [['album.cue:1', 'album.cue:2', 'album.cue:3', 'album.cue:4'], ['ordinary'], ['other.cue:1']]
  )
  assert.equal(cueGroups([entry(3)])[0].length, 1)
  assert.equal(input[0].track.key, 'album.cue:4')
})

test('CUE card expands into four numbered compact children and preserves play/append targets', async () => {
  const dom = new JSDOM('<div id="root"></div>', { url: 'http://localhost/' })
  const previous = new Map(
    ['window', 'document', 'IS_REACT_ACT_ENVIRONMENT'].map((key) => [
      key,
      Object.getOwnPropertyDescriptor(globalThis, key)
    ])
  )
  Object.defineProperties(globalThis, {
    window: { configurable: true, value: dom.window },
    document: { configurable: true, value: dom.window.document },
    IS_REACT_ACT_ENVIRONMENT: { configurable: true, value: true }
  })
  const root = createRoot(document.getElementById('root'))
  const calls = []
  const entries = [1, 2, 3, 4].map((number) => entry(number))
  function Demo() {
    const [expanded, setExpanded] = React.useState(false)
    return React.createElement(CueCard, {
      entries,
      sequence: 1,
      columns: [
        { id: 'index', size: 48 },
        { id: 'title', size: 480 },
        { id: 'album', size: 200 },
        { id: 'source', size: 104 },
        { id: 'duration', size: 80 },
        { id: 'append', size: 48 }
      ],
      onPlayGroup() {},
      expanded,
      onExpandedChange: setExpanded,
      busy: false,
      isCurrent: ({ track }) => track.key === 'album.cue:2',
      onPlay: (entry) => calls.push(['play', entry.track.key]),
      onAppend: (track) => calls.push(['append', track.key])
    })
  }
  try {
    await act(async () => root.render(React.createElement(Demo)))
    assert.equal(document.querySelectorAll('.cue-track-card').length, 0)
    await act(async () => document.querySelector('.cue-card-open').click())
    assert.equal(document.querySelectorAll('.cue-track-card').length, 4)
    assert.deepEqual(
      [...document.querySelectorAll('.cue-track-number')].map((node) => node.textContent),
      ['1', '2', '3', '4']
    )
    assert.ok(document.querySelector('.cue-track-card[data-current="true"] [data-playing]'))
    assert.ok(
      [...document.querySelectorAll('.cue-track-card')].every(
        (node) => !node.textContent.includes('专辑') && !node.textContent.includes('本地')
      )
    )
    await act(async () => document.querySelectorAll('.cue-track-text')[2].click())
    await act(async () => document.querySelectorAll('[aria-label^="下一首播放"]')[1].click())
    assert.deepEqual(calls, [
      ['play', 'album.cue:3'],
      ['append', 'album.cue:2']
    ])
    await act(async () => document.querySelector('[aria-label="折叠 专辑"]').click())
    assert.equal(document.querySelectorAll('.cue-track-card').length, 0)
    assert.equal(document.querySelector('.cue-card-open').getAttribute('aria-expanded'), 'false')
  } finally {
    await act(async () => root.unmount())
    for (const [key, descriptor] of previous) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor)
      else delete globalThis[key]
    }
    dom.window.close()
  }
})
