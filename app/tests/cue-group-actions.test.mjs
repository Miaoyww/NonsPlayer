import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'
import React from 'react'
import * as jsx from 'react/jsx-runtime'
import { renderToStaticMarkup } from 'react-dom/server'

function load(file, modules) {
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
  key: `cue:${number}`,
  title: `Track ${number}`,
  album: 'Album',
  source: { kind: 'local', cue: { sheetPath: 'disc.cue' } }
}))

test('CUE parent menu sends the entire ordered group to play, append, collect and removal', async () => {
  const items = [],
    calls = [],
    removed = []
  const Wrapper = ({ children }) => React.createElement('div', {}, children)
  const actions = {
    run: (work) => work(),
    collectGroup: (value) => calls.push(['collect', value.map((track) => track.key)]),
    copy: (value) => calls.push(['copy', value])
  }
  const { CueContextMenu } = load('../src/features/local/cue-context-menu.tsx', {
    'react/jsx-runtime': jsx,
    '@base-ui/react/context-menu': {
      ContextMenu: {
        Root: Wrapper,
        Trigger: Wrapper,
        Portal: Wrapper,
        Positioner: Wrapper,
        Popup: Wrapper,
        Separator: () => null,
        Item: ({ children, ...props }) => {
          items.push({ children, ...props })
          return React.createElement('span', {}, children)
        }
      }
    },
    'lucide-react': Object.fromEntries(
      ['Copy', 'FolderPlus', 'ListPlus', 'Play', 'Trash2'].map((key) => [key, () => null])
    ),
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': {
      nativeCall: async (command, args) => calls.push([command, Array.from(args.keys)])
    },
    '@/components/music/song-actions': { useSongActions: () => actions }
  })
  renderToStaticMarkup(
    React.createElement(
      CueContextMenu,
      {
        entries: tracks.map((track, index) => ({ track, index: index + 10 })),
        busy: false,
        onPlay: () => calls.push(['play']),
        onRemove: async (track, index) => removed.push([track.key, index]),
        render: React.createElement('div')
      },
      'parent'
    )
  )
  for (const item of items) await item.onClick()
  assert.deepEqual(calls, [
    ['play'],
    ['append_queue', ['cue:1', 'cue:2', 'cue:3', 'cue:4']],
    ['collect', ['cue:1', 'cue:2', 'cue:3', 'cue:4']],
    ['copy', 'disc.cue']
  ])
  assert.deepEqual(removed, [
    ['cue:1', 10],
    ['cue:2', 11],
    ['cue:3', 12],
    ['cue:4', 13]
  ])
})

test('local playlist picker adds every CUE track in order through the existing invalidating mutation', async () => {
  const calls = [],
    buttons = []
  let closed = false
  const Wrapper = ({ children }) => React.createElement('div', {}, children)
  const { LocalPlaylistPicker } = load('../src/features/local/local-playlist-picker.tsx', {
    react: { useState: () => ['', () => {}], useEffect: () => {}, useCallback: (fn) => fn },
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
      ['Dialog', 'DialogContent', 'DialogHeader', 'DialogTitle', 'DialogDescription'].map((key) => [
        key,
        Wrapper
      ])
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
  })
  renderToStaticMarkup(
    LocalPlaylistPicker({
      tracks,
      onClose: () => {
        closed = true
      },
      onNotice: () => {}
    })
  )
  buttons[0]()
  await new Promise((resolve) => setImmediate(resolve))
  assert.deepEqual(
    calls,
    tracks.map((track) => ['add_local_playlist_track', 'playlist', track.key])
  )
  assert.equal(closed, true)
})
