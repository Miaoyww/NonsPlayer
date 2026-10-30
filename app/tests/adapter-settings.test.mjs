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

test('adapter settings persist changes and guard package operations and drag lifecycle', async (t) => {
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
  let installedPackages = []
  const pickers = []
  const dragListeners = []
  let delayDragListener = false
  const selectContext = React.createContext(null)
  const dialogContext = React.createContext(null)
  const modules = {
    '@/features/music/source-label': {
      sourceLabel: (source) => (source === 'netease' ? '网易云音乐' : source)
    },
    '@/features/account/account-login-page': { AdapterLoginCard: () => null },
    '@/features/account/use-account-adapters': {
      useAccountAdapters: () => ({ adapters: [], refresh() {} })
    },
    '@tauri-apps/plugin-dialog': {
      open: (options) =>
        new Promise((resolve, reject) => pickers.push({ options, resolve, reject }))
    },
    '@/components/ui/dialog': {
      Dialog: ({ open, onOpenChange, children }) =>
        open
          ? React.createElement(dialogContext.Provider, { value: onOpenChange }, children)
          : null,
      DialogClose: ({ render, children }) => {
        const onOpenChange = React.useContext(dialogContext)
        return React.cloneElement(render, { onClick: () => onOpenChange(false) }, children)
      },
      DialogContent: ({ children }) => React.createElement('div', { role: 'dialog' }, children),
      DialogDescription: ({ children }) => React.createElement('p', null, children),
      DialogFooter: ({ children }) => React.createElement('div', null, children),
      DialogHeader: ({ children }) => React.createElement('div', null, children),
      DialogTitle: ({ children }) => React.createElement('h2', null, children)
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
    '@tauri-apps/api/webview': {
      getCurrentWebview: () => ({
        onDragDropEvent: (callback) => {
          const listener = { callback, cleanups: 0 }
          dragListeners.push(listener)
          const unlisten = () => listener.cleanups++
          return delayDragListener
            ? new Promise((resolve) => {
                listener.resolve = () => resolve(unlisten)
              })
            : Promise.resolve(unlisten)
        }
      })
    },
    'lucide-react': {
      Blocks: () => null,
      RefreshCw: () => null,
      Search: () => null,
      RotateCw: () => null,
      Trash2: () => null,
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
        command === 'adapter_installed_list'
          ? Promise.resolve(installedPackages)
          : command === 'resource_provider_list'
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
    window: dom.window,
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
    assert.match(document.body.textContent, /选择 ZIP 文件或插件目录/)
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
    const permissions = {
      httpHosts: ['music.example.com'],
      websocketHosts: [],
      credentials: true,
      state: true,
      system: false
    }
    const packageInfo = {
      id: 'fixture-radio',
      name: '测试适配器',
      version: '1.0.0',
      kind: 'adapter',
      permissions,
      permissionChanges: []
    }
    const surface = document.getElementById('root').firstElementChild
    surface.getBoundingClientRect = () => ({
      left: 100,
      top: 50,
      right: 300,
      bottom: 200,
      width: 200,
      height: 150
    })
    Object.defineProperty(dom.window, 'devicePixelRatio', { value: 2 })
    const listener = dragListeners[0]
    const drag = (type, paths = ['C:/adapters/drag.zip'], position = { x: 500, y: 300 }) =>
      act(async () => listener.callback({ payload: { type, paths, position } }))
    const dialog = () => document.querySelector('[role="dialog"]')
    const dialogButton = (label) =>
      [...dialog().querySelectorAll('button')].find((element) => element.textContent === label)
    const lastRequest = () => requests.at(-1)
    const resolvePreview = () => act(async () => lastRequest().resolve(packageInfo))
    const expectLocked = () => assert.equal(surface.getAttribute('aria-busy'), 'true')

    await t.test(
      'drag callback uses physical webview coordinates and rejects outside and multiple files',
      async () => {
        assert.equal(dialog(), null)
        assert.equal(dragListeners.length, 1)
        const count = requests.length
        // These physical coordinates look inside the CSS rect until divided by DPR.
        await drag('enter', undefined, { x: 150, y: 80 })
        assert.doesNotMatch(document.body.textContent, /松开即可预览/)
        await drag('drop', undefined, { x: 150, y: 80 })
        assert.equal(requests.length, count)
        await drag('enter')
        assert.match(document.querySelector('[role="status"]').textContent, /松开即可预览/)
        await drag('leave')
        assert.doesNotMatch(document.body.textContent, /松开即可预览/)
        await drag('over')
        assert.match(document.querySelector('[role="status"]').textContent, /松开即可预览/)
        await drag('drop', ['C:/adapters/one.zip', 'C:/adapters/two.zip'])
        assert.equal(requests.length, count)
        assert.match(document.querySelector('[role="alert"]').textContent, /一次只能拖入一个/)
        assert.doesNotMatch(document.body.textContent, /松开即可预览/)
        await drag('drop')
        assert.equal(lastRequest().command, 'adapter_package_preview')
        assert.equal(lastRequest().args.path, 'C:/adapters/drag.zip')
        expectLocked()
        await drag('drop', ['C:/adapters/ignored.zip'])
        await drag('enter')
        assert.equal(requests.length, count + 1)
        assert.doesNotMatch(document.body.textContent, /松开即可预览/)
        await resolvePreview()
        assert.equal(surface.getAttribute('aria-busy'), 'false')
        assert.match(dialog().textContent, /测试适配器.*music.example.com.*读取当前平台账号凭据/s)
        await click(dialogButton('取消'))
        assert.equal(dialog(), null)
        assert.equal(dragListeners.length, 1)
        assert.equal(listener.cleanups, 0)
      }
    )

    await t.test(
      'picker holds the operation lock until preview and supports cancellation and failure',
      async () => {
        await click(button('安装适配器'))
        await click(button('选择 ZIP'))
        expectLocked()
        assert.equal(pickers.length, 1)
        assert.equal(pickers[0].options.directory, false)
        assert.equal(pickers[0].options.multiple, false)
        assert.deepEqual([...pickers[0].options.filters[0].extensions], ['zip'])
        assert.equal(button('选择 ZIP').disabled, true)
        assert.equal(button('选择目录').disabled, true)
        const count = requests.length
        await drag('drop', ['C:/adapters/competing.zip'])
        await click(button('选择 ZIP'))
        await click(button('选择目录'))
        await click(button('已安装'))
        assert.equal(toggle().disabled, true)
        await click(toggle())
        await click(document.querySelector('[aria-label="重新加载网易云音乐"]'))
        assert.equal(requests.length, count)
        assert.equal(pickers.length, 1)
        await act(async () => pickers[0].resolve('C:/adapters/picked.zip'))
        assert.equal(lastRequest().command, 'adapter_package_preview')
        assert.equal(lastRequest().args.path, 'C:/adapters/picked.zip')
        expectLocked()
        await drag('drop', ['C:/adapters/still-competing.zip'])
        assert.equal(requests.length, count + 1)
        await resolvePreview()
        assert.match(dialog().textContent, /测试适配器/)
        await click(dialogButton('取消'))
        await click(button('安装适配器'))
        await click(button('选择目录'))
        assert.equal(pickers[1].options.directory, true)
        await act(async () => pickers[1].resolve(null))
        assert.equal(surface.getAttribute('aria-busy'), 'false')
        assert.equal(requests.length, count + 1)
        await click(button('选择 ZIP'))
        await act(async () => pickers[2].reject(new Error('文件选择器不可用')))
        assert.match(document.querySelector('[role="alert"]').textContent, /文件选择器不可用/)
        assert.equal(surface.getAttribute('aria-busy'), 'false')
        assert.equal(dragListeners.length, 1)
      }
    )

    await t.test(
      'install requires trust, submits the reviewed path and displays failures inside its dialog',
      async () => {
        await drag('drop', ['C:/adapters/install.zip'])
        await resolvePreview()
        const count = requests.length
        assert.equal(dialogButton('确认安装').disabled, true)
        await click(dialogButton('确认安装'))
        assert.equal(requests.length, count)
        await click(dialog().querySelector('input[type="checkbox"]'))
        assert.equal(dialogButton('确认安装').disabled, false)
        await click(dialogButton('确认安装'))
        assert.equal(lastRequest().command, 'adapter_install')
        assert.equal(lastRequest().args.path, 'C:/adapters/install.zip')
        assert.equal(lastRequest().args.authorized, true)
        expectLocked()
        await click(dialogButton('确认安装'))
        await drag('drop', ['C:/adapters/ignored.zip'])
        await click(dialogButton('取消'))
        assert.ok(dialog())
        assert.equal(requests.length, count + 1)
        await act(async () => lastRequest().reject(new Error('组件加载失败')))
        assert.match(dialog().querySelector('[role="alert"]').textContent, /组件加载失败/)
        assert.equal(document.querySelectorAll('[role="alert"]').length, 1)
        assert.equal(dialogButton('确认安装').disabled, false)
        await click(dialogButton('确认安装'))
        installedPackages = [{ ...packageInfo, authorized: true, enabled: true, loaded: true }]
        await act(async () => lastRequest().resolve())
        assert.equal(dialog(), null)
        await act(async () => lastRequest().resolve(status(true)))
        assert.equal(surface.getAttribute('aria-busy'), 'false')
        assert.match(document.body.textContent, /测试适配器/)
        await drag('drop', ['C:/adapters/after-install.zip'])
        assert.equal(lastRequest().args.path, 'C:/adapters/after-install.zip')
        await resolvePreview()
        assert.equal(dialogButton('确认安装').disabled, true)
        await click(dialogButton('取消'))
        assert.equal(dragListeners.length, 1)
        assert.equal(listener.cleanups, 0)
      }
    )

    await t.test(
      'uninstall failures stay in the modal and completed operations allow another drop',
      async () => {
        await click(document.querySelector('[aria-label="卸载测试适配器"]'))
        assert.equal(dialog().querySelector('[role="alert"]'), null)
        assert.equal(dialog().querySelector('input[type="checkbox"]').checked, false)
        await click(dialogButton('确认卸载'))
        assert.equal(lastRequest().command, 'adapter_uninstall')
        assert.equal(lastRequest().args.source, 'fixture-radio')
        assert.equal(lastRequest().args.removeData, false)
        const count = requests.length
        await click(dialogButton('确认卸载'))
        await drag('drop', ['C:/adapters/ignored.zip'])
        assert.equal(requests.length, count)
        await act(async () => lastRequest().reject(new Error('卸载目录被占用')))
        assert.match(dialog().querySelector('[role="alert"]').textContent, /卸载目录被占用/)
        assert.equal(document.querySelectorAll('[role="alert"]').length, 1)
        await click(dialogButton('取消'))
        await click(document.querySelector('[aria-label="卸载测试适配器"]'))
        assert.equal(dialog().querySelector('[role="alert"]'), null)
        await click(dialog().querySelector('input[type="checkbox"]'))
        await click(dialogButton('确认卸载'))
        assert.equal(lastRequest().args.removeData, true)
        installedPackages = []
        await act(async () => lastRequest().resolve())
        await act(async () => lastRequest().resolve(status(true)))
        assert.equal(dialog(), null)
        await drag('drop', ['C:/adapters/after-uninstall.zip'])
        assert.equal(lastRequest().command, 'adapter_package_preview')
        assert.equal(lastRequest().args.path, 'C:/adapters/after-uninstall.zip')
        await act(async () => lastRequest().reject(new Error('包描述无效')))
        assert.match(document.querySelector('[role="alert"]').textContent, /包描述无效/)
        await drag('drop', ['C:/adapters/retry.zip'])
        await resolvePreview()
        await click(dialogButton('取消'))
        assert.equal(dragListeners.length, 1)
      }
    )

    await t.test(
      'unmount cleans up registered and late-resolving listeners and ignores old callbacks',
      async () => {
        await act(async () => changed())
        const stale = lastRequest()
        await act(async () => root.render(null))
        assert.equal(unsubscribed, true)
        assert.equal(listener.cleanups, 1)
        const count = requests.length
        await drag('drop')
        assert.equal(requests.length, count)
        await act(async () => stale.resolve(status(false)))
        delayDragListener = true
        await act(async () => root.render(React.createElement(exports.AdaptersPage)))
        assert.equal(dragListeners.length, 2)
        const late = dragListeners[1]
        const pending = lastRequest()
        await act(async () => root.unmount())
        assert.equal(late.cleanups, 0)
        await act(async () => late.resolve())
        assert.equal(late.cleanups, 1)
        const afterUnmount = requests.length
        await act(async () =>
          late.callback({
            payload: {
              type: 'drop',
              paths: ['C:/adapters/ignored.zip'],
              position: { x: 500, y: 300 }
            }
          })
        )
        assert.equal(requests.length, afterUnmount)
        await act(async () => pending.resolve(status(false)))
        assert.equal(listener.cleanups, 1)
      }
    )
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
