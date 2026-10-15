import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('homepage allows a newer song selection while the previous collection is preparing', async () => {
  const source = readFileSync(
    new URL('../src/features/library/music-library.tsx', import.meta.url),
    'utf8'
  )
  const placeholder = ({ children }) => React.createElement('div', {}, children)
  const modules = {}
  for (const statement of ts.createSourceFile(
    'library.tsx',
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  ).statements) {
    if (!ts.isImportDeclaration(statement)) continue
    const bindings = statement.importClause?.namedBindings
    if (bindings && ts.isNamedImports(bindings))
      modules[statement.moduleSpecifier.text] = Object.fromEntries(
        bindings.elements.map((item) => [item.name.text, placeholder])
      )
  }
  const collection = { kind: 'playlist', id: 1, name: 'Liked' }
  const summary = {
    profile: { userId: '1', reference: { source: 'netease', id: '1' } },
    likedPlaylist: collection,
    likedTracks: [
      { key: 'A', title: 'A' },
      { key: 'B', title: 'B' }
    ]
  }
  const requests = [],
    errors = [],
    notices = []
  Object.assign(modules, {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': { usePlayer: () => ({ index: null, queue: [] }), errorText: String },
    '@/features/account/account': { useAccount: () => ({ profile: summary.profile }) },
    '@/features/library/collection-actions': {
      useCollectionActions: () => ({ revision: 0 }),
      CollectionContextMenu: placeholder
    },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'library' }, navigate() {} })
    },
    '@/lib/use-paged-list': { usePagedList: () => ({ items: [], more: false, busy: false }) },
    '@/features/library/library-api': {
      peekMusicLibrary: () => summary,
      peekLibraryCollections: () => undefined,
      getMusicLibrary: async () => summary,
      playLibraryCollection: (_collection, key) =>
        new Promise((resolve, reject) => requests.push({ key, resolve, reject }))
    },
    '@/components/music/song-actions': {
      SongContextMenu: ({ render, children }) => React.cloneElement(render, {}, children)
    },
    '@/components/music/track-title': { TrackTitle: ({ track }) => track.title },
    '@/components/music/music-links': { TrackArtists: () => null },
    '@/components/music/cover': { Cover: () => null },
    '@/features/library/reconcile-tracks': { reconcileTracks: (_previous, next) => next }
  })
  const exports = {}
  runInNewContext(
    ts.transpileModule(source, {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    { exports, require: (name) => modules[name] }
  )
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    IS_REACT_ACT_ENVIRONMENT: true
  })
  const root = createRoot(document.getElementById('root'))
  try {
    await act(async () =>
      root.render(
        React.createElement(exports.default, {
          onError: (error) => errors.push(error),
          onNotice: (notice) => notices.push(notice)
        })
      )
    )
    await act(async () => document.querySelector('[aria-label="播放 A"]').click())
    assert.equal(requests[0].key, 'A')
    assert.equal(document.querySelector('[aria-label="播放 B"]').disabled, false)
    await act(async () => document.querySelector('[aria-label="播放 B"]').click())
    assert.equal(requests[1].key, 'B')
    await act(async () => requests[0].reject(new Error('late old failure')))
    assert.deepEqual(errors, [])
    await act(async () => requests[1].resolve(true))
    assert.equal(notices.length, 1)
    await act(async () => document.querySelector('[aria-label="播放 A"]').click())
    await act(async () => document.querySelector('[aria-label="播放 B"]').click())
    await act(async () => requests[3].resolve(false))
    await act(async () => requests[2].resolve(true))
    assert.equal(notices.length, 1, 'a late superseded request cannot publish its notice')
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      IS_REACT_ACT_ENVIRONMENT: previous.act
    })
  }
})
