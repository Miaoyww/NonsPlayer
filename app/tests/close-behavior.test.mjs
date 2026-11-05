import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { errorNoticeModule } from './helpers/error-notice.mjs'

async function harness({ pending = false, native = true, android = false } = {}) {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const calls = []
  const diagnostics = []
  const listeners = new Map()
  const unsubscribed = []
  let preference = 'undecided'
  let confirm
  let read
  let writeFailure
  let register
  let dismiss
  const selectContext = React.createContext(null)
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => native },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => {
        if (register) await register()
        listeners.set(name, callback)
        return () => {
          listeners.delete(name)
          unsubscribed.push(name)
        }
      }
    },
    '@/lib/platform': { isAndroid: () => android },
    '@/lib/diagnostics': {
      reportCaughtError: (operation, cause) => diagnostics.push({ operation, cause })
    },
    '@/lib/player': {
      errorText: (cause) => cause.message,
      nativeCall: async (command, args) => {
        calls.push([command, args])
        if (command === 'take_main_close_request') {
          const value = pending
          pending = false
          return value
        }
        if (command === 'confirm_main_close') {
          if (confirm) await confirm(args)
          if (args.remember) preference = args.action
        }
        if (command === 'close_behavior') return read ? read() : preference
        if (command === 'set_close_behavior') {
          if (writeFailure) throw writeFailure
          preference = args.behavior
          listeners.get('close-behavior-changed')?.({ payload: preference })
        }
      }
    },
    '@/components/ui/button': {
      Button: ({ variant: _variant, ...props }) => React.createElement('button', props)
    },
    '@/components/error-notice': errorNoticeModule,
    '@/components/ui/dialog': {
      Dialog: ({ open, onOpenChange, children }) => {
        dismiss = () => onOpenChange(false)
        return open ? children : null
      },
      DialogContent: ({ children }) => React.createElement('div', { role: 'dialog' }, children),
      DialogHeader: ({ children }) => React.createElement('header', null, children),
      DialogTitle: ({ children }) => React.createElement('h2', null, children),
      DialogDescription: ({ children }) => React.createElement('p', null, children),
      DialogFooter: ({ children }) => React.createElement('footer', null, children)
    },
    '../shell/settings-card': {
      SettingsCard: ({ title, description, children }) =>
        React.createElement('section', null, title, description, children)
    },
    '@/components/ui/select': {
      Select: ({ value, disabled, onValueChange, children }) =>
        React.createElement(
          selectContext.Provider,
          { value: { value, disabled, onValueChange } },
          children
        ),
      SelectTrigger: ({ children, ...props }) => {
        const { disabled, value } = React.useContext(selectContext)
        return React.createElement('button', { ...props, disabled, 'data-value': value }, children)
      },
      SelectValue: () => React.useContext(selectContext).value,
      SelectContent: ({ children }) => children,
      SelectItem: ({ value, children }) => {
        const { disabled, onValueChange } = React.useContext(selectContext)
        return React.createElement(
          'button',
          { disabled, onClick: () => onValueChange(value) },
          children
        )
      }
    }
  }
  function load(filename, exportName) {
    const exports = {}
    const source = readFileSync(
      new URL(`../src/features/settings/general/${filename}.tsx`, import.meta.url),
      'utf8'
    )
    runInNewContext(
      ts.transpileModule(source, {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        require(name) {
          assert.ok(name in modules, `unexpected dependency: ${name}`)
          return modules[name]
        }
      }
    )
    return exports[exportName]
  }
  const root = createRoot(document.getElementById('root'))
  const button = (label) =>
    [...document.querySelectorAll('button')].find((element) => element.textContent === label)
  return {
    calls,
    diagnostics,
    listeners,
    unsubscribed,
    button,
    get preference() {
      return preference
    },
    set confirm(value) {
      confirm = value
    },
    set read(value) {
      read = value
    },
    set writeFailure(value) {
      writeFailure = value
    },
    set register(value) {
      register = value
    },
    async mount(component = 'dialog') {
      const Component =
        component === 'dialog'
          ? load('close-behavior-dialog', 'CloseBehaviorDialog')
          : load('close-behavior-select', 'CloseBehaviorSelect')
      await act(async () => root.render(React.createElement(Component)))
    },
    async request() {
      pending = true
      await act(async () => listeners.get('main-close-requested')?.({ payload: null }))
    },
    async dismiss() {
      await act(async () => dismiss())
    },
    async click(label) {
      assert.ok(button(label), `missing button: ${label}`)
      await act(async () => button(label).click())
    },
    async unmount() {
      await act(async () => root.unmount())
    },
    async cleanup() {
      await act(async () => root.unmount())
      dom.window.close()
      globalThis.window = previous.window
      globalThis.document = previous.document
      globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    }
  }
}

test('pending close opens on mount; cancel and dialog dismissal leave the app running', async () => {
  const state = await harness({ pending: true })
  try {
    await state.mount()
    assert.ok(document.querySelector('[role="dialog"]'))
    assert.equal(document.querySelector('input').checked, false)
    await state.click('取消')
    assert.equal(document.querySelector('[role="dialog"]'), null)
    await state.request()
    await state.dismiss()
    assert.equal(document.querySelector('[role="dialog"]'), null)
    assert.equal(state.calls.filter(([command]) => command === 'confirm_main_close').length, 0)
    assert.equal(state.preference, 'undecided')
  } finally {
    await state.cleanup()
  }
})

test('one-time minimize keeps asking; remember persists the selected action and repeated requests retain the checkbox', async () => {
  const state = await harness()
  try {
    await state.mount()
    assert.equal(document.querySelector('[role="dialog"]'), null)
    await state.request()
    await state.click('最小化')
    assert.equal(state.preference, 'undecided')
    assert.deepEqual(
      JSON.parse(
        JSON.stringify(state.calls.filter(([command]) => command === 'confirm_main_close'))
      ),
      [['confirm_main_close', { action: 'minimize', remember: false }]]
    )
    await state.request()
    await act(async () => document.querySelector('input').click())
    await state.request()
    assert.equal(document.querySelectorAll('[role="dialog"]').length, 1)
    assert.equal(document.querySelector('input').checked, true)
    await state.click('关闭应用')
    assert.equal(state.preference, 'close')
    assert.equal(document.querySelector('[role="dialog"]'), null)
  } finally {
    await state.cleanup()
  }
})

test('confirmation is serialized, busy dialogs cannot dismiss, and failure retains a retry with diagnostics', async () => {
  const state = await harness({ pending: true })
  try {
    let reject
    state.confirm = () =>
      new Promise((_, fail) => {
        reject = fail
      })
    await state.mount()
    await act(async () => {
      state.button('最小化').click()
      state.button('关闭应用').click()
    })
    assert.equal(state.calls.filter(([command]) => command === 'confirm_main_close').length, 1)
    assert.equal(document.querySelector('input').disabled, true)
    await state.dismiss()
    assert.ok(document.querySelector('[role="dialog"]'))
    await act(async () => reject(Error('保存关闭行为失败')))
    assert.match(document.querySelector('[role="alert"]').textContent, /保存关闭行为失败/)
    assert.equal(state.diagnostics.length, 1)
    assert.equal(state.preference, 'undecided')
    state.confirm = undefined
    await state.click('最小化')
    assert.equal(document.querySelector('[role="dialog"]'), null)
  } finally {
    await state.cleanup()
  }
})

test('settings accept undecided and follow remembered changes without an old bootstrap overriding them', async () => {
  const state = await harness()
  try {
    let resolveRead
    state.read = () =>
      new Promise((resolve) => {
        resolveRead = resolve
      })
    await state.mount('settings')
    await act(async () => {
      state.listeners.get('close-behavior-changed')({ payload: 'minimize' })
      resolveRead('undecided')
    })
    assert.equal(document.querySelector('[data-value]').dataset.value, 'minimize')
    await state.click('未定')
    assert.equal(state.preference, 'undecided')
    state.writeFailure = Error('写入失败')
    await state.click('关闭')
    assert.equal(document.querySelector('[data-value]').dataset.value, 'undecided')
    assert.match(document.querySelector('[role="alert"]').textContent, /写入失败/)
    await state.unmount()
    assert.ok(state.unsubscribed.includes('close-behavior-changed'))
  } finally {
    await state.cleanup()
  }
})

test('unmount releases active and late listeners; browser and Android never subscribe', async () => {
  for (const options of [{}, { native: false }, { android: true }]) {
    const state = await harness(options)
    try {
      await state.mount()
      const late = state.listeners.get('main-close-requested')
      await state.unmount()
      const count = state.calls.length
      if (late) await act(async () => late({ payload: null }))
      assert.equal(state.listeners.size, 0)
      assert.equal(state.calls.length, count)
    } finally {
      await state.cleanup()
    }
  }
  const state = await harness()
  try {
    let resolveRegistration
    state.register = () =>
      new Promise((resolve) => {
        resolveRegistration = resolve
      })
    await state.mount()
    await state.unmount()
    await act(async () => resolveRegistration())
    assert.deepEqual(state.unsubscribed, ['main-close-requested'])
    assert.equal(state.calls.length, 0)
  } finally {
    await state.cleanup()
  }
})
