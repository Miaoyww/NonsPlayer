import { musicReference } from './helpers/music-reference.mjs'
import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInThisContext } from 'node:vm'
import { LRUCache } from 'lru-cache'
import ts from 'typescript'

function load(path, modules) {
  modules = {
    '@/features/music/cover-image': coverImage,
    '@/features/library/library-preview': {
      clearLibraryPreviews() {},
      peekLibraryPreview() {},
      saveLibraryPreview() {}
    },
    './library-preview': {
      clearLibraryPreviews() {},
      peekLibraryPreview() {},
      saveLibraryPreview() {}
    },
    ...modules
  }
  const exports = {}
  if (modules['@tauri-apps/api/core'])
    modules['@/lib/diagnostics'] = { diagnosticInvoke: modules['@tauri-apps/api/core'].invoke }
  const { outputText } = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS }
  })
  // Use the same Promise realm as LRUCache, so abort tests exercise real fetch semantics.
  runInThisContext(`(function(exports, require) { ${outputText}\n })`)(exports, (name) => {
    assert.ok(name in modules, `unexpected dependency: ${name}`)
    return modules[name]
  })
  return exports
}
const tick = () => new Promise((resolve) => setImmediate(resolve))
function harness() {
  const requests = []
  let scope = JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 1, 1])
  let enabled = true
  let coverEnabled = true
  let coverGeneration = 1
  const cache = load('../src/lib/runtime-cache.ts', { 'lru-cache': { LRUCache } })
  const scoped = load('../src/features/music/scoped-query.ts', { '@/lib/runtime-cache': cache })
  const player = load('../src/lib/player.ts', {
    '@/lib/platform': { isAndroid: () => false },
    '@/features/playback/player-store': load('../src/features/playback/player-store.ts', {}),
    '@/features/music/scoped-query': scoped,
    '@/lib/runtime-cache': cache,
    '@/features/discovery/private-fm': {},
    react: {},
    '@tauri-apps/api/event': {},
    '@tauri-apps/api/core': {
      isTauri: () => true,
      invoke(command, args) {
        if (command === 'runtime_cover_access')
          return coverEnabled
            ? Promise.resolve(String(coverGeneration))
            : Promise.reject(new Error('cover source unavailable'))
        if (command === 'music_context')
          return enabled ? Promise.resolve(scope) : Promise.reject(new Error('source unavailable'))
        return new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
      }
    }
  })
  const client = load('../src/features/music/client.ts', {
    '@/lib/player': player,
    '@/lib/runtime-cache': cache
  }).musicClient
  const library = load('../src/features/library/library-api.ts', {
    '@/features/music/client': { musicClient: client },
    '@/features/music/queries': { clearContinuations() {} },
    '@/features/music/entities': {},
    '@/features/music/legacy-reference': musicReference,
    '@/features/account/account': {},
    '@/lib/player': player,
    '@/lib/runtime-cache': cache
  })
  return {
    requests,
    disableCovers: () => {
      coverEnabled = false
    },
    reloadCovers: () => {
      coverGeneration++
    },
    cache,
    player,
    library,
    client,
    setScope: (value) => {
      scope = value
    },
    setEnabled: (value) => {
      enabled = value
    }
  }
}
const summary = {
  profile: { userId: 1, nickname: 'Saved', avatarUrl: '' },
  likedPlaylist: { id: 1, kind: 'playlist' },
  likedTracks: [],
  likedError: null
}

test('local file reads remain fresh and cannot replace cached online lyrics for the same track', async () => {
  const { player, requests } = harness()
  const args = { key: 'local:disney' }
  const online = { source: 'amll', format: 'ttml', content: 'words', translation: 'translation' }
  const stream = player.nativeCall('track_lyrics', args)
  await tick()
  requests.at(-1).resolve(online)
  assert.equal(await stream, online)
  for (const content of ['original', 'changed']) {
    const count = requests.length
    const pending = player.nativeCall('local_track_lyrics', args)
    await tick()
    assert.equal(requests.length, count + 1)
    await tick()
    requests.at(-1).resolve({ source: 'local', content })
    assert.equal((await pending).content, content)
  }
  assert.equal(await player.nativeCall('track_lyrics', args), online)
  await tick()
  assert.equal(requests.length, 3)
})

test('same-song local and streaming lyrics remain independent through a local-to-stream-to-local switch', async () => {
  const { player, requests } = harness()
  const local = { source: 'qq', format: 'lrc', content: 'line lyrics', translation: null }
  const stream = {
    source: 'amll',
    format: 'ttml',
    content: 'word lyrics',
    translation: 'translation'
  }
  const streamRead = player.nativeCall('track_lyrics', { key: 'ncm:disney' })
  await tick()
  requests.at(-1).resolve(stream)
  assert.equal(await streamRead, stream)
  const localRead = player.nativeCall('track_lyrics', { key: 'local:disney' })
  await tick()
  requests.at(-1).resolve(local)
  assert.equal(await localRead, local)
  assert.equal(await player.nativeCall('track_lyrics', { key: 'ncm:disney' }), stream)
  assert.equal(await player.nativeCall('track_lyrics', { key: 'local:disney' }), local)
  await tick()
  assert.equal(requests.length, 2)
})

test('Hitokoto requests coalesce and failures stay retryable', async () => {
  const { player, requests } = harness()
  const first = player.nativeCall('discovery_hitokoto')
  const duplicate = player.nativeCall('discovery_hitokoto')
  await tick()
  assert.equal(requests.length, 1)
  const failures = Promise.allSettled([first, duplicate])
  await tick()
  requests[0].reject(new Error('timeout'))
  assert.ok((await failures).every((value) => value.status === 'rejected'))
  const retry = player.nativeCall('discovery_hitokoto')
  await tick()
  assert.equal(requests.length, 2)
  await tick()
  requests[1].resolve('音乐相伴')
  assert.equal(await retry, '音乐相伴')
  assert.equal(await player.nativeCall('discovery_hitokoto'), '音乐相伴')
  await tick()
  assert.equal(requests.length, 2)
})

test('runtime TTL is ten minutes, not extended by reads, and capacity evicts old entries', async () => {
  const { cache } = harness()
  assert.equal(cache.RUNTIME_TTL, 600000)
  const tiny = new cache.RuntimeCache(150, 15)
  let calls = 0
  const read = () => tiny.get('query:a', async () => (++calls, 'a'))
  await read()
  await read()
  assert.equal(calls, 1)
  await new Promise((resolve) => setTimeout(resolve, 30))
  await read()
  assert.equal(calls, 2)
  await tiny.get('query:b', async () => 'b'.repeat(50))
  await tiny.get('query:c', async () => 'c'.repeat(50))
  assert.equal(tiny.peek('query:b'), undefined)
})

test('cover TTL expiry keeps the displayed data URI usable and a later read fetches again', async () => {
  const { cache, player, requests } = harness()
  cache.coverCache = new cache.RuntimeCache(32 * 1024 * 1024, 15)
  const args = { url: 'https://p1.music.126.net/album.jpg?param=512y512' }
  const first = player.nativeCall('runtime_cover', args)
  await tick()
  requests[0].resolve('data:image/jpeg;base64,displayed')
  const displayed = await first
  await new Promise((resolve) => setTimeout(resolve, 30))
  const renewed = player.nativeCall('runtime_cover', args)
  await tick()
  assert.equal(requests.length, 2)
  assert.equal(displayed, 'data:image/jpeg;base64,displayed')
  await tick()
  requests[1].resolve('data:image/jpeg;base64,renewed')
  assert.equal(await renewed, 'data:image/jpeg;base64,renewed')
})

test('lyrics refresh bypasses runtime cache and cover cache stays separate from account reset', async () => {
  const { player, library, requests } = harness()
  const lyrics = { format: 'ttml', content: '<tt><body/></tt>' }
  const first = player.nativeCall('track_lyrics', { key: 'netease:1', refresh: false })
  await tick()
  requests[0].resolve(lyrics)
  await first
  assert.equal(
    await player.nativeCall('track_lyrics', { refresh: false, key: 'netease:1' }),
    lyrics
  )
  const refresh = player.nativeCall('track_lyrics', { key: 'netease:1', refresh: true })
  await tick()
  requests[1].resolve(lyrics)
  await refresh
  const cover = player.nativeCall('runtime_cover', { url: 'https://p1.music.126.net/a.jpg' })
  await tick()
  requests[2].resolve('data:image/jpeg;base64,a')
  await cover
  await tick()
  library.resetAccountCache()
  assert.equal(
    await player.nativeCall('runtime_cover', { url: 'https://p1.music.126.net/a.jpg' }),
    'data:image/jpeg;base64,a'
  )
  await tick()
  assert.equal(requests.length, 3)
})

test('source settings invalidate cached and in-flight lyrics only after a successful write', async () => {
  const { player, requests } = harness()
  const read = player.nativeCall('track_lyrics', { key: 'netease:1' })
  await tick()
  requests.at(-1).resolve({ source: 'amll' })
  await read
  const failed = player.nativeCall('set_lyric_sources', { sources: { amll: false, qq: false } })
  const rejection = assert.rejects(failed, /denied/)
  await tick()
  requests.at(-1).reject(new Error('denied'))
  await rejection
  assert.equal((await player.nativeCall('track_lyrics', { key: 'netease:1' })).source, 'amll')
  const pending = player.nativeCall('track_lyrics', { key: 'netease:2' })
  const discarded = assert.rejects(pending, /缓存请求已失效/)
  const pendingRequest = requests.at(-1)
  const write = player.nativeCall('set_lyric_sources', { sources: { amll: false, qq: false } })
  await tick()
  requests.at(-1).resolve()
  await write
  pendingRequest.resolve({ source: 'qq' })
  await discarded
  for (const key of ['netease:1', 'netease:2']) {
    const count = requests.length
    const updated = player.nativeCall('track_lyrics', { key })
    await tick()
    assert.equal(requests.length, count + 1)
    await tick()
    requests.at(-1).resolve({ source: 'netease' })
    await updated
  }
})

test('unified queries recover after writes using the new scope and coalesce', async () => {
  const { client, requests, cache, setScope } = harness()
  const request = { operation: 'librarySummary' }
  const reads = Promise.all([client.query('netease', request), client.query('netease', request)])
  await tick()
  cache.invalidateNativeCache()
  const scope = JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 1, 2])
  setScope(scope)
  await tick()
  await tick()
  assert.equal(requests.length, 2)
  assert.equal(requests.at(-1).args.scope, scope)
  await tick()
  requests.at(-1).resolve(summary)
  assert.deepEqual(await reads, [summary, summary])
})

test('same-key refresh joins a replacement read instead of exposing deleted', async () => {
  const { client, requests } = harness()
  const first = client.query('qqmusic', {
    operation: 'search',
    keyword: 'q',
    kind: 'track',
    page: { cursor: null, limit: 5 }
  })
  await tick()
  const fresh = client.refresh('qqmusic', {
    operation: 'search',
    keyword: 'q',
    kind: 'track',
    page: { cursor: null, limit: 5 }
  })
  const result = Promise.all([first, fresh])
  await tick()
  await tick()
  assert.equal(requests.length, 2)
  await tick()
  requests.at(-1).resolve(['fresh'])
  assert.deepEqual(await result, [['fresh'], ['fresh']])
})

test('account and instance changes reject old reads rather than crossing identity', async () => {
  for (const scope of [
    JSON.stringify([{ source: 'netease', account: 'b', generation: 2 }, 1, 1]),
    JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 2, 1])
  ]) {
    const { client, requests, cache, setScope } = harness()
    const read = client.query('netease', { operation: 'librarySummary' })
    const failure = assert.rejects(read, /账号或音乐来源状态已变化/)
    await tick()
    setScope(scope)
    cache.invalidateNativeCache()
    await failure
    await tick()
    assert.equal(requests.length, 1)
  }
})

test('real errors remain errors and unrelated invalidation does not retry them', async () => {
  const { player, requests, cache } = harness()
  const read = player.nativeCall('music_library')
  const error = new Error('offline')
  const failure = assert.rejects(read, (cause) => cause === error)
  await tick()
  cache.invalidateNativeCache(['search_music'])
  await tick()
  requests.at(-1).reject(error)
  await failure
  await tick()
  assert.equal(requests.length, 1)
})

test('query recovery is bounded under continuous invalidation', async () => {
  const { client, requests, cache } = harness()
  const read = client.query('netease', { operation: 'librarySummary' })
  const failure = assert.rejects(read, /频繁变化/)
  for (let i = 0; i < 3; i++) {
    await tick()
    cache.invalidateNativeCache()
  }
  await failure
  await tick()
  assert.equal(requests.length, 3)
})

test('write revision changes cannot silently resume an old pagination cursor', async () => {
  const { client, requests, cache, setScope } = harness()
  const read = client.query('netease', { operation: 'tracks', page: { cursor: 'old', limit: 100 } })
  const failure = assert.rejects(read, /账号或音乐来源状态已变化/)
  await tick()
  setScope(JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 1, 2]))
  cache.invalidateNativeCache()
  await failure
  await tick()
  assert.equal(requests.length, 1)
})

test('completed reads are revalidated after a write before publication', async () => {
  const { client, requests, setScope } = harness()
  const read = client.query('netease', { operation: 'librarySummary' })
  await tick()
  setScope(JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 1, 2]))
  requests.at(-1).resolve(summary)
  await tick()
  assert.equal(requests.length, 2)
  const updated = { ...summary, likedTracks: ['updated'] }
  requests.at(-1).resolve(updated)
  assert.equal(await read, updated)
})

test('structured stale-context errors recover but network errors never trigger replay', async () => {
  const { client, requests, setScope } = harness()
  const read = client.getTrack({ source: 'netease', kind: 'track', id: '1' })
  await tick()
  setScope(JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 1, 2]))
  requests.at(-1).reject({ code: 'staleContext' })
  await tick()
  assert.equal(requests.length, 2)
  requests.at(-1).resolve({ title: 'new' })
  assert.equal((await read).title, 'new')
})

test('invalidation during asynchronous cache validation prevents old delivery', async () => {
  const { cache } = harness()
  const runtime = new cache.RuntimeCache()
  let finish
  const validating = new Promise((resolve) => {
    finish = resolve
  })
  const read = runtime.get('query:a', async () => 'old', { validate: () => validating })
  const failed = assert.rejects(read, (cause) => cause instanceof cache.CacheInvalidatedError)
  await tick()
  runtime.delete('query:a')
  assert.equal(await runtime.get('query:a', async () => 'new'), 'new')
  finish()
  await failed
  assert.equal(runtime.peek('query:a'), 'new')
})

test('queue mutations are never replayed after their scope changes', async () => {
  const { client, requests, setScope } = harness()
  const queued = client.queueTrack({ source: 'netease', kind: 'track', id: '1' })
  const failed = assert.rejects(queued)
  await tick()
  setScope(JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 1, 2]))
  requests.at(-1).resolve({ key: 'netease:1' })
  await failed
  assert.equal(requests.length, 1)
})

test('concurrent explicit refreshes share one replacement backend request', async () => {
  const { client, requests } = harness()
  const read = client.query('qqmusic', {
    operation: 'search',
    keyword: 'q',
    kind: 'track',
    page: { cursor: null, limit: 5 }
  })
  await tick()
  const refreshes = [
    client.refresh('qqmusic', {
      operation: 'search',
      keyword: 'q',
      kind: 'track',
      page: { cursor: null, limit: 5 }
    }),
    client.refresh('qqmusic', {
      operation: 'search',
      keyword: 'q',
      kind: 'track',
      page: { cursor: null, limit: 5 }
    })
  ]
  const all = Promise.all([read, ...refreshes])
  await tick()
  assert.equal(requests.length, 2)
  requests.at(-1).resolve(['fresh'])
  assert.deepEqual(await all, [['fresh'], ['fresh'], ['fresh']])
})

test('public cover cache hits recheck adapter permissions and old deliveries fail after reload', async () => {
  const h = harness()
  const args = { url: 'https://covers.example/card.jpg' }
  const first = h.player.nativeCall('runtime_cover', args)
  await tick()
  h.requests[0].resolve('data:image/jpeg;base64,one')
  assert.equal(await first, 'data:image/jpeg;base64,one')
  h.disableCovers()
  await assert.rejects(h.player.nativeCall('runtime_cover', args), /cover source unavailable/)
  assert.equal(h.requests.length, 1)
  const next = harness()
  const pending = next.player.nativeCall('runtime_cover', args)
  await tick()
  next.reloadCovers()
  next.requests[0].resolve('data:image/jpeg;base64,old')
  await assert.rejects(pending, /封面来源状态已变化/)
})

test('reloading an adapter replaces an older public cover entry before returning a cache hit', async () => {
  const h = harness()
  const args = { url: 'https://covers.example/album.jpg' }
  const first = h.player.nativeCall('runtime_cover', args)
  await tick()
  h.requests[0].resolve('data:image/jpeg;base64,first')
  await first
  h.reloadCovers()
  const next = h.player.nativeCall('runtime_cover', args)
  await tick()
  assert.equal(h.requests.length, 2)
  h.requests[1].resolve('data:image/jpeg;base64,reloaded')
  assert.equal(await next, 'data:image/jpeg;base64,reloaded')
})

for (const operation of [
  'detail',
  'artistTracks',
  'artistAlbums',
  'recommendedPlaylists',
  'librarySummary',
  'libraryCollections',
  'trackInformation'
]) {
  test(`uniform ${operation} reads coalesce and separate sources and request fields`, async () => {
    const { client, requests } = harness()
    const request = { operation, marker: 'a' }
    const reads = Promise.all([client.query('qqmusic', request), client.query('qqmusic', request)])
    await tick()
    assert.equal(requests.length, 1)
    requests[0].resolve({ type: 'fixture', data: operation })
    const values = await reads
    assert.equal(values[0], values[1])
    assert.equal(await client.query('qqmusic', request), values[0])
    for (const [source, marker] of [
      ['netease', 'a'],
      ['qqmusic', 'b']
    ]) {
      const next = client.query(source, { operation, marker })
      await tick()
      assert.equal(requests.at(-1).args.source, source)
      requests.at(-1).resolve({ type: 'fixture', data: marker })
      await next
    }
    assert.equal(requests.length, 3)
  })
}
for (const operation of [
  'setFavorite',
  'removePlaylistTrack',
  'addPlaylistTrack',
  'updatePlaylist',
  'deletePlaylist',
  'createPlaylist',
  'dislike'
]) {
  test(`uniform ${operation} invalidates reads only after a successful write`, async () => {
    const { client, requests } = harness()
    const request = { operation: 'librarySummary' }
    const initial = client.query('qqmusic', request)
    await tick()
    const value = { type: 'summary', data: { marker: 'old' } }
    requests.at(-1).resolve(value)
    await initial
    const failed = client.write('qqmusic', { operation })
    const rejected = assert.rejects(failed, /offline/)
    await tick()
    requests.at(-1).reject(new Error('offline'))
    await rejected
    assert.equal(await client.query('qqmusic', request), value)
    const success = client.write('qqmusic', { operation })
    await tick()
    requests.at(-1).resolve({ type: 'write', data: null })
    await success
    const before = requests.length
    const fresh = client.query('qqmusic', request)
    await tick()
    assert.equal(requests.length, before + 1)
    requests.at(-1).resolve({ type: 'summary', data: { marker: 'new' } })
    await fresh
  })
}
