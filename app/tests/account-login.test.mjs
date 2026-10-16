import { errorNoticeModule } from './helpers/error-notice.mjs'
import { sourceCapabilities } from './helpers/source-capabilities.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function load(file, modules, extra = {}) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(file, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name]),
      ...extra
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
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const root = createRoot(document.getElementById('root'))
  try {
    await run(root, dom)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
}

const wrapper = ({ children }) => React.createElement('div', {}, children)
const button = ({ children, variant, size, ...props }) =>
  React.createElement('button', props, children)
const base = {
  '@/features/music/source-capabilities': sourceCapabilities,
  '@/features/music/source-preferences': {
    sourcePreferences: { beginIntent: async () => 1, use: async () => {} }
  },
  '@/features/music/use-music-sources': {
    useMusicSources: () => ({ preferences: { current: 'other' }, error: '' })
  },
  '@/features/music/source-label': { sourceLabel: String },
  react: React,
  'react/jsx-runtime': jsx,
  '@tauri-apps/api/core': { isTauri: () => true },
  'lucide-react': Object.fromEntries(
    ['Check', 'ChevronRight', 'QrCode', 'RefreshCw', 'UserPlus', 'LogOut', 'ArrowRight'].map(
      (name) => [name, () => null]
    )
  ),
  '@/components/music/action-button': { ActionButton: button },
  '@/components/music/music-page': {
    MusicPage: wrapper,
    MusicPageHeader: ({ title, children }) => React.createElement('header', {}, title, children)
  },
  './account-avatar': {
    AccountAvatar: ({ avatar }) => React.createElement('img', { src: avatar || undefined, alt: '' })
  },
  './saved-accounts': { SavedAccounts: () => null },
  '@/lib/player': { errorText: String }
}

test('aggregate login switches adapters, discards old QR and poll responses, and preserves disabled cards', async () => {
  await withDom(async (root, dom) => {
    const adapters = ['netease', 'other', 'disabled'].map((source) => ({
      descriptor: { source, displayName: source, capabilities: ['account', 'userLibrary'] },
      enabled: source !== 'disabled',
      current: null
    }))
    const requests = []
    let refreshes = 0
    const timers = new Map()
    let timerId = 0
    const Page = load(
      '../src/features/account/account-login-page.tsx',
      {
        ...base,
        './use-account-adapters': {
          useAccountAdapters: () => ({
            adapters,
            loading: false,
            error: '',
            refresh: () => refreshes++
          })
        },
        '@/features/workspace/music-navigation': {
          useMusicNavigation: () => ({ page: { view: 'library', query: '' }, navigate() {} })
        },
        '@/features/music/client': {
          musicClient: {
            cancelLogin: async () => {},
            account: (source, request) =>
              new Promise((resolve) => requests.push({ source, request, resolve }))
          }
        }
      },
      {
        setTimeout: (callback) => {
          const id = ++timerId
          timers.set(id, callback)
          return id
        },
        clearTimeout: (id) => timers.delete(id)
      }
    ).default
    const clickSource = (source) =>
      act(async () =>
        [...document.querySelectorAll('nav button')]
          .find((item) => item.textContent.includes(source))
          .dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }))
      )
    await act(async () => root.render(React.createElement(Page)))
    assert.equal(requests[0].source, 'netease')
    await clickSource('other')
    assert.equal(requests[1].source, 'other')
    await act(async () =>
      requests[0].resolve({
        type: 'challenge',
        data: { key: 'old', image: 'data:image/png;base64,b2xk' }
      })
    )
    assert.equal(document.querySelector('[alt="netease登录二维码"]'), null)
    await act(async () =>
      requests[1].resolve({
        type: 'challenge',
        data: { key: 'other', image: 'data:image/png;base64,bmV3' }
      })
    )
    assert.ok(document.querySelector('[alt="other登录二维码"]'))
    await act(async () => {
      const [id, poll] = [...timers.entries()][0]
      timers.delete(id)
      void poll()
    })
    assert.equal(requests[2].request.key, 'other')
    await clickSource('disabled')
    assert.equal(requests.length, 3)
    assert.equal(timers.size, 0)
    await act(async () =>
      requests[2].resolve({ type: 'progress', data: { code: 803, message: 'success' } })
    )
    assert.equal(refreshes, 0, 'unmounted source may not publish login completion')
    assert.match(document.body.textContent, /此适配器已停用/)
    await clickSource('netease')
    assert.equal(requests[3].source, 'netease')
  })
})

test('add-account route generates login even when the adapter already has a selected account', async () => {
  await withDom(async (root) => {
    const requests = []
    const adapters = [
      {
        descriptor: {
          source: 'other',
          displayName: 'Other',
          capabilities: ['account', 'userLibrary']
        },
        enabled: true,
        current: { reference: { source: 'other', id: 'opaque' }, displayName: 'User' }
      }
    ]
    const Page = load(
      '../src/features/account/account-login-page.tsx',
      {
        ...base,
        './use-account-adapters': {
          useAccountAdapters: () => ({ adapters, loading: false, error: '', refresh() {} })
        },
        '@/features/workspace/music-navigation': {
          useMusicNavigation: () => ({ page: { view: 'accounts', query: 'other' }, navigate() {} })
        },
        '@/features/music/client': {
          musicClient: {
            cancelLogin: async () => {},
            account: async (source, request) => {
              requests.push({ source, request })
              return {
                type: 'challenge',
                data: { key: 'new', image: 'data:image/png;base64,bmV3' }
              }
            }
          }
        }
      },
      { setTimeout, clearTimeout }
    ).default
    await act(async () => root.render(React.createElement(Page)))
    assert.equal(requests[0].source, 'other')
    assert.equal(requests[0].request.operation, 'beginLogin')
    assert.ok(document.querySelector('[alt="Other登录二维码"]'))
  })
})

test('login switches source only after success and exposes a persisted-account switch failure', async () => {
  await withDom(async (root) => {
    const timers = new Map()
    let timerId = 0
    let nextIntent = 0
    const switches = []
    const requests = []
    const refresh = () => {}
    const adapter = {
      descriptor: {
        source: 'other',
        displayName: 'Other',
        capabilities: ['account', 'userLibrary']
      },
      enabled: true,
      current: null
    }
    const Page = load(
      '../src/features/account/account-login-page.tsx',
      {
        ...base,
        '@/features/music/source-preferences': {
          sourcePreferences: {
            beginIntent: async () => ++nextIntent,
            use: async (source, intent) => {
              switches.push({ source, intent })
              throw new Error('disk failed')
            }
          }
        },
        './use-account-adapters': {
          useAccountAdapters: () => ({
            adapters: [adapter],
            loading: false,
            error: '',
            refresh
          })
        },
        '@/features/workspace/music-navigation': {
          useMusicNavigation: () => ({ page: { view: 'accounts', query: 'other' }, navigate() {} })
        },
        '@/features/music/client': {
          musicClient: {
            cancelLogin: async () => {},
            account: (source, request) =>
              new Promise((resolve) => requests.push({ source, request, resolve }))
          }
        }
      },
      {
        setTimeout: (callback) => {
          timers.set(++timerId, callback)
          return timerId
        },
        clearTimeout: (id) => timers.delete(id)
      }
    ).default
    await act(async () => root.render(React.createElement(Page)))
    assert.equal(switches.length, 0)
    await act(async () =>
      requests[0].resolve({
        type: 'challenge',
        data: { key: 'qr', image: 'data:image/png;base64,dGVzdA==' }
      })
    )
    await act(async () => {
      const [id, poll] = [...timers][0]
      timers.delete(id)
      void poll()
    })
    // Simulate the account-change event arriving before the poll Promise completes.
    adapter.current = { reference: { source: 'other', id: 'listener' }, displayName: 'Listener' }
    await act(async () => root.render(React.createElement(Page)))
    await act(async () =>
      requests[1].resolve({ type: 'progress', data: { code: 803, message: '登录成功' } })
    )
    assert.deepEqual(switches, [{ source: 'other', intent: 1 }])
    assert.match(document.querySelector('[role="alert"]').textContent, /登录成功，切换来源失败/)
    assert.match(document.body.textContent, /使用此来源/)
  })
})

test('account avatar uses cached image and forwards the cache fallback handler', async () => {
  let failures = 0
  const Avatar = load('../src/features/account/account-avatar.tsx', {
    react: React,
    'react/jsx-runtime': jsx,
    'lucide-react': { UserRound: () => null },
    '@/lib/utils': { cn: (...values) => values.filter(Boolean).join(' ') },
    '@/components/music/use-cover-source': {
      useCoverImageSource: (avatar) => ({
        source: avatar ? 'data:image/png;base64,Y2FjaGU=' : undefined,
        onError: () => failures++
      })
    }
  }).AccountAvatar
  await withDom(async (root, dom) => {
    await act(async () =>
      root.render(React.createElement(Avatar, { avatar: 'https://example.com/avatar.jpg' }))
    )
    const img = document.querySelector('img')
    assert.ok(img.src.startsWith('data:'))
    await act(async () => img.dispatchEvent(new dom.window.Event('error')))
    assert.equal(failures, 1)
    await act(async () => root.render(React.createElement(Avatar)))
    assert.equal(document.querySelector('img'), null)
  })
})

test('account menu and signed-out trigger navigate to the aggregate page without a dialog', async () => {
  await withDom(async (root, dom) => {
    const navigations = []
    let current = null
    const Menu = load('../src/features/account/account-menu.tsx', {
      ...base,
      '@base-ui/react/popover': {
        Popover: Object.fromEntries(
          ['Root', 'Trigger', 'Portal', 'Positioner', 'Popup', 'Title', 'Description'].map(
            (name) => [name, wrapper]
          )
        )
      },
      '@/components/ui/select': Object.fromEntries(
        ['Select', 'SelectContent', 'SelectItem', 'SelectTrigger', 'SelectValue'].map((name) => [
          name,
          wrapper
        ])
      ),
      './use-account-adapters': {
        useAccountAdapters: () => ({
          adapters: [
            {
              descriptor: {
                source: 'other',
                displayName: 'Other',
                capabilities: ['account', 'userLibrary']
              },
              enabled: true,
              current
            }
          ],
          error: '',
          refresh() {}
        })
      },
      '@/features/workspace/music-navigation': {
        useMusicNavigation: () => ({ navigate: (...args) => navigations.push(args) })
      },
      '@/features/music/client': { musicClient: {} }
    }).AccountMenu
    const click = (button) =>
      act(async () => button.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true })))
    await act(async () => root.render(React.createElement(Menu)))
    await click(
      [...document.querySelectorAll('button')].find((item) => item.textContent.includes('登录账号'))
    )
    assert.deepEqual(navigations[0], ['accounts', 'other'])
    current = { reference: { source: 'other', id: 'opaque' }, displayName: 'User' }
    await act(async () => root.render(React.createElement(Menu)))
    await click(
      [...document.querySelectorAll('button')].find((item) => item.textContent.includes('添加账号'))
    )
    assert.deepEqual(navigations[1], ['accounts', 'other'])
    assert.equal(document.querySelector('[role="dialog"]'), null)
  })
})

test('source-only adapter login manages its account without switching personal pages', async () => {
  await withDom(async (root) => {
    let switches = 0,
      intents = 0
    const timers = new Map()
    let timerId = 0
    const { AdapterLoginCard } = load(
      '../src/features/account/account-login-page.tsx',
      {
        ...base,
        '@/features/music/source-preferences': {
          sourcePreferences: {
            beginIntent: async () => {
              intents++
              return 1
            },
            use: async () => switches++
          }
        },
        '@/features/music/client': {
          musicClient: {
            account: async (_, request) =>
              request.operation === 'beginLogin'
                ? {
                    type: 'challenge',
                    data: { key: 'qr', image: 'data:image/png;base64,dGVzdA==' }
                  }
                : { type: 'progress', data: { code: 803, message: '登录成功' } },
            cancelLogin: async () => {}
          }
        }
      },
      {
        setTimeout: (callback) => {
          timers.set(++timerId, callback)
          return timerId
        },
        clearTimeout: (id) => timers.delete(id)
      }
    )
    const adapter = {
      descriptor: {
        source: 'audio',
        displayName: '音源',
        capabilities: ['account'],
        loginMethods: [{ id: 'qr', label: '扫码' }]
      },
      enabled: true,
      current: null
    }
    await act(async () =>
      root.render(
        React.createElement(AdapterLoginCard, { adapter, startLogin: true, onChanged() {} })
      )
    )
    assert.equal(timers.size, 1)
    await act(async () => {
      await [...timers.values()][0]()
    })
    assert.equal(intents, 0)
    assert.equal(switches, 0)
    adapter.current = { reference: { source: 'audio', id: 'user' }, displayName: 'User' }
    await act(async () =>
      root.render(
        React.createElement(AdapterLoginCard, { adapter, startLogin: false, onChanged() {} })
      )
    )
    assert.doesNotMatch(document.body.textContent, /使用此来源/)
    assert.match(document.body.textContent, /退出账号/)
  })
})
