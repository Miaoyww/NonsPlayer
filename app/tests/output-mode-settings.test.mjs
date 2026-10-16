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

for (const supported of [true, false]) {
  test(`WASAPI mode ${supported ? 'waits for persistence and allows recovery' : 'is hidden on unsupported platforms'}`, async () => {
    const dom = new JSDOM('<div id="root"></div>')
    globalThis.window = dom.window
    globalThis.document = dom.window.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = true
    let state = {
      deviceId: null,
      audioOutputMode: 'shared',
      playbackOptions: { fades: false, gapless: true }
    }
    const listeners = new Set()
    const calls = []
    let pending
    const modules = {
      react: React,
      'react/jsx-runtime': jsx,
      '@tauri-apps/api/core': { isTauri: () => true },
      '@/components/ui/switch': { Switch: () => null },
      '@/components/ui/select': {
        Select: ({ value, disabled, onValueChange, children, items }) =>
          React.createElement(
            'select',
            {
              'aria-label': items.some((item) => item.value === 'shared')
                ? 'WASAPI 输出模式'
                : '输出设备',
              value: value ?? '',
              disabled,
              onChange: (event) => onValueChange(event.target.value)
            },
            children
          ),
        SelectTrigger: () => null,
        SelectValue: () => null,
        SelectContent: ({ children }) => children,
        SelectGroup: ({ children }) => children,
        SelectItem: ({ value, children }) =>
          React.createElement('option', { value: value ?? '' }, children)
      },
      '@/features/settings/settings-card': {
        SettingsCard: ({ title, children }) => React.createElement('section', {}, title, children)
      },
      '@/features/playback/music-options': {
        useMusicOptions: () => ({
          options: { allowDowngrade: true },
          busy: false,
          update: () => {}
        })
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
          if (command === 'audio_output_modes') return supported ? ['shared', 'exclusive'] : []
          calls.push([command, args])
          return new Promise((resolve, reject) => {
            pending = {
              reject,
              resolve: () => {
                state = { ...state, audioOutputMode: args.mode, error: null }
                listeners.forEach((listener) => listener())
                resolve()
              }
            }
          })
        }
      }
    }
    const exports = {}
    runInNewContext(
      ts.transpileModule(
        readFileSync(
          new URL('../src/features/settings/pages/playback.tsx', import.meta.url),
          'utf8'
        ),
        { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
      ).outputText,
      {
        exports,
        require: (name) =>
          name === '@/components/error-notice' ? errorNoticeModule : modules[name]
      }
    )
    const root = createRoot(document.getElementById('root'))
    const control = () => document.querySelector('select[aria-label="WASAPI 输出模式"]')
    const choose = async (mode) =>
      React.act(async () => {
        control().value = mode
        control().dispatchEvent(new dom.window.Event('change', { bubbles: true }))
      })
    try {
      await React.act(async () => root.render(React.createElement(exports.PlaybackPage)))
      if (!supported) {
        assert.equal(control(), null)
        return
      }
      assert.equal(control().value, 'shared')
      await choose('exclusive')
      assert.equal(control().value, 'shared')
      assert.equal(control().disabled, true)
      assert.equal(document.querySelector('select[aria-label="输出设备"]').disabled, true)
      assert.match(document.querySelector('[role="status"]').textContent, /正在切换/)
      await React.act(async () => pending.reject(Error('无法保存输出模式')))
      assert.equal(control().value, 'shared')
      assert.equal(control().disabled, false)
      assert.match(document.querySelector('[role="alert"]').textContent, /无法保存输出模式/)
      await choose('exclusive')
      await React.act(async () => pending.resolve())
      assert.equal(control().value, 'exclusive')
      await React.act(async () => {
        state = { ...state, error: '独占模式播放失败，请切回共享模式' }
        listeners.forEach((listener) => listener())
      })
      assert.match(document.querySelector('[role="alert"]').textContent, /切回共享模式/)
      await choose('shared')
      await React.act(async () => pending.resolve())
      assert.equal(control().value, 'shared')
      assert.equal(document.querySelector('[role="alert"]'), null)
      assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
        ['set_audio_output_mode', { mode: 'exclusive' }],
        ['set_audio_output_mode', { mode: 'exclusive' }],
        ['set_audio_output_mode', { mode: 'shared' }]
      ])
    } finally {
      await React.act(async () => root.unmount())
      dom.window.close()
    }
  })
}
