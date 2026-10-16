import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  track,
  entity,
  pageState,
  pageResult,
  checkReference,
  accountRecord
} from '../src/model.js'

test('created and collected playlists retain platform names and creator labels instead of descriptions', () => {
  const created = entity(
    { tid: 123, dirName: '我的自建歌单', desc: 'QQ音乐', creator: { name: '创建者' } },
    'playlist'
  )
  assert.equal(created.name, '我的自建歌单')
  assert.equal(created.subtitle, '创建者')
  const collected = entity(
    { dissid: '456', dissname: '收藏歌单', desc: 'QQ音乐', nickname: '其他作者' },
    'playlist'
  )
  assert.equal(collected.name, '收藏歌单')
  assert.equal(collected.subtitle, '其他作者')
})

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
  assert.deepEqual(value.lyricIdentifiers, [
    { kind: 'song-mid', id: 'SongMID' },
    { kind: 'song-id', id: '42' }
  ])
  assert.equal(value.durationMs, 123000)
  assert.equal(value.artists[0].reference.id, 'ArtistMID')
  assert.equal(value.albumReference.id, 'AlbumMID')
  assert.throws(() => track({ id: 42, title: '缺少 mid' }), { code: 'invalidData' })
})

test('empty upstream creator fields fall back to the authenticated playlist owner', () => {
  assert.equal(
    entity({ tid: 123, creator: { name: '' }, nickname: '', creatorName: 'Miaoyww' }, 'playlist')
      .subtitle,
    'Miaoyww'
  )
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
