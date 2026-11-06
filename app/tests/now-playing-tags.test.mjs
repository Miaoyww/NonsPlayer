import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness() {
  let sourceCalls = 0
  let lyricCalls = 0
  const jsx = (type, props) => ({ type, props })
  const modules = {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@/components/music/song-actions': { useSongActions: () => ({ details() {} }) },
    '@/features/music/source-label': {
      trackSource: (track) =>
        track.source.kind === 'online' ? track.source.track.reference.source : track.source.kind
    }
  }
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(
      new URL('../src/features/playback/now-playing/now-playing-tags.tsx', import.meta.url),
      'utf8'
    ),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, { exports, require: (name) => modules[name] })
  const flatten = (node) =>
    Array.isArray(node)
      ? node.flatMap(flatten)
      : node && typeof node === 'object'
        ? [node, ...flatten(node.props?.children)]
        : []
  return {
    render(track, props = {}) {
      const nodes = flatten(
        exports.NowPlayingTags({
          track,
          quality: 'lossless',
          lyricFormat: 'lrc',
          onSource: () => sourceCalls++,
          onLyrics: () => lyricCalls++,
          ...props
        })
      )
      return {
        sourceTag: nodes.find(
          (node) =>
            (node.type === 'button' || node.type === 'span') &&
            String(node.props?.['aria-label'] ?? node.props?.title ?? '').includes('歌曲来源')
        ),
        lyricTag: nodes.find(
          (node) =>
            node.type === 'button' && String(node.props?.['aria-label'] ?? '').includes('歌词来源')
        ),
        qualityTag: nodes.find(
          (node) =>
            node.type === 'button' && String(node.props?.['aria-label'] ?? '').includes('歌曲详情')
        ),
        sourceCalls: () => sourceCalls,
        lyricCalls: () => lyricCalls
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

test('online source tag requests source switching while local source stays static', () => {
  const app = harness()
  const online = app.render(stream)
  assert.equal(online.sourceTag.type, 'button')
  assert.equal(online.sourceTag.props['aria-label'], '选择歌曲来源，当前平台 NETEASE')
  online.sourceTag.props.onClick()
  assert.equal(online.sourceCalls(), 1)

  const localApp = harness()
  const localSource = localApp.render(local)
  assert.equal(localSource.sourceTag.type, 'span')
  assert.equal(localSource.sourceTag.props.title, '歌曲来源：LOCAL')
})

test('lyric format tag requests lyric source switching', () => {
  const app = harness()
  const rendered = app.render(stream)
  assert.equal(rendered.lyricTag.props['aria-label'], '选择歌词来源，当前格式 LRC')
  rendered.lyricTag.props.onClick()
  assert.equal(rendered.lyricCalls(), 1)
})

test('local playback shows Local while retaining the online source badge', () => {
  const app = harness()
  const rendered = app.render(stream, { quality: null, playbackMode: 'local' })
  assert.equal(rendered.qualityTag.props.children, 'Local')
  assert.match(rendered.qualityTag.props['aria-label'], /Local/)
  assert.equal(rendered.sourceTag.props.children, 'NETEASE')
  assert.equal(app.render(local, { quality: null }).qualityTag.props.children, 'Local')
  assert.equal(
    app.render(stream, { quality: 'lossless', playbackMode: 'stream' }).qualityTag.props.children,
    'Lossless'
  )
  assert.equal(
    app.render(stream, { quality: null, playbackMode: 'stream' }).qualityTag.props.children,
    '—'
  )
})
