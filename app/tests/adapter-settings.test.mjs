import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { loadExtensionList } from './helpers/extension-list.mjs'

test('adapter settings wait for persistence, report failures and reject stale list responses', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    HTMLElement: globalThis.HTMLElement,
    Element: globalThis.Element,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.HTMLElement = dom.window.HTMLElement
  globalThis.Element = dom.window.Element
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const requests = []
  let changed
  let unsubscribed = false
  let invalidations = 0
  let providerStatuses = []
  let providerFailure
  const selectContext = React.createContext(null)
  const modules = {
    '@/features/music/source-label': {
      sourceLabel: (source) => (source === 'netease' ? '网易云音乐' : source)
    },
    '@/features/account/account-login-page': { AdapterLoginCard: () => null },
    '@/features/account/use-account-adapters': {
      useAccountAdapters: () => ({ adapters: [], refresh() {} })
    },
    '@/components/ui/input': {
      Input: ({ onChange, ...props }) =>
        React.createElement('input', { ...props, onInput: onChange })
    },
    '@/components/ui/select': {
      Select: ({ onValueChange, children }) =>
        React.createElement(selectContext.Provider, { value: onValueChange }, children),
      SelectContent: ({ children }) => React.createElement('div', null, children),
      SelectTrigger: ({ children, ...props }) => React.createElement('button', props, children),
      SelectValue: ({ children }) => React.createElement('span', null, children),
      SelectItem: ({ value, children }) => {
        const onValueChange = React.useContext(selectContext)
        return React.createElement(
          'button',
          { 'data-capability': value, onClick: () => onValueChange(value) },
          children
        )
      }
    },
    '@/components/music/loading': { TextSkeleton: 'div' },
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (_, callback) => {
        changed = callback
        return () => {
          unsubscribed = true
        }
      }
    },
    'lucide-react': {
      Blocks: () => null,
      RefreshCw: () => null,
      Search: () => null,
      RotateCw: () => null,
      UserRound: () => null,
      Upload: () => null,
      Store: () => null,
      FolderOpen: () => null
    },
    '@/components/ui/button': {
      Button: ({ variant, size, children, ...props }) =>
        React.createElement('button', props, children)
    },
    '@/components/ui/switch': {
      Switch: ({ checked, onCheckedChange, ...props }) =>
        React.createElement('button', {
          ...props,
          role: 'switch',
          'aria-checked': checked,
          onClick: () => onCheckedChange(!checked)
        })
    },
    '@/features/settings/settings-card': {
      SettingsCard: ({ title, description, children }) =>
        React.createElement('div', {}, title, description, children)
    },
    '@/lib/player': {
      errorText: String,
      nativeCall: (command, args) =>
        command === 'resource_provider_list'
          ? providerFailure
            ? Promise.reject(providerFailure)
            : Promise.resolve(providerStatuses)
          : new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    },
    '@/lib/runtime-cache': {
      invalidateNativeCache: () => {
        invalidations++
      }
    }
  }
  modules['../extension-list'] = loadExtensionList(modules)
  const exports = {}
  const code = ts.transpileModule(
    readFileSync(new URL('../src/features/settings/pages/adapters.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  ).outputText
  runInNewContext(code, {
    exports,
    require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
  })
  const root = createRoot(document.getElementById('root'))
  const status = (enabled) => [
    {
      descriptor: {
        source: 'netease',
        displayName: '网易云音乐',
        description: '网易云账号、曲库与播放服务',
        capabilities: ['search', 'account']
      },
      enabled
    }
  ]
  const toggle = () => document.querySelector('[role="switch"]')
  const click = (element) =>
    act(async () => element.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true })))
  try {
    await act(async () => root.render(React.createElement(exports.AdaptersPage)))
    const reservedInstall = [...document.querySelectorAll('button')].find((button) =>
      button.textContent.includes('安装适配器')
    )
    assert.equal(reservedInstall.getAttribute('role'), 'tab')
    assert.equal(reservedInstall.disabled, false)
    await click(reservedInstall)
    assert.match(document.body.textContent, /适配器商店/)
    assert.match(document.body.textContent, /适配器本地安装暂未开放/)
    assert.equal(
      [...document.querySelectorAll('button')].find((button) => button.textContent === '选择 ZIP')
        .disabled,
      true
    )
    assert.equal(document.querySelector('[type=search]'), null)
    await click(
      [...document.querySelectorAll('[role=tab]')].find((button) => button.textContent === '已安装')
    )
    assert.equal(requests[0].command, 'adapter_list')
    await act(async () => requests[0].resolve(status(true)))
    await click(toggle())
    assert.equal(requests[1].command, 'adapter_set_enabled')
    assert.equal(requests[1].args.source, 'netease')
    assert.equal(requests[1].args.enabled, false)
    assert.equal(toggle().getAttribute('aria-checked'), 'true')
    assert.equal(toggle().disabled, true)
    await act(async () => requests[1].reject(new Error('disk full')))
    assert.match(document.querySelector('[role="alert"]').textContent, /disk full/)
    assert.equal(toggle().getAttribute('aria-checked'), 'true')
    await click(toggle())
    await act(async () => requests[2].resolve())
    assert.equal(invalidations, 1)
    assert.equal(requests[3].command, 'adapter_list')
    await act(async () => requests[3].resolve(status(false)))
    assert.equal(toggle().getAttribute('aria-checked'), 'false')
    assert.equal(
      [...document.querySelectorAll('button')].find(
        (button) => button.getAttribute('aria-label') === '重新加载网易云音乐'
      ).disabled,
      true
    )
    await act(async () => {
      changed()
      changed()
    })
    await act(async () => requests[5].resolve(status(true)))
    await act(async () => requests[4].resolve(status(false)))
    assert.equal(toggle().getAttribute('aria-checked'), 'true')
    await act(async () => changed())
    const pending = status(false)
    pending[0].descriptor.source = 'qqmusic'
    pending[0].descriptor.displayName = 'QQ 音乐'
    pending[0].loading = true
    await act(async () => requests[6].resolve(pending))
    assert.match(document.body.textContent, /QQ 音乐.*加载中/)
    assert.equal(toggle().disabled, true)
    await act(async () => changed())
    await act(async () =>
      requests[7].resolve([{ ...pending[0], loading: false, loadError: '组件校验失败' }])
    )
    assert.match(document.querySelector('[role="alert"]').textContent, /组件校验失败/)
    providerStatuses = [
      {
        descriptor: {
          id: 'gdstudio',
          displayName: 'GD音乐台（星海）',
          description: '星海提供 MP3 备用音源',
          platforms: ['netease'],
          searchPlatforms: ['netease']
        },
        enabled: true
      }
    ]
    await act(async () => changed())
    await act(async () => requests[8].resolve(status(true)))
    const list = () => document.querySelector('section[aria-label="已安装的音乐适配器"]')
    const button = (label) =>
      [...document.querySelectorAll('button')].find((element) => element.textContent === label)
    assert.equal(list().querySelectorAll('h3').length, 2)
    assert.doesNotMatch(document.body.textContent, /独立音源适配器|补充来源|选择音乐库与发现/)
    const gd = list().querySelectorAll('h3')[1].closest('.p-4')
    assert.match(gd.textContent, /播放解析.*搜索/)
    assert.match(gd.textContent, /星海提供 MP3 备用音源/)
    assert.doesNotMatch(gd.textContent, /单曲读取|账号|收藏/)
    assert.equal(gd.querySelectorAll('button').length, 1)
    await click(document.querySelector('[data-capability="account"]'))
    assert.equal(list().querySelectorAll('h3').length, 1)
    assert.match(list().textContent, /网易云音乐/)
    await click(document.querySelector('[data-capability="search"]'))
    assert.equal(list().querySelectorAll('h3').length, 2)
    const input = document.querySelector('input')
    await act(async () => {
      input.value = ' MP3 '
      input.dispatchEvent(new dom.window.Event('input', { bubbles: true }))
    })
    assert.equal(list().querySelectorAll('h3').length, 1)
    assert.match(list().textContent, /GD音乐台/)
    await click(button('已停用'))
    assert.match(list().textContent, /没有符合条件/)
    await click(button('清除筛选'))
    assert.equal(list().querySelectorAll('h3').length, 2)
    await click(document.querySelector('[aria-label="启用GD音乐台（星海）"]'))
    assert.equal(requests[9].command, 'resource_provider_set_enabled')
    assert.equal(requests[9].args.id, 'gdstudio')
    assert.equal(requests[9].args.enabled, false)
    await act(async () => requests[9].reject(new Error('音源设置保存失败')))
    assert.match(document.querySelector('[role="alert"]').textContent, /音源设置保存失败/)
    assert.equal(
      document.querySelector('[aria-label="启用GD音乐台（星海）"]').getAttribute('aria-checked'),
      'true'
    )
    await click(document.querySelector('[aria-label="启用GD音乐台（星海）"]'))
    providerStatuses = [{ ...providerStatuses[0], enabled: false }]
    await act(async () => requests[10].resolve())
    await act(async () => requests[11].resolve(status(true)))
    assert.equal(
      document.querySelector('[aria-label="启用GD音乐台（星海）"]').getAttribute('aria-checked'),
      'false'
    )
    await click(button('已停用'))
    assert.match(list().textContent, /GD音乐台/)
    assert.doesNotMatch(list().textContent, /网易云音乐.*已启用/)
    await click(button('全部'))
    providerFailure = new Error('音源列表不可用')
    await act(async () => changed())
    await act(async () => requests[12].resolve(status(true)))
    assert.match(document.querySelector('[role="alert"]').textContent, /音源列表不可用/)
    assert.match(list().textContent, /网易云音乐/)
    await act(async () => changed())
    await act(async () => root.unmount())
    assert.equal(unsubscribed, true)
    await act(async () => requests[13].resolve(status(false)))
  } finally {
    await act(async () => root.unmount())
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.HTMLElement = previous.HTMLElement
    globalThis.Element = previous.Element
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    dom.window.close()
  }
})
