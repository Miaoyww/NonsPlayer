import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('adapter settings wait for persistence, report failures and reject stale list responses', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const requests = []
  let changed
  let unsubscribed = false
  let invalidations = 0
  const modules = {
    '@/features/music/source-preferences-card': { SourcePreferencesCard: () => null },
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
    'lucide-react': { Blocks: () => null, RefreshCw: () => null, RotateCw: () => null },
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
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    },
    '@/lib/runtime-cache': {
      invalidateNativeCache: () => {
        invalidations++
      }
    }
  }
  const exports = {}
  const code = ts.transpileModule(
    readFileSync(new URL('../src/features/settings/pages/adapters.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  ).outputText
  runInNewContext(code, { exports, require: (name) => modules[name] })
  const root = createRoot(document.getElementById('root'))
  const status = (enabled) => [
    {
      descriptor: {
        source: 'netease',
        displayName: '网易云音乐',
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
      [...document.querySelectorAll('button')].find((button) => button.textContent === '重新加载')
        .disabled,
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
    await act(async () => root.unmount())
    assert.equal(unsubscribed, true)
    await act(async () => requests[6].resolve(status(false)))
  } finally {
    await act(async () => root.unmount())
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    dom.window.close()
  }
})
