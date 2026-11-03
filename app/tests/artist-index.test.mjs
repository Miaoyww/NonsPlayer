import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

function loadArtistIndex() {
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/features/local/artist-index.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  )
  const exports = {}
  runInNewContext(outputText, { exports, require: () => ({}) })
  return exports
}

test('classifies Latin initials case-insensitively and everything else as other', () => {
  const { artistIndexLetter } = loadArtistIndex()
  assert.equal(artistIndexLetter('Adele'), 'A')
  assert.equal(artistIndexLetter('  blur'), 'B')
  assert.equal(artistIndexLetter('周杰伦'), '#')
  assert.equal(artistIndexLetter('2Pac'), '#')
  assert.equal(artistIndexLetter(''), '#')
})

test('groups artists in A-Z order and merges other names under the final section', () => {
  const { groupArtists } = loadArtistIndex()
  const artists = [
    { id: 'zed', name: 'Zed' },
    { id: 'adele', name: 'Adele' },
    { id: 'zhou', name: '周杰伦' },
    { id: 'blur', name: 'blur' },
    { id: 'digits', name: '2Pac' },
    { id: 'zoe', name: 'Zoe' }
  ]
  const groups = groupArtists(artists)
  assert.deepEqual(
    Array.from(groups, (group) => group.letter),
    ['A', 'B', 'Z', '#']
  )
  assert.deepEqual(
    Array.from(groups.find((group) => group.letter === 'Z').items, (item) => item.id),
    ['zed', 'zoe']
  )
  assert.deepEqual(
    Array.from(groups.find((group) => group.letter === '#').items, (item) => item.id),
    ['zhou', 'digits']
  )
})
