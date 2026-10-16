import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('list finder searches metadata, preserves duplicate positions, locates and routes shortcuts', async () => {
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
  let page = { view: 'collection' }
  const located = []
  let globalSearch = 0
  let inputChange
  const modules = {
    react: React,
    '@tanstack/react-virtual': {
      useVirtualizer: ({ count }) => ({
        getTotalSize: () => count * 112,
        measureElement() {},
        getVirtualItems: () =>
          Array.from({ length: count }, (_, index) => ({ index, key: index, start: index * 112 }))
      })
    },
    'react/jsx-runtime': jsx,
    'lucide-react': { Search: () => null, X: () => null },
    '@/components/ui/input': {
      Input: (props) => {
        inputChange = props.onChange
        return React.createElement('input', props)
      }
    },
    './action-button': {
      ActionButton: ({ variant, size, ...props }) => React.createElement('button', props)
    },
    './track-identity': { TrackIdentity: ({ track }) => React.createElement('p', {}, track.title) },
    './song-actions': {
      SongContextMenu: ({ render, children, onLocate }) =>
        React.cloneElement(
          render,
          {},
          children,
          React.createElement('button', { onClick: onLocate }, '定位到')
        )
    },
    '@/features/workspace/music-navigation': { useMusicNavigation: () => ({ page }) }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/components/music/list-find.tsx', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      },
      document: dom.window.document,
      window: dom.window,
      requestAnimationFrame: (callback) => {
        callback()
        return 1
      }
    }
  )
  const tracks = [
    { key: 'a', title: '白夜', artist: 'Artist', album: 'Album', aliases: ['Midnight'] },
    { key: 'a', title: '白夜', artist: 'Artist', album: 'Album', aliases: ['Midnight'] }
  ]
  function Source({ hidden }) {
    const element = React.useRef(null)
    exports.useListFind(
      React.useMemo(
        () => ({
          element,
          entries: tracks.map((track, index) => ({ track, index })),
          hasMore: true,
          play() {},
          locate: (index) => located.push(index)
        }),
        []
      )
    )
    return React.createElement(
      'div',
      {
        ref: (node) => {
          element.current = node
          if (node) node.getClientRects = () => (hidden ? [] : [{}])
        }
      },
      '列表'
    )
  }
  const root = createRoot(document.getElementById('root'))
  const render = async (hidden = false) =>
    act(() =>
      root.render(
        React.createElement(
          exports.ListFindProvider,
          { playerVisible: false, onGlobalSearch: () => globalSearch++ },
          React.createElement(Source, { hidden })
        )
      )
    )
  const key = async (options) => {
    const event = new dom.window.KeyboardEvent('keydown', {
      bubbles: true,
      cancelable: true,
      ...options
    })
    await act(() => document.dispatchEvent(event))
    return event
  }
  const find = () => document.querySelector('[aria-label="快捷搜索列表歌曲"][role="region"]')
  const search = async (value) => act(() => inputChange({ target: { value } }))

  try {
    await render()
    assert.equal((await key({ key: 'f', ctrlKey: true })).defaultPrevented, true)
    assert.ok(find())
    for (const value of ['midnight', 'ARTIST', 'album', '白夜']) {
      await search(value)
      assert.match(find().textContent, /找到 2 首歌曲/)
    }
    assert.match(find().textContent, /仅搜索已加载歌曲/)
    await act(() =>
      [...find().querySelectorAll('button')]
        .filter((button) => button.textContent === '定位到')[1]
        .click()
    )
    assert.deepEqual(located, [1])
    assert.equal(find(), null)
    await key({ key: 'f', ctrlKey: true })
    await key({ key: 'Escape' })
    assert.equal(find(), null)
    for (const view of ['discover', 'queue']) {
      page = { view }
      await render()
      await key({ key: 'f', ctrlKey: true })
      assert.equal(find(), null)
    }
    page = { view: 'artist' }
    await render(true)
    await key({ key: 'f', ctrlKey: true })
    const recorder = document.createElement('button')
    recorder.setAttribute('data-shortcut-recorder', '')
    document.body.append(recorder)
    const recordingKey = new dom.window.KeyboardEvent('keydown', {
      key: 'f',
      ctrlKey: true,
      bubbles: true,
      cancelable: true
    })
    await act(() => recorder.dispatchEvent(recordingKey))
    assert.equal(recordingKey.defaultPrevented, false)
    assert.equal(globalSearch, 3)
    assert.equal(find(), null)
  } finally {
    await act(() => root.unmount())
    Object.assign(globalThis, previous)
    dom.window.close()
  }
})
