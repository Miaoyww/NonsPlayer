import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { errorNoticeModule } from './helpers/error-notice.mjs'

test('cover rebuild restores progress, rejects late snapshots and cleans up its listener', async () => {
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
  const callbacks = []
  let unlistened = 0
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => {
        assert.equal(name, 'local-cover-rebuild-progress')
        callbacks.push(callback)
        return () => unlistened++
      }
    },
    'lucide-react': { RefreshCw: () => null },
    '@/components/ui/button': {
      Button: ({ variant, children, ...props }) => React.createElement('button', props, children)
    },
    '@/components/error-notice': errorNoticeModule,
    '@/features/settings/settings-card': {
      SettingsCard: ({ title, description, children }) =>
        React.createElement('div', null, title, description, children)
    },
    '@/lib/player': {
      errorText: String,
      nativeCall: (command) =>
        new Promise((resolve, reject) => requests.push({ command, resolve, reject }))
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/local/cover-rebuild-settings.tsx', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  const root = createRoot(document.getElementById('root'))
  const idle = { processed: 0, total: 0, failed: 0, running: false, error: null }
  const running = { ...idle, processed: 1, total: 5, running: true }
  const emit = (payload) => act(async () => callbacks.at(-1)({ payload }))
  const mount = () =>
    act(async () => root.render(React.createElement(exports.CoverRebuildSettings)))
  const unmount = () => act(async () => root.render(null))
  const button = () => document.querySelector('button')
  try {
    await mount()
    assert.equal(requests[0].command, 'local_cover_rebuild_status')
    await emit(running)
    await act(async () => requests[0].resolve(idle))
    assert.equal(button().disabled, true)
    assert.match(document.body.textContent, /正在检查 1 \/ 5 首歌曲/)

    await unmount()
    assert.equal(unlistened, 1)
    await mount()
    await act(async () => requests[1].resolve(running))
    assert.equal(button().disabled, true, 'reopening settings restores the running job')
    await emit({ ...running, processed: 5, failed: 1, running: false })
    assert.match(document.body.textContent, /重建完成，已检查 5 \/ 5 首歌曲，1 首处理失败/)
    assert.equal(button().disabled, false)

    await act(async () => button().click())
    assert.equal(requests[2].command, 'local_cover_rebuild')
    assert.equal(button().disabled, true)
    await emit({ ...running, processed: 5, running: false, error: '专辑索引更新失败' })
    await act(async () => requests[2].resolve(running))
    assert.match(document.body.textContent, /重建未完成，已检查 5 \/ 5/)
    assert.match(document.body.textContent, /专辑索引更新失败/)
    assert.equal(button().disabled, false, 'late start response cannot revive a completed job')

    await act(async () => button().click())
    await act(async () => requests[3].resolve(idle))
    assert.match(document.body.textContent, /重建完成，已检查 0 \/ 0 首歌曲/)
    assert.doesNotMatch(document.body.textContent, /专辑索引更新失败/)
  } finally {
    await act(async () => root.unmount())
    assert.equal(unlistened, 2)
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    dom.window.close()
  }
})
