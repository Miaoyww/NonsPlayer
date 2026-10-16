import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { musicReference } from './helpers/music-reference.mjs'

function load(file, modules) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(file, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  return exports
}
async function withDom(run) {
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
    await run(root, dom)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      IS_REACT_ACT_ENVIRONMENT: previous.act
    })
  }
}

async function readonlyFavoritesScenario(scenario = 'complete') {
  return withDom(async (root) => {
    const record = {
      reference: { source: 'qqmusic', id: 'account' },
      displayName: '测试账号',
      avatar: ''
    }
    let currentReads = 0,
      writes = 0,
      snapshot
    let finishPage
    const queried = []
    const modules = {
      react: React,
      'react/jsx-runtime': jsx,
      '@tauri-apps/api/core': { isTauri: () => true },
      '@tauri-apps/api/event': { listen: async () => () => {} },
      '@/lib/player': { errorText: String },
      '@/lib/runtime-cache': { invalidateNativeCache() {} },
      '@/features/library/library-api': {
        invalidateMusicLibrary() {},
        resetAccountCache() {},
        setLibraryAccount() {}
      },
      '@/features/music/use-music-sources': {
        useMusicSources: () => ({
          preferences: { current: 'qqmusic' },
          statuses: [
            {
              enabled: true,
              descriptor: { source: 'qqmusic', capabilities: ['account', 'userLibrary', 'browse'] }
            }
          ]
        })
      },
      '@/features/music/client': {
        musicClient: {
          currentAccount: async () => {
            currentReads++
            if (currentReads > 3) throw new Error('unexpected repeated account reset')
            return record
          },
          account: async () => ({ type: 'profile', data: record }),
          query: async (source, request) => {
            queried.push(request)
            assert.equal(source, 'qqmusic')
            if (request.operation === 'librarySummary')
              return {
                type: 'summary',
                data: {
                  account: record,
                  likedPlaylist: { reference: { source, kind: 'playlist', id: 'chosen-likes' } },
                  likedTracks: [],
                  likedError: scenario === 'preview-failure' ? { code: 'network' } : null
                }
              }
            assert.equal(request.operation, 'tracks')
            assert.equal(request.entity.id, 'chosen-likes')
            if (request.page.cursor && scenario === 'failure') throw new Error('page unavailable')
            if (request.page.cursor && scenario === 'switch')
              return new Promise((resolve) => {
                finishPage = resolve
              })
            return {
              type: 'tracks',
              data: {
                items: [
                  {
                    reference: {
                      source,
                      kind: 'track',
                      id: request.page.cursor ? 'OtherMID' : 'SongMID'
                    }
                  }
                ],
                nextCursor:
                  scenario === 'repeat' ? 'opaque-next' : request.page.cursor ? null : 'opaque-next'
              }
            }
          },
          write: async () => {
            writes++
          }
        }
      }
    }
    const account = load('../src/features/account/account-state.tsx', modules)
    modules['@/features/account/account'] = account
    modules['@/features/music/legacy-reference'] = musicReference
    modules['lucide-react'] = { Heart: () => React.createElement('span', {}, 'heart') }
    modules['@/components/ui/button'] = {
      Button: ({ variant, size, children, ...props }) =>
        React.createElement('button', props, children)
    }
    const { CurrentTrackLike } = load('../src/features/playback/current-track-like.tsx', modules)
    function View() {
      snapshot = account.useAccount()
      return React.createElement(CurrentTrackLike, {
        track: {
          source: {
            kind: 'online',
            track: { reference: { source: 'qqmusic', kind: 'track', id: 'SongMID' } }
          }
        },
        onError: (error) => {
          throw error
        }
      })
    }
    const render = () =>
      act(async () =>
        root.render(React.createElement(account.AccountProvider, {}, React.createElement(View)))
      )
    await render()
    if (['failure', 'repeat'].includes(scenario)) {
      assert.equal(snapshot.likesReady, false)
      assert.equal(snapshot.likedIds.size, 0)
      assert.match(snapshot.likesError, scenario === 'failure' ? /page unavailable/ : /重复游标/)
      return
    }
    if (scenario === 'switch') {
      assert.equal(snapshot.likesReady, false)
      await act(async () => snapshot.setProfile(null))
      await act(async () =>
        finishPage({
          type: 'tracks',
          data: {
            items: [{ reference: { source: 'qqmusic', kind: 'track', id: 'OldMID' } }],
            nextCursor: null
          }
        })
      )
      assert.equal(snapshot.likedIds.size, 0)
      assert.equal(snapshot.profile, null)
      assert.equal(snapshot.likesReady, false)
      return
    }
    assert.equal(snapshot.likesReady, true)
    assert.deepEqual([...snapshot.likedIds].sort(), ['OtherMID', 'SongMID'])
    assert.equal(document.querySelector('button').getAttribute('aria-pressed'), 'true')
    assert.equal(document.querySelector('button').disabled, true)
    await act(async () => snapshot.toggleLike('SongMID'))
    assert.equal(writes, 0)
    const count = queried.length
    await render()
    assert.equal(currentReads, 1)
    assert.equal(queried.length, count)
    assert.ok(queried.some((request) => request.page?.cursor === 'opaque-next'))
  })
}

test('readonly favorites load all pages, display the current red heart, forbid writes and survive equivalent source refreshes', () =>
  readonlyFavoritesScenario())
test('a failed preview does not prevent reading the explicit liked playlist', () =>
  readonlyFavoritesScenario('preview-failure'))
test('a failed liked-playlist page does not publish a partial favorites set', () =>
  readonlyFavoritesScenario('failure'))
test('repeated liked-playlist cursors fail without publishing a partial set', () =>
  readonlyFavoritesScenario('repeat'))
test('account changes discard a late liked-playlist page', () =>
  readonlyFavoritesScenario('switch'))

test('page source selection preserves every platform account and routes favorites to the song source', async () => {
  await withDom(async (root, dom) => {
    let current = 'netease'
    let legacy = false
    const writes = [],
      reads = [],
      listeners = new Map()
    let accounts
    const modules = {
      react: React,
      'react/jsx-runtime': jsx,
      '@tauri-apps/api/core': { isTauri: () => true },
      '@tauri-apps/api/event': {
        listen: async (event, callback) => {
          const handlers = listeners.get(event) ?? new Set()
          handlers.add(callback)
          listeners.set(event, handlers)
          return () => handlers.delete(callback)
        }
      },
      '@/lib/player': { errorText: String },
      '@/lib/runtime-cache': { invalidateNativeCache() {} },
      '@/features/library/library-api': {
        invalidateMusicLibrary() {},
        resetAccountCache() {},
        setLibraryAccount() {}
      },
      '@/features/music/use-music-sources': {
        useMusicSources: () => ({
          preferences: { current },
          statuses: ['netease', 'qqmusic'].map((source) => ({
            enabled: true,
            descriptor: { source, capabilities: ['account', 'favorites'] }
          }))
        })
      },
      '@/features/music/client': {
        musicClient: {
          currentAccount: async (source) => {
            reads.push(source)
            return { reference: { source, id: 'same-id' }, displayName: source }
          },
          account: async (source) => ({
            type: 'profile',
            data: { reference: { source, id: 'same-id' }, displayName: source }
          }),
          query: async (source) => ({
            type: 'favorites',
            data: source === 'qqmusic' ? [{ source, kind: 'track', id: 'song' }] : []
          }),
          write: async (source, request) => writes.push({ source, request })
        }
      }
    }
    const account = load('../src/features/account/account-state.tsx', modules)
    modules['@/features/account/account'] = account
    modules['@/features/music/legacy-reference'] = musicReference
    modules['lucide-react'] = { Heart: () => null }
    modules['@/components/ui/button'] = {
      Button: ({ variant, size, children, ...props }) =>
        React.createElement('button', props, children)
    }
    const { CurrentTrackLike } = load('../src/features/playback/current-track-like.tsx', modules)
    function View() {
      accounts = account.useAccount()
      return React.createElement(CurrentTrackLike, {
        track: {
          source: legacy
            ? { kind: 'netease', id: 123 }
            : {
                kind: 'online',
                track: { reference: { source: 'netease', kind: 'track', id: 'song' } }
              }
        },
        onError: (error) => {
          throw error
        }
      })
    }
    const render = () =>
      act(async () =>
        root.render(React.createElement(account.AccountProvider, {}, React.createElement(View)))
      )
    await render()
    const click = () =>
      act(async () =>
        document
          .querySelector('button')
          .dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }))
      )
    assert.equal(document.querySelector('button').disabled, false)
    current = 'qqmusic'
    await render()
    assert.equal(accounts.profile.reference.source, 'qqmusic')
    assert.equal(document.querySelector('button').disabled, false)
    assert.equal(document.querySelector('button').getAttribute('aria-pressed'), 'false')
    await click()
    assert.equal(writes[0].source, 'netease')
    assert.equal(writes[0].request.track.source, 'netease')
    assert.equal(writes[0].request.liked, true)
    current = 'netease'
    await render()
    assert.equal(document.querySelector('button').getAttribute('aria-pressed'), 'true')
    legacy = true
    current = 'qqmusic'
    await render()
    assert.equal(document.querySelector('button').disabled, false)
    await click()
    assert.equal(writes[1].source, 'netease')
    assert.equal(writes[1].request.track.id, '123')
    assert.deepEqual(reads.sort(), ['netease', 'qqmusic'])
    await act(async () => accounts.getAccount('netease').setProfile(null))
    assert.equal(accounts.getAccount('qqmusic').profile.reference.source, 'qqmusic')
    assert.equal(document.querySelector('button').disabled, true)
  })
})
