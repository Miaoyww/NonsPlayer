import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/library/recent-playlists.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports, require: () => coverImage }
)
const { parseRecentPlaylists, rememberPlaylist, recentPlaylistKey } = exports
const playlist = (id) => ({
  id: String(id),
  reference: { source: 'netease', kind: 'playlist', id: String(id) },
  kind: 'playlist',
  name: `歌单 ${id}`,
  cover: { original: '', small: '', normal: '', large: '', exlarge: '' },
  subtitle: '',
  trackCount: 1
})

test('reopening a playlist moves updated metadata to the front and bounds history', () => {
  const items = Array.from({ length: 20 }, (_, index) => playlist(index + 1))
  const updated = { ...playlist(12), name: '新名称' }
  const reopened = rememberPlaylist(items, updated)
  assert.equal(reopened[0], updated)
  assert.equal(reopened.length, 20)
  assert.equal(reopened.filter((item) => item.id === '12').length, 1)
  assert.equal(rememberPlaylist(reopened, playlist(21)).at(-1).id, '19')
})

test('restoring ignores damaged, duplicate and non-playlist entries', () => {
  for (const saved of [null, '{', '{}', 'null']) assert.equal(parseRecentPlaylists(saved).length, 0)
  const saved = [
    playlist(1),
    playlist(1),
    { ...playlist(2), kind: 'album' },
    { id: 3, kind: 'playlist' },
    null,
    { ...playlist(4), trackCount: -1 },
    ...Array.from({ length: 30 }, (_, index) => playlist(index + 5))
  ]
  const restored = parseRecentPlaylists(JSON.stringify(saved))
  assert.equal(restored.length, 20)
  assert.equal(restored[0].id, '1')
  assert.equal(restored[1].id, '5')
  assert.notEqual(recentPlaylistKey(1), recentPlaylistKey(2))
})
