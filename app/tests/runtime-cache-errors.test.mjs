import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createRuntimeCache } from './helpers/runtime-cache.mjs'

test('capacity eviction keeps one shared read without reporting or refilling the evicted key', async () => {
  const reports = []
  const { RuntimeCache } = createRuntimeCache((_, error) => reports.push(error))
  const cache = new RuntimeCache(128)
  let finish
  const reads = Promise.all([
    cache.get('cover:waiting', () => new Promise((resolve) => (finish = resolve))),
    cache.get('cover:waiting', () => assert.fail('duplicate loader'))
  ])
  await cache.get('cover:ready', async () => 'x'.repeat(51))
  const joined = cache.get('cover:waiting', () =>
    assert.fail('loader must stay shared after eviction')
  )
  assert.equal(cache.peek('cover:waiting'), undefined, 'the in-flight slot was evicted')
  finish('late')
  assert.deepEqual(await reads, ['late', 'late'])
  assert.equal(await joined, 'late')
  assert.deepEqual(reports, [])
  assert.equal(cache.peek('cover:waiting'), undefined, 'late results cannot refill an evicted key')
})

test('explicit invalidation reports a normalized error while real loader errors retain identity', async () => {
  const reports = []
  const { RuntimeCache, CacheInvalidatedError } = createRuntimeCache((_, error) =>
    reports.push(error)
  )
  const cache = new RuntimeCache()
  let finish
  const reads = [
    cache.get('cover:old', () => new Promise((resolve) => (finish = resolve))),
    cache.get('cover:old', () => assert.fail('duplicate loader'))
  ]
  const settlements = Promise.allSettled(reads)
  cache.delete('cover:old', 'clear', 'test.delete')
  const results = await settlements
  const error = results[0].reason
  results.forEach((result) => {
    assert.equal(result.status, 'rejected')
    assert.ok(error instanceof CacheInvalidatedError)
    assert.equal(result.reason, error, 'all readers receive the same deduplicable error')
  })
  assert.equal(error.stage, 'clear')
  assert.equal(error.trigger, 'test.delete')
  assert.ok(reports.includes(error))
  cache.set('cover:old', 'new')
  finish('stale')
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(cache.peek('cover:old'), 'new', 'an old loader cannot overwrite a newer value')

  const original = new Error('network unavailable')
  await assert.rejects(
    cache.get('cover:failed', async () => {
      throw original
    }),
    (error) => error === original
  )
  assert.equal(reports.at(-1), original)
})

test('an old loader rejection after explicit invalidation is reported', async () => {
  const reports = []
  const { RuntimeCache } = createRuntimeCache((_, error) => reports.push(error))
  const cache = new RuntimeCache()
  let reject
  const read = cache.get('query:abandoned', () => new Promise((_, failure) => (reject = failure)))
  cache.invalidate(undefined, 'clear', 'test.clear')
  await assert.rejects(read, { code: 'cacheInvalidated', stage: 'clear' })
  const late = new Error('late network failure')
  reject(late)
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(reports.at(-1), late)
})

test('invalidation during validation rejects immediately and blocks delivery', async () => {
  const reports = []
  const { RuntimeCache, CacheInvalidatedError } = createRuntimeCache((_, error) =>
    reports.push(error)
  )
  const cache = new RuntimeCache()
  let finish
  let validate
  const validating = new Promise((resolve) => (validate = resolve))
  const validation = new Promise((_, reject) => (finish = reject))
  const read = cache.get('query:validating', async () => 'old', {
    validate: () => {
      validate()
      return validation
    }
  })
  const rejected = assert.rejects(read, (error) => {
    assert.ok(error instanceof CacheInvalidatedError)
    assert.equal(error.stage, 'refresh')
    return true
  })
  await validating
  cache.delete('query:validating', 'refresh', 'test.validation')
  await rejected
  const late = new Error('late validation failure')
  finish(late)
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(reports.at(-1), late)
  assert.equal(cache.peek('query:validating'), undefined)
})

test('set invalidates an evicted pending read before the old loader settles', async () => {
  const { RuntimeCache, CacheInvalidatedError } = createRuntimeCache()
  const cache = new RuntimeCache(128)
  let finish
  const read = cache.get('query:evicted', () => new Promise((resolve) => (finish = resolve)))
  await cache.get('query:replacement', async () => 'x'.repeat(45))
  cache.set('query:evicted', 'new')
  await assert.rejects(read, (error) => {
    assert.ok(error instanceof CacheInvalidatedError)
    assert.equal(error.stage, 'replace')
    assert.equal(error.trigger, 'cache.set')
    return true
  })
  finish('stale')
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(cache.peek('query:evicted'), 'new')
})

for (const mode of ['delete', 'clear']) {
  test(`${mode} after eviction rejects old readers and cannot overwrite the new request`, async () => {
    const { RuntimeCache, CacheInvalidatedError } = createRuntimeCache()
    const cache = new RuntimeCache(128)
    const key = `query:${mode}`
    let finishOld
    let finishNew
    const old = cache.get(key, () => new Promise((resolve) => (finishOld = resolve)))
    await cache.get('query:eviction', async () => 'x'.repeat(48))
    if (mode === 'delete') cache.delete(key, 'clear', 'test.evictedDelete')
    else cache.invalidate(undefined, 'clear', 'test.evictedClear')
    await assert.rejects(old, (error) => {
      assert.ok(error instanceof CacheInvalidatedError)
      assert.equal(error.trigger, mode === 'delete' ? 'test.evictedDelete' : 'test.evictedClear')
      return true
    })

    const replacement = cache.get(key, () => new Promise((resolve) => (finishNew = resolve)))
    finishNew('new')
    assert.equal(await replacement, 'new')
    finishOld('stale')
    await new Promise((resolve) => setImmediate(resolve))
    assert.equal(cache.peek(key), 'new')
  })
}

test('retryInvalidated retries immediately after an explicit invalidation', async () => {
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
  assert.equal(await read, 'fresh')
  assert.equal(calls, 2)
  assert.equal(reports.length, 1)
  assert.equal(reports[0].stage, 'unknown')
  finish('stale')
})
