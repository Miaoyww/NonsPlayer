import { test } from 'node:test'
import assert from 'node:assert/strict'
import { protocolDigest, sign } from '../src/signing.js'
import { platforms, apiRequest, decodeResponse } from '../src/protocol.js'

test('website protocol vectors distinguish the modified digest and encode Unicode and punctuation', () => {
  assert.equal(protocolDigest('abc'), '9ef90af686e68195b6f6d89b69d3c584')
  assert.equal(sign('Hello', '1792158848'), 'DA6626C4')
  assert.equal(sign("测试 /+!'()*", '1792158848'), 'C066C8A5')
  assert.equal(sign('Hello', '1792158840'), sign('Hello', '1792158849'))
  assert.notEqual(sign('Hello', '1792158850'), sign('Hello', '1792158849'))
  assert.throws(() => sign('Hello', '<html>'))
  assert.match(sign('雪'.repeat(1024), '1792158848'), /^[A-F0-9]{8}$/)
})

test('all ten platforms resolve opaque IDs without search', () => {
  for (const [platform, upstream] of Object.entries(platforms)) {
    const target = apiRequest({
      track: { reference: { source: platform, kind: 'track', id: 'Opaque+/雪' } },
      preferredQuality: 'lossless',
      allowDowngrade: false
    })
    const url = new URL(target.url)
    assert.equal(url.searchParams.get('source'), upstream)
    assert.equal(url.searchParams.get('id'), 'Opaque+/雪')
    assert.equal(url.searchParams.get('types'), 'url')
    assert.equal(url.searchParams.get('br'), '999')
  }
  assert.throws(
    () =>
      apiRequest({
        track: { reference: { source: 'toString', id: '1' } },
        preferredQuality: 'standard'
      }),
    { code: 'invalidData' }
  )
  assert.throws(
    () => decodeResponse(200, { url: 'https://music-proxy.gdstudio.org/pending', br: -1, size: 0 }),
    { code: 'notFound' }
  )
})
