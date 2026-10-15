import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('aggregate source dialog retains members, supports keyboard dismissal and plays only the selected track', async () => {
  const dom = new JSDOM('<div id="root"></div>', {
    pretendToBeVisual: true,
    url: 'http://localhost/'
  })
  const replacements = {
    window: dom.window,
    document: dom.window.document,
    navigator: dom.window.navigator,
    HTMLElement: dom.window.HTMLElement,
    Element: dom.window.Element,
    Node: dom.window.Node,
    ShadowRoot: dom.window.ShadowRoot,
    MutationObserver: dom.window.MutationObserver,
    getComputedStyle: dom.window.getComputedStyle,
    requestAnimationFrame: dom.window.requestAnimationFrame.bind(dom.window),
    cancelAnimationFrame: dom.window.cancelAnimationFrame.bind(dom.window),
    IS_REACT_ACT_ENVIRONMENT: true
  }
  const previous = new Map(
    Object.keys(replacements).map((key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)])
  )
  for (const [key, value] of Object.entries(replacements))
    Object.defineProperty(globalThis, key, { configurable: true, writable: true, value })
  const calls = []
  const notices = []
  const root = createRoot(document.getElementById('root'))
  try {
    const { Dialog: primitive } = await import('@base-ui/react/dialog')
    const modules = {
      react: React,
      'react/jsx-runtime': jsx,
      'lucide-react': { ChevronDown: () => null, XIcon: () => null },
      '@base-ui/react/dialog': { Dialog: primitive },
      cn: await import('cn'),
      '@/components/ui/button': {
        Button: React.forwardRef(function Button({ variant, size, ...props }, ref) {
          return React.createElement('button', { ...props, ref })
        })
      },
      '@/components/music/track-identity': {
        TrackSourceLabel: ({ track }) =>
          React.createElement('span', {}, track.source.track.reference.source)
      },
      '@/lib/platform': { isAndroid: () => true },
      '@tauri-apps/api/core': { isTauri: () => true },
      '@/lib/player': {
        formatTime: (duration) => `${duration / 1000}s`,
        nativeCall: async (command, args) => calls.push({ command, ...args })
      },
      '@/components/music/track-list': {
        TrackList: ({ tracks, renderSource, getRowKey, isCurrentTrack, onPlay, onAppend }) =>
          React.createElement(
            'div',
            { 'data-list': true },
            tracks.map((track, index) =>
              React.createElement(
                'div',
                {
                  key: getRowKey?.(track) ?? track.key,
                  'data-row': track.key,
                  'data-current': isCurrentTrack?.(track)
                },
                renderSource(track),
                React.createElement(
                  'button',
                  { onClick: () => onPlay(index, tracks), 'aria-label': `播放 ${track.key}` },
                  track.title
                ),
                React.createElement(
                  'button',
                  { onClick: () => onAppend(track), 'aria-label': `下一首 ${track.key}` },
                  '下一首'
                )
              )
            )
          )
      }
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
    modules['@/components/ui/dialog'] = load('../src/components/ui/dialog.tsx')
    const { AggregateTrackResults } = load('../src/features/search/aggregate-track-results.tsx')
    const makeTrack = (key, source) => ({
      key,
      title: '同一首歌',
      album: '原专辑',
      artist: '歌手',
      durationMs: 200000,
      source: { kind: 'online', track: { reference: { source } } }
    })
    const first = makeTrack('n:1', 'n'),
      second = makeTrack('q:1', 'q'),
      edition = makeTrack('n:2', 'n')
    const group = {
      id: 'group',
      representative: first,
      sourceCount: 2,
      members: [
        { source: 'n', track: first },
        { source: 'n', track: edition },
        { source: 'q', track: second }
      ]
    }
    const render = async (groups) =>
      act(async () =>
        root.render(
          React.createElement(AggregateTrackResults, {
            groups,
            hasMore: false,
            currentKey: second.key,
            onError: (cause) => {
              throw cause
            },
            onNotice: (notice) => notices.push(notice)
          })
        )
      )
    await render([group])
    const trigger = document.querySelector('[aria-haspopup="dialog"]')
    assert.ok(document.querySelector('[data-row="n:1"][data-current="true"]'))
    trigger.focus()
    await act(async () => trigger.click())
    const dialog = document.querySelector('[role="dialog"]')
    assert.ok(dialog)
    assert.equal(calls.length, 0)
    assert.equal(dialog.querySelectorAll('[data-row]').length, 3)
    assert.match(dialog.textContent, /主列表展示/)
    assert.match(dialog.textContent, /原专辑 · 200s/)
    await act(async () => dialog.querySelector('[aria-label="播放 q:1"]').click())
    assert.deepEqual(
      calls.map((call) => ({ ...call, keys: [...call.keys] })),
      [{ command: 'play_queue', keys: ['q:1'], index: 0 }]
    )
    await act(async () => dialog.querySelector('[aria-label="下一首 n:2"]').click())
    assert.deepEqual([...calls[1].keys], ['n:2'])
    assert.equal(calls[1].command, 'append_queue')
    assert.match(notices[0], /下一首/)
    await act(async () =>
      document.activeElement.dispatchEvent(
        new dom.window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true })
      )
    )
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 30))
    })
    assert.equal(document.querySelector('[role="dialog"]'), null)
    assert.equal(document.activeElement, trigger)
    await act(async () =>
      document.querySelector('[data-row="n:1"] > button[aria-label="播放 n:1"]').click()
    )
    assert.deepEqual([...calls[2].keys], ['n:1'])
    await render([{ ...group, members: group.members.slice(0, 2), sourceCount: 1 }])
    assert.match(
      document.querySelector('[aria-haspopup="dialog"]').getAttribute('aria-label'),
      /1 个来源，共 2 首/
    )
    await render([{ ...group, members: group.members.slice(0, 1), sourceCount: 1 }])
    assert.equal(document.querySelector('[aria-haspopup="dialog"]'), null)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    for (const [key, descriptor] of previous) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor)
      else delete globalThis[key]
    }
  }
})
