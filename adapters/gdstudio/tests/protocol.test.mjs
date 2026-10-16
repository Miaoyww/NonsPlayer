import { test } from 'node:test'
import assert from 'node:assert/strict'
import { apiRequest, decodeResponse, searchRequest, decodeSearch } from '../src/protocol.js'
import { inspectRemoteAudio } from '../src/audio.js'

async function inspectAudio(prefix, size, duration, quality, downgrade) {
  const { metadata } = await inspectRemoteAudio(
    async (start, end) => {
      const bytes = new Uint8Array(Math.min(end + 1, size) - start)
      if (start === 0) {
        if (prefix.length > 65536) return { bytes: prefix, size }
        bytes.set(prefix)
      }
      return { bytes, size }
    },
    duration,
    quality,
    downgrade
  )
  return metadata
}

export function sample(kbps = 320, seconds = 180) {
  const frameSize = kbps === 320 ? 1044 : 417
  const prefix = new Uint8Array(frameSize * 8)
  for (let i = 0; i < 8; i++) prefix.set([255, 251, kbps === 320 ? 224 : 144, 0], i * frameSize)
  return { prefix, total: Math.floor((seconds * 44100) / 1152) * frameSize }
}
test('platform IDs stay opaque and unsupported quality cannot silently downgrade', () => {
  const request = {
    track: { reference: { source: 'netease', id: 'opaque/id +雪' } },
    preferredQuality: 'exhigh',
    allowDowngrade: false
  }
  assert.equal(
    apiRequest(request).url,
    'https://music.gdstudio.org/api.php?types=url&source=netease&id=opaque%2Fid%20%2B%E9%9B%AA&br=320'
  )
  assert.equal(apiRequest({ ...request, preferredQuality: 'lossless' }).quality, 'lossless')
  assert.equal(
    apiRequest({ ...request, preferredQuality: 'lossless', allowDowngrade: true }).quality,
    'lossless'
  )
})
test('server failures and preview responses remain errors', () => {
  assert.throws(() => decodeResponse(503, null), { code: 'network' })
  assert.throws(() => decodeResponse(429, null), { code: 'rateLimited' })
  assert.throws(() => decodeResponse(200, { code: 0, url: 'https://example.com', preview: true }), {
    code: 'permissionDenied'
  })
})
test('mature parser verifies bitrate and full duration from bounded MPEG prefix', async () => {
  const high = sample()
  const result = await inspectAudio(
    high.prefix,
    high.total,
    180000,
    '320k',
    false,
    new Uint8Array(512)
  )
  assert.equal(result.actualQuality, '320k')
  assert.equal(result.extent.kind, 'full')
  const low = sample(128)
  await assert.rejects(
    inspectAudio(low.prefix, low.total, 180000, '320k', false, new Uint8Array(512)),
    { code: 'permissionDenied' }
  )
  assert.equal(
    (await inspectAudio(low.prefix, low.total, 180000, '320k', true, new Uint8Array(512)))
      .actualQuality,
    '128k'
  )
})
test('short samples, unknown duration, invalid bytes and oversized probes fail closed', async () => {
  const short = sample(320, 30)
  await assert.rejects(
    inspectAudio(short.prefix, short.total, 180000, '320k', true, new Uint8Array(512)),
    { code: 'permissionDenied' }
  )
  await assert.rejects(inspectAudio(short.prefix, short.total, 0, '320k', true), {
    code: 'permissionDenied'
  })
  await assert.rejects(inspectAudio(new Uint8Array(32), short.total, 180000, '320k', true), {
    code: 'invalidData'
  })
  await assert.rejects(inspectAudio(new Uint8Array(65537), short.total, 180000, '320k', true), {
    code: 'invalidData'
  })
})

test('search maps stable platform identity and validated paging without inventing song duration', () => {
  const target = searchRequest({
    platform: 'netease',
    keyword: ' Song ',
    page: { cursor: null, limit: 1 }
  })
  assert(target.url.includes('pages=1'))
  const result = decodeSearch(
    [
      {
        id: '347230',
        url_id: '347230',
        source: 'netease',
        name: 'Song',
        artist: ['A', 'B'],
        album: 'Album'
      }
    ],
    1,
    1
  )
  assert.equal(result.items[0].reference.source, 'netease')
  assert.equal(result.items[0].reference.id, '347230')
  assert.equal(result.items[0].durationMs, 0)
  assert.equal(result.nextCursor, '2')
  assert.throws(
    () => decodeSearch([{ id: '1', source: 'qqmusic', name: 'Song', artist: [] }], 1, 1),
    { code: 'invalidData' }
  )
  assert.throws(
    () =>
      searchRequest({ platform: 'netease', keyword: 'Song', page: { cursor: 'bad', limit: 1 } }),
    { code: 'invalidData' }
  )
})
