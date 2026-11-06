import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

test('panel mounts only the active page and keeps pages that opt in mounted', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const modules = {
    'react/jsx-runtime': jsx,
    '@/lib/utils': { cn: (...values) => values.filter(Boolean).join(' ') }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/components/ui/panel.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  const pages = [
    {
      value: 'lyrics',
      keepMounted: true,
      content: React.createElement('p', { id: 'lyrics' }, 'lyrics')
    },
    {
      value: 'information',
      content: React.createElement('p', { id: 'information' }, 'information')
    }
  ]
  const root = createRoot(document.getElementById('root'))
  const render = (value) =>
    act(async () => root.render(React.createElement(exports.Panel, { value, pages })))
  try {
    await render('lyrics')
    assert.ok(document.getElementById('lyrics'))
    assert.equal(document.getElementById('information'), null)

    await render('information')
    assert.ok(document.getElementById('lyrics'))
    assert.ok(document.getElementById('information'))

    await render('lyrics')
    assert.ok(document.getElementById('lyrics'))
    assert.equal(document.getElementById('information'), null)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
