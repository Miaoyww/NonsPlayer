import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('likes refresh the loaded playlist in the background without unmounting its song list', async () => {
  const source = readFileSync(
    new URL('../src/features/library/music-library.tsx', import.meta.url),
    'utf8'
  )
  const modules = {}
  const placeholder = ({ children }) => React.createElement('div', {}, children)
  const parsed = ts.createSourceFile(
    'library.tsx',
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  )
  for (const statement of parsed.statements) {
    if (!ts.isImportDeclaration(statement)) continue
    const bindings = statement.importClause?.namedBindings
    if (bindings && ts.isNamedImports(bindings))
      modules[statement.moduleSpecifier.text] = Object.fromEntries(
        bindings.elements.map((item) => [item.name.text, placeholder])
      )
  }
  const profile = { userId: '1', reference: { source: 'netease', id: '1' } }
  const collection = { kind: 'playlist', id: 1, name: '我喜欢的音乐', liked: true, creatorId: '1' }
  const summary = { profile, likedPlaylist: collection, likedTracks: [] }
  const account = { profile, likesRevision: 0 }
  const requests = []
  let mounts = 0
  Object.assign(modules, {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': { usePlayer: () => ({ index: null, queue: [] }), errorText: String },
    '@/features/account/account': { useAccount: () => account },
    '@/features/library/collection-actions': { useCollectionActions: () => ({ revision: 0 }) },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'collection', collection } })
    },
    '@/features/library/library-api': {
      peekMusicLibrary: () => summary,
      peekLibraryCollections: () => undefined,
      getMusicLibrary: async () => summary,
      getLibraryTracks: (_collection, offset) =>
        new Promise((resolve, reject) => requests.push({ offset, resolve, reject }))
    },
    '@/components/music/track-list': {
      TrackList: ({ tracks }) => {
        const [value, setValue] = React.useState('')
        React.useEffect(() => {
          mounts++
        }, [])
        return React.createElement(
          'div',
          { id: 'songs' },
          React.createElement('input', {
            value,
            onChange: (event) => setValue(event.target.value)
          }),
          tracks.map((track) =>
            React.createElement('span', { key: track.key, 'data-key': track.key }, track.title)
          )
        )
      }
    },
    '@/components/music/infinite-load': {
      InfiniteLoad: ({ onLoad, error }) =>
        React.createElement('button', { id: 'more', onClick: onLoad }, error ?? 'More')
    }
  })
  function load(path, text) {
    const exports = {}
    runInNewContext(
      ts.transpileModule(text ?? readFileSync(new URL(path, import.meta.url), 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        require: (name) => (name === './player' ? modules['@/lib/player'] : modules[name])
      }
    )
    return exports
  }
  modules['@/lib/use-paged-list'] = load('../src/lib/use-paged-list.ts')
  modules['@/features/library/reconcile-tracks'] = load(
    '../src/features/library/reconcile-tracks.ts'
  )
  const library = load(null, source)
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
  const render = () =>
    act(async () =>
      root.render(React.createElement(library.default, { onError() {}, onNotice() {} }))
    )
  const a = { key: 'a', title: 'A' },
    b = { key: 'b', title: 'B' },
    c = { key: 'c', title: 'C' }
  const page = (tracks, more = true) => ({ tracks, more, total: 300 })
  try {
    await render()
    await act(async () => requests[0].resolve(page([a, b])))
    await act(async () => document.getElementById('more').click())
    await act(async () => requests[1].resolve(page([c])))
    const list = document.getElementById('songs')
    const rowA = document.querySelector('[data-key="a"]')
    const rowC = document.querySelector('[data-key="c"]')
    const input = document.querySelector('input')
    const propsKey = Object.keys(input).find((key) => key.startsWith('__reactProps'))
    await act(async () => input[propsKey].onChange({ target: { value: 'saved query' } }))
    account.likesRevision++
    await render()
    assert.equal(document.getElementById('songs'), list)
    assert.equal(document.querySelectorAll('[data-key]').length, 3)
    assert.equal(requests[2].offset, 0)
    await act(async () => requests[2].resolve(page([{ key: 'new', title: 'New' }, { ...a }])))
    assert.equal(document.querySelectorAll('[data-key]').length, 3)
    assert.equal(requests[3].offset, 100)
    await act(async () => requests[3].resolve(page([{ ...c }], false)))
    assert.deepEqual(
      [...document.querySelectorAll('[data-key]')].map((element) => element.dataset.key),
      ['new', 'a', 'c']
    )
    assert.equal(document.querySelector('[data-key="a"]'), rowA)
    assert.equal(document.querySelector('[data-key="c"]'), rowC)
    assert.equal(document.querySelector('input').value, 'saved query')
    assert.equal(mounts, 1)
    account.likesRevision++
    await render()
    await act(async () => requests[4].reject(new Error('offline')))
    assert.equal(document.getElementById('songs'), list)
    assert.match(document.getElementById('more').textContent, /offline/)
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

test('playlist detail siblings have distinct keys and tabs reset when switching playlists', async () => {
  const source = readFileSync(
    new URL('../src/features/library/music-library.tsx', import.meta.url),
    'utf8'
  )
  const modules = {}
  const placeholder = ({ children }) => React.createElement('div', {}, children)
  const parsed = ts.createSourceFile(
    'music-library.tsx',
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  )
  for (const statement of parsed.statements) {
    if (!ts.isImportDeclaration(statement)) continue
    const bindings = statement.importClause?.namedBindings
    if (bindings && ts.isNamedImports(bindings))
      modules[statement.moduleSpecifier.text] = Object.fromEntries(
        bindings.elements.map((item) => [item.name.text, placeholder])
      )
  }
  let collection = { kind: 'playlist', id: 18282029444, name: 'First' }
  Object.assign(modules, {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => false },
    '@/lib/player': { usePlayer: () => ({ index: null, queue: [] }) },
    '@/features/account/account': { useAccount: () => ({ profile: null }) },
    '@/features/library/collection-actions': { useCollectionActions: () => ({ revision: 0 }) },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'collection', collection } })
    },
    '@/lib/use-paged-list': { usePagedList: () => ({ items: [], more: false, busy: false }) },
    '@/features/library/collection-header': {
      CollectionHeader: ({ collection }) => React.createElement('h1', {}, collection.name)
    },
    '@/features/library/collection-tabs': {
      CollectionTabs: ({ children }) => {
        const [value, setValue] = React.useState(0)
        return React.createElement(
          'section',
          {},
          React.createElement('button', { onClick: () => setValue(value + 1) }, String(value)),
          children
        )
      }
    }
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
  const errors = [],
    originalError = console.error
  console.error = (...args) => errors.push(args.map(String).join(' '))
  const root = createRoot(document.getElementById('root'))
  const render = () =>
    act(async () =>
      root.render(React.createElement(exports.default, { onError() {}, onNotice() {} }))
    )
  try {
    await render()
    assert.deepEqual(
      errors.filter((error) => error.includes('same key')),
      []
    )
    await act(async () => document.querySelector('section button').click())
    assert.equal(document.querySelector('section button').textContent, '1')
    collection = { ...collection, id: 42, name: 'Second' }
    await render()
    assert.equal(document.querySelectorAll('h1').length, 1)
    assert.equal(document.querySelector('h1').textContent, 'Second')
    assert.equal(document.querySelector('section button').textContent, '0')
    assert.deepEqual(
      errors.filter((error) => error.includes('same key')),
      []
    )
  } finally {
    await act(async () => root.unmount())
    console.error = originalError
    dom.window.close()
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      IS_REACT_ACT_ENVIRONMENT: previous.act
    })
  }
})

test('library home limits the featured preview to twelve without changing the favorites total', async () => {
  const source = readFileSync(
    new URL('../src/features/library/music-library.tsx', import.meta.url),
    'utf8'
  )
  const modules = {}
  const placeholder = ({ children }) => React.createElement('div', {}, children)
  const parsed = ts.createSourceFile(
    'library.tsx',
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  )
  for (const statement of parsed.statements) {
    if (!ts.isImportDeclaration(statement)) continue
    const bindings = statement.importClause?.namedBindings
    if (bindings && ts.isNamedImports(bindings))
      modules[statement.moduleSpecifier.text] = Object.fromEntries(
        bindings.elements.map((item) => [item.name.text, placeholder])
      )
  }
  const profile = { userId: 'qq-user', nickname: 'QQ', reference: { source: 'qqmusic' } }
  const summary = {
    profile,
    likedPlaylist: { kind: 'playlist', id: 'liked', trackCount: 456 },
    likedTracks: Array.from({ length: 25 }, (_, i) => ({ key: String(i), title: `Song ${i}` }))
  }
  Object.assign(modules, {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': { usePlayer: () => ({ index: null, queue: [] }) },
    '@/features/account/account': { useAccount: () => ({ profile }) },
    '@/features/library/collection-actions': {
      useCollectionActions: () => ({ revision: 0 }),
      CollectionContextMenu: placeholder
    },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'library' } })
    },
    '@/lib/use-paged-list': { usePagedList: () => ({ items: [], more: false, busy: false }) },
    '@/features/library/library-api': {
      peekMusicLibrary: () => summary,
      getMusicLibrary: async () => summary
    },
    '@/features/library/reconcile-tracks': { reconcileTracks: (_previous, next) => next },
    '@/components/music/music-links': { TrackArtists: () => React.createElement('span') },
    '@/components/music/list-find': {
      FindSongGrid: ({ tracks, children }) =>
        React.createElement('div', { id: 'featured', 'data-count': tracks.length }, children)
    }
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
    IS_REACT_ACT_ENVIRONMENT: globalThis.IS_REACT_ACT_ENVIRONMENT
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
        React.createElement(exports.default, { onError() {}, onNotice() {}, readOnly: true })
      )
    )
    assert.equal(document.getElementById('featured').dataset.count, '12')
    assert.equal(document.getElementById('featured').children.length, 12)
    assert.match(document.querySelector('.library-liked-count').textContent, /456 首歌/)
    assert.equal(summary.likedTracks.length, 25)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    Object.assign(globalThis, previous)
  }
})
