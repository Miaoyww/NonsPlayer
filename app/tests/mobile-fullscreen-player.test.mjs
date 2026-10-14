import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'

function harness() {
  let lyricsPage = false
  let rendered = 0
  let visited = false
  let closed = 0
  const details = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    react: {
      useState: () => [
        visited,
        (next) => {
          visited = next
        }
      ]
    },
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
    get closed() {
      return closed
    },
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
        onClose() {
          closed++
        },
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
  for (const child of [node.props?.children, node.props?.leadingControl].flat()) {
    const found = find(child, predicate)
    if (found) return found
  }
}

test('cover and lyrics switch through the upper area while controls remain and the retained lyrics become hidden', () => {
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
  const overview = app.render()
  assert.equal(overview.props['data-mobile-view'], 'overview')
  assert.equal(
    find(overview, (node) => node.props?.['aria-label'] === '返回封面与播放控制').props[
      'data-active'
    ],
    false
  )
  assert.equal(app.rendered, 2)
})

test('cover retains the shared favorite and menu without dedicated info or queue buttons', () => {
  const app = harness()
  const tree = app.render()
  assert.equal(
    find(tree, (node) => node.props?.['aria-label'] === '查看歌曲信息'),
    undefined
  )
  assert.equal(find(tree, (node) => node.type === 'transport').props.onQueue, undefined)
  assert.ok(find(tree, (node) => node.type === 'like'))
  assert.equal(find(tree, (node) => node.type === 'menu').props.showLike, false)
})

test('full song header appears only for lyrics and cover identity stays above the shared single-row transport', () => {
  const app = harness()
  const cover = app.render()
  assert.equal(
    find(cover, (node) => node.type === 'header'),
    undefined
  )
  assert.ok(find(cover, (node) => node.props?.className === 'mobile-cover-identity'))
  assert.equal(find(cover, (node) => node.type === 'transport').props.withModes, true)
  find(cover, (node) => node.props?.['aria-label'] === '查看歌词').props.onClick()
  assert.ok(find(app.render(), (node) => node.type === 'header'))
})

test('lyrics header down arrow closes the fullscreen player instead of switching back to the cover', () => {
  const app = harness()
  find(app.render(), (node) => node.props?.['aria-label'] === '查看歌词').props.onClick()
  find(app.render(), (node) => node.props?.['aria-label'] === '返回音乐').props.onClick()
  assert.equal(app.closed, 1)
  assert.equal(app.render().props['data-mobile-view'], 'lyrics')
})
