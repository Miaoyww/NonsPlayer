import { errorNoticeModule } from './helpers/error-notice.mjs'
import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'
function harness(writeText = async () => {}) {
  const slots = [],
    effects = [],
    calls = [],
    copied = [],
    errors = [],
    notices = []
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
    '@/components/music/cover': { Cover: 'Cover' },
    '@/features/local/tag-editor-dialog': { LocalTagEditorDialog: 'LocalTagEditorDialog' },
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
    '@/lib/platform': { isAndroid: () => false, isWindows: () => true },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/plugin-clipboard-manager': {
      writeText: async (value) => {
        await writeText(value)
        copied.push(value)
      }
    },
    '@base-ui/react/context-menu': { ContextMenu: {} },
    'lucide-react': {},
    '@/plugins/song-menu': { PluginSongMenuItems: () => null },
    '@/features/downloads/download-provider': {
      useDownloads: () => ({ start: async () => '已加入下载' })
    },
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
          : {
              artists: [
                { name: 'Singer', reference: { source: 'netease', kind: 'artist', id: '42' } }
              ],
              albumReference: { source: 'netease', kind: 'album', id: '99' },
              publishedAt: null
            }
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
    require: (name) => {
      if (name === '@/components/error-notice') return errorNoticeModule
      assert.ok(name === '@/features/music/references' || name in modules, name)
      return name === '@/features/music/references' ? musicReference : modules[name]
    }
  })
  function render() {
    cursor = 0
    const tree = exports.SongActionsProvider({
      children: null,
      onError: (error) => errors.push(error),
      onNotice: (message) => notices.push(message)
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
  function information(tree) {
    return Object.fromEntries(
      nodes(tree, 'div')
        .filter((node) => node.props?.className === 'song-information-field')
        .map((node) => {
          const [label, value] = [node.props.children].flat(Infinity)
          return [label.props.children, value.props.children]
        })
    )
  }
  return { render, nodes, information, calls, copied, errors, notices }
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
test('local information uses file metadata even when linked to NetEase and omits online IDs', async () => {
  const h = harness()
  h.render().props.value.details(track)
  h.render()
  await settle()
  const tree = h.render()
  assert.equal(h.calls.length, 1)
  assert.equal(h.calls[0].command, 'local_track_information')
  const information = h.information(tree)
  assert.equal(information['采样率'], '44100 Hz')
  assert.equal(information['码率'], '1411 kbps')
  assert.equal(information['路径'], 'C:/Music/song.wav')
  assert.equal(information['歌曲名称'], undefined)
  assert.equal(information['艺术家'], undefined)
  assert.equal(information['专辑'], undefined)
  assert.equal(h.nodes(tree, 'Cover').length, 1)
  assert.equal(h.nodes(tree, 'DialogContent').length, 1)
  assert.equal(h.nodes(tree, 'DialogHeader').length, 1)
  assert.equal(h.nodes(tree, 'input').length, 0)
  assert.equal(h.nodes(tree, 'ActionButton').length, 0)
  h.nodes(tree, 'Dialog')[0].props.onOpenChange(false)
  assert.equal(h.nodes(h.render(), 'Dialog')[0].props.open, false)
})
test('streaming details retain structured artist and album IDs', async () => {
  const h = harness()
  h.render().props.value.details({
    ...track,
    key: 'netease:1',
    source: { kind: 'online', track: { reference: { source: 'netease', kind: 'track', id: '1' } } }
  })
  h.render()
  await settle()
  const tree = h.render()
  assert.equal(h.calls[0].command, 'music_query')
  const information = h.information(tree)
  assert.equal(information['歌手 ID'], '42')
  assert.equal(information['专辑 ID'], '99')
  assert.equal(information['歌曲名称'], undefined)
  assert.equal(information['歌手'], undefined)
  assert.equal(information['专辑'], undefined)
  assert.equal(information['采样率'], undefined)
  assert.equal(information['路径'], undefined)
  assert.equal(h.nodes(tree, 'Cover').length, 1)
  assert.equal(h.nodes(tree, 'input').length, 0)
  assert.equal(h.nodes(tree, 'ActionButton').length, 0)
})

test('official clipboard errors reach the user and allow retry', async () => {
  const failure = new Error('clipboard unavailable')
  let fail = true
  const h = harness(async () => {
    if (fail) throw failure
  })
  h.render().props.value.copy('Song - Singer')
  await settle()
  assert.deepEqual(h.errors, [failure])
  assert.deepEqual(h.copied, [])
  assert.deepEqual(h.notices, [])

  fail = false
  h.render().props.value.copy('Song - Singer')
  await settle()
  assert.deepEqual(h.copied, ['Song - Singer'])
  assert.deepEqual(h.notices, ['已复制到剪贴板。'])
})
