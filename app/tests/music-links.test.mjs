import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness() {
  const navigations = []
  const jsx = (type, props) => ({ type, props })
  const exports = {}
  const modules = {
    react: { Fragment: 'fragment' },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@/lib/player': {},
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ navigate: (...args) => navigations.push(args) })
    }
  }
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/components/music/music-links.tsx', import.meta.url), 'utf8'),
    {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }
  )
  runInNewContext(outputText, { exports, require: (name) => modules[name] })
  return { ...exports, navigations }
}

test('uniform artist and album navigation preserve source and opaque reference instead of numeric IDs', () => {
  const app = harness()
  const artist = { source: 'qqmusic', kind: 'artist', id: 'artist/雪:1' }
  const album = { source: 'qqmusic', kind: 'album', id: 'album/雪:1' }
  const tree = app.ArtistLinks({ artists: [{ name: 'Singer', reference: artist }] })
  tree.props.children[0].props.children[1].props.onClick({ stopPropagation() {} })
  app.AlbumLink({ reference: album, name: 'Album' }).props.onClick({ stopPropagation() {} })
  assert.equal(app.navigations[0][2].reference, artist)
  assert.equal(app.navigations[1][2].reference, album)
  assert.equal(app.navigations[0][2].id, 0)
})

test('each structured artist navigates independently and preserves slash-containing names', () => {
  const app = harness()
  const tree = app.ArtistLinks({
    artists: [{ id: 9, name: 'AC/DC' }, { id: 10, name: 'B' }, { name: 'Unknown' }]
  })
  const entries = tree.props.children
  assert.equal(entries[0].props.children[1].props.children, 'AC/DC')
  assert.equal(entries[1].props.children[0].props.children.join(''), ' / ')
  let stopped = 0
  entries[1].props.children[1].props.onClick({ stopPropagation: () => stopped++ })
  assert.equal(stopped, 1)
  assert.equal(app.navigations[0][0], 'artist')
  assert.equal(app.navigations[0][2].id, 10)
  assert.equal(entries[2].props.children[1].type, 'span')
})

test('album navigation retains ID and cover and prevents a parent play action', () => {
  const app = harness()
  const tree = app.AlbumLink({ id: 20, name: 'Album', cover: 'cover' })
  let stopped = false
  tree.props.onClick({
    stopPropagation: () => {
      stopped = true
    }
  })
  assert.equal(stopped, true)
  assert.equal(app.navigations[0][0], 'album')
  assert.equal(app.navigations[0][2].id, 20)
  assert.equal(app.navigations[0][2].cover, 'cover')
  assert.equal(app.AlbumLink({ name: 'Local album' }).type, 'span')
})
