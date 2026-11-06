import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

for (const { name, songOffset } of [
  {
    name: 'the song has no extra offset',
    songOffset: null
  },
  {
    name: 'the song has a positive extra offset',
    songOffset: 500
  },
  {
    name: 'the song has a zero extra offset',
    songOffset: 0
  }
]) {
  test(`the source panel saves platforms without searching or displaying previews when ${name}`, async () => {
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
    const calls = []
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
      '@/lib/utils': { cn: (...values) => values.filter(Boolean).join(' ') },
      '@/components/error-notice': {
        ErrorNotice: ({ error }) =>
          error ? React.createElement('p', { role: 'alert' }, error) : null
      },
      '@/lib/player': {
        usePlayer: () => ({ revision: 1 }),
        getPlayer: () => ({ revision: 1 }),
        errorText: String,
        nativeCall: async (command, args) => {
          calls.push(command)
          if (command === 'lyric_correction')
            return { source: 'netease', candidate: null, localTtml: null, offsetMs: songOffset }
          if (command === 'pick_local_ttml') return 'C:/Lyrics/song.ttml'
          if (command === 'preview_local_ttml') return { content: 'TTML', format: 'ttml' }
          if (command === 'set_lyric_correction') saved.push(args)
          else if (command !== 'lyric_correction') throw new Error(`Unexpected command: ${command}`)
        }
      },
      '@/lib/platform': { isAndroid: () => false },
      './parse-lyrics': { parseLyrics: () => [{ words: [{ word: 'Validated TTML' }] }] },
      './correction': {
        automaticLyrics: { source: null, candidate: null, localTtml: null, offsetMs: null }
      }
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
          React.createElement(exports.LyricCorrectionContent, {
            track: { key: 'original:1', title: 'Song', artist: 'Artist', durationMs: 10000 },
            onClose: () => {
              closed++
            }
          })
        )
      )
      assert.equal(document.querySelector('#lyric-offset'), null)
      assert.equal(document.querySelector('form'), null)
      assert.equal(document.querySelector('[aria-label="歌词候选"]'), null)
      assert.equal(document.querySelector('[aria-label="歌词预览"]'), null)
      assert.equal(button('网易云音乐').getAttribute('aria-pressed'), 'true')
      await click(button('QQ 音乐'))
      await click(button('保存'))
      assert.equal(saved[0].key, 'original:1')
      assert.equal(saved[0].correction.candidate, null)
      assert.equal(saved[0].correction.source, 'qq')
      assert.equal(saved[0].correction.offsetMs, songOffset)
      assert.equal(closed, 1)
      await click(button('网易云音乐'))
      await click(button('保存'))
      assert.equal(saved[1].correction.candidate, null)
      assert.equal(saved[1].correction.localTtml, null)
      assert.equal(saved[1].correction.source, 'netease')
      assert.equal(saved[1].correction.offsetMs, songOffset)
      await click(button('本地歌词'))
      assert.equal(button('保存').disabled, true)
      await click(button('选择 TTML 文件'))
      assert.equal(button('保存').disabled, false)
      assert.equal(document.querySelector('[aria-label="歌词预览"]'), null)
      await click(button('保存'))
      assert.equal(saved[2].correction.source, null)
      assert.equal(saved[2].correction.localTtml.path, 'C:/Lyrics/song.ttml')
      assert.equal(saved[2].correction.offsetMs, songOffset)
      await click(button('恢复自动匹配'))
      assert.equal(saved[3].correction.source, null)
      assert.equal(saved[3].correction.candidate, null)
      assert.equal(saved[3].correction.localTtml, null)
      assert.equal(saved[3].correction.offsetMs, null)
      assert.equal(document.querySelector('input[aria-label="歌词搜索词"]'), null)
      assert.ok(!calls.includes('lyric_candidates'))
      assert.ok(!calls.includes('preview_lyric_candidate'))
    } finally {
      await React.act(async () => root.unmount())
      dom.window.close()
      globalThis.window = previous.window
      globalThis.document = previous.document
      globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    }
  })
}
