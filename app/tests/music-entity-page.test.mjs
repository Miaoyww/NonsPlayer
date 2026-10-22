import { createRuntimeCache } from './helpers/runtime-cache.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('uniform details retain cursor and rows on failure, reload only their own source and discard late pages', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const events = new Map()
  const pending = []
  const calls = []
  const playback = []
  const derivations = []
  const navigation = []
  let table
  const modules = {
    '@/lib/runtime-cache': createRuntimeCache(),
    '@/features/account/account': {
      useAccount: (source) => {
        assert.equal(source, 'qqmusic')
        return { profile: { reference: { source, id: 'owner' } } }
      }
    },
    '@/features/music/legacy-reference': { collectionReference: (item) => item.reference },
    '@/features/music/client': {
      musicClient: {
        currentAccount: async () => ({ reference: { source: 'qqmusic', id: 'owner' } })
      }
    },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ navigate: (...args) => navigation.push(args) })
    },
    './playlist-targets': {
      derivePlaylist: async (reference) => {
        derivations.push(reference)
        return 'local-version'
      }
    },
    './aggregate-library': {
      useAggregateLibrary: () => ({ reload: async () => {} }),
      aggregateCollection: (p) => ({ ...p, localId: p.id })
    },
    '@base-ui/react/menu': {
      Menu: {
        ...Object.fromEntries(
          ['Root', 'Portal', 'Positioner', 'Popup'].map((name) => [
            name,
            ({ children }) => React.createElement('div', {}, children)
          ])
        ),
        Trigger: ({ render, children, ...props }) => React.cloneElement(render, props, children),
        Item: ({ children, ...props }) => React.createElement('button', props, children)
      }
    },
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (event, callback) => {
        events.set(event, callback)
        return () => events.delete(event)
      }
    },
    '@/components/music/music-page': {
      MusicPage: 'section',
      MusicPageHeader: ({ title }) => React.createElement('h1', {}, title)
    },
    '@/components/music/track-list': {
      TrackList: (props) => {
        table = props
        return React.createElement(
          'div',
          { id: 'tracks' },
          props.tracks.map((track) => track.key).join(',')
        )
      }
    },
    '@/components/music/infinite-load': {
      InfiniteLoad: ({ onLoad, error }) =>
        React.createElement('button', { id: 'load-more', onClick: onLoad }, error ?? '续载')
    },
    '@/lib/player': {
      errorText: String,
      usePlayer: () => ({ index: null, queue: [] }),
      nativeCall: async (...args) => {
        calls.push(args)
      }
    },
    './client': {
      musicClient: {
        query: (source, request) =>
          new Promise((resolve, reject) => pending.push({ source, request, resolve, reject })),
        storedTracks: async (source, items) =>
          items.map((item) => ({ key: `${source}:${item.id}` }))
      }
    }
  }
  const exports = {}
  Object.assign(modules, {
    '@/features/library/detail-header': { DetailHeader: 'header' },
    '@/features/library/detail-description': {
      DetailDescription: ({ description }) => React.createElement('p', {}, description)
    },
    '@/components/music/cover': {
      Cover: ({ cover }) => React.createElement('img', { src: cover })
    },
    '@/components/music/action-button': {
      ActionButton: ({ children, ...props }) => React.createElement('button', props, children)
    },
    'lucide-react': { Play: () => null, Ellipsis: () => null, GitBranch: () => null },
    '@/features/library/collection-tabs': { CollectionTabs: ({ children }) => children },
    '@/features/library/library-api': {
      playLibraryCollection: async (collection, key) => {
        playback.push({ collection, key })
        return false
      }
    },
    '@/features/library/artist-page': {
      __esModule: true,
      default: () => React.createElement('p', {}, '歌手完整详情')
    },
    '@/features/library/album-page': {
      __esModule: true,
      default: () => React.createElement('p', {}, '专辑完整详情')
    }
  })
  for (const [name, file] of [
    [
      '@/features/aggregate/platform-playlist-menu',
      '../src/features/aggregate/platform-playlist-menu.tsx'
    ],
    ['@/features/library/collection-header', '../src/features/library/collection-header.tsx'],
    ['./entities', '../src/features/music/entities.ts']
  ]) {
    const loaded = {}
    runInNewContext(
      ts.transpileModule(readFileSync(new URL(file, import.meta.url), 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      { exports: loaded, require: (name) => modules[name] }
    )
    modules[name] = loaded
  }
  const code = ts.transpileModule(
    readFileSync(new URL('../src/features/music/entity-page.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  ).outputText
  runInNewContext(code, { exports, require: (name) => modules[name] })
  const root = createRoot(document.getElementById('root'))
  const collection = {
    id: 0,
    kind: 'playlist',
    name: 'Album',
    reference: { source: 'qqmusic', kind: 'playlist', id: 'album/雪:1' }
  }
  const page = (id, nextCursor) => ({ type: 'tracks', data: { items: [{ id }], nextCursor } })
  const click = () =>
    act(async () =>
      document
        .querySelector('#load-more')
        .dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }))
    )
  try {
    await act(async () =>
      root.render(React.createElement(exports.default, { collection, onError() {} }))
    )
    assert.equal(pending[0].source, 'qqmusic')
    assert.equal(pending[0].request.entity.id, 'album/雪:1')
    await act(async () => pending[0].resolve(page('one', 'opaque:next')))
    await act(async () =>
      pending[1].resolve({
        type: 'detail',
        data: {
          item: {
            ...collection,
            reference: collection.reference,
            name: '完整歌单详情',
            cover: 'cover-fixture.jpg',
            subtitle: '歌单创建者',
            trackCount: 200,
            creator: { source: 'qqmusic', id: 'owner' },
            artists: [],
            liked: false
          },
          description: '歌单简介'
        }
      })
    )
    assert.equal(document.querySelector('img')?.getAttribute('src'), 'cover-fixture.jpg')
    assert.match(document.body.textContent, /歌单创建者/)
    assert.match(document.body.textContent, /200 首音乐/)
    const menu = document.querySelector('header [aria-label="歌单菜单"]')
    assert.ok(menu, 'reference-based playlist detail must show the shared menu beside Play')
    assert.ok(menu.closest('.library-detail-actions'))
    const derive = [...document.querySelectorAll('header button')].find(
      (button) => button.textContent === '派生为聚合歌单'
    )
    assert.ok(derive)
    assert.equal(derive.disabled, false)
    await act(async () => derive.click())
    assert.equal(derivations[0].id, collection.reference.id)
    assert.equal(navigation[0][0], 'local-playlist')
    assert.equal(navigation[0][2].localId, 'local-version')
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:one')
    await act(async () => document.querySelector('header button').click())
    assert.equal(
      playback[0].key,
      'qqmusic:one',
      'start the loaded first song before hydrating the remaining pages'
    )
    await click()
    assert.equal(pending[2].request.page.cursor, 'opaque:next')
    await act(async () => pending[2].reject(new Error('network failed')))
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:one')
    await click()
    assert.equal(pending[3].request.page.cursor, 'opaque:next')
    await act(async () => pending[3].resolve(page('two', null)))
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:one,qqmusic:two')
    await act(async () => table.onAppend({ key: 'qqmusic:two' }))
    assert.equal(calls[0][0], 'append_queue')
    await act(async () => events.get('music-account-changed')({ payload: 'netease' }))
    assert.equal(pending.length, 4)
    await act(async () => events.get('music-account-changed')({ payload: 'qqmusic' }))
    assert.equal(document.getElementById('tracks').textContent, '')
    assert.equal(pending[4].request.page.cursor, null)
    await act(async () => events.get('adapters-changed')({ payload: 'qqmusic' }))
    await act(async () => pending[4].resolve(page('stale', null)))
    assert.equal(document.getElementById('tracks').textContent, '')
    await act(async () => pending[6].resolve(page('fresh', null)))
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:fresh')
    await act(async () => root.render(null))
    await act(async () =>
      root.render(React.createElement(exports.default, { collection, onError() {} }))
    )
    assert.equal(table.loading, false, 'cached first page must render without a skeleton')
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:fresh')
    await act(async () => pending[8].resolve(page('replacement', null)))
    assert.equal(
      document.getElementById('tracks').textContent,
      'qqmusic:replacement',
      'background first page replaces cached rows'
    )
    for (const kind of ['artist', 'album']) {
      await act(async () =>
        root.render(
          React.createElement(exports.default, {
            collection: { ...collection, kind, reference: { ...collection.reference, kind } },
            onError() {}
          })
        )
      )
      assert.match(document.body.textContent, kind === 'artist' ? /歌手完整详情/ : /专辑完整详情/)
    }
  } finally {
    await act(async () => root.unmount())
    assert.equal(events.size, 0)
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    dom.window.close()
  }
})
