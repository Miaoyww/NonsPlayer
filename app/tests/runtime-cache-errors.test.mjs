import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createRuntimeCache } from './helpers/runtime-cache.mjs'

test('capacity eviction reports the same normalized invalidation to concurrent readers', async () => {
  const reports = []
  const { RuntimeCache, CacheInvalidatedError } = createRuntimeCache((_, error) =>
    reports.push(error)
  )
  const cache = new RuntimeCache(128)
  let finish
  const reads = Promise.allSettled([
    cache.get('cover:waiting', () => new Promise((resolve) => (finish = resolve))),
    cache.get('cover:waiting', () => assert.fail('duplicate loader'))
  ])
  await cache.get('cover:ready', async () => 'x'.repeat(51))
  const results = await reads
  finish('late')
  assert.ok(results.every((result) => result.status === 'rejected'))
  const error = results[0].reason
  assert.ok(error instanceof CacheInvalidatedError)
  assert.equal(results[1].reason, error, 'shared failures must remain deduplicable')
  assert.ok(reports.length > 0)
  assert.ok(
    reports.every((report) => report === error),
    'never report the raw evicted error'
  )
  assert.equal(cache.peek('cover:waiting'), undefined, 'late results cannot refill an evicted key')
})

test('explicit invalidation reports a normalized error while real loader errors retain identity', async () => {
  const reports = []
  const { RuntimeCache, CacheInvalidatedError } = createRuntimeCache((_, error) =>
    reports.push(error)
  )
  const cache = new RuntimeCache()
  let finish
  const read = cache.get('cover:old', () => new Promise((resolve) => (finish = resolve)))
  const rejected = assert.rejects(read, (error) => {
    assert.ok(error instanceof CacheInvalidatedError)
    assert.equal(reports.at(-1), error)
    return true
  })
  cache.delete('cover:old')
  await rejected
  finish('late')
  const original = new Error('network unavailable')
  await assert.rejects(
    cache.get('cover:failed', async () => {
      throw original
    }),
    (error) => error === original
  )
  assert.equal(reports.at(-1), original)
})

test('retryInvalidated retries internal invalidation without reporting intermediate errors', async () => {
  const reports = []
  const { RuntimeCache } = createRuntimeCache((_, error) => reports.push(error))
  const cache = new RuntimeCache()
  let calls = 0
  let finish
  let started
  const firstStarted = new Promise((resolve) => (started = resolve))
  const read = cache.get(
    'local_entity_tracks:playlist',
    () => {
      calls++
      if (calls === 1)
        return new Promise((resolve) => {
          finish = resolve
          started()
        })
      return Promise.resolve('fresh')
    },
    { retryInvalidated: 2 }
  )
  await firstStarted
  cache.delete('local_entity_tracks:playlist')
  finish('stale')
  assert.equal(await read, 'fresh')
  assert.equal(calls, 2)
  assert.deepEqual(reports, [])
})
