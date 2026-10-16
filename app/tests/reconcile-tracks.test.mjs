import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/library/reconcile-tracks.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports }
)
const { reconcileTracks } = exports
const track = (key) => ({
  key,
  title: key,
  artist: 'artist',
  album: 'album',
  cover: '',
  durationMs: 1000,
  source: { kind: 'netease', id: Number(key) }
})

test('identical server snapshots preserve the array and track identities', () => {
  const previous = [track('1'), track('2')]
  assert.equal(reconcileTracks(previous, structuredClone(previous)), previous)
})

test('insertions, deletions, reordering and metadata changes replace only affected tracks', () => {
  const previous = [track('1'), track('2'), track('3')]
  const next = [track('4'), track('3'), { ...track('1'), title: 'updated', cover: 'updated cover' }]
  const result = reconcileTracks(previous, next)
  assert.equal(result[0], next[0])
  assert.equal(result[1], previous[2])
  assert.equal(result[2], next[2])
  assert.equal(previous.length, 3)
  assert.equal(previous[0].title, '1')
})

test('duplicate occurrences stay separate and a fully removed list becomes empty', () => {
  const previous = [track('1'), track('1')]
  assert.equal(reconcileTracks(previous, structuredClone(previous)), previous)
  assert.equal(reconcileTracks(previous, [track('1')])[0], previous[0])
  assert.equal(reconcileTracks(previous, []).length, 0)
})
