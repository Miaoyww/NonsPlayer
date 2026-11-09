import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import * as React from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

const source = readFileSync(
  new URL('../src/features/settings/network-service/network-service-page.tsx', import.meta.url),
  'utf8'
)
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
}).outputText

function status({
  enabled = true,
  port = 46321,
  lan = false,
  running = true,
  addresses = ['127.0.0.1:46321'],
  connections = 0,
  error = null
} = {}) {
  return { config: { enabled, port, lan }, running, addresses, connections, error }
}

async function fixture({ initial = status(), failFirst = false } = {}) {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = Object.fromEntries(
    ['window', 'document', 'IS_REACT_ACT_ENVIRONMENT'].map((key) => [
      key,
      Object.getOwnPropertyDescriptor(globalThis, key)
    ])
  )
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let current = initial
  let statusReads = 0
  const calls = []
  const copied = []
  const opened = []
  const listeners = new Map()
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (event, listener) => {
        listeners.set(event, listener)
        return () => listeners.delete(event)
      }
    },
    '@tauri-apps/plugin-clipboard-manager': {
      writeText: async (value) => copied.push(value)
    },
    '@tauri-apps/plugin-opener': {
      openUrl: async (value) => opened.push(value)
    },
    'lucide-react': Object.fromEntries(
      ['BookOpen', 'Copy', 'Globe2', 'Link2', 'Server'].map((name) => [name, () => null])
    ),
    '@/components/error-notice': {
      ErrorNotice: ({ error }) => React.createElement('p', { role: 'alert' }, error)
    },
    '@/components/ui/button': {
      Button: ({ variant: _variant, size: _size, ...props }) =>
        React.createElement('button', { type: 'button', ...props })
    },
    '@/components/ui/input': {
      Input: (props) => React.createElement('input', props)
    },
    '@/components/ui/switch': {
      Switch: ({ onCheckedChange, ...props }) =>
        React.createElement('input', {
          ...props,
          type: 'checkbox',
          onChange: (event) => onCheckedChange(event.target.checked)
        })
    },
    '@/features/settings/shell/settings-card': {
      SettingsCard: ({ title, description, children }) =>
        React.createElement(
          'section',
          { 'data-title': title },
          React.createElement('p', {}, description),
          children
        )
    },
    '@/lib/player': {
      errorText: (error) => error.message,
      nativeCall: async (command, args) => {
        calls.push([command, args])
        if (command === 'open_api_status') {
          statusReads += 1
          if (failFirst && statusReads === 1) throw Error('状态读取失败')
          return current
        }
        if (command === 'open_api_configure') {
          current = { ...current, config: args.config }
          return current
        }
        throw Error(`Unexpected command: ${command}`)
      }
    }
  }
  const exports = {}
  runInNewContext(compiled, {
    exports,
    require: (name) => modules[name],
    setTimeout,
    clearTimeout
  })
  const root = createRoot(document.getElementById('root'))
  await React.act(async () => root.render(React.createElement(exports.NetworkServicePage)))
  return {
    root,
    calls,
    copied,
    opened,
    listeners,
    async cleanup() {
      await React.act(async () => root.unmount())
      dom.window.close()
      for (const [key, descriptor] of Object.entries(previous))
        if (descriptor) Object.defineProperty(globalThis, key, descriptor)
        else delete globalThis[key]
    }
  }
}

async function change(input, value) {
  await React.act(async () => {
    const props = Object.keys(input).find((key) => key.startsWith('__reactProps$'))
    if (props) input[props].onChange({ currentTarget: { value } })
    else {
      Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set.call(
        input,
        value
      )
      input.dispatchEvent(new window.Event('input', { bubbles: true }))
    }
  })
}

test('status events refresh runtime information without replacing a dirty port draft', async () => {
  const app = await fixture()
  try {
    const port = document.querySelector('#open-api-port')
    await change(port, '47000')
    await React.act(async () =>
      app.listeners.get('open-api-status-changed')({
        payload: status({ port: 46321, connections: 2 })
      })
    )
    assert.equal(port.value, '47000')
    assert.match(document.body.textContent, /2 个连接/)
  } finally {
    await app.cleanup()
  }
})

test('disabling the service uses the saved config when the port draft is invalid', async () => {
  const app = await fixture()
  try {
    await change(document.querySelector('#open-api-port'), '0')
    await React.act(async () => document.querySelector('[aria-label="启用开放 API"]').click())
    const configure = app.calls.find(([command]) => command === 'open_api_configure')
    assert.equal(
      JSON.stringify(configure[1]),
      JSON.stringify({
        config: { enabled: false, port: 46321, lan: false }
      })
    )
  } finally {
    await app.cleanup()
  }
})

test('failed status loading exposes a retry that restores the page', async () => {
  const app = await fixture({ failFirst: true })
  try {
    assert.match(document.body.textContent, /状态读取失败/)
    await React.act(async () => document.querySelector('button').textContent)
    const retry = [...document.querySelectorAll('button')].find(
      (button) => button.textContent === '重试'
    )
    await React.act(async () => retry.click())
    assert.match(document.body.textContent, /运行中/)
    assert.equal(app.calls.filter(([command]) => command === 'open_api_status').length, 2)
  } finally {
    await app.cleanup()
  }
})

test('HTTP and WebSocket addresses copy and protocol docs open from the active address', async () => {
  const app = await fixture()
  try {
    await React.act(async () =>
      document.querySelector('[aria-label="复制 HTTP 地址 http://127.0.0.1:46321"]').click()
    )
    await React.act(async () =>
      document
        .querySelector('[aria-label="复制 WebSocket 地址 ws://127.0.0.1:46321/v1/events"]')
        .click()
    )
    const docs = [...document.querySelectorAll('button')].find(
      (button) => button.textContent === '打开文档'
    )
    await React.act(async () => docs.click())
    assert.deepEqual(app.copied, ['http://127.0.0.1:46321', 'ws://127.0.0.1:46321/v1/events'])
    assert.deepEqual(app.opened, ['http://127.0.0.1:46321/v1/docs'])
  } finally {
    await app.cleanup()
  }
})
