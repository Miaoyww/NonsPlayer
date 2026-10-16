import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { loadExtensionList } from './helpers/extension-list.mjs'

test('plugin settings scopes native drops, serializes ZIP installs and removes its listener', async () => {
  const dom = new JSDOM('<section aria-label="插件设置"><div id="root"></div></section>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.HTMLElement = dom.window.HTMLElement
  globalThis.Element = dom.window.Element
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  Object.defineProperty(window, 'devicePixelRatio', { value: 2 })
  document.querySelector('section').getBoundingClientRect = () => ({
    left: 100,
    top: 50,
    right: 700,
    bottom: 500
  })
  let listener, finishInstall
  let cleanups = 0
  const calls = []
  const openedUrls = []
  const dialogs = []
  let plugins = []
  const selectContext = React.createContext(null)
  const modules = {
    '@/components/ui/select': {
      Select: ({ onValueChange, children }) =>
        React.createElement(selectContext.Provider, { value: onValueChange }, children),
      SelectContent: ({ children }) => React.createElement('div', null, children),
      SelectTrigger: ({ children, ...props }) => React.createElement('button', props, children),
      SelectValue: ({ children }) => React.createElement('span', null, children),
      SelectItem: ({ value, children }) => {
        const change = React.useContext(selectContext)
        return React.createElement(
          'button',
          { 'data-permission': value, onClick: () => change(value) },
          children
        )
      }
    },
    '../plugins/configuration-page': { PluginConfigurationPage: 'div' },
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/webview': {
      getCurrentWebview: () => ({
        onDragDropEvent: async (fn) => {
          listener = fn
          return () => cleanups++
        }
      })
    },
    '@tauri-apps/plugin-opener': { openUrl: async (url) => openedUrls.push(url) },
    '@tauri-apps/plugin-dialog': {
      open: async (options) => {
        dialogs.push(options)
        return options.directory ? 'chosen-directory' : 'chosen.zip'
      }
    },
    'lucide-react': Object.fromEntries(
      [
        'ExternalLink',
        'FolderOpen',
        'Package',
        'RefreshCw',
        'RotateCw',
        'Search',
        'Settings',
        'Store',
        'Trash2',
        'Upload'
      ].map((name) => [name, 'svg'])
    ),
    '@/components/ui/button': {
      Button: ({ variant: _variant, size: _size, ...props }) => React.createElement('button', props)
    },
    '@/components/ui/input': {
      Input: ({ onChange, ...props }) =>
        React.createElement('input', { ...props, onInput: onChange })
    },
    '@/components/ui/switch': {
      Switch: ({ onCheckedChange, ...props }) =>
        React.createElement('input', {
          ...props,
          type: 'checkbox',
          role: 'switch',
          onChange: (event) => onCheckedChange(event.target.checked)
        })
    },
    '@/plugins/host': { usePlugins: () => ({ plugins }) },
    '@/lib/player': {
      errorText: (error) => error.message,
      nativeCall: async (command, args) => {
        calls.push([command, args])
        if (args?.path === 'bad.zip') throw new Error('损坏的插件包')
        if (args?.path === 'slow.zip')
          await new Promise((resolve) => {
            finishInstall = resolve
          })
      }
    }
  }
  modules['../extension-list'] = loadExtensionList(modules)
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/settings/pages/plugins.tsx', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    {
      exports,
      window,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  const root = createRoot(document.getElementById('root'))
  const emit = async (type, paths = [], position = { x: 400, y: 200 }) =>
    React.act(async () => listener({ payload: { type, paths, position } }))
  try {
    await React.act(async () => root.render(React.createElement(exports.PluginsPage)))
    assert.equal(document.querySelector('[aria-pressed=true]').textContent, '全部')
    await emit('drop', ['outside.zip'], { x: 20, y: 20 })
    assert.equal(calls.length, 0)
    await emit('enter', ['one.zip'])
    assert.match(document.body.textContent, /松开即可安装/)
    await emit('leave')
    assert.doesNotMatch(document.body.textContent, /松开即可安装/)
    await emit('drop', ['one.zip', 'notes.txt'])
    assert.equal(calls.length, 0)
    assert.match(document.querySelector('[role="alert"]').textContent, /\.zip/)
    await emit('drop', ['bad.zip', 'ONE.ZIP', 'ONE.ZIP'])
    assert.deepEqual(
      calls.map(([, args]) => args.path),
      ['bad.zip', 'ONE.ZIP']
    )
    assert.match(document.querySelector('[role="alert"]').textContent, /bad.zip：损坏的插件包/)
    await emit('drop', ['slow.zip'])
    await emit('drop', ['ignored.zip'])
    assert.deepEqual(
      calls.map(([, args]) => args.path),
      ['bad.zip', 'ONE.ZIP', 'slow.zip']
    )
    await React.act(async () => finishInstall())
    assert.equal(document.querySelector('[aria-busy]').getAttribute('aria-busy'), 'false')
    await React.act(async () =>
      [...document.querySelectorAll('button')]
        .find((button) => button.textContent.includes('打开文件夹'))
        .click()
    )
    assert.equal(calls.at(-1)[0], 'plugin_open_folder')
    plugins = [
      {
        manifest: {
          id: 'download',
          name: '下载管理',
          description: '保存歌曲与管理下载队列。',
          repository: 'https://github.com/Miaoyww/Nons',
          version: '1.1.0',
          permissions: [
            'ui',
            'account:credentials',
            'http:request',
            'http:transfer',
            'files:selected'
          ],
          httpHosts: ['*', 'example.org', 'api.example.org'],
          fileRoots: ['*']
        },
        enabled: false,
        loaded: false
      }
    ]
    await React.act(async () => root.render(React.createElement(exports.PluginsPage)))
    const list = () => document.querySelector('section[aria-label="已安装的插件"]')
    assert.match(list().textContent, /保存歌曲与管理下载队列。/)
    await React.act(async () =>
      document.querySelector('[aria-label="打开 下载管理 的代码仓库"]').click()
    )
    assert.deepEqual(openedUrls, ['https://github.com/Miaoyww/Nons'])
    assert.equal(document.querySelector('[aria-pressed=true]').textContent, '未加载')
    const selectStatus = async (value) =>
      React.act(async () =>
        [...document.querySelectorAll('button[aria-pressed]')]
          .find((button) => button.textContent === value)
          .click()
      )
    plugins.push({
      manifest: { id: 'island', name: '灵动岛', version: '1.3.0', permissions: ['ui'] },
      enabled: true,
      loaded: true
    })
    await React.act(async () => root.render(React.createElement(exports.PluginsPage)))
    assert.doesNotMatch(list().textContent, /灵动岛/)
    await selectStatus('已加载')
    assert.match(list().textContent, /灵动岛/)
    assert.doesNotMatch(list().textContent, /下载管理/)
    await selectStatus('全部')
    assert.equal(list().querySelectorAll('h3').length, 2)
    await React.act(async () => document.querySelector('[data-permission="http:transfer"]').click())
    assert.match(list().textContent, /下载管理/)
    assert.doesNotMatch(list().textContent, /灵动岛/)
    const search = document.querySelector('[type=search]')
    await React.act(async () => {
      search.value = '不存在'
      search.dispatchEvent(new dom.window.Event('input', { bubbles: true }))
    })
    assert.match(list().textContent, /没有符合条件/)
    await React.act(async () =>
      [...document.querySelectorAll('button')]
        .find((button) => button.textContent === '清除筛选')
        .click()
    )
    assert.equal(list().querySelectorAll('h3').length, 2)
    plugins.push({
      manifest: { id: 'failed', name: '失败插件', version: '1.0.0', permissions: [] },
      enabled: true,
      loaded: false,
      error: '加载失败测试'
    })
    await React.act(async () => root.render(React.createElement(exports.PluginsPage)))
    await selectStatus('加载异常')
    assert.equal(list().querySelectorAll('h3').length, 1)
    assert.match(list().textContent, /加载失败测试/)
    await React.act(async () =>
      [...document.querySelectorAll('button')]
        .find((button) => button.textContent === '安装插件')
        .click()
    )
    assert.ok(document.querySelector('section[aria-label="本地安装插件"]'))
    assert.ok(
      [...document.querySelectorAll('button')].some((button) =>
        button.textContent.includes('选择目录')
      )
    )
    await React.act(async () =>
      [...document.querySelectorAll('button')]
        .find((button) => button.textContent.includes('选择 ZIP'))
        .click()
    )
    assert.equal(calls.at(-1)[0], 'plugin_install')
    assert.equal(calls.at(-1)[1].path, 'chosen.zip')
    assert.equal(document.querySelector('[aria-pressed=true]').textContent, '未加载')
    assert.equal(document.querySelector('[type=search]').value, '')
    assert.equal(dialogs.at(-1).directory, false)
    await React.act(async () =>
      [...document.querySelectorAll('button')]
        .find((button) => button.textContent === '安装插件')
        .click()
    )
    await React.act(async () =>
      [...document.querySelectorAll('button')]
        .find((button) => button.textContent === '选择目录')
        .click()
    )
    assert.equal(dialogs.at(-1).directory, true)
    assert.equal(calls.at(-1)[0], 'plugin_install')
    assert.equal(calls.at(-1)[1].path, 'chosen-directory')
    const beforeEnable = calls.length
    await React.act(async () => document.querySelector('[aria-label="启用下载管理"]').click())
    assert.match(document.body.textContent, /敏感权限：读取 NonsPlayer 的网易云账户凭证/)
    assert.equal(calls.length, beforeEnable)
    assert.equal(
      [...document.querySelectorAll('button')].find((button) => button.textContent === '确认启用')
        .disabled,
      true
    )
    const warning = [...document.querySelectorAll('[role="alert"]')].find((element) =>
      element.textContent.includes('登录 Cookie')
    )
    assert.ok(warning.className.includes('text-destructive'))
    assert.match(document.body.textContent, /当前用户能够访问的任意文件/)
    assert.match(document.body.textContent, /敏感权限：会请求任意 URL/)
    assert.match(document.body.textContent, /敏感权限：会请求任意 URL 并传输资源到授权目录/)
    assert.ok(!document.body.textContent.includes('example.org'))
    assert.ok(!document.body.textContent.includes('：*'))
    const networkWarning = [...document.querySelectorAll('[role="alert"]')].find((element) =>
      element.textContent.includes('此插件会请求任意 URL')
    )
    assert.ok(networkWarning.className.includes('text-destructive'))
    assert.match(networkWarning.textContent, /Cookie 也可能被发送到其他网站/)
    const clickButton = async (text) =>
      React.act(async () =>
        [...document.querySelectorAll('button')]
          .find((button) => button.textContent === text)
          .click()
      )
    const beginUninstall = async () =>
      React.act(async () => document.querySelector('[aria-label="卸载 下载管理"]').click())
    await React.act(async () => document.querySelector('[aria-label="同意插件权限"]').click())
    await clickButton('确认启用')
    assert.equal(calls.at(-1)[0], 'plugin_action')
    assert.equal(calls.at(-1)[1].action, 'enable')
    assert.equal(calls.at(-1)[1].confirmed, true)
    await beginUninstall()
    assert.equal(document.querySelector('[aria-label="保留插件数据"]').checked, true)
    await React.act(async () => document.querySelector('[aria-label="保留插件数据"]').click())
    await clickButton('取消')
    await beginUninstall()
    assert.equal(document.querySelector('[aria-label="保留插件数据"]').checked, true)
    await clickButton('确认卸载')
    assert.equal(calls.at(-1)[1].action, 'uninstall-keep-data')
    await beginUninstall()
    await React.act(async () => document.querySelector('[aria-label="保留插件数据"]').click())
    await clickButton('确认卸载')
    assert.equal(calls.at(-1)[1].action, 'uninstall')
    const finalCall = calls.at(-1)
    await React.act(async () => root.unmount())
    assert.equal(cleanups, 1)
    await emit('drop', ['after-unmount.zip'])
    assert.equal(calls.at(-1), finalCall)
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
    delete globalThis.HTMLElement
    delete globalThis.Element
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
  }
})
