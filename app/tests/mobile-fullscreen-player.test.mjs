import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'

function harness() {
  let lyricsPage = false
  let rendered = 0
  const details = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@tauri-apps/api/core': { isTauri: () => true },
    'lucide-react': {},
    '@/components/music/action-button': { ActionButton: 'button' },
    '@/components/music/cover': { Cover: 'cover' },
    '@/components/music/music-links': { TrackArtists: 'artists' },
    '@/components/music/song-actions': {
      useSongActions: () => ({ details: (track) => details.push(track) })
    },
    '@/features/lyrics/use-mobile-lyrics-page': {
      useMobileLyricsPage: () => ({
        lyricsPage,
        showLyricsPage: () => {
          lyricsPage = true
        },
        showOverview: () => {
          lyricsPage = false
        }
      })
    },
    '@/lib/player': { usePlayer: () => ({ repeatMode: 'off', shuffle: false }) },
    './current-track-like': { CurrentTrackLike: 'like' },
    './music-options': { QualitySelect: 'quality' },
    './now-playing-menu': { NowPlayingMenu: 'menu' },
    './playback-timeline': { PlaybackTimeline: 'timeline' },
    './playback-transport': { PlaybackTransport: 'transport' }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/playback/mobile-fullscreen-player.tsx', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    { exports, require: (name) => modules[name] ?? {} }
  )
  const track = { key: 'test', title: '测试歌曲', artist: '测试歌手', cover: 'cover.jpg' }
  return {
    details,
    track,
    get rendered() {
      return rendered
    },
    render: () =>
      exports.MobileFullscreenPlayer({
        track,
        present: true,
        loading: false,
        onError() {},
        onQueue() {},
        onRefresh() {},
        renderLyrics() {
          rendered++
          return 'lyrics'
        }
      })
  }
}

function find(node, predicate) {
  if (!node || typeof node !== 'object') return
  if (predicate(node)) return node
  for (const child of [node.props?.children].flat()) {
    const found = find(child, predicate)
    if (found) return found
  }
}

test('cover and lyrics switch through the upper area while controls remain and hidden lyrics are not rendered', () => {
  const app = harness()
  let tree = app.render()
  assert.equal(app.rendered, 0)
  find(tree, (node) => node.props?.['aria-label'] === '查看歌词').props.onClick()
  tree = app.render()
  assert.equal(tree.props['data-mobile-view'], 'lyrics')
  assert.equal(app.rendered, 1)
  assert.ok(find(tree, (node) => node.type === 'transport'))
  assert.equal(find(tree, (node) => node.type === 'timeline').props.timeRowContent.type, 'quality')
  let prevented = false,
    stopped = false
  find(tree, (node) => node.props?.['aria-label'] === '返回封面与播放控制').props.onClickCapture({
    preventDefault() {
      prevented = true
    },
    stopPropagation() {
      stopped = true
    }
  })
  assert.ok(prevented && stopped)
  assert.equal(app.render().props['data-mobile-view'], 'overview')
  assert.equal(app.rendered, 1)
})

test('info opens existing song details and favorite is extracted from the menu', () => {
  const app = harness()
  const tree = app.render()
  find(tree, (node) => node.props?.['aria-label'] === '查看歌曲信息').props.onClick()
  assert.deepEqual(app.details, [app.track])
  assert.ok(find(tree, (node) => node.type === 'like'))
  assert.equal(find(tree, (node) => node.type === 'menu').props.showLike, false)
})
