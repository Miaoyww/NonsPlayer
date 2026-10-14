import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInThisContext } from 'node:vm'
import { LRUCache } from 'lru-cache'
import ts from 'typescript'

function load(path, modules) {
  modules = { '@/features/music/cover-image': coverImage, ...modules }
  const exports = {}
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
    '@/features/music/client': { musicClient: {} },
    '@/lib/player': player,
    '@/lib/runtime-cache': cache
  })
  return {
    requests,
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
const summary = { likedPlaylist: { id: 1, kind: 'playlist' }, likedTracks: [], likedError: null }

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

test('artist and album reads coalesce while preserving entity, type and page boundaries', async () => {
  const { player, requests } = harness()
  for (const [command, args] of [
    ['music_entity_detail', { kind: 'artist', id: 1 }],
    ['music_entity_detail', { kind: 'album', id: 1 }],
    ['artist_tracks', { id: 1, offset: 0 }],
    ['artist_tracks', { id: 1, offset: 100 }],
    ['artist_albums', { id: 1, offset: 0 }],
    ['artist_albums', { id: 2, offset: 0 }],
    ['artist_albums', { id: 1, offset: 30 }]
  ]) {
    const before = requests.length
    const first = player.nativeCall(command, args)
    const duplicate = player.nativeCall(command, args)
    await tick()
    assert.equal(requests.length, before + 1)
    await tick()
    requests.at(-1).resolve({ items: [], more: false })
    assert.equal(await first, await duplicate)
    await player.nativeCall(command, args)
    await tick()
    assert.equal(requests.length, before + 1)
  }
})

test('discovery reads coalesce and keep category, order and page results separate', async () => {
  const { player, requests } = harness()
  const args = { section: 'square', category: '日语', order: 'hot', offset: 0 }
  const first = player.nativeCall('discovery_playlists', args)
  const duplicate = player.nativeCall('discovery_playlists', {
    offset: 0,
    order: 'hot',
    category: '日语',
    section: 'square'
  })
  await tick()
  assert.equal(requests.length, 1)
  await tick()
  requests.at(-1).resolve({ items: [{ id: 1, playCount: 25000 }], more: true })
  assert.equal(await first, await duplicate)
  for (const changed of [{ category: '华语' }, { order: 'new' }, { offset: 30 }]) {
    const before = requests.length
    const read = player.nativeCall('discovery_playlists', { ...args, ...changed })
    await tick()
    assert.equal(requests.length, before + 1)
    await tick()
    requests.at(-1).resolve({ items: [], more: false })
    await read
  }
})

test('FM refresh bypasses cached batches and dislikes invalidate only after success', async () => {
  const { player, requests } = harness()
  const args = { kind: 'fm' }
  const initial = player.nativeCall('discovery_tracks', args)
  await tick()
  requests.at(-1).resolve([{ key: 'netease:1' }])
  await initial
  const refresh = player.nativeCall('discovery_tracks', { ...args, refresh: true })
  await tick()
  assert.equal(requests.length, 2)
  await tick()
  requests.at(-1).resolve([{ key: 'netease:2' }])
  await refresh
  const failed = player.nativeCall('discovery_dislike', { id: 2 })
  const rejection = assert.rejects(failed, /denied/)
  await tick()
  requests.at(-1).reject(new Error('denied'))
  await rejection
  assert.equal((await player.nativeCall('discovery_tracks', args))[0].key, 'netease:2')
  const write = player.nativeCall('discovery_dislike', { id: 2 })
  await tick()
  requests.at(-1).resolve()
  await write
  const before = requests.length
  const read = player.nativeCall('discovery_tracks', args)
  await tick()
  assert.equal(requests.length, before + 1)
  await tick()
  requests.at(-1).resolve([{ key: 'netease:3' }])
  await read
})

test('native reads coalesce by canonical arguments and reuse the same summary and detail', async () => {
  const { library, requests } = harness()
  const first = library.getMusicLibrary(1),
    duplicate = library.getMusicLibrary(1)
  await tick()
  assert.equal(requests.length, 1)
  await tick()
  requests[0].resolve(summary)
  assert.equal(await first, summary)
  assert.equal(await duplicate, summary)
  assert.equal(await library.getMusicLibrary(1), summary)
  const page = library.getLibraryTracks(summary.likedPlaylist, 0, 1)
  const same = library.getLibraryTracks(summary.likedPlaylist, 0, 1)
  await tick()
  assert.equal(requests.length, 2)
  await tick()
  requests[1].resolve({ tracks: [], total: 0, more: false })
  await page
  await same
  await library.getLibraryTracks(summary.likedPlaylist, 0, 1)
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

test('failed requests and incomplete summaries stay retryable', async () => {
  const { library, requests } = harness()
  const first = library.getMusicLibrary(1),
    failure = assert.rejects(first, /offline/)
  await tick()
  requests[0].reject(new Error('offline'))
  await failure
  const second = library.getMusicLibrary(1),
    partial = assert.rejects(second, /offline/)
  await tick()
  requests[1].resolve({ ...summary, likedError: 'offline' })
  await partial
  assert.equal(library.peekMusicLibrary(1), undefined)
  const third = library.getMusicLibrary(1)
  await tick()
  requests[2].resolve(summary)
  await third
})

test('account changes and invalidation discard in-flight responses', async () => {
  const { library, requests } = harness()
  const old = library.getMusicLibrary(1),
    invalidated = assert.rejects(old)
  await tick()
  library.resetAccountCache()
  const fresh = library.getMusicLibrary(2)
  await tick()
  requests[0].resolve(summary)
  await invalidated
  assert.equal(library.peekMusicLibrary(2), undefined)
  await tick()
  requests[1].resolve(summary)
  await fresh
  assert.equal(library.peekMusicLibrary(1), undefined)
})

test('successful writes invalidate related reads; failures preserve cached data', async () => {
  const { player, library, requests } = harness()
  const initial = library.getMusicLibrary(1)
  await tick()
  requests[0].resolve(summary)
  await initial
  const bad = player.nativeCall('set_song_liked', { id: 1, liked: true })
  const failure = assert.rejects(bad, /offline/)
  await tick()
  requests[1].reject(new Error('offline'))
  await failure
  assert.equal(await library.getMusicLibrary(1), summary)
  const good = player.nativeCall('set_song_liked', { id: 1, liked: true })
  await tick()
  requests[2].resolve()
  await good
  assert.equal(library.peekMusicLibrary(1), undefined)
  const refreshed = library.getMusicLibrary(1)
  await tick()
  assert.equal(requests.length, 4)
  await tick()
  requests[3].resolve(summary)
  await refreshed
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

test('song details coalesce, and playlist deletion invalidates collection and likes caches only on success', async () => {
  const { player, requests } = harness()
  const first = player.nativeCall('song_information', { key: 'netease:1' })
  const duplicate = player.nativeCall('song_information', { key: 'netease:1' })
  await tick()
  assert.equal(requests.length, 1)
  await tick()
  requests[0].resolve({ albumId: 3 })
  await Promise.all([first, duplicate])
  const reads = ['music_library', 'library_tracks', 'library_collections', 'liked_song_ids']
  const data = []
  for (const command of reads) {
    const read = player.nativeCall(command)
    data.push({ command })
    await tick()
    requests.at(-1).resolve(data.at(-1))
    await read
  }
  const failed = player.nativeCall('remove_playlist_song', { playlistId: 1, songId: 1 })
  const rejection = assert.rejects(failed, /denied/)
  await tick()
  requests.at(-1).reject(new Error('denied'))
  await rejection
  for (const [index, command] of reads.entries())
    assert.equal(await player.nativeCall(command), data[index])
  const deletion = player.nativeCall('remove_playlist_song', { playlistId: 1, songId: 1 })
  await tick()
  requests.at(-1).resolve()
  await deletion
  for (const command of reads) {
    const before = requests.length
    const refresh = player.nativeCall(command)
    await tick()
    assert.equal(requests.length, before + 1)
    await tick()
    requests.at(-1).resolve({ fresh: true })
    await refresh
  }
  assert.equal((await player.nativeCall('song_information', { key: 'netease:1' })).albumId, 3)
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

for (const command of ['add_playlist_song', 'update_library_playlist', 'delete_library_playlist']) {
  test(`${command} invalidates playlist reads after success and preserves them on failure`, async () => {
    const { player, requests } = harness()
    const reads = [
      'music_library',
      'library_collections',
      'library_tracks',
      ...(command === 'add_playlist_song' ? ['liked_song_ids'] : ['discovery_playlists'])
    ]
    for (const read of reads) {
      const pending = player.nativeCall(read)
      await tick()
      requests.at(-1).resolve({ marker: 'old' })
      await pending
    }
    const failed = player.nativeCall(command, { id: 1 })
    const rejection = assert.rejects(failed, /offline/)
    await tick()
    requests.at(-1).reject(new Error('offline'))
    await rejection
    const before = requests.length
    for (const read of reads) assert.equal((await player.nativeCall(read)).marker, 'old')
    await tick()
    assert.equal(requests.length, before)
    const write = player.nativeCall(command, { id: 1 })
    await tick()
    requests.at(-1).resolve()
    await write
    for (const read of reads) {
      const count = requests.length
      const pending = player.nativeCall(read)
      await tick()
      assert.equal(requests.length, count + 1)
      await tick()
      requests.at(-1).resolve({ marker: 'new' })
      await pending
    }
  })
}

test('compatibility cache cannot bypass disabled sources and reloads after an instance change', async () => {
  const app = harness()
  const initial = app.player.nativeCall('search_music', { keyword: 'q', offset: 0 })
  await tick()
  app.requests.at(-1).resolve([{ key: 'netease:1' }])
  await initial
  app.setEnabled(false)
  await assert.rejects(
    app.player.nativeCall('search_music', { keyword: 'q', offset: 0 }),
    /source unavailable/
  )
  app.setEnabled(true)
  app.setScope('account-a:instance-3')
  const refreshed = app.player.nativeCall('search_music', { keyword: 'q', offset: 0 })
  await new Promise((resolve) => setImmediate(resolve))
  await tick()
  assert.equal(app.requests.length, 2)
  await tick()
  app.requests.at(-1).resolve([{ key: 'netease:2' }])
  assert.equal((await refreshed)[0].key, 'netease:2')
})

test('like revision refresh coalesces concurrent library readers without deleted', async () => {
  const { library, requests, setScope } = harness()
  const first = library.getMusicLibrary(1)
  await tick()
  await tick()
  requests.at(-1).resolve(summary)
  await first
  setScope(JSON.stringify([{ source: 'netease', account: 'a', generation: 1 }, 1, 2]))
  library.invalidateMusicLibrary()
  const reads = [library.getMusicLibrary(1), library.getMusicLibrary(1)]
  await tick()
  await tick()
  requests.at(-1).resolve(summary)
  assert.deepEqual(await Promise.all(reads), [summary, summary])
  await tick()
  assert.equal(requests.length, 2)
})

test('event invalidation restarts shared library reads and cannot cache their old result', async () => {
  const { library, requests, cache } = harness()
  const reads = [library.getMusicLibrary(1), library.getMusicLibrary(1)]
  const results = Promise.all(reads)
  await tick()
  const old = requests.at(-1)
  cache.invalidateNativeCache()
  await tick()
  await tick()
  assert.equal(requests.length, 2)
  const updated = { ...summary, likedTracks: ['new'] }
  await tick()
  requests.at(-1).resolve(updated)
  assert.deepEqual(await results, [updated, updated])
  old.resolve(summary)
  await tick()
  assert.equal(library.peekMusicLibrary(1), updated)
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
  const { player, requests } = harness()
  const first = player.nativeCall('search_music', { keyword: 'q', offset: 0 })
  await tick()
  const fresh = player.nativeCall('search_music', { keyword: 'q', offset: 0, refresh: true })
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
    const { library, requests, cache, setScope } = harness()
    const read = library.getMusicLibrary(1)
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
  const { library, requests, cache } = harness()
  const read = library.getMusicLibrary(1)
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
  const { library, requests, setScope } = harness()
  const read = library.getMusicLibrary(1)
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
  const { player, requests } = harness()
  const read = player.nativeCall('search_music', { keyword: 'q', offset: 0 })
  await tick()
  const refreshes = [
    player.nativeCall('search_music', { keyword: 'q', offset: 0, refresh: true }),
    player.nativeCall('search_music', { keyword: 'q', offset: 0, refresh: true })
  ]
  const all = Promise.all([read, ...refreshes])
  await tick()
  assert.equal(requests.length, 2)
  requests.at(-1).resolve(['fresh'])
  assert.deepEqual(await all, [['fresh'], ['fresh'], ['fresh']])
})
