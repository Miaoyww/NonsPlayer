import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('library preview renders through account reset without throwing or showing another source with the same ID', async () => {
  const library = {}
  const modules = {
    './library-preview': {
      peekLibraryPreview: (key) => ({
        summary: { title: JSON.parse(key).source },
        collections: { items: [JSON.parse(key).source] }
      })
    },
    '@/lib/runtime-cache': { invalidateNativeCache() {} },
    '@/features/music/queries': { clearContinuations() {} }
  }
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/library/library-api.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    { exports: library, require: (name) => modules[name] ?? {} }
  )
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    IS_REACT_ACT_ENVIRONMENT: true
  })
  const root = createRoot(document.getElementById('root'))
  const Preview = () =>
    React.createElement('p', {}, library.peekMusicLibrary('7', 'netease')?.title ?? 'loading')
  try {
    library.setLibraryAccount({ source: 'netease', id: '7' })
    await act(async () => root.render(React.createElement(Preview)))
    assert.equal(document.querySelector('p').textContent, 'netease')
    library.setLibraryAccount(null)
    await act(async () => root.render(React.createElement(Preview)))
    assert.equal(document.querySelector('p').textContent, 'loading')
    library.setLibraryAccount({ source: 'qqmusic', id: '7' })
    await act(async () => root.render(React.createElement(Preview)))
    assert.equal(document.querySelector('p').textContent, 'loading')
    assert.equal(library.peekLibraryCollections('7', 'netease'), undefined)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
