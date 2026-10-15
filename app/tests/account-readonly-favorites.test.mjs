import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
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
    { exports, require: (name) => modules[name] }
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

test('readonly favorites load all pages, display the current red heart, forbid writes and survive equivalent source refreshes', async () =>
  withDom(async (root) => {
    const record = {
      reference: { source: 'qqmusic', id: 'account' },
      displayName: '测试账号',
      avatar: ''
    }
    let currentReads = 0,
      writes = 0,
      snapshot
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
              descriptor: { source: 'qqmusic', capabilities: ['account', 'favoritesRead'] }
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
            assert.equal(request.operation, 'favoritesPage')
            return {
              type: 'favoritesPage',
              data: {
                items: [
                  { source, kind: 'track', id: request.page.cursor ? 'OtherMID' : 'SongMID' }
                ],
                nextCursor: request.page.cursor ? null : 'opaque-next'
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
    assert.ok(queried.some((request) => request.page.cursor === 'opaque-next'))
  }))

test('closing an error dismisses all current origins and still allows a later new error', async () =>
  withDom(async (root, dom) => {
    const { WorkspaceErrorBanner } = load('../src/features/workspace/workspace-error-banner.tsx', {
      react: React,
      'react/jsx-runtime': jsx,
      'lucide-react': { X: () => null },
      '@/components/music/action-button': {
        ActionButton: ({ variant, size, children, ...props }) =>
          React.createElement('button', props, children)
      }
    })
    let errors = ['状态已变化', '状态已变化', '旧媒体错误']
    const render = () =>
      act(async () =>
        root.render(
          React.createElement(WorkspaceErrorBanner, {
            errors,
            onDismiss: () => {
              errors = [undefined, '状态已变化', '旧媒体错误']
            }
          })
        )
      )
    await render()
    await act(async () =>
      document
        .querySelector('button')
        .dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }))
    )
    await render()
    assert.equal(document.querySelector('[role="alert"]'), null)
    errors = [undefined, '状态已变化', '新错误']
    await render()
    assert.match(document.body.textContent, /新错误/)
    errors = []
    await render()
    errors = ['状态已变化']
    await render()
    assert.match(document.body.textContent, /状态已变化/)
  }))
