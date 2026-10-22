import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

test('the correction dialog rejects late previews, saves the selected version and can restore automatic matching', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const saved = []
  const candidate = (id) => ({
    source: 'qq',
    id,
    mid: `mid${id}`,
    title: `Version ${id}`,
    artist: 'Artist',
    album: 'Album',
    durationMs: 10000
  })
  let late
  let closed = 0
  const wrapper = (tag, props) => React.createElement(tag, props, props.children)
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    'lucide-react': new Proxy({}, { get: () => () => null }),
    '@/components/ui/dialog': {
      Dialog: ({ children }) => children,
      DialogContent: ({ children, className }) =>
        React.createElement('section', { role: 'dialog', className }, children),
      DialogHeader: (props) => wrapper('header', props),
      DialogTitle: (props) => wrapper('h2', props),
      DialogDescription: (props) => wrapper('p', props)
    },
    '@/components/ui/button': {
      Button: ({ children, variant, size, ...props }) =>
        React.createElement('button', props, children)
    },
    '@/components/ui/input': { Input: (props) => React.createElement('input', props) },
    '@/components/error-notice': {
      ErrorNotice: ({ error }) =>
        error ? React.createElement('p', { role: 'alert' }, error) : null
    },
    '@/lib/player': {
      usePlayer: () => ({ revision: 1 }),
      getPlayer: () => ({ revision: 1 }),
      errorText: String,
      nativeCall: async (command, args) => {
        if (command === 'lyric_correction') return { candidate: null, offsetMs: -200 }
        if (command === 'lyric_candidates') return [candidate(1), candidate(2)]
        if (command === 'preview_lyric_candidate')
          return args.candidate.id === 1
            ? new Promise((resolve) => {
                late = resolve
              })
            : { content: 'Correct version', format: 'lrc' }
        if (command === 'set_lyric_correction') saved.push(args)
      }
    },
    './parse-lyrics': { parseLyrics: (value) => [{ words: [{ word: value.content }] }] },
    './correction': { automaticLyrics: { candidate: null, offsetMs: 0 } }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/lyrics/correction-dialog.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  const root = createRoot(document.getElementById('root'))
  const click = async (element) =>
    React.act(async () => {
      element.click()
    })
  const button = (text) =>
    [...document.querySelectorAll('button')].find((element) => element.textContent === text)
  try {
    await React.act(async () =>
      root.render(
        React.createElement(exports.LyricCorrectionDialog, {
          track: { key: 'original:1', title: 'Song', artist: 'Artist', durationMs: 10000 },
          onClose: () => {
            closed++
          }
        })
      )
    )
    await React.act(async () =>
      document
        .querySelector('form')
        .dispatchEvent(new dom.window.Event('submit', { bubbles: true, cancelable: true }))
    )
    await click(button('Version 1Artist · Album · 0:10'))
    assert.equal(button('保存').disabled, true)
    await click(button('Version 2Artist · Album · 0:10'))
    assert.equal(
      document.querySelector('[aria-label="歌词预览"]').textContent.includes('Correct version'),
      true
    )
    await React.act(async () => late({ content: 'Stale version', format: 'lrc' }))
    assert.equal(
      document.querySelector('[aria-label="歌词预览"]').textContent.includes('Stale version'),
      false
    )
    await click(document.querySelector('[aria-label="歌词提前 100 毫秒"]'))
    await click(button('保存'))
    assert.equal(saved[0].key, 'original:1')
    assert.equal(saved[0].correction.candidate.id, 2)
    assert.equal(saved[0].correction.offsetMs, -300)
    assert.equal(closed, 1)
    await click(button('恢复自动匹配'))
    assert.equal(saved[1].correction.candidate, null)
    assert.equal(saved[1].correction.offsetMs, 0)
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
