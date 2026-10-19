import { errorNoticeModule } from './helpers/error-notice.mjs'
import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'
function harness() {
  const slots = [],
    effects = [],
    calls = [],
    copied = []
  let cursor = 0
  const jsx = (type, props) => ({ type, props })
  const modules = {
    '@/features/music/queries': {
      readTrackInformation: async () => {
        calls.push({ command: 'music_query' })
        return {
          artists: [{ name: 'Singer', reference: { source: 'netease', kind: 'artist', id: '42' } }],
          albumReference: { source: 'netease', kind: 'album', id: '99' },
          publishedAt: null
        }
      }
    },
    '@/features/music/use-music-sources': {},
    '@/features/music/use-music-sources': { useMusicSources: () => ({ statuses: [] }) },
    '@/features/music/share-links': { trackShareLink: () => '' },
    '@/components/music/loading': { TextSkeleton: 'div' },
    'react/jsx-runtime': { jsx, jsxs: jsx, Fragment: 'Fragment' },
    react: {
      createContext: () => ({ Provider: 'Provider' }),
      useContext: () => ({}),
      useState(initial) {
        const i = cursor++
        if (!(i in slots)) slots[i] = initial
        return [
          slots[i],
          (value) => {
            slots[i] = typeof value === 'function' ? value(slots[i]) : value
          }
        ]
      },
      useRef(initial) {
        const i = cursor++
        return (slots[i] ??= { current: initial })
      },
      useEffect(fn, deps) {
        const i = cursor++,
          previous = slots[i]
        if (!previous || deps.some((v, j) => v !== previous.deps[j])) {
          previous?.cleanup?.()
          slots[i] = { deps }
          effects.push(() => {
            slots[i].cleanup = fn()
          })
        }
      }
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@base-ui/react/context-menu': { ContextMenu: {} },
    'lucide-react': {},
    '@/plugins/song-menu': { PluginSongMenuItems: () => null },
    '@/features/library/playlist-picker': { PlaylistPicker: 'PlaylistPicker' },
    '@/features/aggregate/aggregate-playlist-picker': {
      AggregatePlaylistPicker: 'AggregatePlaylistPicker'
    },
    '@/components/music/action-button': { ActionButton: 'ActionButton' },
    '@/features/account/account': {},
    '@/components/ui/dialog': Object.fromEntries(
      ['Dialog', 'DialogContent', 'DialogHeader', 'DialogTitle', 'DialogDescription'].map((v) => [
        v,
        v
      ])
    ),
    '@/lib/player': {
      errorText: String,
      formatTime: () => '2:00',
      nativeCall: async (command, args) => {
        calls.push({ command, args })
        return command === 'local_track_information'
          ? {
              trackNumber: 3,
              discNumber: 1,
              bitrate: 1411,
              sampleRate: 44100,
              bitDepth: 16,
              channels: 2,
              format: 'WAV',
              fileSize: 1048576
            }
          : { artists: [{ name: 'Singer', id: 42 }], albumId: 99, publishedAt: null }
      }
    }
  }
  const exports = {}
  const code = ts.transpileModule(
    readFileSync(new URL('../src/components/music/song-actions.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  ).outputText
  runInNewContext(code, {
    exports,
    Date,
    navigator: { clipboard: { writeText: async (value) => copied.push(value) } },
    require: (name) => {
      if (name === '@/components/error-notice') return errorNoticeModule
      assert.ok(name === '@/features/music/legacy-reference' || name in modules, name)
      return name === '@/features/music/legacy-reference' ? musicReference : modules[name]
    }
  })
  function render() {
    cursor = 0
    const tree = exports.SongActionsProvider({
      children: null,
      onError: (error) => {
        throw error
      },
      onNotice() {}
    })
    effects.splice(0).forEach((fn) => fn())
    return tree
  }
  function nodes(tree, type) {
    if (!tree || typeof tree !== 'object') return []
    return [
      ...(tree.type === type ? [tree] : []),
      ...[tree.props?.children].flat(Infinity).flatMap((child) => nodes(child, type))
    ]
  }
  return { render, nodes, calls, copied }
}
const track = {
  key: 'local:1',
  title: 'Song',
  artist: 'Singer',
  album: 'Album',
  durationMs: 120000,
  cover: '',
  source: { kind: 'local', path: 'C:/Music/song.wav', neteaseId: 123 }
}
const settle = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
test('local information uses file metadata even when linked to NetEase and copies it without online IDs', async () => {
  const h = harness()
  h.render().props.value.details(track)
  h.render()
  await settle()
  const tree = h.render()
  assert.equal(h.calls.length, 1)
  assert.equal(h.calls[0].command, 'local_track_information')
  const values = h.nodes(tree, 'input').map((node) => node.props.value)
  assert.ok(values.includes('44100 Hz'))
  assert.ok(values.includes('1411 kbps'))
  assert.ok(values.includes('C:/Music/song.wav'))
  assert.equal(h.nodes(tree, 'DialogContent').length, 1)
  assert.equal(h.nodes(tree, 'DialogHeader').length, 1)
  h.nodes(tree, 'ActionButton')
    .find((node) => node.props.children === '复制全部信息')
    .props.onClick()
  await settle()
  assert.match(h.copied[0], /音轨：3/)
  assert.match(h.copied[0], /路径：C:\/Music\/song.wav/)
  assert.doesNotMatch(h.copied[0], /歌手 ID|专辑 ID|歌曲 ID|歌曲链接/)
  h.nodes(tree, 'Dialog')[0].props.onOpenChange(false)
  assert.equal(h.nodes(h.render(), 'Dialog')[0].props.open, false)
})
test('streaming details retain structured artist and album IDs', async () => {
  const h = harness()
  h.render().props.value.details({ ...track, key: 'netease:1', source: { kind: 'netease', id: 1 } })
  h.render()
  await settle()
  const tree = h.render()
  assert.equal(h.calls[0].command, 'music_query')
  h.nodes(tree, 'ActionButton')
    .find((node) => node.props.children === '复制全部信息')
    .props.onClick()
  await settle()
  assert.match(h.copied[0], /歌手 ID：42/)
  assert.match(h.copied[0], /专辑 ID：99/)
  assert.doesNotMatch(h.copied[0], /采样率|路径：/)
})
