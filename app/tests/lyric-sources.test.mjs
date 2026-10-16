import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness(responses, kind = 'netease') {
  const calls = []
  const commands = []
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/load-lyrics.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  )
  runInNewContext(outputText, {
    exports,
    require: () => ({
      nativeCall: async (_command, args) => {
        commands.push(_command)
        calls.push(args)
        return responses.shift()
      }
    })
  })
  return {
    calls,
    commands,
    version: exports.lyricTrackVersion,
    load: (sources = { amll: true, qq: true }, current) =>
      exports.loadLyrics(
        { key: `${kind}:1`, source: { kind } },
        true,
        sources,
        (lyrics) => {
          if (lyrics?.content === 'broken') throw new Error('invalid lyrics')
          return lyrics?.content
        },
        current
      )
  }
}

test('semantic failures continue through AMLL, QQ and NetEase once', async () => {
  const { load, calls } = harness(
    ['amll', 'qq']
      .map((source) => ({ source, content: 'broken' }))
      .concat([{ source: 'netease', content: 'usable' }])
  )
  assert.equal((await load()).parsed, 'usable')
  assert.deepEqual(
    calls.map((c) => [c.skipAmll, c.skipQq, c.refresh]),
    [
      [false, false, true],
      [true, false, false],
      [true, true, false]
    ]
  )
})

test('local selection reads only local lyrics, including when none are available', async () => {
  const app = harness([null, { source: 'netease', content: 'online' }], 'local')
  assert.equal((await app.load({ localSource: 'local', amll: true, qq: true })).lyrics, null)
  assert.deepEqual(app.commands, ['local_track_lyrics'])
})

test('invalid embedded lyrics can try the sidecar but never online lyrics', async () => {
  const app = harness(
    [
      { source: 'local', content: 'broken' },
      { source: 'local', content: 'usable' }
    ],
    'local'
  )
  assert.equal((await app.load({ localSource: 'local', amll: true, qq: true })).parsed, 'usable')
  assert.deepEqual(app.commands, ['local_track_lyrics', 'local_track_lyrics'])
  assert.equal(app.calls[1].skipEmbedded, true)
})

test('local selection never changes streaming lyrics routing or their translation', async () => {
  for (const kind of ['netease', 'online']) {
    const app = harness(
      [{ source: 'netease', format: 'yrc', content: 'words', translation: 'translation' }],
      kind
    )
    const result = await app.load({ localSource: 'local', amll: true, qq: true })
    assert.equal(result.lyrics.translation, 'translation')
    assert.deepEqual(app.commands, ['track_lyrics'])
  }
})

test('online selection never reads local files when online lyrics are missing', async () => {
  const app = harness([null], 'local')
  assert.equal((await app.load({ localSource: 'online', amll: true, qq: true })).lyrics, null)
  assert.deepEqual(app.commands, ['track_lyrics'])
})

test('disabled online sources are skipped for every combination', async () => {
  for (const amll of [false, true])
    for (const qq of [false, true]) {
      const { load, calls } = harness([{ source: 'netease', content: 'usable' }])
      await load({ amll, qq })
      assert.deepEqual([calls[0].skipAmll, calls[0].skipQq], [!amll, !qq])
    }
})

test('late responses are discarded and final-source failures do not loop', async () => {
  const stale = harness([{ source: 'amll', content: 'broken' }])
  let current = true
  const pending = stale.load(undefined, () => current)
  current = false
  assert.equal(await pending, null)
  assert.equal(stale.calls.length, 1)
  const terminal = harness([{ source: 'netease', content: 'broken' }])
  await assert.rejects(terminal.load(), /invalid lyrics/)
  assert.equal(terminal.calls.length, 1)
})

test('invalid QRC tries QQ LRC before disabling QQ and requesting NetEase', async () => {
  const { load, calls } = harness([
    { source: 'qq', format: 'qrc', content: 'broken' },
    { source: 'qq', format: 'lrc', content: 'broken' },
    { source: 'netease', format: 'yrc', content: 'usable' }
  ])
  assert.equal((await load()).parsed, 'usable')
  assert.deepEqual(
    calls.map((c) => [c.skipQq, c.skipQrc]),
    [
      [false, false],
      [false, true],
      [true, true]
    ]
  )
})

test('local tracks stop when online lyrics are exhausted', async () => {
  const { load, calls } = harness([{ source: 'netease', content: 'broken' }, null], 'local')
  assert.equal((await load()).lyrics, null)
  assert.deepEqual(
    calls.map((c) => c.skipNetease),
    [false, true]
  )
  assert.ok(calls.every((c) => !('skipLocal' in c)))
})

test('lyric request identity changes with typed IDs but ignores object property order', () => {
  const { version } = harness([])
  const original = {
    key: 'qq:1',
    title: 'Song',
    artist: 'Artist',
    source: {
      kind: 'online',
      track: {
        reference: { source: 'qqmusic', kind: 'track', id: '001mid' },
        lyricIdentifiers: [{ kind: 'song-id', id: '123' }],
        associations: []
      }
    }
  }
  const changed = structuredClone(original)
  changed.source.track.lyricIdentifiers[0].id = '456'
  assert.notEqual(version(original), version(changed))
  const reordered = structuredClone(original)
  reordered.source.track.reference = { id: '001mid', kind: 'track', source: 'qqmusic' }
  assert.equal(version(original), version(reordered))
})
