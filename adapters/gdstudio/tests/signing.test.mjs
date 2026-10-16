import { test } from 'node:test'
import assert from 'node:assert/strict'
import { protocolDigest, sign } from '../src/signing.js'
import {
  platforms,
  searchRequest,
  decodeSearch,
  apiRequest,
  decodeResponse
} from '../src/protocol.js'

test('website protocol vectors distinguish the modified digest and encode Unicode and punctuation', () => {
  assert.equal(protocolDigest('abc'), '9ef90af686e68195b6f6d89b69d3c584')
  assert.equal(sign('Hello', '1792158848'), 'DA6626C4')
  assert.equal(sign("测试 /+!'()*", '1792158848'), 'C066C8A5')
  assert.equal(sign('Hello', '1792158840'), sign('Hello', '1792158849'))
  assert.notEqual(sign('Hello', '1792158850'), sign('Hello', '1792158849'))
  assert.throws(() => sign('Hello', '<html>'))
  assert.match(sign('雪'.repeat(1024), '1792158848'), /^[A-F0-9]{8}$/)
})

test('all ten website sources retain canonical identity, opaque IDs, and real duration', () => {
  for (const [platform, upstream] of Object.entries(platforms)) {
    const request = searchRequest({ platform, keyword: 'Hello', page: { cursor: null, limit: 2 } })
    assert.equal(new URL(request.url).searchParams.get('source'), upstream)
    const page = decodeSearch(
      [
        {
          source: upstream,
          id: 'Opaque+/雪',
          name: 'Hello',
          artist: ['A'],
          extra_data: { duration: 180 }
        }
      ],
      1,
      2,
      platform
    )
    assert.deepEqual(page.items[0].reference, { source: platform, kind: 'track', id: 'Opaque+/雪' })
    assert.equal(page.items[0].durationMs, 180000)
    assert.equal(
      new URL(
        apiRequest({ track: page.items[0], preferredQuality: 'lossless', allowDowngrade: false })
          .url
      ).searchParams.get('br'),
      '999'
    )
  }
  assert.throws(() =>
    searchRequest({ platform: 'toString', keyword: 'Hello', page: { cursor: null, limit: 2 } })
  )
  assert.throws(
    () => decodeResponse(200, { url: 'https://music-proxy.gdstudio.org/pending', br: -1, size: 0 }),
    { code: 'notFound' }
  )
})
