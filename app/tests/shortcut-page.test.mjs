import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import * as React from 'react'
import * as jsxRuntime from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function load(path, modules = {}) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  return exports
}

test('shortcut page hides disabled bindings, automatically saves four-key chords and clears bindings without a save button', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const keys = load('../src/features/playback/shortcuts/shortcut-keys.ts')
  const actions = [
    ['toggle', '播放/暂停音乐'],
    ['previous', '上一首'],
    ['next', '下一首'],
    ['like', '收藏此音乐'],
    ['volumeUp', '音量加'],
    ['volumeDown', '音量减']
  ]
  let status = {
    settings: { enabled: false, bindings: Object.fromEntries(actions.map(([name]) => [name, ''])) },
    busy: false
  }
  const listeners = new Set(),
    pauses = []
  let failNextSave = false
  const page = load('../src/features/settings/pages/shortcuts.tsx', {
    react: React,
    'react/jsx-runtime': jsxRuntime,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/components/ui/button': { Button: 'button' },
    '@/components/ui/kbd': { Kbd: 'kbd', KbdGroup: 'div' },
    '@/components/ui/switch': {
      Switch: ({ checked, onCheckedChange, ...props }) =>
        React.createElement('button', {
          ...props,
          role: 'switch',
          'aria-checked': checked,
          onClick: () => onCheckedChange(!checked)
        })
    },
    '@/features/playback/shortcuts/shortcut-keys': keys,
    '@/features/playback/shortcuts/shortcuts': {
      shortcutActions: actions,
      getShortcutStatus: () => status,
      subscribeShortcuts: (fn) => {
        listeners.add(fn)
        return () => listeners.delete(fn)
      },
      setShortcutRecording: async (value) => {
        pauses.push(value)
        return true
      },
      saveShortcuts: async (settings) => {
        if (failNextSave) {
          failNextSave = false
          status = { ...status, error: '快捷键已被占用' }
          listeners.forEach((fn) => fn())
          return false
        }
        status = { ...status, error: undefined, settings }
        listeners.forEach((fn) => fn())
        return true
      }
    },
    '@/features/settings/settings-card': {
      SettingsCard: ({ title, children }) => React.createElement('div', {}, title, children)
    }
  }).ShortcutsPage
  const root = createRoot(document.getElementById('root'))
  const click = async (element) => React.act(async () => element.click())
  const press = async (element, code, props = {}) =>
    React.act(async () => {
      element.dispatchEvent(
        new dom.window.KeyboardEvent('keydown', {
          code,
          key: code === 'Escape' ? 'Escape' : 'p',
          bubbles: true,
          cancelable: true,
          ...props
        })
      )
    })
  try {
    await React.act(async () => root.render(React.createElement(page)))
    assert.equal(document.querySelector('kbd').textContent, 'Space')
    assert.equal(document.querySelectorAll('[data-shortcut-recorder]').length, 0)
    await click(document.querySelector('[role="switch"]'))
    assert.equal(document.querySelectorAll('[data-shortcut-recorder]').length, 6)
    const recorder = document.querySelector('[aria-label="播放/暂停音乐全局快捷键"]')
    await click(recorder)
    assert.equal(document.activeElement, recorder)
    await press(recorder, 'KeyP')
    assert.match(document.querySelector('[role="alert"]').textContent, /组合键/)
    await press(recorder, 'KeyP', { ctrlKey: true, altKey: true, shiftKey: true, metaKey: true })
    assert.match(document.querySelector('[role="alert"]').textContent, /最多四个键/)
    await press(recorder, 'KeyP', { ctrlKey: true, altKey: true, shiftKey: true })
    assert.deepEqual(
      [...recorder.querySelectorAll('kbd')].map((kbd) => kbd.textContent),
      ['Ctrl', 'Alt', 'Shift', 'P']
    )
    assert.ok(
      ![...document.querySelectorAll('button')].some(
        (button) => button.textContent === '保存快捷键'
      )
    )
    assert.equal(status.settings.bindings.toggle, 'Ctrl+Alt+Shift+P')
    failNextSave = true
    await click(recorder)
    await press(recorder, 'KeyQ', { ctrlKey: true, altKey: true, shiftKey: true })
    assert.equal(status.settings.bindings.toggle, 'Ctrl+Alt+Shift+P')
    assert.match(document.querySelector('[role="alert"]').textContent, /占用/)
    assert.deepEqual(
      [...recorder.querySelectorAll('kbd')].map((kbd) => kbd.textContent),
      ['Ctrl', 'Alt', 'Shift', 'P']
    )
    const previous = document.querySelector('[aria-label="上一首全局快捷键"]')
    await click(previous)
    await press(previous, 'KeyP', { ctrlKey: true, altKey: true, shiftKey: true })
    assert.match(document.querySelector('[role="alert"]').textContent, /其他操作/)
    await press(previous, 'Escape')
    await click(document.querySelector('[aria-label="清除播放/暂停音乐绑定"]'))
    assert.equal(status.settings.bindings.toggle, '')
    await click(document.querySelector('[role="switch"]'))
    assert.equal(document.querySelectorAll('[data-shortcut-recorder]').length, 0)
    assert.ok(pauses.includes(true) && pauses.includes(false))
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
  }
})
