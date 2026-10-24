import { createRuntimeCache } from './helpers/runtime-cache.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'
import React from 'react'
import * as jsx from 'react/jsx-runtime'
import { renderToStaticMarkup } from 'react-dom/server'

function load(file, modules) {
  modules = { '@/lib/runtime-cache': createRuntimeCache(), ...modules }
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(file, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  return exports
}
const tracks = [1, 2, 3, 4].map((number) => ({
  key: `track:${number}`,
  title: `Track ${number}`,
  album: 'Album',
  source: { kind: 'local' }
}))

test('aggregate playlist picker atomically adds the selected tracks', async () => {
  const calls = [],
    buttons = []
  let closed = false
  const Wrapper = ({ children }) => React.createElement('div', {}, children)
  const { AggregatePlaylistPicker } = load(
    '../src/features/aggregate/aggregate-playlist-picker.tsx',
    {
      './playlist-targets': {
        addToAggregatePlaylist: async (id, tracks) =>
          calls.push({ operation: 'add', id, keys: tracks.map((track) => track.key) })
      },
      '@/features/library/create-playlist': { CreatePlaylist: () => null },
      '@/features/library/collection-actions': { useCollectionActions: () => ({ changed() {} }) },
      '@tanstack/react-virtual': {
        useVirtualizer: ({ count }) => ({
          getTotalSize: () => count * 64,
          getVirtualItems: () =>
            Array.from({ length: count }, (_, index) => ({ index, start: index * 64, size: 64 }))
        })
      },
      '@/features/music/references': {},
      '@/features/music/use-music-sources': {
        useMusicSources: () => ({ statuses: [], revision: 0 })
      },
      './aggregate-library': { useAggregateReplacement: () => () => undefined },
      '@/features/aggregate/aggregate-library': {
        useAggregateLibrary: () => ({ reload: async () => {} }),
        writeAggregate: async (request) => calls.push(request)
      },
      react: {
        useRef: (value) => ({ current: value }),
        useState: (value) => [value, () => {}],
        useEffect: () => {},
        useCallback: (fn) => fn
      },
      'react/jsx-runtime': jsx,
      '@/components/error-notice': { ErrorNotice: () => null },
      '@/lib/player': {
        nativeCall: async (command, args) => calls.push([command, args.id, args.key]),
        errorText: String
      },
      '@/lib/use-paged-list': {
        usePagedList: () => ({
          items: [{ id: 'playlist', name: 'Playlist', trackCount: 0 }],
          more: false
        })
      },
      '@/components/ui/dialog': Object.fromEntries(
        ['Dialog', 'DialogContent', 'DialogHeader', 'DialogTitle', 'DialogDescription'].map(
          (key) => [key, Wrapper]
        )
      ),
      '@/components/ui/input': { Input: () => null },
      '@/components/music/action-button': {
        ActionButton: ({ children, onClick }) => {
          if (onClick) buttons.push(onClick)
          return React.createElement('span', {}, children)
        }
      },
      '@/components/music/cover': { Cover: () => null },
      '@/components/music/infinite-load': { InfiniteLoad: () => null }
    }
  )
  renderToStaticMarkup(
    AggregatePlaylistPicker({
      tracks,
      onClose: () => {
        closed = true
      },
      onNotice: () => {}
    })
  )
  buttons[0]()
  await new Promise((resolve) => setImmediate(resolve))
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
    { operation: 'add', id: 'playlist', keys: tracks.map((track) => track.key) }
  ])
  assert.equal(closed, true)
})
