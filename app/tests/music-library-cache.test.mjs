import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import { LRUCache } from 'lru-cache'
import ts from 'typescript'

function load(path, modules) {
  const exports = {}
  const { outputText } = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS }
  })
  runInNewContext(outputText, {
    exports,
    require: (name) => {
      assert.ok(name in modules, `unexpected dependency: ${name}`)
      return modules[name]
    }
  })
  return exports
}
function harness() {
  const requests = []
  let scope = 'account-a:instance-1'
  let enabled = true
  const cache = load('../src/lib/runtime-cache.ts', { 'lru-cache': { LRUCache } })
  const player = load('../src/lib/player.ts', {
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
    setScope: (value) => {
      scope = value
    },
    setEnabled: (value) => {
      enabled = value
    }
  }
}
const summary = { likedPlaylist: { id: 1, kind: 'playlist' }, likedTracks: [], likedError: null }

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
  requests.at(-1).resolve(stream)
  assert.equal(await streamRead, stream)
  const localRead = player.nativeCall('track_lyrics', { key: 'local:disney' })
  requests.at(-1).resolve(local)
  assert.equal(await localRead, local)
  assert.equal(await player.nativeCall('track_lyrics', { key: 'ncm:disney' }), stream)
  assert.equal(await player.nativeCall('track_lyrics', { key: 'local:disney' }), local)
  assert.equal(requests.length, 2)
})

test('Hitokoto requests coalesce and failures stay retryable', async () => {
  const { player, requests } = harness()
  const first = player.nativeCall('discovery_hitokoto')
  const duplicate = player.nativeCall('discovery_hitokoto')
  assert.equal(requests.length, 1)
  const failures = Promise.allSettled([first, duplicate])
  requests[0].reject(new Error('timeout'))
  assert.ok((await failures).every((value) => value.status === 'rejected'))
  const retry = player.nativeCall('discovery_hitokoto')
  assert.equal(requests.length, 2)
  requests[1].resolve('音乐相伴')
  assert.equal(await retry, '音乐相伴')
  assert.equal(await player.nativeCall('discovery_hitokoto'), '音乐相伴')
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
    assert.equal(requests.length, before + 1)
    requests.at(-1).resolve({ items: [], more: false })
    assert.equal(await first, await duplicate)
    await player.nativeCall(command, args)
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
  assert.equal(requests.length, 1)
  requests.at(-1).resolve({ items: [{ id: 1, playCount: 25000 }], more: true })
  assert.equal(await first, await duplicate)
  for (const changed of [{ category: '华语' }, { order: 'new' }, { offset: 30 }]) {
    const before = requests.length
    const read = player.nativeCall('discovery_playlists', { ...args, ...changed })
    assert.equal(requests.length, before + 1)
    requests.at(-1).resolve({ items: [], more: false })
    await read
  }
})

test('FM refresh bypasses cached batches and dislikes invalidate only after success', async () => {
  const { player, requests } = harness()
  const args = { kind: 'fm' }
  const initial = player.nativeCall('discovery_tracks', args)
  requests.at(-1).resolve([{ key: 'netease:1' }])
  await initial
  const refresh = player.nativeCall('discovery_tracks', { ...args, refresh: true })
  assert.equal(requests.length, 2)
  requests.at(-1).resolve([{ key: 'netease:2' }])
  await refresh
  const failed = player.nativeCall('discovery_dislike', { id: 2 })
  const rejection = assert.rejects(failed, /denied/)
  requests.at(-1).reject(new Error('denied'))
  await rejection
  assert.equal((await player.nativeCall('discovery_tracks', args))[0].key, 'netease:2')
  const write = player.nativeCall('discovery_dislike', { id: 2 })
  requests.at(-1).resolve()
  await write
  const before = requests.length
  const read = player.nativeCall('discovery_tracks', args)
  assert.equal(requests.length, before + 1)
  requests.at(-1).resolve([{ key: 'netease:3' }])
  await read
})

test('native reads coalesce by canonical arguments and reuse the same summary and detail', async () => {
  const { library, requests } = harness()
  const first = library.getMusicLibrary(1),
    duplicate = library.getMusicLibrary(1)
  assert.equal(requests.length, 1)
  requests[0].resolve(summary)
  assert.equal(await first, summary)
  assert.equal(await duplicate, summary)
  assert.equal(await library.getMusicLibrary(1), summary)
  const page = library.getLibraryTracks(summary.likedPlaylist, 0, 1)
  const same = library.getLibraryTracks(summary.likedPlaylist, 0, 1)
  assert.equal(requests.length, 2)
  requests[1].resolve({ tracks: [], total: 0, more: false })
  await page
  await same
  await library.getLibraryTracks(summary.likedPlaylist, 0, 1)
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
  requests[0].resolve('data:image/jpeg;base64,displayed')
  const displayed = await first
  await new Promise((resolve) => setTimeout(resolve, 30))
  const renewed = player.nativeCall('runtime_cover', args)
  assert.equal(requests.length, 2)
  assert.equal(displayed, 'data:image/jpeg;base64,displayed')
  requests[1].resolve('data:image/jpeg;base64,renewed')
  assert.equal(await renewed, 'data:image/jpeg;base64,renewed')
})

test('failed requests and incomplete summaries stay retryable', async () => {
  const { library, requests } = harness()
  const first = library.getMusicLibrary(1),
    failure = assert.rejects(first, /offline/)
  requests[0].reject(new Error('offline'))
  await failure
  const second = library.getMusicLibrary(1),
    partial = assert.rejects(second, /offline/)
  requests[1].resolve({ ...summary, likedError: 'offline' })
  await partial
  assert.equal(library.peekMusicLibrary(1), undefined)
  const third = library.getMusicLibrary(1)
  requests[2].resolve(summary)
  await third
})

test('account changes and invalidation discard in-flight responses', async () => {
  const { library, requests } = harness()
  const old = library.getMusicLibrary(1),
    invalidated = assert.rejects(old)
  library.resetAccountCache()
  const fresh = library.getMusicLibrary(2)
  requests[0].resolve(summary)
  await invalidated
  assert.equal(library.peekMusicLibrary(2), undefined)
  requests[1].resolve(summary)
  await fresh
  assert.equal(library.peekMusicLibrary(1), undefined)
})

test('successful writes invalidate related reads; failures preserve cached data', async () => {
  const { player, library, requests } = harness()
  const initial = library.getMusicLibrary(1)
  requests[0].resolve(summary)
  await initial
  const bad = player.nativeCall('set_song_liked', { id: 1, liked: true })
  const failure = assert.rejects(bad, /offline/)
  requests[1].reject(new Error('offline'))
  await failure
  assert.equal(await library.getMusicLibrary(1), summary)
  const good = player.nativeCall('set_song_liked', { id: 1, liked: true })
  requests[2].resolve()
  await good
  assert.equal(library.peekMusicLibrary(1), undefined)
  const refreshed = library.getMusicLibrary(1)
  assert.equal(requests.length, 4)
  requests[3].resolve(summary)
  await refreshed
})

test('lyrics refresh bypasses runtime cache and cover cache stays separate from account reset', async () => {
  const { player, library, requests } = harness()
  const lyrics = { format: 'ttml', content: '<tt><body/></tt>' }
  const first = player.nativeCall('track_lyrics', { key: 'netease:1', refresh: false })
  requests[0].resolve(lyrics)
  await first
  assert.equal(
    await player.nativeCall('track_lyrics', { refresh: false, key: 'netease:1' }),
    lyrics
  )
  const refresh = player.nativeCall('track_lyrics', { key: 'netease:1', refresh: true })
  requests[1].resolve(lyrics)
  await refresh
  const cover = player.nativeCall('runtime_cover', { url: 'https://p1.music.126.net/a.jpg' })
  requests[2].resolve('data:image/jpeg;base64,a')
  await cover
  library.resetAccountCache()
  assert.equal(
    await player.nativeCall('runtime_cover', { url: 'https://p1.music.126.net/a.jpg' }),
    'data:image/jpeg;base64,a'
  )
  assert.equal(requests.length, 3)
})

test('song details coalesce, and playlist deletion invalidates collection and likes caches only on success', async () => {
  const { player, requests } = harness()
  const first = player.nativeCall('song_information', { key: 'netease:1' })
  const duplicate = player.nativeCall('song_information', { key: 'netease:1' })
  assert.equal(requests.length, 1)
  requests[0].resolve({ albumId: 3 })
  await Promise.all([first, duplicate])
  const reads = ['music_library', 'library_tracks', 'library_collections', 'liked_song_ids']
  const data = []
  for (const command of reads) {
    const read = player.nativeCall(command)
    data.push({ command })
    requests.at(-1).resolve(data.at(-1))
    await read
  }
  const failed = player.nativeCall('remove_playlist_song', { playlistId: 1, songId: 1 })
  const rejection = assert.rejects(failed, /denied/)
  requests.at(-1).reject(new Error('denied'))
  await rejection
  for (const [index, command] of reads.entries())
    assert.equal(await player.nativeCall(command), data[index])
  const deletion = player.nativeCall('remove_playlist_song', { playlistId: 1, songId: 1 })
  requests.at(-1).resolve()
  await deletion
  for (const command of reads) {
    const before = requests.length
    const refresh = player.nativeCall(command)
    assert.equal(requests.length, before + 1)
    requests.at(-1).resolve({ fresh: true })
    await refresh
  }
  assert.equal((await player.nativeCall('song_information', { key: 'netease:1' })).albumId, 3)
})

test('source settings invalidate cached and in-flight lyrics only after a successful write', async () => {
  const { player, requests } = harness()
  const read = player.nativeCall('track_lyrics', { key: 'netease:1' })
  requests.at(-1).resolve({ source: 'amll' })
  await read
  const failed = player.nativeCall('set_lyric_sources', { sources: { amll: false, qq: false } })
  const rejection = assert.rejects(failed, /denied/)
  requests.at(-1).reject(new Error('denied'))
  await rejection
  assert.equal((await player.nativeCall('track_lyrics', { key: 'netease:1' })).source, 'amll')
  const pending = player.nativeCall('track_lyrics', { key: 'netease:2' })
  const discarded = assert.rejects(pending, /deleted/)
  const pendingRequest = requests.at(-1)
  const write = player.nativeCall('set_lyric_sources', { sources: { amll: false, qq: false } })
  requests.at(-1).resolve()
  await write
  pendingRequest.resolve({ source: 'qq' })
  await discarded
  for (const key of ['netease:1', 'netease:2']) {
    const count = requests.length
    const updated = player.nativeCall('track_lyrics', { key })
    assert.equal(requests.length, count + 1)
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
      requests.at(-1).resolve({ marker: 'old' })
      await pending
    }
    const failed = player.nativeCall(command, { id: 1 })
    const rejection = assert.rejects(failed, /offline/)
    requests.at(-1).reject(new Error('offline'))
    await rejection
    const before = requests.length
    for (const read of reads) assert.equal((await player.nativeCall(read)).marker, 'old')
    assert.equal(requests.length, before)
    const write = player.nativeCall(command, { id: 1 })
    requests.at(-1).resolve()
    await write
    for (const read of reads) {
      const count = requests.length
      const pending = player.nativeCall(read)
      assert.equal(requests.length, count + 1)
      requests.at(-1).resolve({ marker: 'new' })
      await pending
    }
  })
}

test('compatibility cache cannot bypass disabled sources and reloads after an instance change', async () => {
  const app = harness()
  const initial = app.player.nativeCall('search_music', { keyword: 'q', offset: 0 })
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
  assert.equal(app.requests.length, 2)
  app.requests.at(-1).resolve([{ key: 'netease:2' }])
  assert.equal((await refreshed)[0].key, 'netease:2')
})
