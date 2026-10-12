import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('uniform details retain cursor and rows on failure, reload only their own source and discard late pages', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const events = new Map()
  const pending = []
  const calls = []
  let table
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (event, callback) => {
        events.set(event, callback)
        return () => events.delete(event)
      }
    },
    '@/components/music/music-page': {
      MusicPage: 'section',
      MusicPageHeader: ({ title }) => React.createElement('h1', {}, title)
    },
    '@/components/music/track-list': {
      TrackList: (props) => {
        table = props
        return React.createElement(
          'div',
          { id: 'tracks' },
          props.tracks.map((track) => track.key).join(',')
        )
      }
    },
    '@/components/music/infinite-load': {
      InfiniteLoad: ({ onLoad, error }) =>
        React.createElement('button', { onClick: onLoad }, error ?? '续载')
    },
    '@/lib/player': {
      errorText: String,
      usePlayer: () => ({ index: null, queue: [] }),
      nativeCall: async (...args) => {
        calls.push(args)
      }
    },
    './client': {
      musicClient: {
        query: (source, request) =>
          new Promise((resolve, reject) => pending.push({ source, request, resolve, reject })),
        queueTracks: async (source, items) => items.map((item) => ({ key: `${source}:${item.id}` }))
      }
    }
  }
  const exports = {}
  const code = ts.transpileModule(
    readFileSync(new URL('../src/features/music/entity-page.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  ).outputText
  runInNewContext(code, { exports, require: (name) => modules[name] })
  const root = createRoot(document.getElementById('root'))
  const collection = {
    id: 0,
    kind: 'album',
    name: 'Album',
    reference: { source: 'qqmusic', kind: 'album', id: 'album/雪:1' }
  }
  const page = (id, nextCursor) => ({ type: 'tracks', data: { items: [{ id }], nextCursor } })
  const click = () =>
    act(async () =>
      document
        .querySelector('button')
        .dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }))
    )
  try {
    await act(async () =>
      root.render(React.createElement(exports.default, { collection, onError() {} }))
    )
    assert.equal(pending[0].source, 'qqmusic')
    assert.equal(pending[0].request.entity.id, 'album/雪:1')
    await act(async () => pending[0].resolve(page('one', 'opaque:next')))
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:one')
    await click()
    assert.equal(pending[2].request.page.cursor, 'opaque:next')
    await act(async () => pending[2].reject(new Error('network failed')))
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:one')
    await click()
    assert.equal(pending[3].request.page.cursor, 'opaque:next')
    await act(async () => pending[3].resolve(page('two', null)))
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:one,qqmusic:two')
    await act(async () => table.onAppend({ key: 'qqmusic:two' }))
    assert.equal(calls[0][0], 'append_queue')
    await act(async () => events.get('music-account-changed')({ payload: 'netease' }))
    assert.equal(pending.length, 4)
    await act(async () => events.get('music-account-changed')({ payload: 'qqmusic' }))
    assert.equal(document.getElementById('tracks').textContent, '')
    assert.equal(pending[4].request.page.cursor, null)
    await act(async () => events.get('adapters-changed')({ payload: 'qqmusic' }))
    await act(async () => pending[4].resolve(page('stale', null)))
    assert.equal(document.getElementById('tracks').textContent, '')
    await act(async () => pending[6].resolve(page('fresh', null)))
    assert.equal(document.getElementById('tracks').textContent, 'qqmusic:fresh')
  } finally {
    await act(async () => root.unmount())
    assert.equal(events.size, 0)
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    dom.window.close()
  }
})
