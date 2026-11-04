import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness() {
  let sourcesOpen = false
  const jsx = (type, props) => ({ type, props })
  const modules = {
    react: { useState: () => [sourcesOpen, (value) => (sourcesOpen = value)] },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@/components/music/song-actions': { useSongActions: () => ({ details() {} }) },
    '@/features/music/source-label': {
      trackSource: (track) =>
        track.source.kind === 'online' ? track.source.track.reference.source : track.source.kind
    },
    './song-source-dialog': { SongSourceDialog: 'SongSourceDialog' }
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
    render(track) {
      const nodes = flatten(
        exports.NowPlayingTags({
          track,
          quality: 'lossless',
          lyricFormat: 'lrc',
          onLyrics() {}
        })
      )
      return {
        sourceTag: nodes.find(
          (node) =>
            (node.type === 'button' || node.type === 'span') &&
            String(node.props?.['aria-label'] ?? node.props?.title ?? '').includes('歌曲来源')
        ),
        dialog: nodes.find((node) => node.type === 'SongSourceDialog')
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

test('online source tag opens source switching while local source stays static', () => {
  const app = harness()
  const online = app.render(stream)
  assert.equal(online.sourceTag.type, 'button')
  assert.equal(online.sourceTag.props['aria-label'], '选择歌曲来源，当前平台 NETEASE')
  assert.equal(online.dialog, undefined)
  online.sourceTag.props.onClick()
  const opened = app.render(stream)
  assert.equal(opened.dialog.props.track.key, stream.key)
  opened.dialog.props.onClose()
  assert.equal(app.render(stream).dialog, undefined)

  const localApp = harness()
  const localSource = localApp.render(local)
  assert.equal(localSource.sourceTag.type, 'span')
  assert.equal(localSource.sourceTag.props.title, '歌曲来源：LOCAL')
  assert.equal(localSource.dialog, undefined)
})
