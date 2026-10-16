import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function load(path, modules = {}, globals = {}) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    { exports, require: (name) => modules[name], ...globals }
  )
  return exports
}

test('playback errors expire independently after 3 seconds and active duplicates are suppressed', () => {
  const timers = new Map()
  let clock = 0
  const store = load(
    '../src/features/playback/error-notices.ts',
    {},
    { setTimeout, clearTimeout }
  ).createErrorNotices(
    (fn, delay) => {
      const timer = { fn, at: clock + delay }
      timers.set(timer, timer)
      return timer
    },
    (timer) => timers.delete(timer)
  )
  const advance = (ms) => {
    clock += ms
    for (const timer of [...timers.values()]) if (timer.at <= clock) timer.fn()
  }
  store.push('第一条')
  store.push('第一条')
  advance(1000)
  store.push('第二条')
  advance(1999)
  assert.equal(store.getSnapshot().length, 2)
  advance(1)
  assert.equal(store.getSnapshot().length, 1)
  assert.equal(store.getSnapshot()[0].message, '第二条')
  advance(1000)
  assert.equal(store.getSnapshot().length, 0)
  store.push('第一条')
  assert.equal(store.getSnapshot().length, 1)
})

test('visible playback slots own transient errors; dialogs and pages without a bar keep local errors', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const callbacks = new Map()
  const timer = (fn, delay) => {
    assert.equal(delay, 3000)
    callbacks.set(fn, fn)
    return fn
  }
  const clearTimer = (fn) => callbacks.delete(fn)
  const storeModule = load(
    '../src/features/playback/error-notices.ts',
    {},
    { setTimeout: timer, clearTimeout: clearTimer }
  )
  const notices = load(
    '../src/components/error-notice.tsx',
    {
      react: React,
      'react/jsx-runtime': jsx,
      '@/features/playback/error-notices': storeModule
    },
    { document, window }
  )
  const state = {
    status: 'loading',
    revision: 1,
    playbackSource: { changed: true, displayName: 'GD', title: 'Song', artist: 'Artist' }
  }
  const playback = load(
    '../src/features/playback/playback-notice.tsx',
    {
      react: React,
      'react/jsx-runtime': jsx,
      '@/lib/player': { usePlayer: () => state },
      '@/components/error-notice': notices,
      './error-notices': storeModule
    },
    { window: { setTimeout: timer, clearTimeout: clearTimer } }
  )
  const root = createRoot(document.getElementById('root'))
  const render = async (bar, dialog = false, error = '请求失败') =>
    React.act(async () =>
      root.render(
        React.createElement(
          'main',
          {},
          React.createElement(
            'section',
            { role: dialog ? 'dialog' : undefined },
            React.createElement(notices.ErrorNotice, { error })
          ),
          bar &&
            React.createElement(
              'div',
              { className: bar === 'fullscreen' ? 'fullscreen-playback-notice' : undefined },
              React.createElement(playback.PlaybackNotice)
            )
        )
      )
    )
  const expire = async () =>
    React.act(async () => {
      for (const fn of [...callbacks.keys()]) {
        callbacks.delete(fn)
        fn()
      }
    })
  try {
    await render(true)
    assert.equal(
      document.querySelector('[data-playback-notices] [role="alert"]').textContent,
      '请求失败'
    )
    assert.match(document.body.textContent, /正在尝试其它音源 GD/)
    await expire()
    assert.equal(document.querySelector('[role="alert"]'), null)
    await render(true, true, '登录失败')
    assert.equal(document.querySelector('[role="dialog"] [role="alert"]').textContent, '登录失败')
    assert.equal(storeModule.errorNotices.getSnapshot().length, 0)
    await expire()
    assert.match(document.querySelector('[role="dialog"]').textContent, /登录失败/)
    await render(false, false, '没有播放栏')
    assert.equal(document.querySelector('section [role="alert"]').textContent, '没有播放栏')
    state.status = 'error'
    await render('fullscreen', false, '全屏错误')
    assert.match(document.querySelector('.fullscreen-playback-notice').textContent, /全屏错误/)
    assert.match(
      document.querySelector('.fullscreen-playback-notice').textContent,
      /备用音源获取失败 GD/
    )
    await expire()
    assert.equal(document.querySelector('.fullscreen-playback-notice').textContent, '')
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
