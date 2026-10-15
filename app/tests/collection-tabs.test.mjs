import { musicReference } from './helpers/music-reference.mjs'
import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function load(path, modules) {
  modules = { '@/features/music/cover-image': coverImage, ...modules }
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name === '@/features/music/legacy-reference' || name in modules, name)
        return name === '@/features/music/legacy-reference' ? musicReference : modules[name]
      }
    }
  )
  return exports
}
function registry() {
  const scopes = load('../src/plugins/scope.tsx', { react: React })
  return load('../src/plugins/collection-tabs.ts', { './scope': scopes })
}
function scope(id = 'sample', generation = 1) {
  return { active: true, descriptor: { generation, manifest: { id, permissions: ['ui'] } } }
}

test('tab registry namespaces plugins and generations, validates registration and cleans up once', () => {
  const store = registry(),
    first = scope(),
    second = scope('other')
  const tab = { id: 'comments', label: '评论', component() {} }
  let notifications = 0
  const unsubscribe = store.subscribeCollectionTabs(() => notifications++)
  const cleanup = store.registerCollectionTab(first, tab)
  store.registerCollectionTab(second, tab)
  const snapshot = store.collectionTabSnapshot()
  assert.equal(snapshot.length, 2)
  assert.equal(store.collectionTabSnapshot(), snapshot, 'snapshot stays stable until a mutation')
  assert.notEqual(snapshot[0].key, snapshot[1].key)
  assert.throws(() => store.registerCollectionTab(first, tab), /重复/)
  for (const invalid of [
    { id: '../escape' },
    { label: '' },
    { component: null },
    { kinds: ['video'] },
    { sources: ['other'] }
  ])
    assert.throws(() => store.registerCollectionTab(first, { ...tab, ...invalid }))
  first.active = false
  first.cleanups.forEach((dispose) => dispose())
  cleanup()
  assert.equal(notifications, 3)
  assert.equal(store.collectionTabSnapshot().length, 1)
  assert.equal(store.collectionTabSnapshot()[0].scope, second)
  const replacement = scope('sample', 2)
  store.registerCollectionTab(replacement, tab)
  assert.notEqual(store.collectionTabSnapshot()[1].key, snapshot[0].key)
  unsubscribe()
})

test('collection tabs default to songs, match loaded scopes and return to songs after removal', async () => {
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
  const store = registry(),
    owner = scope(),
    loaded = new Map()
  const tabContext = React.createContext(null),
    searchContext = React.createContext(undefined)
  const tabs = {
    Tabs: ({ value, onValueChange, children, className }) =>
      React.createElement(
        tabContext.Provider,
        { value: { value, onValueChange } },
        React.createElement('div', { className }, children)
      ),
    TabsList: ({ children, ...props }) =>
      React.createElement('div', { ...props, role: 'tablist' }, children),
    TabsTab: ({ value, children }) => {
      const tab = React.useContext(tabContext)
      return React.createElement(
        'button',
        {
          role: 'tab',
          'aria-selected': tab.value === value,
          onClick: () => tab.onValueChange(value)
        },
        children
      )
    },
    TabsPanel: ({ value, children }) =>
      React.useContext(tabContext).value === value
        ? React.createElement('div', { role: 'tabpanel' }, children)
        : null
  }
  let mounts = 0,
    unmounts = 0,
    received
  function Comments({ collection }) {
    received = collection
    React.useEffect(() => {
      mounts++
      return () => unmounts++
    }, [])
    return React.createElement('p', {}, '插件评论内容')
  }
  function Songs() {
    return React.createElement('p', {}, `歌曲搜索：${React.useContext(searchContext)}`)
  }
  const { CollectionTabs } = load('../src/features/library/collection-tabs.tsx', {
    '@/lib/platform': { isAndroid: () => false },
    react: React,
    'react/jsx-runtime': jsx,
    'lucide-react': { Music2: () => null, Search: () => null },
    '@/components/animate-ui/components/base/tabs': tabs,
    '@/components/ui/input': { Input: 'input' },
    '@/components/music/track-search-context': { TrackSearchContext: searchContext },
    '@/plugins/collection-tabs': store,
    '@/plugins/host': {
      usePlugins: () => ({ loaded }),
      PluginCollectionTab: ({ entry, collection }) =>
        React.createElement(entry.component, { collection })
    }
  })
  const root = createRoot(document.getElementById('root'))
  const collection = {
    id: 42,
    kind: 'playlist',
    name: '歌单',
    subtitle: '作者',
    cover: 'C:/private/cover.jpg',
    trackCount: 3
  }
  const render = async (item = collection) =>
    act(async () =>
      root.render(
        React.createElement(
          CollectionTabs,
          { key: `${item.kind}:${item.localId ?? item.id}`, collection: item },
          React.createElement(Songs)
        )
      )
    )
  const click = async (name) =>
    act(async () =>
      Array.from(document.querySelectorAll('[role="tab"]'))
        .find((tab) => tab.textContent === name)
        .click()
    )
  try {
    const dispose = store.registerCollectionTab(owner, {
      id: 'comments',
      label: '评论',
      kinds: ['playlist'],
      sources: ['netease'],
      component: Comments
    })
    await render()
    assert.equal(
      document.querySelectorAll('[role="tab"]').length,
      1,
      'unpublished activation is hidden'
    )
    assert.equal(document.querySelector('[role="tab"]').getAttribute('aria-selected'), 'true')
    assert.equal(document.querySelector('[role="tablist"]').className, 'music-tabs')
    loaded.set('sample', { scope: owner })
    await render()
    assert.equal(document.querySelectorAll('[role="tab"]').length, 2)
    assert.equal(mounts, 0, 'inactive plugin panel must not mount')
    const input = document.querySelector('input')
    const propsKey = Object.keys(input).find((key) => key.startsWith('__reactProps'))
    await act(async () => input[propsKey].onChange({ target: { value: 'Beta' } }))
    assert.match(document.body.textContent, /歌曲搜索：Beta/)
    await click('评论')
    assert.equal(mounts, 1)
    assert.equal(received.id, 42)
    assert.equal(received.source, 'netease')
    assert.equal(received.cover, '', 'local cover paths are omitted')
    assert.equal(document.querySelector('input'), null)
    await click('歌曲')
    assert.equal(unmounts, 1)
    assert.equal(document.querySelector('input').value, 'Beta')
    await click('评论')
    await act(async () => dispose())
    assert.equal(document.querySelectorAll('[role="tab"]').length, 1)
    assert.equal(document.querySelector('[role="tab"]').getAttribute('aria-selected'), 'true')
    assert.equal(unmounts, 2)
    let again
    await act(async () => {
      again = store.registerCollectionTab(owner, {
        id: 'comments',
        label: '评论',
        kinds: ['playlist'],
        sources: ['netease'],
        component: Comments
      })
    })
    await render()
    assert.equal(
      document.querySelector('[role="tab"]').getAttribute('aria-selected'),
      'true',
      'reregistering does not revive selection'
    )
    await click('评论')
    await render({ ...collection, id: 43 })
    assert.equal(document.querySelector('[role="tab"]').getAttribute('aria-selected'), 'true')
    assert.equal(document.querySelector('input').value, '')
    await render({ ...collection, localId: 'local-playlist' })
    assert.equal(
      document.querySelectorAll('[role="tab"]').length,
      1,
      'source filter hides online tabs'
    )
    await render({ ...collection, kind: 'album' })
    assert.equal(
      document.querySelectorAll('[role="tab"]').length,
      1,
      'kind filter hides playlist tabs'
    )
    await act(async () => again())
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
