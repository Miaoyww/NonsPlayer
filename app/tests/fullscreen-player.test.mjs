import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

async function harness(t, delayed = false) {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    IS_REACT_ACT_ENVIRONMENT: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    IS_REACT_ACT_ENVIRONMENT: true
  })
  let mounts = 0,
    unmounts = 0,
    loads = 0,
    exits = 0,
    release
  const gate = new Promise((resolve) => {
    release = resolve
  })
  Object.defineProperty(gate, '__esModule', { value: true })
  function Player({ open }) {
    React.useEffect(() => {
      mounts++
      return () => {
        unmounts++
      }
    }, [])
    return React.createElement('div', { 'data-player': true, 'data-open': open })
  }
  const playerModule = { __esModule: true, default: Player }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/lyrics/fullscreen-player.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    {
      exports,
      require(name) {
        if (name === 'react') return React
        if (name === 'react/jsx-runtime') return jsx
        assert.equal(name, './lyrics-view')
        loads++
        return delayed ? gate : playerModule
      }
    }
  )
  const root = createRoot(document.getElementById('root'))
  const onExitComplete = () => {
    exits++
  }
  t.after(async () => {
    await act(async () => root.unmount())
    Object.assign(globalThis, previous)
    dom.window.close()
  })
  return {
    render: (open) =>
      act(async () =>
        root.render(
          React.createElement(exports.FullscreenPlayer, {
            open,
            onExitComplete,
            onClose() {},
            onQueue() {}
          })
        )
      ),
    release: () => act(async () => release(playerModule)),
    counts: () => ({ mounts, unmounts, loads, exits }),
    player: () => document.querySelector('[data-player]')
  }
}

test('fullscreen loads on first open and keeps exactly one mounted player across repeated closes', async (t) => {
  const h = await harness(t)
  await h.render(false)
  assert.deepEqual(h.counts(), { mounts: 0, unmounts: 0, loads: 0, exits: 0 })
  await h.render(true)
  const player = h.player()
  assert.ok(player)
  for (let i = 0; i < 4; i++) {
    await h.render(false)
    assert.equal(h.player(), player)
    assert.equal(player.dataset.open, 'false')
    await h.render(true)
    assert.equal(h.player(), player)
  }
  assert.deepEqual(h.counts(), { mounts: 1, unmounts: 0, loads: 1, exits: 0 })
})

test('closing while the first module is loading releases fullscreen ownership without waiting for it', async (t) => {
  const h = await harness(t, true)
  await h.render(true)
  assert.equal(h.player(), null)
  await h.render(false)
  assert.equal(h.counts().exits, 1)
  await h.release()
  assert.equal(h.player().dataset.open, 'false')
  assert.equal(h.counts().mounts, 1)
})
