import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'
const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/music/share-links.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports, URL, encodeURIComponent, require: () => musicReference }
)
const sources = [
  {
    source: 'first',
    shareLinkTemplates: {
      track: 'https://first.example/song/{id}',
      album: 'https://first.example/album?id={id}'
    }
  },
  { source: 'second', shareLinkTemplates: { track: 'https://second.example/#/music?id={id}' } },
  {
    source: 'netease',
    shareLinkTemplates: {
      track: 'https://legacy.example/song/{id}',
      playlist: 'https://legacy.example/list/{id}'
    }
  }
]

test('unresolved collections have no share link while their source identity is unavailable', () => {
  for (const id of [0, -1, 'unknown', NaN, Number.MAX_SAFE_INTEGER + 1]) {
    const item = { kind: 'playlist', id }
    assert.equal(exports.collectionShareLink(item, sources), '')
    assert.throws(() => musicReference.collectionReference(item), /收藏缺少来源引用/)
  }
})
test('share links retain real source and encode opaque IDs without changing URL structure', () => {
  const reference = { source: 'second', kind: 'track', id: '雪/a?b&c#d' }
  assert.equal(
    exports.trackShareLink({ source: { kind: 'online', track: { reference } } }, sources),
    `https://second.example/#/music?id=${encodeURIComponent(reference.id)}`
  )
  assert.equal(
    exports.collectionShareLink(
      { reference: { source: 'first', kind: 'album', id: '42' }, id: 99, kind: 'album' },
      sources
    ),
    'https://first.example/album?id=42'
  )
})
test('legacy entities still use adapter templates and local associations never become share identities', () => {
  assert.equal(
    exports.trackShareLink({ source: { kind: 'netease', id: 42 } }, sources),
    'https://legacy.example/song/42'
  )
  assert.equal(
    exports.collectionShareLink({ kind: 'playlist', id: 42 }, sources),
    'https://legacy.example/list/42'
  )
  assert.equal(exports.trackShareLink({ source: { kind: 'local', neteaseId: 42 } }, sources), '')
  assert.equal(
    exports.collectionShareLink({ localId: '42', kind: 'playlist', id: 42 }, sources),
    ''
  )
})
test('missing sources, unsupported entities and invalid templates yield no share link', () => {
  const reference = { source: 'first', kind: 'track', id: '1' }
  assert.equal(exports.entityShareLink(reference, []), '')
  assert.equal(exports.entityShareLink({ ...reference, kind: 'artist' }, sources), '')
  for (const template of [
    'javascript:{id}',
    'https://user:secret@example.com/{id}',
    'https://example.com/{id}/{id}',
    'https://example.com/'
  ])
    assert.equal(
      exports.entityShareLink(reference, [
        { source: 'first', shareLinkTemplates: { track: template } }
      ]),
      ''
    )
})
