import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('album cards fill missing list dates from visible album details and ignore stale responses', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const requests = [],
    observers = []
  class Observer {
    constructor(callback) {
      this.callback = callback
      observers.push(this)
    }
    observe() {}
    disconnect() {}
  }
  const modules = {
    '@/features/music/legacy-reference': musicReference,
    '@/features/music/entities': { musicCollection: (item) => item },
    '@/features/music/queries': {
      queryMusic: (request, source) =>
        new Promise((resolve, reject) =>
          requests.push({
            command: 'music_query',
            args: { source, request },
            resolve: (data) => resolve({ type: 'detail', data }),
            reject
          })
        )
    },
    '@/components/ui/skeleton': { Skeleton: 'div' },
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': {
      errorText: String,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    },
    '@/features/library/collection-actions': {
      CollectionContextMenu: ({ render, children }) => React.cloneElement(render, {}, children)
    },
    'lucide-react': { Play: () => null },
    '@/components/music/action-button': {
      ActionButton: ({ children, size, ...props }) => React.createElement('button', props, children)
    },
    '@/components/music/cover': { Cover: () => null }
  }
  function load(path) {
    const exports = {}
    const code = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText
    runInNewContext(code, {
      exports,
      require: (name) => modules[name],
      IntersectionObserver: Observer
    })
    return exports
  }
  modules['@/features/library/music-entities'] = load('../src/features/library/music-entities.ts')
  modules['@/features/workspace/music-navigation'] = {}
  modules['@/lib/player'] = {}
  modules['@/components/music/music-links'] = load('../src/components/music/music-links.tsx')
  const { AlbumCard } = load('../src/features/library/album-card.tsx')
  const root = createRoot(document.getElementById('root'))
  const album = {
    id: 142163139,
    kind: 'album',
    name: 'Summer Ghost',
    trackCount: 29,
    cover: '',
    subtitle: '',
    publishedAt: null
  }
  const render = (item) =>
    act(async () =>
      root.render(React.createElement(AlbumCard, { item, busy: false, onOpen() {}, onPlay() {} }))
    )
  const visible = () => act(async () => observers.at(-1)?.callback([{ isIntersecting: true }]))
  try {
    await render(album)
    assert.equal(requests.length, 0, 'offscreen cards must not fetch details')
    await visible()
    assert.equal(requests.length, 1, 'missing list date must trigger album detail lookup')
    assert.equal(requests[0].command, 'music_query')
    assert.equal(requests[0].args.request.entity.id, String(album.id))
    await act(async () => requests[0].resolve({ item: { ...album, publishedAt: 1648137600000 } }))
    assert.match(document.body.textContent, /29 首 · 2022-03-25/)
    await render({ ...album, id: 2 })
    const stale = requests.at(-1)
    await render({ ...album, id: 3 })
    await act(async () => stale.resolve({ item: { ...album, id: 2, publishedAt: 1648137600000 } }))
    assert.doesNotMatch(document.body.textContent, /2022-03-25/)
    await act(async () => requests.at(-1).reject(new Error('offline')))
    assert.match(document.body.textContent, /发行日期未知/)
    const count = requests.length
    await render({ ...album, id: 4, publishedAt: 1648137600000 })
    assert.equal(requests.length, count, 'complete list metadata must not fetch details')
    assert.match(document.body.textContent, /2022-03-25/)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
