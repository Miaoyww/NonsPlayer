import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

function harness(android) {
  let expanded = false,
    lyrics = 0,
    queue = 0,
    actions = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    react: {
      useState: () => [
        expanded,
        (update) => {
          expanded = update(expanded)
        }
      ]
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    'lucide-react': { ChevronUp: 'up', ChevronDown: 'down' },
    '@/lib/platform': { isAndroid: () => android },
    '@/lib/player': {
      usePlayer: () => ({
        index: 0,
        queue: [{ title: 'Song', artist: 'Artist' }],
        status: 'playing'
      }),
      useProgress: () => ({ positionMs: 0, durationMs: 0 }),
      nativeCall: async (_command, payload) => actions.push(payload.action),
      statusLabels: { playing: '播放中' }
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/components/music/track-title': { trackDisplayTitle: (track) => track.title },
    '@/components/music/track-identity': {
      TrackIdentity: ({ cover }) => jsx('track-identity', { children: cover })
    },
    '@/components/music/action-button': { ActionButton: 'button' },
    '@/components/music/cover': { Cover: 'cover' },
    '@/features/playback/now-playing/current-track-like': { CurrentTrackLike: 'like' },
    '@/features/queue/queue-popover': { QueuePopover: 'queue' },
    '@/features/playback/now-playing/music-options': { QualitySelect: 'quality' },
    '@/features/playback/controls/volume-control': { VolumeControl: 'volume' },
    '@/features/playback/controls/playback-timeline': { PlaybackTimeline: 'timeline' },
    '@/features/playback/controls/playback-transport': { PlaybackTransport: 'transport' },
    '@/features/playback/bar/playback-notice': { PlaybackNotice: 'notice' },
    '@/features/desktop-lyrics/desktop-lyrics-button': {
      DesktopLyricsButton: 'desktop-lyrics-button'
    }
  }
  const mobileExports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/playback/bar/mobile-playback-bar.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    { exports: mobileExports, require: (name) => modules[name] }
  )
  modules['@/features/playback/bar/mobile-playback-bar'] = mobileExports
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/playback/bar/persistent-playback-bar.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  return {
    render: () =>
      exports.PersistentPlaybackBar({
        onLyrics: () => lyrics++,
        onQueue: () => queue++,
        onError: assert.fail
      }),
    counts: () => ({ lyrics, queue }),
    actions: () => actions
  }
}
function find(node, predicate) {
  if (!node || typeof node !== 'object') return
  if (typeof node.type === 'function') return find(node.type(node.props), predicate)
  if (predicate(node)) return node
  for (const child of [node.props?.children].flat()) {
    const result = find(child, predicate)
    if (result) return result
  }
}
const byType = (type) => (node) => node.type === type
const details = (node) => node.props?.id === 'mobile-playback-details'
const toggle = (node) => node.props?.['aria-controls'] === 'mobile-playback-details'

test('Android uses the capsule with track, toggle and queue actions', () => {
  const app = harness(true)
  const tree = app.render()
  assert.equal(find(tree, byType('volume')), undefined)
  assert.equal(find(tree, byType('timeline')), undefined)
  assert.equal(find(tree, byType('desktop-lyrics-button')), undefined)
  assert.equal(find(tree, byType('like')), undefined)
  find(tree, (node) => node.props?.['aria-label'] === '打开正在播放').props.onClick()
  find(tree, (node) => node.props?.['aria-label'] === '暂停').props.onClick()
  find(tree, (node) => node.props?.['aria-label'] === '打开播放队列').props.onClick()
  assert.deepEqual(app.counts(), { lyrics: 1, queue: 1 })
  assert.deepEqual(app.actions(), ['toggle'])
})

test('desktop retains track details, timeline and volume without the mobile expand control', () => {
  const tree = harness(false).render()
  assert.equal(find(tree, toggle), undefined)
  assert.equal(find(tree, details), undefined)
  for (const type of ['transport', 'like', 'queue', 'timeline', 'volume'])
    assert.ok(find(tree, byType(type)))
  const options = find(tree, (node) => node.props?.className === 'persistent-options')
  assert.deepEqual(
    Array.from(options.props.children, (child) => child.type),
    ['like', 'desktop-lyrics-button', 'volume', 'queue']
  )
})
