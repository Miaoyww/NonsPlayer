import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

function harness(android) {
  let expanded = false,
    lyrics = 0,
    queue = 0
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
      statusLabels: { playing: '播放中' }
    },
    '@/components/music/track-title': { trackDisplayTitle: (track) => track.title },
    '@/components/music/track-identity': {
      TrackIdentity: ({ cover }) => jsx('track-identity', { children: cover })
    },
    '@/components/music/action-button': { ActionButton: 'button' },
    '@/components/music/cover': { Cover: 'cover' },
    '@/features/playback/current-track-like': { CurrentTrackLike: 'like' },
    '@/features/queue/queue-popover': { QueuePopover: 'queue' },
    '@/features/playback/music-options': { QualitySelect: 'quality' },
    '@/features/playback/volume-control': { VolumeControl: 'volume' },
    '@/features/playback/playback-timeline': { PlaybackTimeline: 'timeline' },
    '@/features/playback/playback-transport': { PlaybackTransport: 'transport' },
    '@/features/playback/playback-notice': { PlaybackNotice: 'notice' }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/playback/persistent-playback-bar.tsx', import.meta.url),
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
    counts: () => ({ lyrics, queue })
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

test('Android starts collapsed, expands the upper card and retains the same transport on collapse', () => {
  const app = harness(true)
  let tree = app.render()
  assert.equal(find(tree, details).props.inert, true)
  assert.equal(find(tree, details).props['aria-hidden'], true)
  assert.equal(find(tree, toggle).props['aria-expanded'], false)
  assert.equal(find(tree, byType('volume')), undefined)
  assert.equal(find(tree, byType('timeline')), undefined)
  assert.equal(find(tree, byType('transport')).props.withModes, true)
  find(tree, toggle).props.onClick()
  tree = app.render()
  const card = find(tree, details)
  assert.equal(card.props.inert, false)
  assert.equal(find(tree, toggle).props['aria-expanded'], true)
  find(card, (node) => node.props?.['aria-label'] === '打开正在播放').props.onClick()
  find(card, byType('queue')).props.onPage()
  assert.deepEqual(app.counts(), { lyrics: 1, queue: 1 })
  assert.ok(find(card, byType('like')))
  find(tree, toggle).props.onClick()
  tree = app.render()
  assert.equal(find(tree, details).props.inert, true)
  assert.ok(find(tree, byType('transport')))
})

test('desktop retains track details, timeline and volume without the mobile expand control', () => {
  const tree = harness(false).render()
  assert.equal(find(tree, toggle), undefined)
  assert.equal(find(tree, details), undefined)
  for (const type of ['transport', 'like', 'queue', 'timeline', 'volume', 'quality'])
    assert.ok(find(tree, byType(type)))
})
