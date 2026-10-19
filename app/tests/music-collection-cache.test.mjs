import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createRuntimeCache } from './helpers/runtime-cache.mjs'

test('collection update invalidates only matching queries and first-page presentation', () => {
  const { requestCache, requestKey, presentationKey, invalidateCollectionCache } =
    createRuntimeCache()
  const entity = { source: 'qqmusic', kind: 'playlist', id: 'opaque:歌单' }
  const other = { ...entity, id: 'another' }
  const key = (reference) =>
    requestKey('music_query', {
      source: reference.source,
      request: { operation: 'tracks', entity: reference, page: { cursor: null, limit: 100 } }
    })
  const display = presentationKey('entity-tracks', entity)
  requestCache.set(key(entity), 'old')
  requestCache.set(key(other), 'other')
  requestCache.set(display, 'presentation')
  const contextRevision = requestCache.contextRevision
  invalidateCollectionCache(entity)
  assert.equal(requestCache.peek(key(entity)), undefined)
  assert.equal(requestCache.peek(display), undefined)
  assert.equal(requestCache.peek(key(other)), 'other')
  assert.equal(requestCache.contextRevision, contextRevision)
})

test('collection update rejects a pending old delivery', async () => {
  const { requestCache, requestKey, invalidateCollectionCache, CacheInvalidatedError } =
    createRuntimeCache()
  const entity = { source: 'netease', kind: 'playlist', id: '42' }
  const key = requestKey('music_query', { request: { operation: 'tracks', entity } })
  let finish
  const reading = requestCache.get(
    key,
    () =>
      new Promise((resolve) => {
        finish = resolve
      })
  )
  const rejected = assert.rejects(reading, CacheInvalidatedError)
  invalidateCollectionCache(entity)
  finish('late')
  await rejected
  assert.equal(requestCache.peek(key), undefined)
})
