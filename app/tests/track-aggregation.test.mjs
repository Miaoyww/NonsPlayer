import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'

const require = createRequire(import.meta.url)
function load(name) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL(`../src/features/search/${name}.ts`, import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 }
      }
    ).outputText,
    { exports, require }
  )
  return exports
}
const { TrackAggregation, trackMatchScore, normalizeMetadata } = load('track-aggregation')
const { AggregateQuery } = load('aggregate-query')
const track = (key, changes = {}) => ({
  key,
  title: '同一首歌',
  artist: '歌手',
  artists: [{ name: '歌手' }],
  album: '原专辑',
  durationMs: 200000,
  cover: '',
  source: {
    kind: 'online',
    track: { reference: { source: key.split(':')[0], kind: 'track', id: key } }
  },
  ...changes
})
const append = (model, ...tracks) =>
  model.append(tracks.map((item) => ({ source: item.key.split(':')[0], item })))

test('same recording can cross albums, artist credit ordering and source identity remain intact', () => {
  const first = track('q:1', { artists: [{ name: 'A' }, { name: 'B' }] })
  const second = track('n:1', {
    artists: [{ name: 'b' }, { name: 'Ａ' }],
    album: '精选集',
    durationMs: 202000
  })
  assert.equal(trackMatchScore(first, second), 90)
  const model = new TrackAggregation(['n', 'q'])
  append(model, first)
  const before = model.getSnapshot()
  append(model, second)
  const after = model.getSnapshot()
  assert.equal(after.length, 1)
  assert.equal(after[0].id, before[0].id)
  assert.equal(before[0].members.length, 1)
  assert.equal(after[0].representative, second)
  assert.equal(after[0].members[1].track, first)
  assert.equal(after[0].sourceCount, 2)
})

test('normalization preserves meaningful text and supports equivalent punctuation', () => {
  assert.equal(normalizeMetadata(' ＳＯＮＧ （“Ｌｉｖｅ”） '), 'song("live")')
  assert.notEqual(
    trackMatchScore(track('n:1', { title: 'ＳＯＮＧ' }), track('q:1', { title: 'song' })),
    null
  )
  assert.equal(trackMatchScore(track('n:1'), track('q:1', { title: '同一首哥' })), null)
  assert.equal(trackMatchScore(track('n:1'), track('q:1', { title: '同一首歌 (电影原声)' })), null)
  const title = 'a sufficiently long recording title'
  assert.notEqual(
    trackMatchScore(track('n:1', { title }), track('q:1', { title: title + 's' })),
    null
  )
})

test('missing metadata, translated names, aliases and partial artist sets cannot merge', () => {
  const first = track('n:1')
  for (const changes of [
    { durationMs: 0 },
    { durationMs: NaN },
    { durationMs: Infinity },
    { durationMs: -1 },
    { title: '' },
    { artists: [{ name: '' }] },
    { artists: [], artist: '' },
    { artists: [{ name: '歌手' }, { name: '嘉宾' }] },
    { artists: [{ name: 'Singer' }] },
    { title: 'The Same Song', aliases: ['同一首歌'] }
  ])
    assert.equal(trackMatchScore(first, track('q:1', changes)), null, JSON.stringify(changes))
  assert.equal(
    trackMatchScore(
      track('n:1', { artists: [], artist: 'A / B' }),
      track('q:1', { artists: [{ name: 'A' }, { name: 'B' }] })
    ),
    null
  )
  assert.equal(trackMatchScore(track('n:1', { album: '' }), track('q:1', { album: '' })), 90)
})

test('duration gates and score thresholds have exact boundaries', () => {
  const first = track('n:1')
  assert.equal(trackMatchScore(first, track('q:1', { durationMs: 204000 })), 90)
  assert.equal(trackMatchScore(first, track('q:1', { durationMs: 204001 })), null)
  assert.equal(trackMatchScore(first, track('q:1', { durationMs: 205001 })), null)
  assert.equal(trackMatchScore(first, track('q:1', { durationMs: 202001, album: '' })), null)
})

test('all explicit recording variants are isolated and equivalent labels can merge', () => {
  const first = track('n:1')
  const variants = [
    'Live',
    'Studio Live',
    'Remix',
    'Instrumental',
    'Karaoke',
    'Acoustic',
    'Demo',
    '2011 Remastered',
    'Re-recorded',
    'Clean',
    'Explicit',
    'Sped Up',
    'Slowed',
    'Radio Edit',
    'Extended Mix',
    '现场版',
    '伴奏版',
    '不插电版',
    '重制版',
    '重新录制',
    '加速版',
    '减速版',
    '剪辑版',
    '延长版'
  ]
  for (const version of variants) {
    const second = track('q:1', { title: `同一首歌 (${version})` })
    assert.equal(trackMatchScore(first, second), null, version)
    assert.notEqual(trackMatchScore(second, track('n:2', { title: second.title })), null, version)
  }
  assert.equal(trackMatchScore(first, track('q:1', { title: '同一首歌 (Studio Version)' })), 100)
  assert.equal(
    trackMatchScore(
      track('n:1', { title: '同一首歌 (Live)' }),
      track('q:1', { title: '同一首歌 (现场版)' })
    ),
    100
  )
  assert.equal(
    trackMatchScore(
      track('n:1', { title: '同一首歌 - Instrumental' }),
      track('q:1', { title: '同一首歌 (伴奏)' })
    ),
    100
  )
})

test('live events, remix artists and remaster years require matching qualifiers', () => {
  for (const [left, right] of [
    ['Live', 'Live at Wembley 1986'],
    ['Live at Wembley 1986', 'Live at Wembley 1987'],
    ['Alice Remix', 'Bob Remix'],
    ['Remastered', '2011 Remastered'],
    ['2011 Remastered', '2012 Remastered'],
    ['Clean', 'Explicit'],
    ['Radio Edit', 'Edit']
  ])
    assert.equal(
      trackMatchScore(
        track('n:1', { title: `同一首歌 (${left})` }),
        track('q:1', { title: `同一首歌 (${right})` })
      ),
      null,
      `${left}/${right}`
    )
  assert.equal(trackMatchScore(track('n:1'), track('q:1', { album: '专辑 (Live)' })), null)
})

test('same-source editions retain every member but count distinct sources only', () => {
  const model = new TrackAggregation(['n', 'q'])
  append(model, track('n:1'), track('n:2', { album: '精选集' }), track('n:1'))
  assert.equal(model.getSnapshot().length, 1)
  assert.equal(model.getSnapshot()[0].members.length, 2)
  assert.equal(model.getSnapshot()[0].sourceCount, 1)
  append(model, track('q:1'))
  assert.equal(model.getSnapshot()[0].members.length, 3)
  assert.equal(model.getSnapshot()[0].sourceCount, 2)
})

test('pairwise membership prevents transitive merging and chooses best minimum score', () => {
  const model = new TrackAggregation(['n', 'q'])
  append(
    model,
    track('n:1'),
    track('q:1', { durationMs: 204000 }),
    track('q:2', { durationMs: 208000 })
  )
  assert.equal(model.getSnapshot().length, 2)
  assert.equal(model.getSnapshot()[0].members.length, 2)
  const best = new TrackAggregation(['n', 'q'])
  append(
    best,
    track('n:1'),
    track('n:2', { durationMs: 206000 }),
    track('q:1', { durationMs: 203500 })
  )
  assert.equal(best.getSnapshot()[0].members.length, 1)
  assert.equal(best.getSnapshot()[1].members.length, 2)
  const tie = new TrackAggregation(['n', 'q'])
  append(
    tie,
    track('n:1'),
    track('n:2', { durationMs: 206000 }),
    track('q:1', { durationMs: 203000 })
  )
  assert.equal(tie.getSnapshot()[0].members.length, 2)
})

test('pagination, source filtering and failures preserve groups; stale completions cannot append', async () => {
  const model = new TrackAggregation(['n', 'q'])
  const requests = []
  const query = new AggregateQuery(
    [
      { source: 'n', name: 'N' },
      { source: 'q', name: 'Q' }
    ],
    (source, cursor) =>
      new Promise((resolve, reject) => requests.push({ source, cursor, resolve, reject })),
    (item) => item.key,
    String,
    1000,
    (batch) => model.append(batch)
  )
  const firstCurrent = query.load('n')
  const firstSupplementary = query.load('q')
  requests[1].resolve({ items: [track('q:1')], nextCursor: 'q2' })
  await firstSupplementary
  requests[0].reject(new Error('offline'))
  await firstCurrent
  const nextSupplementary = query.load('q')
  requests[2].resolve({ items: [track('q:2', { album: '精选集' })], nextCursor: null })
  await nextSupplementary
  const id = model.getSnapshot()[0].id
  const retry = query.load('n', true)
  requests[3].resolve({ items: [track('n:1')], nextCursor: 'n2' })
  await retry
  assert.equal(model.getSnapshot()[0].id, id)
  assert.equal(model.getSnapshot()[0].representative.key, 'n:1')
  assert.equal(query.getSnapshot().items.filter((value) => value.source === 'q').length, 2)
  const stale = query.load('n')
  query.dispose()
  requests[4].resolve({ items: [track('n:old')], nextCursor: null })
  await stale
  assert.equal(model.getSnapshot()[0].members.length, 3)
})
