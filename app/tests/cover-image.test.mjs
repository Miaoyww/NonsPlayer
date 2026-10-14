import assert from 'node:assert/strict'
import { test } from 'node:test'
import { coverImage } from './helpers/cover-image.mjs'
const image = { original: 'raw', small: 'tiny', normal: 'card', large: 'detail', exlarge: 'media' }
test('candidate fallback is source-neutral, bounded and removes repeated URLs', () => {
  assert.deepEqual(Array.from(coverImage.coverCandidates(image, 'normal')), ['card', 'tiny', 'raw'])
  assert.deepEqual(Array.from(coverImage.coverCandidates(image, 'original')), [
    'raw',
    'media',
    'detail',
    'card',
    'tiny'
  ])
  assert.deepEqual(Array.from(coverImage.coverCandidates({ ...image, small: 'card' }, 'normal')), [
    'card',
    'raw'
  ])
  assert.deepEqual(Array.from(coverImage.coverCandidates('local-cover.png', 'small')), [
    'local-cover.png'
  ])
  assert.equal(coverImage.coverOriginal(image), 'raw')
  assert.equal(coverImage.validCoverInput({ original: 'raw' }), false)
  assert.equal(coverImage.validCoverInput(image), true)
})
