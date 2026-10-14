import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness() {
  const calls = []
  let profile = { userId: 7 }
  const actions = {
    link: () => 'https://source.example/share',
    busy: false,
    play: (...args) => calls.push(['play', ...args]),
    copy: (item) => calls.push(['copy', item]),
    edit: (item) => calls.push(['edit', item]),
    remove: (item) => calls.push(['remove', item])
  }
  const menu = Object.fromEntries(
    ['Root', 'Trigger', 'Portal', 'Positioner', 'Popup', 'Item', 'Separator'].map((key) => [
      key,
      key
    ])
  )
  const jsx = (type, props) => ({ type, props })
  const modules = {
    '@/components/music/loading': { TextSkeleton: 'div' },
    react: {
      createContext: () => ({}),
      useContext: () => actions,
      cloneElement: (element, props, children) => ({
        ...element,
        props: { ...element.props, ...props, children }
      })
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@base-ui/react/context-menu': { ContextMenu: menu },
    '@tauri-apps/api/core': { isTauri: () => true },
    'lucide-react': {},
    '@/plugins/song-menu': { PluginSongMenuItems: () => null },
    '@/lib/player': {},
    '@/features/library/library-api': {},
    '@/components/ui/dialog': {},
    '@/components/ui/button': {},
    '@/features/account/account': { useAccount: () => ({ profile }) },
    '@/features/workspace/music-navigation': {}
  }
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(
      new URL('../src/features/library/collection-actions.tsx', import.meta.url),
      'utf8'
    ),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, { exports, require: (name) => modules[name] })
  function flatten(node) {
    return Array.isArray(node)
      ? node.flatMap(flatten)
      : node && typeof node === 'object'
        ? [node, ...flatten(node.props?.children)]
        : []
  }
  return {
    calls,
    logout: () => {
      profile = undefined
    },
    menu(item, busy = false, options = {}) {
      return flatten(
        exports.CollectionContextMenu({
          item,
          busy,
          render: jsx('article', {}),
          children: 'card',
          ...options
        })
      ).filter((node) => node.type === 'Item')
    }
  }
}
const playlist = { id: 12, kind: 'playlist', creatorId: 7, liked: false, name: 'Mine' }
test('playlist menus replace the queue or insert the whole collection next and expose management', () => {
  const app = harness(),
    items = app.menu(playlist)
  assert.equal(items.length, 5)
  items[0].props.onClick()
  items[1].props.onClick()
  items[2].props.onClick()
  assert.deepEqual(
    app.calls.map((call) => [call[0], call[1].id, call[2]]),
    [
      ['play', 12, undefined],
      ['play', 12, true],
      ['copy', 12, undefined]
    ]
  )
  assert.equal(items[3].props.disabled, undefined)
  assert.equal(items[4].props.disabled, undefined)
})
test("albums omit edit and delete; other people's and liked playlists cannot be managed", () => {
  const app = harness()
  assert.equal(app.menu({ ...playlist, kind: 'album' }).length, 3)
  for (const item of [
    { ...playlist, creatorId: 8 },
    { ...playlist, liked: true }
  ]) {
    assert.equal(app.menu(item).length, 3)
  }
  app.logout()
  assert.equal(app.menu(playlist).length, 3)
})
test('busy cards hide unavailable actions while share copying remains available', () => {
  const app = harness(),
    items = app.menu(playlist, true)
  assert.equal(items.length, 1)
  items[0].props.onClick()
  assert.equal(app.calls[0][0], 'copy')
})

test('song favorite action precedes the divider and unavailable remove actions are hidden', () => {
  const jsx = (type, props) => ({ type, props })
  const menu = Object.fromEntries(
    [
      'Root',
      'Trigger',
      'Portal',
      'Positioner',
      'Popup',
      'Item',
      'Separator',
      'SubmenuRoot',
      'SubmenuTrigger'
    ].map((key) => [key, key])
  )
  const modules = {
    '@/components/music/loading': { TextSkeleton: 'div' },
    react: {
      createContext: () => ({}),
      useContext: () => ({ songLink: () => 'https://source.example/share' })
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@base-ui/react/context-menu': { ContextMenu: menu },
    '@tauri-apps/api/core': { isTauri: () => true },
    'lucide-react': {},
    '@/plugins/song-menu': { PluginSongMenuItems: () => null },
    '@/lib/player': {},
    '@/components/animate-ui/components/base/dialog': {},
    '@/components/music/action-button': {},
    '@/features/library/playlist-picker': {},
    '@/features/account/account': {
      useAccount: () => ({
        profile: { userId: 7 },
        likedIds: new Set([12]),
        likesReady: true,
        pendingLikes: new Set()
      })
    }
  }
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/components/music/song-actions.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, { exports, require: (name) => modules[name] })
  function flatten(node) {
    return Array.isArray(node)
      ? node.flatMap(flatten)
      : node && typeof node === 'object'
        ? [node, ...flatten(node.props?.children)]
        : []
  }
  const props = {
    track: { title: 'Song', source: { kind: 'netease', id: 12 } },
    render: jsx('div', {}),
    children: null,
    onPlay() {}
  }
  const nodes = flatten(exports.SongContextMenu(props))
  const favorite = nodes.findIndex(
    (node) => node.type === 'Item' && node.props.children?.includes('取消收藏')
  )
  const separator = nodes.findIndex((node) => node.type === 'Separator')
  assert.ok(favorite >= 0 && favorite < separator)
  assert.equal(
    nodes.some((node) => node.props?.className === 'song-menu-remove'),
    false
  )
  assert.equal(
    flatten(exports.SongContextMenu({ ...props, onRemove() {} })).some(
      (node) => node.props?.className === 'song-menu-remove'
    ),
    true
  )
})

test('daily recommendation menu uses its own ordered batch callbacks without a fake share link', () => {
  const app = harness()
  const calls = []
  const items = app.menu(undefined, false, {
    name: '每日推荐',
    onPlay: () => calls.push('replace'),
    onNext: () => calls.push('next')
  })
  assert.equal(items.length, 2)
  items[0].props.onClick()
  items[1].props.onClick()
  assert.deepEqual(calls, ['replace', 'next'])
  assert.equal(app.calls.length, 0)
  assert.equal(app.menu(undefined).length, 0)
  assert.equal(app.menu(undefined, true, { onPlay() {}, onNext() {} }).length, 0)
})
test('liked music and private radar expose playback and real share links without management', () => {
  const app = harness()
  for (const item of [
    { ...playlist, name: '我喜欢的音乐', liked: true },
    { ...playlist, id: 3136952023, creatorId: 0, name: '私人雷达' }
  ]) {
    const items = app.menu(item)
    assert.equal(items.length, 3)
    items[2].props.onClick()
    assert.equal(app.calls.at(-1)[1].id, item.id)
  }
})
