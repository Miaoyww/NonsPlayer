import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
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
        if (command === 'output_devices' || command === 'audio_output_modes') return []
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
    { exports, require: (name) => modules[name] }
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
