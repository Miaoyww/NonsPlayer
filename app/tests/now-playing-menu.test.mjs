import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness() {
  const calls = []
  const requestedSources = []
  const account = {
    profile: { reference: { source: 'netease', id: '1' } },
    likesReady: true,
    likedIds: new Set(['12']),
    favoriteStatus: (id) =>
      !account.likesReady ? 'unknown' : account.likedIds.has(id) ? 'liked' : 'unliked',
    reloadLikes: () => calls.push(['reloadLikes']),
    pendingLikes: new Set(),
    toggleLike: (id) => calls.push(['like', id])
  }
  const jsx = (type, props) => ({ type, props })
  const Menu = Object.fromEntries(
    [
      'Root',
      'Trigger',
      'Portal',
      'Positioner',
      'Popup',
      'Item',
      'Separator',
      'Group',
      'GroupLabel'
    ].map((name) => [name, name])
  )
  const modules = {
    react: { useState: (initial) => [initial, (value) => calls.push(['sources', value])] },
    './song-source-dialog': { SongSourceDialog: 'SongSourceDialog' },
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({
        statuses: [
          {
            enabled: true,
            descriptor: { source: 'netease', capabilities: ['playlistWrite', 'favorites'] }
          }
        ]
      })
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@base-ui/react/menu': { Menu },
    'lucide-react': {},
    '@/components/music/action-button': { ActionButton: 'button' },
    '@/components/music/song-actions': {
      songLink: (track) => `https://music.163.com/#/song?id=${track.source.track.reference.id}`,
      songCopyName: (track) => `${track.title} - ${track.artist}`,
      useSongActions: () => ({
        songLink: (track) =>
          track.source.kind === 'online'
            ? `https://music.163.com/#/song?id=${track.source.track.reference.id}`
            : '',
        run: (operation) => operation(),
        copy: (value) => calls.push(['copy', value]),
        details: (track) => calls.push(['details', track.key]),
        collect: (track) => calls.push(['collect', track.key])
      })
    },
    '@/features/account/account': {
      useAccount: (source) => {
        requestedSources.push(source)
        return account
      }
    },
    '@tauri-apps/api/core': { isTauri: () => true }
  }
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(
      new URL('../src/features/playback/now-playing/now-playing-menu.tsx', import.meta.url),
      'utf8'
    ),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, {
    exports,
    require: (name) => (name === '@/features/music/references' ? musicReference : modules[name])
  })
  const flatten = (node) =>
    Array.isArray(node)
      ? node.flatMap(flatten)
      : node && typeof node === 'object'
        ? [node, ...flatten(node.props?.children)]
        : []
  return {
    calls,
    requestedSources,
    account,
    render(track, options = {}) {
      const nodes = flatten(
        exports.NowPlayingMenu({
          track,
          loading: false,
          onRefresh: () => calls.push(['refresh']),
          ...options
        })
      )
      return {
        root: nodes.find((node) => node.type === 'Root'),
        item: (label) =>
          nodes.find((node) => node.type === 'Item' && node.props.children?.includes(label))
      }
    }
  }
}
const stream = {
  key: 'netease:12',
  title: 'Song',
  artist: 'Artist',
  source: { kind: 'online', track: { reference: { source: 'netease', kind: 'track', id: '12' } } }
}
const local = {
  ...stream,
  key: 'local:12',
  source: { kind: 'local', path: 'C:/Music/song.flac', neteaseId: 12 }
}

test('stream menu uses existing favorite, collection and copy actions without lyric or detail entries', () => {
  const app = harness()
  const menu = app.render(stream)
  for (const label of ['取消收藏歌曲', '收藏到歌单', '复制歌曲链接', '刷新歌词']) {
    assert.equal(menu.item(label).props.disabled ?? false, false)
    menu.item(label).props.onClick()
  }
  assert.equal(menu.item('歌词来源'), undefined)
  assert.equal(menu.item('歌曲详情'), undefined)
  assert.deepEqual(app.calls, [
    ['like', '12'],
    ['collect', stream.key],
    ['copy', 'https://music.163.com/#/song?id=12'],
    ['refresh']
  ])
})

test('bound local songs expose local actions and never use their lyric association as song identity', () => {
  const app = harness()
  const menu = app.render(local, { source: '本地' })
  assert.equal(menu.item('取消收藏歌曲'), undefined)
  assert.equal(menu.item('复制歌曲链接'), undefined)
  assert.equal(menu.item('歌曲来源'), undefined)
  for (const label of ['添加到本地歌单', '复制歌曲名称', '复制文件路径'])
    menu.item(label).props.onClick()
  assert.deepEqual(app.calls, [
    ['collect', local.key],
    ['copy', 'Song - Artist'],
    ['copy', local.source.path]
  ])
})

test('favorite respects account readiness and pending writes; lyric refresh cannot be repeated during loading', () => {
  const app = harness()
  app.account.pendingLikes.add('12')
  assert.equal(app.render(stream).item('取消收藏歌曲').props.disabled, true)
  app.account.pendingLikes.clear()
  app.account.profile = undefined
  assert.equal(app.render(stream).item('收藏歌曲'), undefined)
  const loading = app.render(stream, { loading: true })
  assert.equal(loading.item('正在获取歌词…').props.disabled, true)
  assert.equal(app.render(stream).item('刷新歌词').props.disabled, false)
})

test('MV unlinking is only offered for a bound track and refreshes when the menu opens', () => {
  const app = harness()
  assert.equal(app.render(stream).item('解除 MV 绑定'), undefined)
  let opened = 0
  const linked = app.render(stream, {
    mvLinked: true,
    onMvMenuOpen: () => opened++,
    onUnlinkMv: () => app.calls.push(['unlink'])
  })
  linked.root.props.onOpenChange(true)
  linked.item('解除 MV 绑定').props.onClick()
  assert.equal(opened, 1)
  assert.deepEqual(app.calls, [['unlink']])
  assert.equal(
    app
      .render(stream, { mvLinked: true, mvBusy: true, onUnlinkMv: () => {} })
      .item('正在解除 MV 绑定…').props.disabled,
    true
  )
})

test('song menu reads the track source and exposes unknown and stale favorite recovery', () => {
  const app = harness()
  const qq = {
    ...stream,
    source: { kind: 'online', track: { reference: { source: 'qqmusic', kind: 'track', id: '12' } } }
  }
  app.account.profile.reference.source = 'qqmusic'
  app.account.likesReady = false
  let menu = app.render(qq)
  assert.equal(app.requestedSources.at(-1), 'qqmusic')
  assert.equal(menu.item('收藏状态待确认').props.disabled, true)
  assert.equal(menu.item('收藏歌曲'), undefined)
  app.account.likesRefreshing = true
  app.account.likesStale = true
  assert.equal(app.render(qq).item('收藏状态待确认').props.title, '正在读取收藏状态')
  app.account.likesRefreshing = false
  app.account.likesError = 'offline'
  assert.match(app.render(qq).item('收藏状态待确认').props.title, /待确认.*读取失败.*重新读取/)
  menu.item('重新读取收藏状态').props.onClick()
  assert.deepEqual(app.calls, [['reloadLikes']])
  app.account.likesReady = true
  app.account.likesStale = true
  app.account.likesError = 'network unavailable'
  menu = app.render(qq)
  assert.equal(menu.item('取消收藏歌曲').props.disabled, false)
  assert.match(menu.item('取消收藏歌曲').props.title, /读取失败/)
  assert.ok(menu.item('重新读取收藏状态'))
})
