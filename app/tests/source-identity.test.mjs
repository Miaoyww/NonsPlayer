import { musicReference } from './helpers/music-reference.mjs'
import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

test('plugin song menus preserve online references and never classify unknown platforms as local or Netease', () => {
  const exports = {}
  const code = ts.transpileModule(
    readFileSync(new URL('../src/plugins/song-menu-context.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText
  runInNewContext(code, {
    exports,
    require: (name) => (name === '@/features/music/references' ? musicReference : coverImage)
  })
  const reference = { source: 'qqmusic', kind: 'track', id: 'music/雪:1' }
  const track = {
    key: JSON.stringify(['qqmusic', 'track', reference.id]),
    title: 'Same song',
    artist: 'Singer',
    album: '',
    durationMs: 1000,
    cover: 'local/private/path',
    source: { kind: 'online', track: { reference } }
  }
  const song = exports.publicMenuSong(track)
  assert.equal(song.source.kind, 'online')
  assert.equal(song.source.reference, reference)
  assert.equal(song.key, track.key)
  assert.equal(song.cover, '')
  const manifest = {
    contributes: {
      contextMenus: [
        { target: 'song', source: 'netease' },
        { target: 'song', source: 'local' }
      ]
    }
  }
  assert.equal(exports.songMenuContributions(manifest, track).length, 0)
})
