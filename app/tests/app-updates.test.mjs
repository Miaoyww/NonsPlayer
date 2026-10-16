import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { errorNoticeModule } from './helpers/error-notice.mjs'

test('updates require a download click and separate installation confirmation; portable opens its download', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let listener
  let status = {
    phase: 'idle',
    version: null,
    supported: true,
    portable: false,
    source: 'r2',
    downloaded: 0
  }
  const calls = []
  const container = ({ children }) => React.createElement('div', {}, children)
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
    '@/components/ui/button': {
      Button: ({ children, ...props }) => React.createElement('button', props, children)
    },
    '@/components/ui/dialog': {
      Dialog: ({ open, children }) => (open ? container({ children }) : null),
      DialogContent: container,
      DialogDescription: container,
      DialogFooter: container,
      DialogHeader: container,
      DialogTitle: container
    },
    '@/components/error-notice': errorNoticeModule,
    '@/features/settings/settings-card': { SettingsCard: container },
    '@/lib/platform': { isAndroid: () => false },
    '@/lib/player': {
      errorText: String,
      nativeCall: async (command) => {
        if (command === 'update_status') return status
        calls.push(command)
        if (command === 'update_download') {
          status = { ...status, phase: 'ready' }
          listener({ payload: status })
        }
      }
    }
  }
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
    await React.act(async () => {
      status = { ...status, phase: 'available', version: '1.4.0', notes: '新版说明' }
      listener({ payload: status })
    })
    assert.match(document.body.textContent, /新版说明/)
    assert.deepEqual(calls, [])
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
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
  }
})
