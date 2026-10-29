import { errorNoticeModule } from './helpers/error-notice.mjs'
import { musicReference } from './helpers/music-reference.mjs'
import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import { createHash, webcrypto } from 'node:crypto'
import ts from 'typescript'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'

function load(path, modules = {}, globals = {}) {
  modules = {
    '@/features/music/cover-image': coverImage,
    ...modules
  }
  const exports = {}
  const { outputText } = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  runInNewContext(outputText, {
    exports,
    TextEncoder,
    TextDecoder,
    URL,
    URLSearchParams,
    Uint8Array,
    atob,
    setInterval: () => 1,
    clearInterval: () => {},
    setTimeout: (fn) => setTimeout(fn, 0),
    require: (name) => {
      if (name === '@/components/error-notice') return errorNoticeModule
      assert.ok(name === '@/features/music/references' || name in modules, name)
      return name === '@/features/music/references' ? musicReference : modules[name]
    },
    ...globals
  })
  return exports
}
const menus = load('../src/plugins/song-menu-context.ts')

test('completed songs resolve authorized local files, retain available rows and play through the scoped file capability', async () => {
  const dom = new JSDOM('<div id="root"></div>', { url: 'http://localhost/' })
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const root = createRoot(document.getElementById('root'))
  const calls = []
  let list
  const scope = {}
  const component = load(
    '../src/plugins/downloaded-song-list.tsx',
    {
      react: React,
      'react/jsx-runtime': jsx,
      '@/components/music/track-list': {
        TrackList: (props) => {
          list = props
          return null
        }
      },
      './scope': { useScope: () => scope },
      './sdk': {
        createPluginClient: () => ({
          call: async (operation, args) => {
            calls.push({ operation, args })
            if (operation === 'files.resolve') {
              if (args.path === 'missing.mp3') throw new Error('File missing')
              return { path: 'C:/Music/song.mp3' }
            }
          }
        })
      }
    },
    { crypto: webcrypto }
  )
  const song = {
    key: 'ncm:12',
    title: 'Title',
    artist: 'Artist',
    album: 'Album',
    cover: '',
    durationMs: 100,
    source: { kind: 'online', reference: { source: 'netease', kind: 'track', id: '12' } }
  }
  const songs = [
    { song, root: 'grant', path: 'song.mp3', completedAt: 1234, size: 1048576 },
    { song, root: 'grant', path: 'missing.mp3', size: 0 }
  ]
  try {
    await React.act(async () => {
      root.render(React.createElement(component.DownloadedSongList, { songs }))
      await new Promise((resolve) => setTimeout(resolve, 30))
    })
    await React.act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 30))
    })
    assert.equal(list.tracks.length, 1)
    assert.equal(
      list.tracks[0].key,
      `local:${createHash('sha256').update('C:/Music/song.mp3').digest('hex')}`
    )
    assert.equal(list.tracks[0].source.kind, 'local')
    assert.match(document.body.textContent, /1 首歌曲文件不可用/)
    assert.equal(list.extraColumns[1].render(null, 0), '1.0 MiB')
    await React.act(async () => list.onPlay(0))
    const playback = calls.find((call) => call.operation === 'files.play')
    assert.equal(playback.args.root, 'grant')
    assert.equal(playback.args.path, 'song.mp3')
    assert.equal(playback.args.track.source.path, 'C:/Music/song.mp3')
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
  }
})
test('song contributions match the stream source, never a linked local ID or lyric source', () => {
  const manifest = {
    contributes: { contextMenus: [{ target: 'song', source: 'netease', label: '下载' }] }
  }
  const song = {
    key: 'ncm:12',
    title: 'Title',
    artist: 'Artist',
    album: '',
    durationMs: 100,
    cover: 'https://example.org/cover',
    source: { kind: 'online', track: { reference: { source: 'netease', kind: 'track', id: '12' } } }
  }
  assert.equal(menus.songMenuContributions(manifest, song).length, 1)
  const local = { ...song, source: { kind: 'local', path: 'C:/private/song.mp3', neteaseId: 12 } }
  assert.equal(menus.songMenuContributions(manifest, local).length, 0)
  assert.deepEqual(JSON.parse(JSON.stringify(menus.publicMenuSong(local).source)), {
    kind: 'local'
  })
  assert.ok(!JSON.stringify(menus.publicMenuSong(local)).includes('private'))
})
test('actual contributed menu invokes the plugin with a scoped client and rejects clicks after disable', async () => {
  const song = {
    key: 'ncm:12',
    title: 'Title',
    artist: 'Singer',
    album: '',
    cover: '',
    source: { kind: 'online', track: { reference: { source: 'netease', kind: 'track', id: '12' } } }
  }
  const scope = {
    active: true,
    descriptor: {
      manifest: {
        id: 'download',
        contributes: {
          contextMenus: [
            {
              id: 'download',
              target: 'song',
              source: 'netease',
              label: '下载',
              export: 'downloadSong'
            }
          ]
        }
      }
    }
  }
  let invocations = 0,
    pending,
    notice
  const client = { call: async () => null }
  const jsx = (type, props) => ({ type, props })
  const module = load('../src/plugins/song-menu.tsx', {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@base-ui/react/context-menu': { ContextMenu: { Item: 'Item' } },
    '@tauri-apps/api/core': { isTauri: () => true },
    'lucide-react': { Download: 'Download', LayoutGrid: 'LayoutGrid' },
    './host': {
      usePlugins: () => ({
        loaded: new Map([
          [
            'download',
            {
              scope,
              module: {
                downloadSong: async (selected, sdk) => {
                  assert.equal(selected.source.reference.id, '12')
                  assert.equal(sdk, client)
                  invocations++
                  return '已加入下载队列'
                }
              }
            }
          ]
        ])
      })
    },
    './scope': {
      checkScope: (value) => {
        if (!value.active) throw new Error('插件已禁用')
      }
    },
    './sdk': { createPluginClient: () => client },
    './song-menu-context': menus,
    '@/features/workspace/music-navigation': { useMusicNavigation: () => ({ navigate: () => {} }) }
  })
  const render = module.PluginSongMenuItems({
    track: song,
    run: (work, feedback) => {
      pending = work().then((result) => {
        notice = feedback(result)
      })
    }
  })
  const item = render.props.children[0]
  assert.equal(item.props.children[1], '下载')
  assert.equal(item.props.children[0].type, 'LayoutGrid')
  assert.equal(item.props.children[0].props['aria-hidden'], 'true')
  const contribution = scope.descriptor.manifest.contributes.contextMenus[0]
  for (const [icon, expected] of [
    ['download', 'Download'],
    ['unknown-icon', 'LayoutGrid']
  ]) {
    contribution.icon = icon
    const next = module.PluginSongMenuItems({ track: song, run: () => {} })
    assert.equal(next.props.children[0].props.children[0].type, expected)
  }
  item.props.onClick()
  await pending
  assert.equal(invocations, 1)
  assert.equal(notice, '已加入下载队列')
  scope.active = false
  item.props.onClick()
  await assert.rejects(pending, /禁用/)
  assert.equal(module.PluginSongMenuItems({ track: song, run: () => {} }).props.children.length, 0)
})
