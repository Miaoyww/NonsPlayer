import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('autostart reflects system veto, refreshes on focus and re-reads partial write failures', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let status = { enabled: false, blockedBySystem: true }
  let fail = false
  let toggle
  const calls = []
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/components/ui/switch': {
      Switch: ({ onCheckedChange, ...props }) => {
        toggle = onCheckedChange
        return React.createElement('input', { ...props, type: 'checkbox', readOnly: true })
      }
    },
    './settings-card': { SettingsCard: ({ children }) => React.createElement('div', {}, children) },
    '@/lib/player': {
      errorText: (e) => e.message,
      nativeCall: async (command, args) => {
        calls.push([command, args])
        if (command === 'set_autostart') {
          status = { enabled: args.enabled, blockedBySystem: false }
          if (fail) throw Error('写入失败')
        }
        return status
      }
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/settings/autostart-switch.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    {
      exports,
      window,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  const root = createRoot(document.getElementById('root'))
  try {
    await React.act(async () => root.render(React.createElement(exports.AutostartSwitch)))
    assert.equal(document.querySelector('input').checked, false)
    assert.match(document.body.textContent, /已被 Windows 禁用/)
    assert.equal(calls.filter(([c]) => c === 'set_autostart').length, 0)
    await React.act(async () => toggle(true))
    assert.equal(document.querySelector('input').checked, true)
    status = { enabled: false, blockedBySystem: true }
    await React.act(async () => window.dispatchEvent(new window.Event('focus')))
    assert.equal(document.querySelector('input').checked, false)
    fail = true
    await React.act(async () => toggle(true))
    assert.equal(document.querySelector('input').checked, true)
    assert.match(document.querySelector('[role="alert"]').textContent, /写入失败/)
    await React.act(async () => root.unmount())
    const count = calls.length
    window.dispatchEvent(new window.Event('focus'))
    assert.equal(calls.length, count)
  } finally {
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
  }
})
