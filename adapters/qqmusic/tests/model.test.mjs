import { test } from 'node:test'
import assert from 'node:assert/strict'
import { track, pageState, pageResult, checkReference, accountRecord } from '../src/model.js'

test('numeric song ID and audio media ID do not become the stable track identity', () => {
  const value = track({
    id: 42,
    mid: 'SongMID',
    file: { media_mid: 'AudioMID' },
    title: '曲目',
    interval: 123,
    singer: [{ name: '歌手', mid: 'ArtistMID' }],
    album: { name: '专辑', mid: 'AlbumMID' }
  })
  assert.deepEqual(value.reference, { source: 'qqmusic', kind: 'track', id: 'SongMID' })
  assert.equal(value.durationMs, 123000)
  assert.equal(value.artists[0].reference.id, 'ArtistMID')
  assert.equal(value.albumReference.id, 'AlbumMID')
  assert.throws(() => track({ id: 42, title: '缺少 mid' }), { code: 'invalidData' })
})
test('cursor rejects changed query, page size, malformed offset and foreign source', () => {
  const state = pageState({ limit: 25 }, 'query-a')
  const result = pageResult(new Array(25).fill({}), state, 26)
  assert.equal(pageState({ cursor: result.nextCursor, limit: 25 }, 'query-a').offset, 25)
  assert.throws(() => pageState({ cursor: result.nextCursor, limit: 25 }, 'query-b'), {
    code: 'invalidData'
  })
  assert.throws(() => pageState({ cursor: result.nextCursor, limit: 10 }, 'query-a'), {
    code: 'invalidData'
  })
  assert.throws(() => pageState({ cursor: JSON.stringify({ ...state, offset: -1 }) }, 'query-a'), {
    code: 'invalidData'
  })
  assert.equal(pageResult([], state).nextCursor, null)
  assert.throws(() => checkReference({ source: 'other', kind: 'track', id: 'SongMID' }, 'track'), {
    code: 'invalidData'
  })
})
test('WeChat real string music ID takes precedence over placeholder numeric ID', () => {
  const record = accountRecord(
    { info: { nick: '测试账号' } },
    { musicid: 0, str_musicid: 'wx-real-id', musickey: 'secret' }
  )
  assert.equal(record.reference.id, 'wx-real-id')
  assert.equal(record.displayName, '测试账号')
  assert.ok(!JSON.stringify(record).includes('secret'))
})
