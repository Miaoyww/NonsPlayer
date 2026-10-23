import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { errorNoticeModule } from './helpers/error-notice.mjs'
import Markdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

test('updates require a download click and separate installation confirmation; portable opens its download', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let listener
  let status = {
    autoCheck: true,
    phase: 'idle',
    version: null,
    supported: true,
    portable: false,
    source: 'r2',
    downloaded: 0
  }
  const calls = []
  let failPreferenceSave = false
  let finishCheck
  const container = ({ children }) => React.createElement('div', {}, children)
  let now = 0
  let tick
  dom.window.setInterval = (callback) => {
    tick = callback
    return 1
  }
  dom.window.clearInterval = () => {
    tick = undefined
  }
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (_, callback) => {
        listener = callback
        return () => {}
      }
    },
    'lucide-react': { Download: () => null, RefreshCw: () => null },
    '@/components/ui/progress': {
      Progress: ({ value, children }) =>
        React.createElement(
          'div',
          { role: 'progressbar', 'aria-valuenow': value ?? undefined },
          children
        ),
      ProgressLabel: container
    },
    '@/components/ui/button': {
      Button: ({ children, ...props }) => React.createElement('button', props, children)
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
    '@/components/ui/dialog': {
      Dialog: ({ open, children }) =>
        open ? React.createElement('div', { role: 'dialog' }, children) : null,
      DialogContent: container,
      DialogDescription: container,
      DialogFooter: container,
      DialogHeader: container,
      DialogTitle: container
    },
    '@/components/error-notice': errorNoticeModule,
    '@/features/settings/settings-card': { SettingsCard: container },
    '@/lib/platform': { isAndroid: () => false },
    'react-markdown': { __esModule: true, default: Markdown },
    'remark-gfm': { __esModule: true, default: remarkGfm },
    '@tauri-apps/plugin-opener': { openUrl: async () => {} },
    './release-notes.css': {},
    '@/lib/player': {
      errorText: String,
      nativeCall: async (command, args) => {
        if (command === 'update_status') return status
        calls.push(command)
        if (command === 'set_update_auto_check') {
          if (failPreferenceSave) throw new Error('设置保存失败')
          status = { ...status, autoCheck: args.enabled }
          listener({ payload: status })
          return status
        }
        if (command === 'update_check') {
          return new Promise((resolve, reject) => {
            finishCheck = { resolve, reject }
          })
        }
        if (command === 'update_download') {
          status = { ...status, phase: 'ready' }
          listener({ payload: status })
        }
      }
    }
  }
  const notesExports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/updates/release-notes.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    { exports: notesExports, require: (name) => modules[name] }
  )
  modules['./release-notes'] = notesExports
  const progressExports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/updates/update-download-progress.tsx', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports: progressExports,
      require: (name) => modules[name],
      window: dom.window,
      performance: { now: () => now }
    }
  )
  modules['./update-download-progress'] = progressExports
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/updates/update-provider.tsx', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  const root = createRoot(document.getElementById('root'))
  const click = async (label) => {
    const button = [...document.querySelectorAll('button')].find(
      (node) => node.textContent === label
    )
    assert.ok(button, `Missing ${label}`)
    await React.act(async () => button.click())
  }
  try {
    await React.act(async () =>
      root.render(
        React.createElement(exports.UpdateProvider, {}, React.createElement(exports.UpdateCard))
      )
    )
    const autoCheckSwitch = () => document.querySelector('[role="switch"]')
    assert.equal(autoCheckSwitch().getAttribute('aria-checked'), 'true')
    assert.deepEqual(calls, [], 'loading settings does not trigger an extra update check')
    await React.act(async () => autoCheckSwitch().click())
    assert.equal(status.autoCheck, false)
    assert.equal(autoCheckSwitch().getAttribute('aria-checked'), 'false')
    await React.act(async () => root.render(null))
    await React.act(async () =>
      root.render(
        React.createElement(exports.UpdateProvider, {}, React.createElement(exports.UpdateCard))
      )
    )
    assert.equal(autoCheckSwitch().getAttribute('aria-checked'), 'false')
    failPreferenceSave = true
    await React.act(async () => autoCheckSwitch().click())
    assert.equal(autoCheckSwitch().getAttribute('aria-checked'), 'false')
    assert.match(document.body.textContent, /设置保存失败/)
    failPreferenceSave = false
    await React.act(async () => autoCheckSwitch().click())
    assert.equal(autoCheckSwitch().getAttribute('aria-checked'), 'true')
    calls.length = 0
    await click('检查更新')
    assert.equal(document.querySelector('[role="dialog"]'), null)
    assert.ok(
      [...document.querySelectorAll('button')].find(
        (button) => button.textContent === '正在检查' && button.disabled
      )
    )
    await React.act(async () => {
      status = { ...status, phase: 'current' }
      finishCheck.resolve(status)
    })
    assert.equal(document.querySelector('[role="dialog"]'), null)
    assert.match(document.body.textContent, /当前已是最新版本/)
    await click('检查更新')
    await React.act(async () => finishCheck.reject(new Error('检查失败')))
    assert.equal(document.querySelector('[role="dialog"]'), null)
    assert.match(document.body.textContent, /检查失败/)
    assert.doesNotMatch(document.body.textContent, /当前已是最新版本/)
    calls.length = 0
    await React.act(async () => {
      status = {
        ...status,
        phase: 'available',
        version: '1.4.0',
        notes:
          '### 新版说明\n\n- **播放改进**\n\n[发布页面](https://example.com)\n\n| 项目 | 状态 |\n| --- | --- |\n| 播放 | 完成 |'
      }
      listener({ payload: status })
    })
    assert.match(document.body.textContent, /新版说明/)
    assert.equal(document.querySelector('h3').textContent, '新版说明')
    assert.equal(document.querySelector('li strong').textContent, '播放改进')
    assert.equal(document.querySelector('a').getAttribute('target'), '_blank')
    assert.equal(document.querySelectorAll('table td').length, 2)
    assert.deepEqual(calls, [])
    await React.act(async () => {
      status = { ...status, phase: 'downloading', downloaded: 1048576, total: 4194304 }
      listener({ payload: status })
    })
    assert.equal(document.querySelector('[role="progressbar"]').getAttribute('aria-valuenow'), '25')
    assert.match(document.body.textContent, /1.0 MB \/ 4.0 MB/)
    await React.act(async () => {
      status = { ...status, downloaded: 2097152 }
      listener({ payload: status })
    })
    await React.act(async () => {
      now = 1000
      tick()
    })
    assert.match(document.body.textContent, /1.0 MB\/s/)
    await React.act(async () => {
      now = 2000
      tick()
    })
    assert.match(document.body.textContent, /0 B\/s/)
    await React.act(async () => {
      status = { ...status, downloaded: 0, total: null }
      listener({ payload: status })
    })
    assert.match(document.body.textContent, /0 B \/ 未知/)
    assert.equal(
      document.querySelector('[role="progressbar"]').hasAttribute('aria-valuenow'),
      false
    )
    assert.doesNotMatch(document.body.textContent, /B\/s/)
    await React.act(async () => {
      status = { ...status, phase: 'available' }
      listener({ payload: status })
    })
    assert.equal(tick, undefined, 'leaving download clears the speed timer')
    await click('下载更新')
    assert.deepEqual(calls, ['update_download'])
    assert.match(document.body.textContent, /重启安装会中断当前播放/)
    await click('重启并安装')
    assert.deepEqual(calls, ['update_download', 'update_install'])
    await React.act(async () => {
      status = { ...status, phase: 'available', version: '1.5.0', portable: true }
      listener({ payload: status })
    })
    await click('下载便携版')
    assert.equal(calls.at(-1), 'update_open_download')
    assert.equal(calls.filter((command) => command === 'update_install').length, 1)
    await click('稍后')
    assert.equal(document.querySelector('[role="dialog"]'), null)
    await click('检查更新')
    assert.equal(document.querySelector('[role="dialog"]'), null)
    await React.act(async () => finishCheck.resolve(status))
    assert.ok(
      document.querySelector('[role="dialog"]'),
      'manual check reopens an already announced update'
    )
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
  }
})
