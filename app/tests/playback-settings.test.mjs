import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('playback switches wait for native persistence and retain the previous setting on failure', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let state = { deviceId: null, playbackOptions: { fades: false, gapless: true } }
  const listeners = new Set()
  const calls = []
  let pending
  const modules = {
    '@/components/ui/button': {
      Button: ({ children }) => React.createElement('button', {}, children)
    },
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/components/ui/switch': {
      Switch: ({ onCheckedChange, ...props }) =>
        React.createElement('input', {
          ...props,
          type: 'checkbox',
          onChange: (event) => onCheckedChange(event.target.checked)
        })
    },
    '@/components/ui/select': Object.fromEntries(
      ['Select', 'SelectContent', 'SelectGroup', 'SelectItem', 'SelectTrigger', 'SelectValue'].map(
        (name) => [name, ({ children }) => React.createElement('div', {}, children)]
      )
    ),
    '@/features/settings/settings-card': {
      SettingsCard: ({ title, children }) => React.createElement('section', {}, title, children)
    },
    '@/features/playback/music-options': {
      useMusicOptions: () => ({ options: { allowDowngrade: true }, busy: false, update: () => {} })
    },
    '@/features/playback/audio-output': {
      browserOutput: () => ({ supportsDevices: true }),
      browserOutputDevices: async () => []
    },
    '@/lib/player': {
      usePlayer: () =>
        React.useSyncExternalStore(
          (listener) => {
            listeners.add(listener)
            return () => listeners.delete(listener)
          },
          () => state
        ),
      errorText: (error) => error.message,
      nativeCall: async (command, args) => {
        if (command === 'output_devices') return []
        calls.push([command, args])
        return new Promise((resolve, reject) => {
          pending = {
            resolve: () => {
              state = { ...state, playbackOptions: args.options }
              listeners.forEach((listener) => listener())
              resolve()
            },
            reject
          }
        })
      }
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/settings/pages/playback.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  const root = createRoot(document.getElementById('root'))
  const input = (label) =>
    [...document.querySelectorAll('label')]
      .find((element) => element.textContent.includes(label))
      .querySelector('input')
  try {
    await React.act(async () => root.render(React.createElement(exports.PlaybackPage)))
    assert.equal(input('启用播放/暂停渐变').checked, false)
    assert.equal(input('启用无缝播放').checked, true)
    await React.act(async () => input('启用播放/暂停渐变').click())
    assert.equal(input('启用播放/暂停渐变').checked, false)
    assert.equal(input('启用无缝播放').disabled, true)
    assert.match(document.body.textContent, /正在保存/)
    await React.act(async () => pending.resolve())
    assert.equal(input('启用播放/暂停渐变').checked, true)
    await React.act(async () => input('启用无缝播放').click())
    await React.act(async () => pending.reject(Error('无法保存设置')))
    assert.equal(input('启用无缝播放').checked, true)
    assert.match(document.querySelector('[role="alert"]').textContent, /无法保存设置/)
    await React.act(async () => input('启用无缝播放').click())
    await React.act(async () => pending.resolve())
    assert.equal(input('启用无缝播放').checked, false)
    assert.equal(input('启用播放/暂停渐变').checked, true)
    assert.deepEqual(JSON.parse(JSON.stringify(calls.at(-1))), [
      'set_playback_options',
      { options: { fades: true, gapless: false } }
    ])
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
  }
})

test('WebAudio keeps concise card details, enables fades and devices, and waits for the engine switch', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let state = {
    playbackEngine: 'webAudio',
    deviceId: 'saved-device',
    playbackOptions: { fades: true, gapless: true }
  }
  const listeners = new Set(),
    calls = []
  let complete
  const modules = {
    '@/components/ui/button': {
      Button: ({ children }) => React.createElement('button', {}, children)
    },
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/components/ui/switch': {
      Switch: (props) =>
        React.createElement('input', {
          type: 'checkbox',
          checked: props.checked,
          disabled: props.disabled,
          readOnly: true
        })
    },
    '@/components/ui/select': {
      Select: ({ value, items = [], disabled, onValueChange }) =>
        React.createElement(
          'select',
          { value: value ?? '', disabled, onChange: (e) => onValueChange(e.target.value) },
          items.map((item) =>
            React.createElement(
              'option',
              { key: item.value ?? '', value: item.value ?? '' },
              item.label
            )
          )
        ),
      ...Object.fromEntries(
        ['SelectContent', 'SelectGroup', 'SelectItem', 'SelectTrigger', 'SelectValue'].map(
          (name) => [name, () => null]
        )
      )
    },
    '@/features/settings/settings-card': {
      SettingsCard: ({ title, description, children }) =>
        React.createElement(
          'section',
          { 'data-title': title },
          React.createElement('p', { 'data-details': true }, description),
          children
        )
    },
    '@/features/playback/music-options': {
      useMusicOptions: () => ({ options: { allowDowngrade: true }, busy: false, update() {} })
    },
    '@/features/playback/audio-output': {
      browserOutput: () => ({ supportsDevices: true }),
      browserOutputDevices: async () => []
    },
    '@/lib/player': {
      usePlayer: () =>
        React.useSyncExternalStore(
          (listener) => {
            listeners.add(listener)
            return () => listeners.delete(listener)
          },
          () => state
        ),
      errorText: String,
      nativeCall: async (command, args) => {
        calls.push([command, args])
        if (command === 'output_devices') return []
        return new Promise((resolve) => {
          complete = () => {
            state = { ...state, playbackEngine: args.engine }
            listeners.forEach((listener) => listener())
            resolve()
          }
        })
      }
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/settings/pages/playback.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      navigator: { userAgent: 'Windows' },
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  const root = createRoot(document.getElementById('root'))
  const card = (title) => document.querySelector(`[data-title="${title}"]`)
  try {
    await React.act(async () => root.render(React.createElement(exports.PlaybackPage)))
    const engine = card('播放引擎')
    assert.equal(engine.querySelectorAll('p').length, 1)
    assert.equal(
      engine.querySelector('[data-details]').textContent,
      '使用浏览器 audio 播放，支持的音频格式由系统 WebView 决定。'
    )
    assert.equal(card('输出设备').querySelector('select').disabled, false)
    assert.equal(card('播放/暂停渐变').querySelector('input').disabled, false)
    assert.equal(card('播放/暂停渐变').querySelector('input').checked, true)
    for (const title of ['无缝播放（Gapless）']) {
      const input = card(title).querySelector('input')
      assert.equal(input.disabled, true)
      assert.equal(input.checked, false)
    }
    assert.equal(calls.length, 0)
    await React.act(async () => {
      const select = engine.querySelector('select')
      select.value = 'gstreamer'
      select.dispatchEvent(new dom.window.Event('change', { bubbles: true }))
    })
    assert.equal(engine.querySelector('select').disabled, true)
    assert.equal(state.playbackEngine, 'webAudio')
    await React.act(async () => complete())
    assert.equal(card('播放/暂停渐变').querySelector('input').checked, true)
    assert.equal(card('无缝播放（Gapless）').querySelector('input').checked, true)
    assert.equal(engine.querySelectorAll('p').length, 1)
    assert.match(engine.querySelector('[data-details]').textContent, /GStreamer/)
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
  }
})
