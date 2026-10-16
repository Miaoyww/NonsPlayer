import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

test('source filter clicks reuse loaded pages; only scrolling loads more and retry recovers a failed page', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const requests = []
  const observers = new Set()
  let intersecting = false
  class Observer {
    constructor(callback) {
      this.callback = callback
    }
    observe() {
      observers.add(this)
      this.callback([{ isIntersecting: intersecting }])
    }
    disconnect() {
      observers.delete(this)
    }
  }
  const Empty = () => null
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': { usePlayer: () => ({ index: null, queue: [] }), errorText: String },
    '@/lib/platform': { isAndroid: () => false },
    '@/features/workspace/music-navigation': { useMusicNavigation: () => ({ navigate() {} }) },
    '@/features/music/client': {
      musicClient: {
        query: (source, request) =>
          new Promise((resolve, reject) => requests.push({ source, request, resolve, reject })),
        queueTracks: async (_, tracks) => tracks
      }
    },
    '@/features/music/source-label': { trackSource: (track) => track.source },
    '@/components/music/action-button': {
      ActionButton: ({ size, variant, ...props }) => React.createElement('button', props)
    },
    '@/components/music/loading': { TrackListSkeleton: Empty, CollectionGridSkeleton: Empty },
    '@/components/ui/skeleton': { Skeleton: Empty },
    '@/components/music/track-list': {
      TrackList: ({ tracks }) =>
        React.createElement('div', { 'data-tracks': true }, tracks.map((t) => t.key).join(','))
    },
    './aggregate-track-results': { AggregateTrackResults: Empty },
    './track-aggregation': {
      TrackAggregation: class {
        append() {}
        getSnapshot() {
          return []
        }
      }
    }
  }
  function load(path, transform = (value) => value) {
    const exports = {}
    runInNewContext(
      ts.transpileModule(transform(readFileSync(new URL(path, import.meta.url), 'utf8')), {
        compilerOptions: {
          module: ts.ModuleKind.CommonJS,
          jsx: ts.JsxEmit.ReactJSX,
          target: ts.ScriptTarget.ES2022
        }
      }).outputText,
      {
        exports,
        IntersectionObserver: Observer,
        getComputedStyle: dom.window.getComputedStyle.bind(dom.window),
        require: (name) => modules[name] ?? new Proxy({}, { get: () => Empty })
      }
    )
    return exports
  }
  modules['./aggregate-query'] = load('../src/features/search/aggregate-query.ts')
  modules['@/components/music/infinite-load'] = load('../src/components/music/infinite-load.tsx')
  const { Results } = load('../src/features/search/search-page.tsx', (source) =>
    source.replace('function Results(', 'export function Results(')
  )
  const root = createRoot(document.getElementById('root'))
  const sources = [
    { source: 'netease', name: '网易云音乐' },
    { source: 'qqmusic', name: 'QQ音乐' }
  ]
  const page = (source, id) => ({
    type: 'tracks',
    data: { items: [{ key: `${source}:${id}`, source }], nextCursor: `${id}-next` }
  })
  const click = async (name) => {
    const button = [...document.querySelectorAll('button')].find((b) =>
      b.textContent.startsWith(name)
    )
    assert.ok(button, name)
    await act(async () => button.click())
  }
  try {
    await act(async () =>
      root.render(
        React.createElement(Results, {
          kind: 'track',
          keyword: '烂泥',
          sources,
          onError() {},
          onNotice() {}
        })
      )
    )
    assert.equal(requests.length, 2)
    await act(async () => {
      for (const r of requests.slice()) r.resolve(page(r.source, '1'))
    })
    await click('网易云音乐')
    await click('网易云音乐')
    await click('QQ音乐')
    await click('聚合')
    assert.equal(
      requests.length,
      2,
      'Changing an already-loaded filter must not fetch another page'
    )
    await click('QQ音乐')
    await act(async () => {
      intersecting = true
      for (const observer of [...observers]) observer.callback([{ isIntersecting: true }])
    })
    assert.equal(requests.length, 3)
    assert.equal(requests[2].source, 'qqmusic')
    assert.equal(requests[2].request.page.cursor, '1-next')
    intersecting = false
    await act(async () => requests[2].reject(new Error('网络失败')))
    assert.match(document.querySelector('[data-tracks]').textContent, /qqmusic:1/)
    await click('重试')
    assert.equal(requests.length, 4)
    assert.equal(requests[3].request.page.cursor, '1-next')
    await act(async () => requests[3].resolve(page('qqmusic', '2')))
    assert.match(document.querySelector('[data-tracks]').textContent, /qqmusic:1,qqmusic:2/)
  } finally {
    await act(async () => root.unmount())
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    dom.window.close()
  }
})
