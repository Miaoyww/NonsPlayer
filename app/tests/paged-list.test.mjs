import { createRuntimeCache } from './helpers/runtime-cache.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness(cache = createRuntimeCache(), initialCacheKey) {
  const slots = [],
    effects = [],
    requests = []
  let cursor = 0
  const react = {
    useState(initial) {
      const index = cursor++
      if (!(index in slots)) slots[index] = initial
      return [
        slots[index],
        (value) => {
          slots[index] = typeof value === 'function' ? value(slots[index]) : value
        }
      ]
    },
    useRef(initial) {
      return (slots[cursor++] ??= { current: initial })
    },
    useCallback(fn, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i])) slots[index] = { deps, fn }
      return slots[index].fn
    },
    useMemo(factory, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i]))
        slots[index] = { deps, value: factory() }
      return slots[index].value
    },
    useEffect(fn, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i])) {
        old?.cleanup?.()
        slots[index] = { deps }
        effects.push(() => {
          slots[index].cleanup = fn()
        })
      }
    }
  }
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/lib/use-paged-list.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  )
  const exports = {}
  runInNewContext(outputText, {
    exports,
    require: (name) =>
      name === '@/lib/runtime-cache' ? cache : name === 'react' ? react : { errorText: String }
  })
  const newLoader = () => (offset) =>
    new Promise((resolve, reject) => requests.push({ offset, resolve, reject }))
  let loader = newLoader()
  let size = 50,
    enabled = true,
    refreshKey = 0,
    reconcile,
    cacheKey = initialCacheKey
  const render = (flushEffects = true) => {
    cursor = 0
    const value = exports.usePagedList(loader, size, enabled, { refreshKey, reconcile, cacheKey })
    if (flushEffects) while (effects.length) effects.shift()()
    return value
  }
  return {
    render,
    requests,
    cache,
    switchLoader() {
      loader = newLoader()
    },
    configure(options) {
      size = options.size ?? size
      enabled = options.enabled ?? enabled
      reconcile = options.reconcile ?? reconcile
      cacheKey = options.cacheKey ?? cacheKey
    },
    refresh() {
      refreshKey++
      return render()
    },
    changeQuery() {
      loader = newLoader()
      render()
    }
  }
}
const settle = async () => {
  await Promise.resolve()
  await Promise.resolve()
}

test('switching collections to history never exposes collection rows as tracks before effects', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve({ items: [{ kind: 'playlist', id: 1 }], more: true })
  await settle()
  assert.equal(app.render().items.length, 1)
  app.switchLoader()
  const history = app.render(false)
  // TrackList reads source.kind during render, before the pagination reset effect.
  assert.doesNotThrow(() => history.items.some((track) => track.source.kind === 'netease'))
  assert.equal(history.items.length, 0)
})

test('repeated end-of-list notifications append each page once and stop at the end', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve({ items: ['first'], more: true })
  await settle()
  const list = app.render()
  const pending = list.loadMore()
  void list.loadMore()
  assert.equal(app.requests.length, 2)
  assert.equal(app.requests[1].offset, 50)
  assert.deepEqual([...app.render().items], ['first'])
  app.requests[1].resolve({ items: ['second'], more: false })
  await pending
  assert.deepEqual([...app.render().items], ['first', 'second'])
  await app.render().loadMore()
  assert.equal(app.requests.length, 2)
})

test('failed continuation preserves results and retries the same offset', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve({ items: ['first'], more: true })
  await settle()
  const pending = app.render().loadMore()
  app.requests[1].reject(new Error('offline'))
  await pending
  assert.deepEqual([...app.render().items], ['first'])
  assert.match(app.render().error, /offline/)
  const retry = app.render().loadMore()
  assert.equal(app.requests[2].offset, 50)
  app.requests[2].resolve({ items: ['second'], more: false })
  await retry
  assert.deepEqual([...app.render().items], ['first', 'second'])
})

test('query changes discard stale continuation responses', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve({ items: ['old'], more: true })
  await settle()
  const old = app.render().loadMore()
  app.changeQuery()
  app.requests[2].resolve({ items: ['new'], more: false })
  await settle()
  app.requests[1].resolve({ items: ['stale'], more: true })
  await old
  assert.deepEqual([...app.render().items], ['new'])
  assert.equal(app.render().more, false)
})

test('query changes hide metadata and errors immediately and block continuation before effects', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve({ items: ['old'], metadata: { title: 'old' }, more: true })
  await settle()
  const pending = app.render().loadMore()
  app.requests[1].reject(new Error('old error'))
  await pending
  assert.match(app.render().error, /old error/)
  app.switchLoader()
  const changed = app.render(false)
  assert.equal(changed.items.length, 0)
  assert.equal(changed.metadata, undefined)
  assert.equal(changed.error, undefined)
  assert.equal(changed.more, false)
  assert.equal(changed.busy, true)
  await changed.loadMore()
  assert.equal(app.requests.length, 2)
  app.render()
  assert.equal(app.requests[2].offset, 0)
})

test('disabling or changing page size hides old rows before effects and restarts at zero', async () => {
  for (const options of [{ enabled: false }, { size: 100 }]) {
    const app = harness()
    app.render()
    app.requests[0].resolve({ items: ['old'], more: true })
    await settle()
    app.configure(options)
    const changed = app.render(false)
    assert.equal(changed.items.length, 0)
    assert.equal(changed.busy, options.enabled !== false)
    await changed.loadMore()
    assert.equal(app.requests.length, 1)
    app.render()
    if (options.enabled === false) {
      assert.equal(app.requests.length, 1)
      app.configure({ enabled: true })
      app.render()
    }
    assert.equal(app.requests[1].offset, 0)
  }
})

test('late first-page success and failure cannot overwrite a replacement query', async () => {
  for (const fail of [false, true]) {
    const app = harness()
    app.render()
    app.changeQuery()
    app.requests[1].resolve({ items: ['new'], metadata: 'new detail', more: false })
    await settle()
    if (fail) app.requests[0].reject(new Error('stale failure'))
    else app.requests[0].resolve({ items: ['old'], metadata: 'old detail', more: true })
    await settle()
    const current = app.render()
    assert.deepEqual([...current.items], ['new'])
    assert.equal(current.metadata, 'new detail')
    assert.equal(current.error, undefined)
    assert.equal(current.more, false)
    assert.equal(current.busy, false)
  }
})

test('background refresh retains every loaded page until all replacements arrive and reconciles rows', async () => {
  const app = harness()
  const a = { key: 'a' },
    b = { key: 'b' }
  app.configure({
    reconcile: (previous, next) =>
      next.map((item) => previous.find((old) => old.key === item.key) ?? item)
  })
  app.render()
  app.requests[0].resolve({ items: [a], more: true })
  await settle()
  const continuation = app.render().loadMore()
  app.requests[1].resolve({ items: [b], metadata: 'old detail', more: true })
  await continuation
  const original = app.render().items
  app.refresh()
  assert.equal(app.render().items, original)
  assert.equal(app.render().metadata, 'old detail')
  assert.equal(app.render().busy, true)
  app.requests[2].resolve({ items: [{ key: 'new' }, { key: 'a' }], more: true })
  await settle()
  assert.equal(app.requests[3].offset, 50)
  assert.equal(app.render().items, original, 'first page cannot discard loaded continuation rows')
  app.requests[3].resolve({ items: [{ key: 'b' }], metadata: 'new detail', more: true })
  await settle()
  const refreshed = app.render()
  assert.deepEqual(
    [...refreshed.items].map((item) => item.key),
    ['new', 'a', 'b']
  )
  assert.equal(refreshed.items[1], a)
  assert.equal(refreshed.items[2], b)
  assert.equal(refreshed.metadata, 'new detail')
  const next = refreshed.loadMore()
  assert.equal(app.requests[4].offset, 100)
  app.requests[4].resolve({ items: [], more: false })
  await next
})

test('failed background refresh preserves all pages and retries the complete loaded range', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve({ items: ['first'], more: true })
  await settle()
  const next = app.render().loadMore()
  app.requests[1].resolve({ items: ['second'], more: true })
  await next
  const original = app.render().items
  app.refresh()
  app.requests[2].resolve({ items: ['replacement'], more: true })
  await settle()
  app.requests[3].reject(new Error('offline'))
  await settle()
  assert.equal(app.render().items, original)
  assert.match(app.render().error, /offline/)
  const retry = app.render().loadMore()
  assert.equal(app.requests[4].offset, 0)
  app.requests[4].resolve({ items: ['replacement'], more: true })
  await settle()
  assert.equal(app.requests[5].offset, 50)
  app.requests[5].resolve({ items: ['last'], more: false })
  await retry
  assert.deepEqual([...app.render().items], ['replacement', 'last'])
})

test('a newer refresh discards the old refresh and preserves the loaded depth', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve({ items: ['first'], more: true })
  await settle()
  const next = app.render().loadMore()
  app.requests[1].resolve({ items: ['second'], more: true })
  await next
  app.refresh()
  app.refresh()
  app.requests[2].resolve({ items: ['stale'], more: false })
  app.requests[3].resolve({ items: ['new first'], more: true })
  await settle()
  assert.equal(app.requests[4].offset, 50)
  assert.deepEqual([...app.render().items], ['first', 'second'])
  app.requests[4].resolve({ items: ['new second'], more: false })
  await settle()
  assert.deepEqual([...app.render().items], ['new first', 'new second'])
})

test('returning to a cached list displays its first page before effects and reloads its cursor', async () => {
  const cache = createRuntimeCache()
  const key = cache.presentationKey('songs', 'account-a')
  const first = harness(cache, key)
  assert.equal(first.render().busy, true)
  first.requests[0].resolve({ items: ['cached'], more: true, metadata: { total: 90 } })
  await settle()
  const second = harness(cache, key)
  const initial = second.render(false)
  assert.deepEqual([...initial.items], ['cached'])
  assert.equal(initial.metadata.total, 90)
  assert.equal(initial.busy, false)
  second.render()
  assert.equal(second.requests[0].offset, 0)
  void second.render().loadMore()
  assert.equal(second.requests.length, 1, 'wait for first-page cursor reconstruction')
  second.requests[0].resolve({ items: ['fresh'], more: true })
  await settle()
  const next = second.render().loadMore()
  assert.equal(second.requests[1].offset, 50)
  second.requests[1].resolve({ items: ['next'], more: false })
  await next
  assert.deepEqual([...second.render().items], ['fresh', 'next'])
})

test('cached empty lists do not flash a skeleton, and disabled or different queries never reuse them', () => {
  const cache = createRuntimeCache()
  const key = cache.presentationKey('songs', 'account-a')
  cache.requestCache.set(key, { items: [], more: false })
  const app = harness(cache, key)
  assert.equal(app.render().busy, false)
  assert.equal(app.render().busy, false)
  app.configure({ cacheKey: cache.presentationKey('songs', 'account-b') })
  assert.equal(app.render(false).busy, true)
  app.configure({ enabled: false, cacheKey: key })
  assert.equal(app.render(false).busy, false)
  assert.equal(app.render(false).items.length, 0)
})

test('invalidation clears display snapshots and prevents late requests from repopulating them', async () => {
  const cache = createRuntimeCache()
  const key = cache.presentationKey('songs', 'account-a')
  cache.requestCache.set(key, { items: ['old'], more: true })
  const app = harness(cache, key)
  app.render()
  cache.invalidateNativeCache(['music_query'])
  assert.equal(cache.requestCache.peekFresh(key), undefined)
  app.requests[0].resolve({ items: ['late'], more: false })
  await settle()
  assert.equal(cache.requestCache.peekFresh(key), undefined)
  const fresh = harness(cache, key)
  assert.equal(fresh.render(false).busy, true)
  assert.equal(fresh.render(false).items.length, 0)
})

test('expired display snapshots miss, and refreshing an expired entry starts a new TTL', async () => {
  const { RuntimeCache } = createRuntimeCache()
  const cache = new RuntimeCache(1024, 1)
  cache.set('presentation:ttl', { items: ['old'], more: false })
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(cache.peekFresh('presentation:ttl'), undefined)
  cache.set('presentation:ttl', { items: ['new'], more: false })
  assert.equal(cache.peekFresh('presentation:ttl').items[0], 'new')
})
