import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness({ enabled = false, desktop = true, failure } = {}) {
  const calls = []
  const errors = []
  const reported = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    react: { useState: (value) => [value, () => {}] },
    'lucide-react': { MicVocal: 'mic-vocal' },
    '@tauri-apps/api/core': { isTauri: () => desktop },
    '@/components/music/action-button': { ActionButton: 'button' },
    '@/lib/diagnostics': { reportCaughtError: (...args) => reported.push(args) },
    './state': {
      useDesktopLyricsState: () => ({ enabled }),
      setDesktopLyricsEnabled: async (next) => {
        calls.push(next)
        if (failure) throw failure
      }
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/desktop-lyrics/desktop-lyrics-button.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  return {
    render: () =>
      exports.DesktopLyricsButton({
        onError: (cause) => errors.push(cause)
      }),
    calls,
    errors,
    reported
  }
}

test('desktop lyrics button toggles the current state and exposes it to assistive tech', async () => {
  const off = harness()
  const enable = off.render()
  assert.equal(enable.type, 'button')
  assert.equal(enable.props.className, 'desktop-lyrics-button playback-mode-button')
  assert.equal(enable.props['aria-label'], '开启桌面歌词')
  assert.equal(enable.props['aria-pressed'], false)
  assert.equal(enable.props.disabled, false)
  enable.props.onClick()
  await new Promise((resolve) => setImmediate(resolve))
  assert.deepEqual(off.calls, [true])

  const on = harness({ enabled: true }).render()
  assert.equal(on.props['aria-label'], '关闭桌面歌词')
  assert.equal(on.props['aria-pressed'], true)
})

test('desktop lyrics button requires the desktop runtime', () => {
  const button = harness({ desktop: false }).render()
  assert.equal(button.props.disabled, true)
})

test('desktop lyrics button reports native failures through the playback error channel', async () => {
  const failure = new Error('desktop lyrics unavailable')
  const app = harness({ failure })
  app.render().props.onClick()
  await new Promise((resolve) => setImmediate(resolve))
  assert.deepEqual(app.errors, [failure])
  assert.equal(app.reported.length, 1)
})
