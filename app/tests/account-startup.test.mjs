import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInThisContext } from 'node:vm'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { LRUCache } from 'lru-cache'

function load(file, modules) {
  const exports = {}
  const { outputText } = ts.transpileModule(readFileSync(new URL(file, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  runInThisContext(`(function(exports, require) { ${outputText}\n })`)(exports, (name) =>
    name === '@/lib/diagnostics' ? { ...modules[name], reportCaughtError() {} } : modules[name]
  )
  return exports
}

for (const changedMetadata of [false, true])
  test(`startup validation and another adapter becoming ready preserve pending library queries (metadata change: ${changedMetadata})`, async () => {
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
    const record = {
      reference: { source: 'netease', id: 'listener' },
      displayName: 'Listener',
      avatar: ''
    }
    let statuses = [
      {
        enabled: true,
        descriptor: {
          source: 'netease',
          capabilities: ['account', 'favorites', 'userLibrary', 'browse']
        }
      },
      {
        enabled: false,
        loading: true,
        descriptor: { source: 'qqmusic', capabilities: ['account'] }
      }
    ]
    let finishProfile,
      snapshot,
      active = 0,
      currentReads = 0,
      accountResets = 0
    const listeners = new Map()
    const cache = load('../src/lib/runtime-cache.ts', { 'lru-cache': { LRUCache } })
    const jobs = [],
      failures = [],
      queries = []
    const query = (operation, data) =>
      cache.requestCache.get(operation, () => {
        queries.push(operation)
        // Mirror AdapterManager's four ordinary-query permits. Keep the first
        // wave pending while the saved profile is validated.
        if (active >= 4) {
          failures.push(operation)
          return Promise.reject(new Error('RateLimited'))
        }
        active++
        return new Promise((resolve) =>
          jobs.push(() => {
            active--
            resolve(data)
          })
        )
      })
    const empty = () => null
    const wrapper = ({ children }) => React.createElement('div', {}, children)
    const modules = new Proxy(
      {
        react: React,
        'react/jsx-runtime': jsx,
        '@tauri-apps/api/core': { isTauri: () => true },
        '@tauri-apps/api/event': {
          listen: async (event, changed) => {
            const callbacks = listeners.get(event) ?? new Set()
            listeners.set(event, callbacks)
            callbacks.add(changed)
            return () => callbacks.delete(changed)
          }
        },
        '@/lib/player': { errorText: String, usePlayer: () => ({ index: null, queue: [] }) },
        '@/lib/runtime-cache': cache,
        '@/features/music/use-music-sources': {
          useMusicSources: () => ({ preferences: { current: 'netease' }, statuses })
        },
        '@/features/music/client': {
          musicClient: {
            currentAccount: async (source) => {
              currentReads++
              return source === 'netease' ? structuredClone(record) : null
            },
            account: (source) =>
              source === 'netease'
                ? new Promise((resolve) => {
                    finishProfile = resolve
                  })
                : Promise.resolve({ type: 'profile', data: null }),
            query: async (_, request) => ({
              type: 'favorites',
              data: await query(request.operation, [])
            })
          }
        },
        '@/features/library/library-api': {
          resetAccountCache() {
            accountResets++
            cache.invalidateNativeCache(undefined, 'context')
          },
          setLibraryAccount() {},
          invalidateMusicLibrary() {},
          peekMusicLibrary() {},
          peekLibraryCollections() {},
          getMusicLibrary: () =>
            query('librarySummary', { profile: account.accountProfile(record), likedTracks: [] }),
          getLibraryCollections: () => query('libraryCollections', { items: [], more: false })
        },
        '@/features/aggregate/aggregate-library': {
          useAggregateLibrary: () => ({ playlists: [] }),
          useAggregateReplacement: () => () => undefined,
          aggregateCollection: (p) => p
        },
        '@/features/aggregate/playlist-targets': {
          mergeLibraryPlaylists: (remote, local) => [...remote, ...local]
        },
        '@/features/library/collection-actions': {
          useCollectionActions: () => ({ revision: 0 }),
          CollectionContextMenu: wrapper
        },
        '@/features/workspace/music-navigation': {
          useMusicNavigation: () => ({ page: {}, navigate() {} })
        },
        '@/features/library/reconcile-tracks': { reconcileTracks: (_, next) => next },
        '@/components/music/music-page': { MusicPage: wrapper }
      },
      { get: (target, name) => target[name] ?? new Proxy({}, { get: () => empty }) }
    )
    modules['@/lib/use-paged-list'] = load('../src/lib/use-paged-list.ts', {
      react: React,
      './player': { errorText: String },
      '@/lib/runtime-cache': cache
    })
    const account = load('../src/features/account/account-state.tsx', modules)
    modules['@/features/account/account'] = account
    const { default: MusicLibrary } = load('../src/features/library/music-library.tsx', modules)
    function View() {
      snapshot = account.useAccount()
      return React.createElement(MusicLibrary, { onError() {}, onNotice() {} })
    }
    const render = () =>
      act(async () =>
        root.render(React.createElement(account.AccountProvider, {}, React.createElement(View)))
      )
    const emit = (event, payload) =>
      act(async () => {
        for (const changed of listeners.get(event) ?? []) changed({ payload })
      })
    try {
      await render()
      assert.equal(active, 3)
      if (changedMetadata) {
        record.displayName = 'Updated listener'
        record.avatar = 'updated-avatar'
      }
      await act(async () => finishProfile({ type: 'profile', data: structuredClone(record) }))
      assert.equal(snapshot.profile.nickname, record.displayName)
      assert.equal(snapshot.profile.avatarUrl, record.avatar)
      // Bundled loading emits a bulk notification; settings emit a source ID.
      // Neither should restart netease when only qqmusic becomes ready.
      await emit('adapters-changed', null)
      await emit('adapters-changed', 'qqmusic')
      statuses = statuses.map((status) => ({ ...status, enabled: true, loading: false }))
      await render()
      assert.equal(accountResets, 2)
      assert.equal(currentReads, 2)
      await act(async () => jobs.splice(0).forEach((finish) => finish()))
      assert.deepEqual(failures, [], `startup requests were rejected: ${failures.join(', ')}`)
      assert.equal(snapshot.likesReady, true)
      assert.deepEqual(queries.sort(), ['favorites', 'libraryCollections', 'librarySummary'])
      // Account changes refresh the profile without replacing app-global favorites.
      record.reference.id = 'another-listener'
      await emit('music-account-changed', 'netease')
      assert.equal(snapshot.profile.reference.id, 'another-listener')
      assert.equal(snapshot.likesReady, true)
      assert.equal(accountResets, 4)
      await act(async () => finishProfile({ type: 'profile', data: structuredClone(record) }))
      await act(async () => jobs.splice(0).forEach((finish) => finish()))
      assert.equal(snapshot.likesReady, true)
      assert.equal(queries.length, 5)
    } finally {
      await act(async () => root.unmount())
      dom.window.close()
      Object.assign(globalThis, previous)
    }
  })
