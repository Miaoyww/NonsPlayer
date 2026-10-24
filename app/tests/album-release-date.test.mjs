import { createRuntimeCache } from './helpers/runtime-cache.mjs'
import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('album cards show current metadata dates without per-card detail requests', async () => {
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
    '@/lib/runtime-cache': createRuntimeCache(),
    '@/features/music/references': musicReference,
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
    id: '142163139',
    reference: { source: 'netease', kind: 'album', id: '142163139' },
    kind: 'album',
    name: 'Summer Ghost',
    trackCount: 29,
    cover: '',
    subtitle: '',
    publishedAt: null
  }
  const render = (item) =>
    act(async () =>
      root.render(
        React.createElement(AlbumCard, {
          item: { ...item, reference: { ...item.reference, id: String(item.id) } },
          busy: false,
          onOpen() {},
          onPlay() {}
        })
      )
    )
  try {
    await render(album)
    assert.match(document.body.textContent, /发行日期未知/)
    await render({ ...album, publishedAt: 1648137600000 })
    assert.match(document.body.textContent, /29 首 · 2022-03-25/)
    assert.equal(requests.length, 0, 'album cards use their current metadata')
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
