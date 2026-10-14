import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import { createCipheriv, createHash, webcrypto } from 'node:crypto'
import ts from 'typescript'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import { Progress as ProgressPrimitive } from '@base-ui/react/progress'
import { cn } from 'cn'
import { ecb } from '../../plugins/netease-download/node_modules/@noble/ciphers/aes.js'
import { md5 } from '../../plugins/netease-download/node_modules/@noble/hashes/legacy.js'
import { bytesToHex } from '../../plugins/netease-download/node_modules/@noble/hashes/utils.js'

function load(path, modules = {}, globals = {}) {
  modules = { '@/features/music/cover-image': coverImage, ...modules }
  const exports = {}
  const { outputText } = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  runInNewContext(outputText, {
    exports,
    TextEncoder,
    URL,
    URLSearchParams,
    Uint8Array,
    atob,
    setInterval: () => 1,
    clearInterval: () => {},
    setTimeout: (fn) => setTimeout(fn, 0),
    require: (name) => {
      assert.ok(name in modules, name)
      return modules[name]
    },
    ...globals
  })
  return exports
}
const protocol = load('../../plugins/netease-download/frontend/protocol.ts', {
  '@noble/ciphers/aes.js': { ecb },
  '@noble/hashes/legacy.js': { md5 },
  '@noble/hashes/utils.js': { bytesToHex }
})
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
    source: { kind: 'netease', id: 12 }
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
    source: { kind: 'netease', id: 12 }
  }
  assert.equal(menus.songMenuContributions(manifest, song).length, 1)
  const local = { ...song, source: { kind: 'local', path: 'C:/private/song.mp3', neteaseId: 12 } }
  assert.equal(menus.songMenuContributions(manifest, local).length, 0)
  assert.deepEqual(JSON.parse(JSON.stringify(menus.publicMenuSong(local).source)), {
    kind: 'local'
  })
  assert.ok(!JSON.stringify(menus.publicMenuSong(local)).includes('private'))
})
test('EAPI matches independent Node AES/MD5 implementation and cookie merging handles rotation', () => {
  const path = '/api/login/qrcode/unikey',
    data = { type: 3 }
  const text = JSON.stringify(data)
  const hash = createHash('md5').update(`nobody${path}use${text}md5forencrypt`).digest('hex')
  const cipher = createCipheriv('aes-128-ecb', Buffer.from('e82ckenh8dichen8'), null)
  const expected = Buffer.concat([
    cipher.update(`${path}-36cd479b6b5-${text}-36cd479b6b5-${hash}`),
    cipher.final()
  ])
    .toString('hex')
    .toUpperCase()
  assert.equal(protocol.encryptRequest(path, data), expected)
  assert.equal(
    protocol.mergeCookies('MUSIC_U=old; __csrf=old', [
      'MUSIC_U=new; Path=/; HttpOnly',
      '__csrf=; Max-Age=0'
    ]),
    'MUSIC_U=new'
  )
})
test('quality fallback respects the minimum and strict quality; auth and trial failures stop immediately', async () => {
  assert.deepEqual(Array.from(protocol.qualityCandidates('lossless', true, 'higher')), [
    'lossless',
    'exhigh',
    'higher'
  ])
  assert.throws(() => protocol.qualityCandidates('standard', true, 'lossless'), /最低/)
  const replies = [
    { code: 200, data: { url: null } },
    { code: 200, data: { url: 'https://m801.music.126.net/song.mp3', level: 'exhigh', size: 100 } }
  ]
  let calls = 0
  const client = {
    call: async () => ({ status: 200, body: JSON.stringify(replies[calls++]), cookies: [] })
  }
  const result = await protocol.resolveResource(
    client,
    12,
    'lossless',
    true,
    'standard',
    'MUSIC_U=plugin-only'
  )
  assert.equal(result.quality, 'exhigh')
  assert.equal(calls, 2)
  calls = 0
  await assert.rejects(
    protocol.resolveResource(client, 12, 'lossless', false, 'standard', ''),
    /不符合/
  )
  assert.equal(calls, 1)
  for (const reply of [
    { code: 301 },
    { code: 200, data: { url: 'https://example.org/song.mp3', freeTrialInfo: {} } }
  ]) {
    calls = 0
    const failing = {
      call: async () => {
        calls++
        return { status: 200, body: JSON.stringify(reply) }
      }
    }
    await assert.rejects(
      protocol.resolveResource(failing, 12, 'lossless', true, 'standard', ''),
      /登录|试听/
    )
    assert.equal(calls, 1)
  }
})
test('audio headers determine extensions and reject HTML; file names cannot escape directories', () => {
  for (const [header, format] of [
    ['fLaC', 'flac'],
    ['ID3', 'mp3'],
    ['OggS', 'ogg'],
    ['0000ftyp', 'm4a'],
    ['RIFF0000WAVE', 'wav']
  ])
    assert.equal(protocol.audioFormat(new TextEncoder().encode(header)), format)
  assert.equal(
    protocol.audioFormat(new TextEncoder().encode('<html>login required</html>')),
    undefined
  )
  assert.ok(!protocol.filename('../song:*?', 'Singer/../../', 12).includes('/'))
})

async function until(condition) {
  for (let i = 0; i < 100; i++) {
    if (condition()) return
    await new Promise((r) => setTimeout(r, 2))
  }
  assert.fail('download did not reach expected state')
}
test('menu download persists settings snapshot, resolves once, streams and publishes without playback APIs', async () => {
  const runtime = load('../../plugins/netease-download/frontend/runtime.ts', {
    './protocol': protocol
  })
  const config = {
    quality: 'lossless',
    minimumQuality: 'standard',
    allowFallback: true,
    directory: 'dir-one'
  }
  const calls = []
  const client = {
    openPage: () => {},
    openConfiguration: () => {},
    call: async (op, args) => {
      calls.push({ op, args })
      if (op === 'netease.account-credentials')
        return { cookie: 'MUSIC_U=host-cookie', generation: 1 }
      if (op === 'storage.get') return null
      if (op === 'config.get') return { values: config }
      if (op === 'files.roots') return [{ id: 'dir-one', writable: true }]
      if (op === 'http.request')
        return {
          status: 200,
          body: JSON.stringify({
            code: 200,
            data: { url: 'https://m801.music.126.net/song', level: 'lossless', size: 100 }
          })
        }
      if (op === 'transfers.start') return { id: 7 }
      if (op === 'transfers.get') return { state: 'completed', bytes: 100 }
      if (op === 'files.open') return { handle: 8 }
      if (op === 'files.read') return { data: Buffer.from('fLaC').toString('base64') }
      return null
    }
  }
  const dispose = await runtime.activate(client)
  const song = {
    key: 'ncm:12',
    title: 'Title',
    artist: 'Singer',
    album: '',
    cover: '',
    durationMs: 100,
    source: { kind: 'netease', id: 12 }
  }
  await runtime.downloadSong(song)
  config.quality = 'standard'
  await runtime.downloadSong(song)
  await until(() => runtime.snapshot().tasks[0].state === 'completed')
  assert.equal(runtime.snapshot().tasks.length, 1)
  assert.equal(runtime.snapshot().tasks[0].options.quality, 'lossless')
  assert.equal(calls.filter((c) => c.op === 'transfers.start').length, 1)
  assert.equal(calls.find((c) => c.op === 'files.publish').args.to, 'Singer - Title [12].flac')
  assert.ok(!calls.some((c) => c.op === 'netease.play-song' || c.op.startsWith('player.')))
  assert.ok(!JSON.stringify(calls.filter((c) => c.op === 'storage.set')).includes('host-cookie'))
  assert.ok(!JSON.stringify(calls.filter((c) => c.op === 'storage.set')).includes('m801.music'))
  dispose()
})
test('missing directory retains the selected song and opens configuration; linked local songs are rejected', async () => {
  const runtime = load('../../plugins/netease-download/frontend/runtime.ts', {
    './protocol': protocol
  })
  let opened = 0
  const client = {
    openPage: () => {},
    openConfiguration: () => {
      opened++
    },
    call: async (op) => {
      if (op === 'config.get')
        return {
          values: {
            quality: 'lossless',
            minimumQuality: 'standard',
            allowFallback: true,
            directory: ''
          }
        }
      if (op === 'files.roots') return []
      if (op === 'netease.account-credentials') return { cookie: null, generation: 1 }
      return null
    }
  }
  const dispose = await runtime.activate(client)
  const song = {
    key: 'ncm:12',
    title: 'Title',
    artist: 'Singer',
    album: '',
    source: { kind: 'netease', id: 12 }
  }
  await runtime.downloadSong(song)
  await until(() => runtime.snapshot().tasks[0].state === 'setup')
  assert.equal(opened, 1)
  assert.equal(runtime.snapshot().tasks[0].song.source.id, 12)
  await assert.rejects(
    runtime.downloadSong({ ...song, source: { kind: 'local', neteaseId: 12 } }),
    /只支持/
  )
  dispose()
})

test('actual contributed menu invokes the plugin with a scoped client and rejects clicks after disable', async () => {
  const song = {
    key: 'ncm:12',
    title: 'Title',
    artist: 'Singer',
    album: '',
    cover: '',
    source: { kind: 'netease', id: 12 }
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
                  assert.equal(selected.source.id, 12)
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

test('download management renders shared progress, search and individual controls without plugin login', async () => {
  const dom = new JSDOM('<div id="root"></div>', { url: 'http://localhost/' })
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const root = createRoot(document.getElementById('root'))
  let configurations = 0
  const view = {
    message: '已加入下载队列',
    tasks: [
      {
        id: 1,
        song: { title: 'Title', artist: 'Singer' },
        state: 'setup',
        bytes: 0,
        error: '请选择并授权保存目录'
      },
      {
        id: 2,
        song: { title: 'Downloading', artist: 'Singer' },
        state: 'running',
        bytes: 10,
        total: 40
      }
    ]
  }
  const runtime = {
    snapshot: () => view,
    subscribe: () => () => {},
    configure: () => {
      configurations++
    },
    activate: () => {},
    downloadSong: () => {},
    checkLogin: async () => 801,
    createLogin: async () => 'key',
    logout: async () => {},
    retryTask: async () => {},
    cancelTask: async () => {},
    pauseTask: async () => {},
    resumeTask: async () => {},
    deleteTask: async () => {},
    openDirectory: async () => {}
  }
  const module = load('../../plugins/netease-download/frontend/index.tsx', {
    react: React,
    'react/jsx-runtime': jsx,
    './runtime': runtime,
    './protocol': protocol,
    qrcode: { default: { toDataURL: async () => 'data:image/png;base64,' } },
    '@app/plugin-sdk': {
      Progress: load('../src/components/ui/progress.tsx', {
        'react/jsx-runtime': jsx,
        '@base-ui/react/progress': { Progress: ProgressPrimitive },
        cn: { cn }
      }).Progress,
      Input: (props) => React.createElement('input', props),
      Icon: () => null,
      SongIdentity: ({ song }) => React.createElement('span', null, `${song.title} ${song.artist}`),
      DownloadedSongList: () => React.createElement('div', null, 'completed-list'),
      Tabs: ({ children }) => React.createElement('div', null, children),
      TabsList: ({ children }) => React.createElement('div', null, children),
      TabsTab: ({ children }) => React.createElement('span', null, children),
      TabsPanels: ({ children }) => React.createElement('div', null, children),
      TabsPanel: ({ children }) => React.createElement('div', null, children),
      Button: ({ variant: _, size: __, ...props }) => React.createElement('button', props)
    }
  })
  try {
    await React.act(async () => root.render(React.createElement(module.DownloadsPage)))
    assert.equal(document.querySelectorAll('input').length, 1)
    assert.equal(document.querySelector('input').getAttribute('type'), 'search')
    assert.match(document.body.textContent, /Title.*需要设置/)
    assert.match(document.body.textContent, /请选择并授权保存目录/)
    const progress = document.querySelector(
      '[data-slot="progress"][aria-label="Downloading 下载进度"]'
    )
    assert.equal(progress.getAttribute('role'), 'progressbar')
    assert.equal(progress.getAttribute('aria-label'), 'Downloading 下载进度')
    assert.ok(progress.querySelector('[data-slot="progress-indicator"]'))
    assert.equal(document.querySelectorAll('progress').length, 0)
    const settings = [...document.querySelectorAll('button')].find(
      (button) => button.getAttribute('aria-label') === '下载设置'
    )
    await React.act(async () => settings.click())
    assert.equal(configurations, 1)
    assert.ok(!document.body.textContent.includes('扫码登录'))
    assert.ok(document.querySelector('[aria-label="暂停下载 Downloading"]'))
    assert.ok(document.querySelector('[aria-label="删除下载任务 Downloading"]'))
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
  }
})

test('task pause and resume use the same transfer; delete cancels it without storing host credentials', async () => {
  const runtime = load('../../plugins/netease-download/frontend/runtime.ts', {
    './protocol': protocol
  })
  const calls = []
  let transferState = 'running'
  const client = {
    openPage() {},
    openConfiguration() {},
    async call(op, args) {
      calls.push({ op, args })
      if (op === 'storage.get') return null
      if (op === 'config.get')
        return {
          values: {
            quality: 'lossless',
            minimumQuality: 'standard',
            allowFallback: true,
            directory: 'dir-one'
          }
        }
      if (op === 'files.roots') return [{ id: 'dir-one', writable: true }]
      if (op === 'netease.account-credentials')
        return { cookie: 'MUSIC_U=private-cookie', generation: 1 }
      if (op === 'http.request')
        return {
          status: 200,
          body: JSON.stringify({
            code: 200,
            data: { url: 'https://m801.music.126.net/song', level: 'lossless', size: 100 }
          })
        }
      if (op === 'transfers.start') return { id: 7 }
      if (op === 'transfers.get') return { state: transferState, bytes: 20 }
      if (op === 'transfers.pause') transferState = 'paused'
      if (op === 'transfers.resume') transferState = 'running'
      if (op === 'transfers.cancel') transferState = 'cancelled'
      return null
    }
  }
  const dispose = await runtime.activate(client)
  await runtime.downloadSong({
    key: 'ncm:12',
    title: 'Title',
    artist: 'Singer',
    album: 'Album',
    cover: 'https://example.org/cover',
    durationMs: 100,
    source: { kind: 'netease', id: 12 }
  })
  await until(() => runtime.snapshot().tasks[0].state === 'running')
  const id = runtime.snapshot().tasks[0].id
  await runtime.pauseTask(id)
  assert.equal(runtime.snapshot().tasks[0].state, 'paused')
  await runtime.resumeTask(id)
  assert.equal(runtime.snapshot().tasks[0].state, 'running')
  assert.equal(calls.filter((c) => c.op === 'transfers.start').length, 1)
  await runtime.deleteTask(id)
  assert.equal(runtime.snapshot().tasks.length, 0)
  assert.ok(calls.some((c) => c.op === 'transfers.cancel' && c.args.id === 7))
  assert.ok(!JSON.stringify(calls.filter((c) => c.op === 'storage.set')).includes('private-cookie'))
  dispose()
})

test('missing host login retains the task and never starts a transfer or separate login', async () => {
  const runtime = load('../../plugins/netease-download/frontend/runtime.ts', {
    './protocol': protocol
  })
  const calls = []
  const dispose = await runtime.activate({
    openPage() {},
    openConfiguration() {},
    async call(op, args) {
      calls.push({ op, args })
      if (op === 'storage.get') return null
      if (op === 'config.get')
        return {
          values: {
            quality: 'lossless',
            minimumQuality: 'standard',
            allowFallback: true,
            directory: 'dir-one'
          }
        }
      if (op === 'files.roots') return [{ id: 'dir-one', writable: true }]
      if (op === 'netease.account-credentials') return { cookie: null, generation: 2 }
      return null
    }
  })
  await runtime.downloadSong({
    key: 'ncm:12',
    title: 'Title',
    artist: 'Singer',
    album: '',
    cover: '',
    durationMs: 100,
    source: { kind: 'netease', id: 12 }
  })
  await until(() => runtime.snapshot().tasks[0].state === 'setup')
  assert.match(runtime.snapshot().tasks[0].error, /NonsPlayer 中登录/)
  assert.ok(!calls.some((c) => c.op === 'transfers.start' || c.op === 'http.request'))
  dispose()
})
