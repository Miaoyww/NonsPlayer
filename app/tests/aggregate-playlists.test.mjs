import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { createRuntimeCache } from './helpers/runtime-cache.mjs'

function load(file, modules) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(file, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      setTimeout: (fn) => {
        fn()
        return 0
      },
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  return exports
}

test('aggregate changes invalidate only their member pages, including pending reads', async () => {
  const { requestCache, requestKey, invalidateAggregateCache } = createRuntimeCache()
  let reads = 0
  const read = (id) =>
    requestCache.get(requestKey('aggregate_tracks', { id, offset: 0 }), async () => ++reads)
  assert.equal(await read('one'), 1)
  assert.equal(await read('two'), 2)
  let release
  const stale = requestCache.get(
    requestKey('aggregate_tracks', { id: 'one', offset: 100 }),
    () =>
      new Promise((resolve) => {
        release = resolve
      })
  )
  invalidateAggregateCache('one')
  release([])
  await assert.rejects(stale, { code: 'cacheInvalidated' })
  assert.equal(await read('two'), 2)
  assert.equal(await read('one'), 3)
})

function targetsHarness({ query, write, native, aggregate, current } = {}) {
  const account = { source: 'netease', id: '42' }
  const calls = []
  const api = load('../src/features/aggregate/playlist-targets.ts', {
    '@/features/music/client': {
      musicClient: {
        currentAccount: current ?? (async () => ({ reference: account })),
        query:
          query ??
          (async () => {
            throw new Error('unexpected query')
          }),
        write: write ?? (async (...args) => calls.push(['remote', ...args]))
      }
    },
    '@/features/music/entities': {
      musicCollection: (entity) => ({
        ...entity,
        id: entity.reference.id,
        kind: 'playlist',
        creatorId: entity.creator?.id
      })
    },
    '@/features/music/legacy-reference': {
      collectionReference: (item) => item.reference,
      onlineReference: (track) => track.reference ?? null
    },
    '@/lib/player': {
      nativeCall:
        native ??
        (async (command) => {
          if (command === 'aggregate_playlists') return []
          if (command === 'aggregate_playlist')
            return {
              id: 'existing',
              link: { reference: { source: 'netease', kind: 'playlist', id: 'p' }, account }
            }
          calls.push(command)
          return { key: 'task', running: false, playlistId: 'local', error: null }
        })
    },
    './aggregate-library': { writeAggregate: aggregate ?? (async (request) => calls.push(request)) }
  })
  return {
    ...api,
    calls,
    account,
    target: {
      item: { id: 'p', reference: { source: 'netease', kind: 'playlist', id: 'p' } },
      account,
      writable: true
    }
  }
}

test('same source adds remotely; foreign or local tracks create a local version; converted targets also upload matching platform tracks', async () => {
  const h = targetsHarness()
  const same = { key: 'n', reference: { source: 'netease', kind: 'track', id: 'n' } }
  await h.addToPlaylist(h.target, [same])
  assert.equal(h.calls[0][0], 'remote')
  h.calls.length = 0
  await h.addToPlaylist({ ...h.target, writable: false }, [
    { key: 'qq', reference: { source: 'qqmusic', kind: 'track', id: 'q' } }
  ])
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls)), [
    'aggregate_sync',
    { operation: 'add', id: 'local', keys: ['qq'] }
  ])
  h.calls.length = 0
  await h.addToPlaylist(h.target, [{ key: 'file' }])
  assert.equal(h.calls[0], 'aggregate_sync')
  h.calls.length = 0
  await h.addToPlaylist(h.target, [same], 'existing')
  assert.equal(h.calls[0][0], 'remote')
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls.at(-1))), {
    operation: 'add',
    id: 'existing',
    keys: ['n']
  })
})

test('conversion waits for complete synchronization and rejects failed reads or changed accounts before adding', async () => {
  let added = 0,
    checks = 0
  const h = targetsHarness({
    native: async (command) =>
      command === 'aggregate_playlists'
        ? []
        : command === 'aggregate_sync'
          ? { running: true, key: 'task' }
          : { running: false, playlistId: 'local', error: null },
    aggregate: async () => added++,
    current: async () => ({ reference: { source: 'netease', id: ++checks === 1 ? '42' : 'other' } })
  })
  await assert.rejects(h.addToPlaylist(h.target, [{ key: 'file' }]), /账号已变化/)
  assert.equal(added, 0)
  const failed = targetsHarness({
    native: async (command) =>
      command === 'aggregate_playlists' ? [] : { running: false, error: '后页失败' },
    aggregate: async () => added++
  })
  await assert.rejects(failed.addToPlaylist(failed.target, [{ key: 'file' }]), /后页失败/)
  assert.equal(added, 0)
})

test('owned targets traverse 30-item library pages and reject other owners; library grid avoids duplicate local versions', async () => {
  const pages = []
  const h = targetsHarness({
    query: async (source, request) => {
      pages.push(request.page)
      return {
        type: 'entities',
        data: {
          items: [
            {
              name: 'Mine',
              reference: { source, kind: 'playlist', id: request.page.cursor ?? 'first' },
              creator: { source, id: request.page.cursor ? 'other' : '42' }
            }
          ],
          nextCursor: request.page.cursor ? null : 'next'
        }
      }
    }
  })
  const result = await h.ownedPlaylistTargets('netease', false)
  assert.equal(result.length, 1)
  assert.equal(result[0].writable, false)
  assert.deepEqual(JSON.parse(JSON.stringify(pages)), [
    { cursor: null, limit: 30 },
    { cursor: 'next', limit: 30 }
  ])
  const remote = [{ id: 'remote' }],
    local = [
      { id: 'local', localId: 'local' },
      { id: 'independent', localId: 'independent' }
    ]
  assert.deepEqual(
    JSON.parse(JSON.stringify(h.mergeLibraryPlaylists(remote, local, () => ({ id: 'local' })))),
    [...local]
  )
  assert.equal(h.mergeLibraryPlaylists([], local, () => undefined).length, 2)
})

test('a persisted conversion overrides stale presentation state before a same-platform remote write', async () => {
  const h = targetsHarness({
    native: async (command) => {
      if (command === 'aggregate_playlist')
        return {
          id: 'persisted',
          link: {
            reference: { source: 'netease', id: 'p' },
            account: { source: 'netease', id: '42' }
          }
        }
      assert.equal(command, 'aggregate_playlists')
      return [
        {
          id: 'persisted',
          link: {
            reference: { source: 'netease', id: 'p' },
            account: { source: 'netease', id: '42' }
          }
        }
      ]
    }
  })
  await h.addToPlaylist(h.target, [{ key: 'n', reference: { source: 'netease', id: 'n' } }])
  assert.equal(h.calls[0][0], 'remote')
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls.at(-1))), {
    operation: 'add',
    id: 'persisted',
    keys: ['n']
  })
})

test('linked native targets upload only matching platform tracks before one local batch', async () => {
  const h = targetsHarness()
  await h.addToAggregatePlaylist('existing', [
    { key: 'n', reference: { source: 'netease', id: 'n' } },
    { key: 'q', reference: { source: 'qqmusic', id: 'q' } },
    { key: 'file' }
  ])
  assert.equal(h.calls.length, 2)
  assert.equal(h.calls[0][0], 'remote')
  assert.equal(h.calls[0][2].track.id, 'n')
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls[1])), {
    operation: 'add',
    id: 'existing',
    keys: ['n', 'q', 'file']
  })
})

test('remote failure or a non-selected linked account rejects the whole local addition', async () => {
  const h = targetsHarness({
    write: async () => {
      throw new Error('server refused')
    }
  })
  await assert.rejects(
    h.addToAggregatePlaylist('existing', [{ key: 'n', reference: { source: 'netease', id: 'n' } }]),
    /上传失败，本机尚未添加/
  )
  assert.deepEqual(h.calls, [])
  const switched = targetsHarness({
    current: async () => ({ reference: { source: 'netease', id: 'other' } })
  })
  await assert.rejects(
    switched.addToAggregatePlaylist('existing', [
      { key: 'n', reference: { source: 'netease', id: 'n' } }
    ]),
    /关联账号/
  )
  assert.deepEqual(switched.calls, [])
})

test('detached playlists never upload and partial remote success does not commit the local batch', async () => {
  const detached = targetsHarness({ native: async () => ({ id: 'detached', link: null }) })
  await detached.addToAggregatePlaylist('detached', [
    { key: 'n', reference: { source: 'netease', id: 'n' } }
  ])
  assert.equal(detached.calls.length, 1)
  assert.equal(detached.calls[0].operation, 'add')
  let uploaded = 0,
    local = 0
  const h = targetsHarness({
    write: async () => {
      if (++uploaded === 2) throw new Error('network')
    },
    aggregate: async () => local++
  })
  await assert.rejects(
    h.addToAggregatePlaylist(
      'existing',
      [1, 2].map((id) => ({ key: String(id), reference: { source: 'netease', id: String(id) } }))
    ),
    /已有 1 首上传成功/
  )
  assert.equal(local, 0)
})

test('aggregate detail reuses the platform header and keeps deletion and refresh inside the ellipsis menu', () => {
  const jsx = (type, props) => ({ type, props })
  const edits = [],
    calls = []
  const menu = Object.fromEntries(
    ['Root', 'Trigger', 'Portal', 'Positioner', 'Popup', 'Item', 'Separator'].map((name) => [
      name,
      `Menu.${name}`
    ])
  )
  const { AggregateHeader } = load('../src/features/aggregate/aggregate-header.tsx', {
    '@base-ui/react/menu': { Menu: menu },
    'lucide-react': Object.fromEntries(
      [
        'Ellipsis',
        'Unlink',
        'Trash2',
        'Pencil',
        'ExternalLink',
        'RefreshCw',
        'X',
        'ListFilter'
      ].map((name) => [name, name])
    ),
    'react/jsx-runtime': { jsx, jsxs: jsx, Fragment: 'Fragment' },
    '@/components/music/action-button': { ActionButton: 'Button' },
    '@/components/error-notice': { ErrorNotice: 'Error' },
    '@/features/library/collection-header': { CollectionHeader: 'SharedHeader' },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ navigate: (...args) => calls.push(args) })
    },
    './aggregate-library': {
      aggregateCollection: (p) => ({ id: p.id, localId: p.id, kind: 'playlist', name: p.name })
    },
    './aggregate-cover': { AggregateCover: 'Cover' },
    './sync-control': {
      usePlaylistSync: (...args) => {
        calls.push(['sync', ...args])
        return { start: (force) => calls.push(['refresh', force]) }
      }
    }
  })
  const playlist = {
    id: 'local',
    name: 'Mix',
    trackCount: 3,
    covers: [],
    link: { reference: { source: 'netease', kind: 'playlist', id: 'original' }, lastSuccess: 0 }
  }
  const tree = AggregateHeader({
    playlist,
    busy: false,
    onPlay() {},
    onEdit: (operation) => edits.push(operation),
    onExclusions: () => calls.push(['exclude'])
  })
  const flatten = (node) =>
    Array.isArray(node)
      ? node.flatMap(flatten)
      : node && typeof node === 'object'
        ? [node, ...flatten(node.props?.children)]
        : []
  const header = flatten(tree).find((node) => node.type === 'SharedHeader')
  assert.equal(header.props.cover.props.className, 'library-detail-cover')
  assert.equal(header.props.total, 3)
  const actions = flatten(header.props.actions)
  assert.deepEqual(
    actions.filter((node) => node.type === 'Button').map((node) => node.props.children[1]),
    ['解除关联']
  )
  const trigger = actions.find((node) => node.type === 'Menu.Trigger')
  assert.equal(trigger.props.render.props['aria-label'], '歌单菜单')
  assert.equal(trigger.props.children.type, 'Ellipsis')
  const item = (label) =>
    actions.find((node) => node.type === 'Menu.Item' && node.props.children[1] === label)
  item('删除本机歌单').props.onClick()
  item('从原歌单更新').props.onClick()
  item('查看原歌单').props.onClick()
  assert.deepEqual(edits, ['delete'])
  assert.ok(calls.some((call) => call[0] === 'refresh' && call[1] === true))
  const origin = calls.find((call) => call[0] === 'collection')[2]
  assert.equal(origin.bypassAggregate, true)
  assert.equal(origin.localId, undefined)
  assert.equal(origin.id, 'original')
})
