// Executes the real composed JS/Rust Component. Node is a test runner only.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFile, mkdir } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const root = resolve(project, '../..')
const tools = resolve(root, 'packages/wasm-crypto/node_modules/@bytecodealliance')
const output = resolve(root, '.local/qq-component-tests')
await mkdir(output, { recursive: true })
const result = spawnSync(
  process.execPath,
  [
    resolve(tools, 'jco/dist/jco.js'),
    'transpile',
    resolve(project, 'dist/adapter.component.wasm'),
    '-o',
    output,
    '--instantiation',
    'async'
  ],
  { encoding: 'utf8' }
)
assert.equal(result.status, 0, result.stderr)
const { instantiate } = await import(pathToFileURL(resolve(output, 'adapter.component.js')).href)
const { WASIShim } = await import(
  pathToFileURL(resolve(tools, 'preview2-shim/dist/common/instantiation.js')).href
)
const png = Buffer.from('89504e470d0a1a0a00000000', 'hex')
const fixtureTrack = {
  mid: 'SongMID',
  id: 42,
  title: '测试曲目',
  interval: 123,
  singer: [{ name: '歌手', mid: 'ArtistMID' }],
  album: { mid: 'AlbumMID', name: '专辑' },
  file: { media_mid: 'AudioMID', size_128mp3: 4096, size_320mp3: 4096, size_flac: 4096 }
}
async function component({
  credential = null,
  rejectRefresh = false,
  searchRejections = 0,
  searchFailureCode = 2001,
  searchFailureLevel = 'root',
  resource = 'full',
  flacSize = 4096,
  songDetail = fixtureTrack,
  playlistSongs = [fixtureTrack],
  createdPlaylists = [
    {
      tid: 123,
      dirName: 'Private playlist',
      dirId: 1,
      dirShow: 2,
      logo: 'https://y.gtimg.cn/private.jpg'
    }
  ]
} = {}) {
  const frames = [],
    requests = [],
    states = new Map(),
    sockets = new Set(),
    diagnostics = []
  let subscribed = false,
    searchCalls = 0,
    confirmed = false,
    published = null,
    replaced = null,
    invalidated = false
  const transport = {
    httpRequest(request) {
      requests.push(request)
      // Reproduces the real Host rule that rejected automatic Request framing.
      if (
        request.headers.some((header) =>
          ['content-length', 'host', 'transfer-encoding'].includes(header.name.toLowerCase())
        )
      )
        throw 'framingDenied'
      const url = new URL(request.url)
      if (url.hostname.endsWith('.stream.qqmusic.qq.com'))
        return {
          status: resource === 'forbidden' ? 403 : 200,
          headers: [{ name: 'content-length', value: resource === 'unknown' ? '3' : '4096' }],
          body: new Uint8Array()
        }
      let body,
        status = 200
      if (url.hostname === 'api.tencentmusic.com')
        body = {
          code: 0,
          data: {
            code: 0,
            data: { q16: '1234567890123456', q36: '123456789012345678901234567890123456' }
          }
        }
      else if (url.hostname === 'open.weixin.qq.com')
        body = url.pathname.includes('/qrcode/') ? png : '<a href="?uuid=fakeUuid123">code</a>'
      else if (url.hostname === 'lp.open.weixin.qq.com')
        body = confirmed
          ? "window.wx_errcode=405;window.wx_code='fixture-code';"
          : "window.wx_errcode=408;window.wx_code='';"
      else {
        const value = JSON.parse(new TextDecoder().decode(request.body))
        if (value.req_0) {
          const method = value.req_0.method
          body = {
            code: 0,
            req_0: {
              code: method === 'Login' && rejectRefresh ? 1000 : 0,
              data:
                method === 'UrlGetVkey'
                  ? {
                      sip: ['https://dl.stream.qqmusic.qq.com/'],
                      midurlinfo: [
                        {
                          songmid: 'SongMID',
                          result: resource === 'empty' ? 1000 : 0,
                          purl:
                            resource === 'empty'
                              ? ''
                              : resource === 'preview'
                                ? 'RS02SongMID.m4a'
                                : resource === 'encrypted'
                                  ? 'F0M0AudioMID.mflac'
                                  : resource === 'lower-quality'
                                    ? 'M500AudioMID.mp3'
                                    : resource === 'canonical-file'
                                      ? value.req_0.param.filename[0].replace(
                                          'SongMIDAudioMID',
                                          'AudioMID'
                                        )
                                      : value.req_0.param.filename[0]
                        }
                      ]
                    }
                  : method === 'GetPlaylistByUin'
                    ? {
                        v_playlist: createdPlaylists
                      }
                    : method === 'CgiGetDiss'
                      ? {
                          songlist: playlistSongs.slice(
                            value.req_0.param.song_begin,
                            value.req_0.param.song_begin + value.req_0.param.song_num
                          ),
                          total_song_num: playlistSongs.length,
                          desc: 'Private'
                        }
                      : method === 'Login'
                        ? {
                            musicid: 42,
                            str_musicid: '42',
                            musickey: 'fixture-sensitive-key',
                            encryptUin: 'fixture-encrypted-id',
                            nick: '测试账号'
                          }
                        : method === 'GetLoginUserInfo'
                          ? { info: { nick: '测试账号', logo: 'https://thirdqq.qlogo.cn/avatar' } }
                          : method === 'GetSession'
                            ? {
                                session: { uid: 'bootstrap-user', sid: 'bootstrap-secret', vkey: 1 }
                              }
                            : {
                                qrcodeID: 'fixture-qr',
                                qrcode: png.toString('base64'),
                                expiresIn: 180
                              }
            }
          }
        } else
          body = {
            code: 0,
            request: {
              code: value.request.method === 'get_song_detail_yqq' && songDetail === null ? 500 : 0,
              data:
                value.request.method === 'get_song_detail_yqq'
                  ? {
                      track_info: songDetail && {
                        ...songDetail,
                        id: value.request.param.song_id || songDetail.id,
                        file: { ...songDetail.file, size_flac: flacSize }
                      }
                    }
                  : value.request.method === 'uniform_get_Dissinfo'
                    ? {
                        dirinfo: { dissname: 'Public playlist' },
                        songlist: playlistSongs.slice(
                          value.request.param.song_begin,
                          value.request.param.song_begin + value.request.param.song_num
                        ),
                        total_song_num: playlistSongs.length
                      }
                    : { body: { item_song: [fixtureTrack] }, meta: { sum: 1 } }
            }
          }
      }
      if (url.hostname === 'u.y.qq.com') {
        const sent = JSON.parse(new TextDecoder().decode(request.body))
        if (sent.request?.method === 'DoSearchForQQMusicLite' && searchCalls++ < searchRejections)
          body =
            searchFailureLevel === 'root'
              ? { code: searchFailureCode }
              : { code: 0, request: { code: searchFailureCode } }
      }
      const bytes =
        body instanceof Uint8Array
          ? body
          : new TextEncoder().encode(typeof body === 'string' ? body : JSON.stringify(body))
      return { status, headers: [{ name: 'content-type', value: 'application/json' }], body: bytes }
    },
    websocketOpen() {
      sockets.add(1)
      return 1
    },
    websocketSend(socket, data) {
      if (data[0] >> 4 === 1)
        frames.push({ tag: 'binary', val: { socket, data: new Uint8Array([0x20, 3, 0, 0, 0]) } })
      if (data[0] >> 4 === 8) {
        subscribed = true
        frames.push({ tag: 'binary', val: { socket, data: new Uint8Array([0x90, 4, 0, 1, 0, 0]) } })
      }
    },
    websocketClose(socket) {
      sockets.delete(socket)
    },
    nextEvent() {
      return frames.shift()
    },
    credentialRead() {
      return credential ? new TextEncoder().encode(JSON.stringify(credential)) : new Uint8Array()
    },
    credentialReplace(data) {
      replaced = JSON.parse(new TextDecoder().decode(data))
    },
    credentialPublish(metadata, data) {
      const record = JSON.parse(metadata)
      assert.deepEqual(Object.keys(record.avatar).sort(), [
        'exlarge',
        'large',
        'normal',
        'original',
        'small'
      ])
      assert.equal(record.avatar.original, 'https://thirdqq.qlogo.cn/avatar')
      published = { record, credential: JSON.parse(new TextDecoder().decode(data)) }
    },
    credentialInvalidate() {
      invalidated = true
    },
    stateGet(key) {
      return states.get(key)
    },
    stateSet(key, value) {
      states.set(key, value)
    }
  }
  const instance = await instantiate(
    async (path) => WebAssembly.compile(await readFile(resolve(output, path))),
    {
      ...new WASIShim().getImportObject(),
      'nons:host/transport': transport,
      'nons:plugin/host': {
        call(operation, args) {
          diagnostics.push(JSON.parse(args))
          return 'null'
        }
      }
    }
  )
  instance.initialize()
  return {
    instance,
    requests,
    states,
    sockets,
    subscribed: () => subscribed,
    published: () => published,
    replaced: () => replaced,
    invalidated: () => invalidated,
    event(value) {
      return JSON.parse(instance.call('runtime:event', JSON.stringify(value)))
    },
    diagnostics,
    confirm(method) {
      confirmed = true
      if (method !== 'qq') return
      const utf8 = (value) => {
        const bytes = Buffer.from(value)
        return Buffer.concat([Buffer.from([bytes.length >> 8, bytes.length & 255]), bytes])
      }
      const variable = (value) => {
        const bytes = []
        do {
          let b = value % 128
          value = Math.floor(value / 128)
          if (value) b |= 128
          bytes.push(b)
        } while (value)
        return Buffer.from(bytes)
      }
      const properties = Buffer.concat([Buffer.from([0x26]), utf8('type'), utf8('cookies')])
      const payload = Buffer.from(
        JSON.stringify({
          cookies: {
            qqmusic_uin: { value: '42' },
            qqmusic_key: { value: 'fixture-exchange-token' }
          }
        })
      )
      const body = Buffer.concat([
        utf8('fixture-qr'),
        variable(properties.length),
        properties,
        payload
      ])
      frames.push({
        tag: 'binary',
        val: { socket: 1, data: Buffer.concat([Buffer.from([0x30]), variable(body.length), body]) }
      })
    },
    start(method) {
      const preparation = JSON.parse(
        instance.call(
          'music.prepare-login',
          JSON.stringify({
            session: { source: 'qqmusic' },
            request: { operation: 'beginLogin', method }
          })
        )
      )
      assert.equal(preparation.status, 'ok', JSON.stringify(preparation))
      assert.equal(preparation.data, null)
      assert.equal(sockets.size, 0, 'preparation cannot create or expose a QR subscription')
      return JSON.parse(
        instance.call(
          'music.account',
          JSON.stringify({
            session: { source: 'qqmusic' },
            request: { operation: 'beginLogin', method }
          })
        )
      )
    },
    call(method, request) {
      return JSON.parse(
        instance.call(method, JSON.stringify({ session: { source: 'qqmusic' }, request }))
      )
    }
  }
}
test('actual Component search leaves Content-Length to Host and keeps stable mid', async () => {
  const fixture = await component()
  try {
    const response = fixture.call('music.business', {
      operation: 'search',
      kind: 'track',
      keyword: '测试',
      page: { limit: 5, cursor: null }
    })
    assert.equal(response.status, 'ok', JSON.stringify(response))
    assert.equal(response.data.data.items[0].reference.id, 'SongMID')
    const detail = fixture.call('music.read-track', response.data.data.items[0].reference)
    assert.equal(detail.status, 'ok')
    assert.equal(fixture.requests.length, 2)
  } catch (error) {
    try {
      fixture.instance.shutdown()
    } catch {}
    throw error
  }
  fixture.instance.shutdown()
})
for (const searchFailureLevel of ['root', 'request'])
  test(`actual Component first search recovers upstream 2001 at ${searchFailureLevel} without waiting or reloading`, async () => {
    const fixture = await component({ searchRejections: 1, searchFailureLevel })
    try {
      const request = {
        operation: 'search',
        kind: 'track',
        keyword: '测试',
        page: { limit: 5, cursor: null }
      }
      const response = fixture.call('music.business', request)
      assert.equal(response.status, 'ok', JSON.stringify(response))
      assert.equal(response.data.data.items[0].reference.id, 'SongMID')
      assert.equal(fixture.requests.length, 2)
      const [first, retry] = fixture.requests.map((value) =>
        JSON.parse(new TextDecoder().decode(value.body))
      )
      assert.ok(first.comm)
      assert.equal(retry.comm, undefined)
      assert.deepEqual(retry.request, first.request)
      assert.ok(
        fixture.diagnostics.some(
          (value) => value.stage === 'search.retry-public' && value.code === 'upstream-2001'
        )
      )
    } finally {
      fixture.instance.shutdown()
    }
  })

for (const [searchFailureCode, attempts] of [
  [2001, 2],
  [500, 1]
])
  test(`actual Component bounds search retries and preserves upstream ${searchFailureCode} failure`, async () => {
    const fixture = await component({ searchRejections: 3, searchFailureCode })
    try {
      const response = fixture.call('music.business', {
        operation: 'search',
        kind: 'track',
        keyword: '测试',
        page: { limit: 5, cursor: null }
      })
      assert.equal(response.status, 'error')
      assert.equal(response.data.code, 'sourceUnavailable')
      assert.equal(fixture.requests.length, attempts)
      assert.equal(fixture.diagnostics.at(-1).stage, `rpc.upstream-${searchFailureCode}`)
    } finally {
      fixture.instance.shutdown()
    }
  })

for (const method of ['qq', 'wechat'])
  test(`actual Component ${method} QR bootstrap uses Rust crypto and excludes session keys from device state`, async () => {
    const fixture = await component()
    try {
      const response = fixture.start(method)
      assert.equal(response.status, 'ok', JSON.stringify(response))
      assert.equal(response.data.type, 'challenge')
      assert.ok(response.data.data.image.startsWith('data:image/png;base64,'))
      if (method === 'qq')
        assert.equal(fixture.subscribed(), true, 'QR cannot be shown before MQTT SUBACK')
      const device = fixture.states.get('device')
      assert.ok(device)
      assert.ok(!device.includes('bootstrap-secret'))
      assert.ok(!device.includes('sessionSid'))
    } catch (error) {
      try {
        fixture.instance.shutdown()
      } catch {}
      throw error
    }
    fixture.instance.shutdown()
    assert.equal(fixture.sockets.size, 0)
  })

for (const method of ['qq', 'wechat'])
  test(`actual Component ${method} confirmation publishes uniform account artwork and opaque credentials`, async () => {
    const fixture = await component()
    try {
      const begin = fixture.start(method)
      assert.equal(begin.status, 'ok')
      fixture.confirm(method)
      let response
      for (let i = 0; i < 5; i++) {
        response = fixture.call('music.account', {
          operation: 'pollLogin',
          key: begin.data.data.key
        })
        assert.equal(response.status, 'ok', JSON.stringify(response))
        if (response.data.data.code === 803) break
      }
      assert.equal(response.data.data.code, 803, JSON.stringify(fixture.diagnostics))
      const result = fixture.published()
      assert.equal(result.record.reference.id, '42')
      assert.equal(result.credential.credential.musickey, 'fixture-sensitive-key')
      assert.ok(!JSON.stringify(response).includes('fixture-sensitive-key'))
      assert.ok(!JSON.stringify(result.record).includes('fixture-sensitive-key'))
    } finally {
      fixture.instance.shutdown()
    }
    assert.equal(fixture.sockets.size, 0)
  })

async function credentialFixture() {
  const fixture = await component()
  try {
    const begin = fixture.start('qq')
    fixture.confirm('qq')
    for (let i = 0; i < 5 && !fixture.published(); i++)
      fixture.call('music.account', { operation: 'pollLogin', key: begin.data.data.key })
    assert.ok(fixture.published())
    return fixture.published().credential
  } finally {
    fixture.instance.shutdown()
  }
}
test('expired credentials refresh through Host and never publish the old result', async () => {
  const credential = await credentialFixture()
  credential.expiresAt = 1
  for (const rejected of [false, true]) {
    const fixture = await component({ credential, rejectRefresh: rejected })
    try {
      const result = fixture.call('music.account', { operation: 'profile' })
      assert.equal(result.status, 'error')
      assert.equal(result.data.code, rejected ? 'unauthenticated' : 'staleContext')
      assert.equal(fixture.invalidated(), rejected)
      assert.equal(!!fixture.replaced(), !rejected)
    } finally {
      fixture.instance.shutdown()
    }
  }
})
test('private owned playlists use authenticated read and keep uniform artwork', async () => {
  const fixture = await component({ credential: await credentialFixture() })
  try {
    const page = { cursor: null, limit: 5 }
    const lists = fixture.call('music.business', {
      operation: 'libraryCollections',
      kind: 'playlist',
      filter: 'created',
      page
    })
    assert.equal(lists.status, 'ok', JSON.stringify(lists))
    assert.equal(lists.data.data.items[0].reference.id, '123')
    assert.equal(lists.data.data.items[0].name, 'Private playlist')
    assert.equal(lists.data.data.items[0].subtitle, '测试账号')
    assert.equal(lists.data.data.items[0].cover.original, 'https://y.gtimg.cn/private.jpg')
    const songs = fixture.call('music.business', {
      operation: 'tracks',
      entity: { source: 'qqmusic', kind: 'playlist', id: '123' },
      page
    })
    assert.equal(songs.status, 'ok', JSON.stringify(songs))
    assert.equal(songs.data.data.items[0].reference.id, 'SongMID')
    assert.deepEqual(songs.data.data.items[0].lyricIdentifiers, [
      { kind: 'song-mid', id: 'SongMID' },
      { kind: 'song-id', id: '42' }
    ])
    const request = fixture.requests
      .map((r) => {
        try {
          return JSON.parse(new TextDecoder().decode(r.body))
        } catch {
          return null
        }
      })
      .find((r) => r?.req_0?.method === 'CgiGetDiss')
    assert.equal(request.req_0.param.enc_host_uin, 'fixture-encrypted-id')
  } finally {
    fixture.instance.shutdown()
  }
})

test('playlist metadata is independent of malformed songs and track diagnostics reveal fields without their values', async () => {
  const fixture = await component({
    credential: await credentialFixture(),
    playlistSongs: [{ broken: true, title: 'PRIVATE_METADATA' }]
  })
  try {
    const entity = { source: 'qqmusic', kind: 'playlist', id: '123' }
    const detail = fixture.call('music.business', { operation: 'detail', entity })
    assert.equal(detail.status, 'ok')
    assert.equal(detail.data.data.item.name, 'Private playlist')
    const tracks = fixture.call('music.business', {
      operation: 'tracks',
      entity,
      page: { cursor: null, limit: 5 }
    })
    assert.equal(tracks.status, 'error')
    assert.equal(tracks.data.code, 'invalidData')
    assert.ok(
      fixture.diagnostics.some(
        (entry) => entry.stage === 'track.mid' && entry.class.includes('broken.title')
      )
    )
    assert.ok(!JSON.stringify(fixture.diagnostics).includes('PRIVATE_METADATA'))
  } finally {
    fixture.instance.shutdown()
  }
})

test('playlist repairs missing MID by numeric song ID and retains unavailable songs in place', async () => {
  const credential = await credentialFixture()
  for (const songDetail of [fixtureTrack, null]) {
    const fixture = await component({
      credential,
      songDetail,
      playlistSongs: [
        fixtureTrack,
        { ...fixtureTrack, id: 99, mid: '', title: '下架曲目' },
        fixtureTrack
      ]
    })
    try {
      const result = fixture.call('music.business', {
        operation: 'tracks',
        entity: { source: 'qqmusic', kind: 'playlist', id: '123' },
        page: { cursor: null, limit: 5 }
      })
      assert.equal(result.status, 'ok', JSON.stringify(result))
      const items = result.data.data.items
      assert.equal(items.length, 3)
      assert.equal(items[0].reference.id, 'SongMID')
      assert.equal(items[2].reference.id, 'SongMID')
      assert.equal(items[1].reference.id, songDetail ? 'SongMID' : 'id:99')
      if (!songDetail) {
        assert.equal(items[1].title, '下架曲目')
        assert.equal(items[1].unavailableReason, 'notFound')
        assert.ok(
          fixture.diagnostics.some(
            (entry) => entry.stage === 'track.mid-unavailable' && entry.code === 'notFound'
          )
        )
      }
      const requests = fixture.requests
        .filter((request) => request.body.length)
        .map((request) => JSON.parse(new TextDecoder().decode(request.body)))
      assert.ok(
        requests.some(
          (request) =>
            request.request?.param?.song_id === 99 && request.request.param.song_mid === ''
        )
      )
    } finally {
      fixture.instance.shutdown()
    }
  }
})

test('readonly favorites enumerate every page without advertising favorite writes', async () => {
  const credential = await credentialFixture()
  const fixture = await component({
    credential,
    playlistSongs: Array.from({ length: 60 }, (_, index) => ({
      ...fixtureTrack,
      mid: `Liked${index}`
    }))
  })
  try {
    const descriptor = fixture.call('music.descriptor', null).data
    assert.ok(!descriptor.capabilities.includes('favoritesRead'))
    assert.ok(!descriptor.capabilities.includes('favorites'))
    let cursor = null
    const ids = []
    do {
      const result = fixture.call('music.business', {
        operation: 'tracks',
        entity: { source: 'qqmusic', kind: 'playlist', id: 'liked' },
        page: { limit: 25, cursor }
      })
      assert.equal(result.status, 'ok', JSON.stringify(result))
      assert.equal(result.data.type, 'tracks')
      ids.push(...result.data.data.items.map((item) => item.reference.id))
      cursor = result.data.data.nextCursor
    } while (cursor)
    assert.deepEqual(
      ids,
      Array.from({ length: 60 }, (_, index) => `Liked${index}`)
    )
    const write = fixture.call('music.business', {
      operation: 'setFavorite',
      track: { source: 'qqmusic', kind: 'track', id: 'Liked0' },
      liked: false
    })
    assert.equal(write.data.code, 'unsupported')
  } finally {
    fixture.instance.shutdown()
  }
})

test('public playlists read bounded upstream pages and preserve cursor order', async () => {
  const playlistSongs = Array.from({ length: 1000 }, (_, index) => ({
    ...fixtureTrack,
    mid: `Song${index}`
  }))
  const fixture = await component({ playlistSongs })
  try {
    let cursor = null
    for (let page = 0; page < 2; page++) {
      const result = fixture.call('music.business', {
        operation: 'tracks',
        entity: { source: 'qqmusic', kind: 'playlist', id: '123' },
        page: { cursor, limit: 25 }
      })
      assert.equal(result.status, 'ok', JSON.stringify(result))
      assert.equal(result.data.data.items.length, 25)
      assert.equal(result.data.data.items[0].reference.id, `Song${page * 25}`)
      cursor = result.data.data.nextCursor
      assert.ok(cursor)
    }
    assert.equal(fixture.requests.length, 2)
    assert.ok(
      fixture.requests.every(
        (request) =>
          request.method === 'POST' &&
          JSON.parse(new TextDecoder().decode(request.body)).request.method ===
            'uniform_get_Dissinfo'
      )
    )
  } finally {
    fixture.instance.shutdown()
  }
})
test('plaintext resources report actual quality and reject previews, unknown size and forbidden downgrade', async () => {
  const credential = await credentialFixture()
  const request = {
    track: { source: 'qqmusic', kind: 'track', id: 'SongMID' },
    preferredQuality: 'lossless',
    allowDowngrade: false
  }
  for (const options of [
    {},
    { resource: 'preview' },
    { resource: 'unknown' },
    { resource: 'forbidden' },
    { resource: 'encrypted' },
    { resource: 'lower-quality' },
    { flacSize: 0 },
    { flacSize: 0, downgrade: true }
  ]) {
    const fixture = await component({ credential, ...options })
    try {
      const result = fixture.call('music.resolve-playback', {
        ...request,
        allowDowngrade: options.downgrade ?? false
      })
      if (options.resource || (options.flacSize === 0 && !options.downgrade)) {
        assert.equal(result.status, 'error')
        assert.equal(
          result.data.code,
          ['preview', 'unknown', 'encrypted', 'lower-quality'].includes(options.resource)
            ? 'invalidData'
            : 'permissionDenied'
        )
        const stage = ['preview', 'encrypted', 'lower-quality'].includes(options.resource)
          ? 'playback.filename'
          : options.resource === 'forbidden'
            ? 'playback.http-403'
            : options.resource === 'unknown'
              ? 'playback.content-length'
              : 'playback.file-size'
        assert.ok(
          fixture.diagnostics.some((entry) => entry.stage === stage),
          JSON.stringify(fixture.diagnostics)
        )
      } else {
        assert.equal(result.status, 'ok', JSON.stringify(result))
        assert.equal(result.data.metadata.actualQuality, options.downgrade ? 'exhigh' : 'lossless')
        assert.equal(result.data.metadata.extent.kind, 'full')
      }
    } finally {
      fixture.instance.shutdown()
    }
  }
})
test('authorized plaintext links may use a canonical filename different from the Vkey request filename', async () => {
  const credential = await credentialFixture()
  for (const [quality, filename] of [
    ['standard', 'M500AudioMID.mp3'],
    ['exhigh', 'M800AudioMID.mp3'],
    ['lossless', 'F000AudioMID.flac']
  ]) {
    const fixture = await component({ credential, resource: 'canonical-file' })
    try {
      const result = fixture.call('music.resolve-playback', {
        track: { source: 'qqmusic', kind: 'track', id: 'SongMID' },
        preferredQuality: quality,
        allowDowngrade: false
      })
      assert.equal(result.status, 'ok', JSON.stringify(result))
      assert.equal(new URL(result.data.url).pathname.split('/').pop(), filename)
      assert.equal(result.data.metadata.actualQuality, quality)
      assert.equal(result.data.metadata.extent.kind, 'full')
      assert.ok(fixture.requests.some((request) => request.method === 'HEAD'))
    } finally {
      fixture.instance.shutdown()
    }
  }
})

test('empty playback addresses preserve the upstream per-song result without logging credentials or URLs', async () => {
  const fixture = await component({ credential: await credentialFixture(), resource: 'empty' })
  try {
    const result = fixture.call('music.resolve-playback', {
      track: { source: 'qqmusic', kind: 'track', id: 'SongMID' },
      preferredQuality: 'standard',
      allowDowngrade: false
    })
    assert.equal(result.status, 'error')
    assert.equal(result.data.code, 'sourceUnavailable')
    assert.ok(
      fixture.diagnostics.some(
        (entry) => entry.stage === 'playback.vkey-empty.M500' && entry.code === 'upstream-1000'
      ),
      JSON.stringify(fixture.diagnostics)
    )
    assert.ok(!JSON.stringify(fixture.diagnostics).includes('fixture-sensitive-key'))
    assert.ok(!JSON.stringify(fixture.diagnostics).includes('https://'))
  } finally {
    fixture.instance.shutdown()
  }
})

test('a disconnected QR channel fails explicitly and releases its transport', async () => {
  const fixture = await component()
  try {
    const begin = fixture.start('qq')
    fixture.sockets.clear()
    const event = fixture.event({ kind: 'closed', socket: 1 })
    assert.equal(event.status, 'ok')
    let result
    for (let i = 0; i < 5; i++) {
      result = fixture.call('music.account', { operation: 'pollLogin', key: begin.data.data.key })
      if (result.data?.data?.code === 800) break
    }
    assert.equal(result.data.data.code, 800)
    assert.equal(fixture.published(), null)
  } finally {
    fixture.instance.shutdown()
  }
  assert.equal(fixture.sockets.size, 0)
})

test('library lists exclude the system likes directory before pagination but retain a same-name regular playlist', async () => {
  const credential = await credentialFixture()
  const fixture = await component({
    credential,
    createdPlaylists: [
      { tid: 900, dirId: 201, dirName: '我喜欢', dirShow: 2 },
      { tid: 901, dirId: 1, dirName: '我喜欢', dirShow: 2 }
    ]
  })
  try {
    for (const filter of ['created', 'all']) {
      const result = fixture.call('music.business', {
        operation: 'libraryCollections',
        kind: 'playlist',
        filter,
        page: { cursor: null, limit: 25 }
      })
      assert.equal(result.status, 'ok', JSON.stringify(result))
      assert.deepEqual(
        result.data.data.items.map((item) => item.reference.id),
        ['901']
      )
      assert.equal(result.data.data.nextCursor, null)
    }
    const summary = fixture.call('music.business', { operation: 'librarySummary' })
    assert.equal(summary.status, 'ok', JSON.stringify(summary))
    assert.equal(summary.data.data.likedPlaylist.reference.id, 'liked')
  } finally {
    fixture.instance.shutdown()
  }
})
