import { errorNoticeModule } from './helpers/error-notice.mjs'
import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import * as table from '@tanstack/react-table'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

async function harness(run, android = false) {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    IS_REACT_ACT_ENVIRONMENT: true
  })
  const tracks = [
    { key: 'a', title: 'Zulu', artist: 'Alpha', album: 'Second', durationMs: 3000 },
    {
      key: 'b',
      title: '白夜',
      aliases: ['Midnight'],
      artist: 'Beta',
      album: 'First',
      durationMs: 1000
    },
    { key: 'a', title: 'Zulu', artist: 'Alpha', album: 'Second', durationMs: 3000 }
  ].map((track) => ({ ...track, source: { kind: 'local' }, cover: '' }))
  const plays = [],
    removes = [],
    menus = [],
    copies = []
  const virtualizer = {
    measure() {},
    measureElement() {},
    getTotalSize: () => 0,
    scrollToIndex() {},
    getVirtualItems: () => []
  }
  const trackSearchContext = React.createContext(undefined)
  const modules = {
    '@/features/music/use-music-sources': { useMusicSources: () => ({ statuses: [] }) },
    '@/lib/platform': { isAndroid: () => android },
    './list-find': { useListFind() {} },
    '@/components/ui/skeleton': {
      Skeleton: (props) => React.createElement('div', { 'data-slot': 'skeleton', ...props })
    },
    react: React,
    '@tanstack/react-table': table,
    'react/jsx-runtime': jsx,
    '@tanstack/react-virtual': {
      useVirtualizer: ({ count }) => {
        virtualizer.getVirtualItems = () =>
          Array.from({ length: count }, (_, index) => ({ index, start: 0, end: 0 }))
        return virtualizer
      }
    },
    '@/components/music/infinite-load': {
      scrollParent: (element) => {
        assert.equal(element.className, 'track-table-container')
        return null
      }
    },
    '@/components/music/track-search-context': {
      TrackSearchContext: trackSearchContext
    },
    '@/components/ui/input': { Input: (props) => React.createElement('input', props) },
    '@/components/music/music-links': { TrackAlbum: ({ track }) => track.album },
    './music-links': { TrackArtists: ({ track }) => track.artist },
    './cover': { Cover: () => null },
    './track-title': {
      TrackTitle: ({ track }) => track.title,
      trackDisplayTitle: (track) => track.title
    },
    '@/components/music/action-button': {
      ActionButton: ({ size, variant, ...props }) => React.createElement('button', props)
    },
    '@/components/music/cover': { Cover: () => null },
    '@/components/music/playing-indicator': {
      PlayingIndicator: () => React.createElement('span', { 'aria-label': '当前曲目' })
    },
    '@/components/music/track-identity': {
      TrackIdentity: ({ track, cover }) => React.createElement('div', {}, cover, track.title)
    },
    '@/components/music/song-actions': {
      useSongTitleCopy: () => (title) => copies.push(title),
      SongContextMenu: ({ render, children, ...props }) => {
        menus.push(props)
        return React.cloneElement(render, {}, children)
      }
    },
    '@/features/account/account': {
      useAccount: () => ({ getAccount: () => ({ likedIds: new Set(), pendingLikes: new Set() }) })
    },
    '@/features/music/source-label': {
      sourceLabel: (source) => (source === 'netease' ? '网易云音乐' : source),
      trackSource: (track) =>
        track.source.kind === 'online' ? track.source.track.reference.source : track.source.kind
    },
    '@/features/settings/use-interface-density': { useInterfaceDensity: () => ['compact'] },
    '@/lib/player': { errorText: String, formatTime: String },
    'lucide-react': Object.fromEntries(
      ['Heart', 'ListPlus', 'Play', 'Search', 'ArrowUpDown', 'ArrowUp', 'ArrowDown'].map((name) => [
        name,
        () => null
      ])
    )
  }
  function load(path) {
    const exports = {}
    runInNewContext(
      ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        require: (name) => {
          if (name === '@/components/error-notice') return errorNoticeModule
          assert.ok(name === '@/features/music/legacy-reference' || name in modules, name)
          return name === '@/features/music/legacy-reference' ? musicReference : modules[name]
        },
        ResizeObserver: class {
          observe() {}
          disconnect() {}
        }
      }
    )
    return exports
  }
  modules['./track-column-sizing'] = load('../src/components/music/track-column-sizing.tsx')
  modules['@/components/music/track-identity'] = load('../src/components/music/track-identity.tsx')
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/components/music/track-list.tsx', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    {
      exports,
      require: (name) => {
        if (name === '@/components/error-notice') return errorNoticeModule
        assert.ok(name === '@/features/music/legacy-reference' || name in modules, name)
        return name === '@/features/music/legacy-reference' ? musicReference : modules[name]
      },
      ResizeObserver: class {
        observe() {}
        disconnect() {}
      }
    }
  )
  const root = createRoot(document.getElementById('root'))
  const pagination = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/lib/use-paged-list.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    {
      exports: pagination,
      require: (name) =>
        name === '@/components/error-notice'
          ? errorNoticeModule
          : name === 'react'
            ? React
            : modules['@/lib/player']
    }
  )
  function PagedResults({ pageLoader, kind, ...props }) {
    const list = pagination.usePagedList(pageLoader, kind === 'history' ? 100 : 30, true)
    return kind === 'history'
      ? React.createElement(exports.TrackList, { ...props, tracks: list.items })
      : React.createElement('p', {}, list.items.map((item) => item.name).join(', '))
  }
  const render = async (props = {}) => {
    menus.length = 0
    const { externalQuery, ...listProps } = props
    await act(async () =>
      root.render(
        React.createElement(
          trackSearchContext.Provider,
          { value: externalQuery },
          React.createElement(props.pageLoader ? PagedResults : exports.TrackList, {
            tracks,
            busy: false,
            searchable: true,
            onPlay: (...args) => plays.push(args),
            onRemove: (...args) => removes.push(args),
            ...listProps
          })
        )
      )
    )
  }
  const search = async (value) => {
    const input = document.querySelector('input')
    const propsKey = Object.keys(input).find((key) => key.startsWith('__reactProps'))
    await act(async () => input[propsKey].onChange({ target: { value } }))
  }
  try {
    await render()
    await run({ tracks, plays, removes, menus, copies, render, search, dom })
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      IS_REACT_ACT_ENVIRONMENT: previous.act
    })
  }
}

test('mobile detail filtering removes extra columns and plays the filtered queue without an internal input', async () => {
  await harness(async ({ render, plays }) => {
    await render({ externalQuery: 'first', searchable: false, showLikes: true, sortable: true })
    assert.equal(document.querySelectorAll('input').length, 0)
    assert.equal(document.querySelectorAll('.track-column-resizer').length, 0)
    assert.deepEqual(
      [...document.querySelectorAll('thead th')].map((cell) => cell.textContent.includes('标题')),
      [true, false]
    )
    assert.equal(document.querySelectorAll('.track-row').length, 1)
    assert.equal(document.querySelector('.track-row').children.length, 2)
    assert.equal(document.querySelector('.track-list').style.minWidth, '')
    await act(async () => document.querySelector('.track-cover').click())
    assert.deepEqual(
      plays.at(-1)[1].map((track) => track.key),
      ['b']
    )
    await render({ externalQuery: 'no-match', searchable: false })
    assert.equal(document.querySelectorAll('.track-row').length, 0)
    assert.match(document.querySelector('[role="status"]').textContent, /没有匹配/)
    await render({ externalQuery: '', searchable: false })
    assert.equal(document.querySelectorAll('.track-row').length, 3)
  }, true)
})

test('the playback indicator follows the current queue occurrence through search and sorting', async () => {
  await harness(async ({ render, search }) => {
    await render({ currentKey: 'a', currentIndex: 2, sortable: true })
    const indicators = () => document.querySelectorAll('[aria-label="当前曲目"]')
    assert.equal(indicators().length, 1)
    const currentRow = indicators()[0].closest('.track-row')
    assert.equal(currentRow, document.querySelectorAll('.track-row')[2])
    await search('Zulu')
    assert.equal(indicators().length, 1)
    assert.equal(indicators()[0].closest('.track-row'), currentRow)
    await act(async () => document.querySelector('[aria-label^="时长排序："]').click())
    assert.equal(indicators().length, 1)
    assert.equal(indicators()[0].closest('.track-row'), currentRow)
    await search('Beta')
    assert.equal(indicators().length, 0)
  })
})

test('list search matches aliases, artists and albums and plays the filtered queue', async () => {
  await harness(async ({ plays, search }) => {
    for (const query of [' MIDNIGHT ', 'beta', 'first']) {
      await search(query)
      const buttons = document.querySelectorAll('.track-cover')
      assert.equal(buttons.length, 1)
      await act(async () => buttons[0].click())
      assert.equal(plays.at(-1)[0], 0)
      assert.deepEqual(
        plays.at(-1)[1].map((track) => track.key),
        ['b']
      )
    }
    await search('missing')
    assert.match(document.body.textContent, /没有匹配的歌曲/)
    await search('')
    assert.equal(document.querySelectorAll('.track-cover').length, 3)
  })
})

test('loaded collection cards can switch to history through real pagination and TrackList', async () => {
  await harness(async ({ render, tracks, plays }) => {
    const collections = async () => ({
      items: [{ kind: 'playlist', id: 1, name: 'Collection without a track source' }],
      more: true
    })
    let resolveHistory
    const history = () => new Promise((resolve) => (resolveHistory = resolve))
    await render({ pageLoader: collections, kind: 'playlist' })
    assert.match(document.body.textContent, /Collection without a track source/)
    await render({ pageLoader: history, kind: 'history' })
    assert.equal(document.querySelectorAll('.track-cover').length, 0)
    await act(async () => resolveHistory({ items: tracks, more: false }))
    assert.equal(document.querySelectorAll('.track-cover').length, tracks.length)
    await act(async () => document.querySelector('.track-cover').click())
    assert.deepEqual([...plays.at(-1)[1]], tracks)
  })
})

test('filtered duplicate rows retain original removal indices', async () => {
  await harness(async ({ menus, removes, search }) => {
    await search('Alpha')
    menus.at(-1).onRemove()
    assert.equal(removes.at(-1)[1], 2)
  })
})

test('inserting and deleting songs preserves existing row DOM and search state', async () => {
  await harness(async ({ render, tracks, search }) => {
    await search('Zulu')
    const oldRows = [...document.querySelectorAll('.track-row')]
    const added = { ...tracks[0], key: 'new', title: 'Zulu New' }
    await render({ tracks: [added, ...tracks] })
    let rows = [...document.querySelectorAll('.track-row')]
    assert.equal(rows.length, 3)
    assert.equal(rows[1], oldRows[0])
    assert.equal(rows[2], oldRows[1])
    await render({ tracks: tracks.slice(0, 2) })
    rows = [...document.querySelectorAll('.track-row')]
    assert.equal(rows.length, 1)
    assert.equal(rows[0], oldRows[0])
    assert.equal(document.querySelector('input').value, 'Zulu')
  })
})

test('sequence header restores source order directly after sorting and filtering', async () => {
  await harness(async ({ render, plays, search }) => {
    await render({ sortable: true })
    const reset = () => document.querySelector('button[aria-label="按序排列"]')
    assert.ok(reset(), 'original sequence has a dedicated header control')
    await act(async () => document.querySelector('[aria-label^="时长排序："]').click())
    await search('Zulu')
    await act(async () => reset().click())
    await search('')
    await act(async () => document.querySelector('.track-cover').click())
    assert.deepEqual(
      plays.at(-1)[1].map((track) => track.key),
      ['a', 'b', 'a']
    )
    assert.equal(reset().getAttribute('aria-pressed'), 'true')
    assert.ok(document.querySelector('[aria-label^="标题排序："]').textContent.includes('按序排列'))
  })
})

test('sorting preserves stable ties, filtered playback order and source indices', async () => {
  await harness(async ({ render, plays, removes, menus, tracks, search }) => {
    await render({ sortable: true })
    const clickSort = async (column) =>
      act(async () => document.querySelector(`[aria-label^="${column}排序："]`).click())
    await clickSort('时长')
    await act(async () => document.querySelector('.track-cover').click())
    assert.deepEqual(
      plays.at(-1)[1].map((track) => track.key),
      ['b', 'a', 'a']
    )
    assert.deepEqual(
      tracks.map((track) => track.key),
      ['a', 'b', 'a'],
      'cached input order is unchanged'
    )
    menus.at(-1).onRemove()
    assert.equal(removes.at(-1)[1], 2)
    await clickSort('时长')
    await act(async () => document.querySelectorAll('.track-cover')[2].click())
    assert.equal(plays.at(-1)[0], 2)
    assert.deepEqual(
      plays.at(-1)[1].map((track) => track.key),
      ['a', 'a', 'b']
    )
    await clickSort('专辑')
    await search('Beta')
    await act(async () => document.querySelector('.track-cover').click())
    assert.deepEqual(
      plays.at(-1)[1].map((track) => track.key),
      ['b']
    )
    await search('')
    await clickSort('专辑')
    await clickSort('专辑')
    await act(async () => document.querySelector('.track-cover').click())
    assert.deepEqual(
      plays.at(-1)[1].map((track) => track.key),
      ['a', 'b', 'a']
    )
  })
})

test('title header cycles title and artist directions then restores default, other headers reset the cycle', async () => {
  await harness(async ({ render, plays }) => {
    await render({ sortable: true })
    const click = async (column) =>
      act(async () => document.querySelector(`[aria-label^="${column}排序："]`).click())
    for (const [next, expected, direction] of [
      ['标题降序', ['b', 'a', 'a'], 'ascending'],
      ['歌手升序', ['a', 'a', 'b'], 'descending'],
      ['歌手降序', ['a', 'a', 'b'], 'ascending'],
      ['恢复默认排序', ['b', 'a', 'a'], 'descending'],
      ['标题升序', ['a', 'b', 'a'], null]
    ]) {
      await click('标题')
      assert.equal(
        document.querySelector('[aria-label^="标题排序："]').getAttribute('aria-label'),
        `标题排序：${next}`
      )
      assert.equal(
        document
          .querySelector('[aria-label^="标题排序："]')
          .closest('th')
          .getAttribute('aria-sort'),
        direction
      )
      await act(async () => document.querySelector('.track-cover').click())
      assert.deepEqual(
        plays.at(-1)[1].map((track) => track.key),
        expected
      )
    }
    await click('专辑')
    assert.equal(
      document.querySelector('[aria-label^="专辑排序："]').closest('th').getAttribute('aria-sort'),
      'ascending'
    )
    assert.equal(
      document.querySelector('[aria-label^="标题排序："]').closest('th').getAttribute('aria-sort'),
      null
    )
    await click('标题')
    assert.equal(
      document.querySelector('[aria-label^="标题排序："]').closest('th').getAttribute('aria-sort'),
      'ascending'
    )
    assert.equal(
      document.querySelector('[aria-label^="专辑排序："]').closest('th').getAttribute('aria-sort'),
      null
    )
  })
})

test('detail toolbar search filters the actual list without rendering a duplicate input', async () => {
  await harness(async ({ render, plays }) => {
    await render({ externalQuery: 'Beta' })
    assert.equal(document.querySelector('input'), null)
    assert.equal(document.querySelectorAll('.track-cover').length, 1)
    await act(async () => document.querySelector('.track-cover').click())
    assert.deepEqual(
      plays.at(-1)[1].map((track) => track.key),
      ['b']
    )
    await render({ externalQuery: 'missing' })
    assert.match(document.body.textContent, /没有匹配的歌曲/)
  })
})

test('selection enables title copying only on the selected occurrence and survives insertion', async () => {
  await harness(async ({ dom, copies, tracks, render, plays }) => {
    const row = document.querySelectorAll('.track-row')[2]
    assert.equal(document.querySelector('.track-title-copy'), null)
    await act(async () => row.querySelector('.track-title').click())
    assert.equal(row.dataset.selected, 'true')
    assert.equal(copies.length, 0)
    await act(async () => row.querySelector('.track-title-copy').click())
    assert.deepEqual(copies, ['Zulu'])
    assert.equal(plays.length, 0)
    await render({ tracks: [{ ...tracks[1], key: 'new' }, ...tracks] })
    assert.equal(document.querySelectorAll('.track-row')[3], row)
    assert.equal(row.dataset.selected, 'true')
    await act(async () => row.querySelector('.track-cover').click())
    assert.equal(plays.length, 1)
    await act(async () =>
      row.dispatchEvent(new dom.window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    )
    assert.equal(document.querySelector('.track-title-copy'), null)
  })
})

test('resizing redistributes adjacent columns, clamps minimums and restores default widths', async () => {
  await harness(async ({ dom, render }) => {
    const separator = () => document.querySelector('[aria-label="调整标题列宽"]')
    const widths = () =>
      [...document.querySelectorAll('col')].map((col) => parseFloat(col.style.width))
    const initial = widths()
    await act(async () =>
      separator().dispatchEvent(
        new dom.window.MouseEvent('mousedown', { bubbles: true, clientX: 300 })
      )
    )
    await act(async () =>
      document.dispatchEvent(
        new dom.window.MouseEvent('mousemove', { bubbles: true, clientX: 340 })
      )
    )
    await act(async () =>
      document.dispatchEvent(new dom.window.MouseEvent('mouseup', { bubbles: true, clientX: 340 }))
    )
    assert.ok(widths()[1] > initial[1])
    assert.ok(widths()[2] < initial[2])
    assert.ok(Math.abs(widths().reduce((a, b) => a + b, 0) - 100) < 0.001)
    const resized = widths()
    await render({ sortable: true })
    assert.deepEqual(widths(), resized)
    for (let i = 0; i < 20; i++)
      await act(async () =>
        separator().dispatchEvent(
          new dom.window.KeyboardEvent('keydown', {
            key: 'ArrowRight',
            shiftKey: true,
            bubbles: true
          })
        )
      )
    assert.equal(Math.round(widths()[2] * 9), 100)
    await act(async () =>
      separator().dispatchEvent(new dom.window.MouseEvent('dblclick', { bubbles: true }))
    )
    assert.deepEqual(widths(), initial)
  })
})

test('restricted official tracks retain their row and metadata and allow playback to try fallback audio', async () => {
  await harness(async ({ render, tracks }) => {
    await render({
      tracks: [
        tracks[0],
        {
          ...tracks[1],
          source: {
            kind: 'online',
            track: {
              reference: { source: 'qqmusic', kind: 'track', id: 'id:99' },
              unavailableReason: 'notFound'
            }
          }
        },
        tracks[2]
      ]
    })
    const rows = [...document.querySelectorAll('tbody tr')]
    assert.equal(rows.length, 3)
    assert.match(rows[1].textContent, /白夜/)
    assert.equal(
      rows[1].querySelector('[aria-label="官方音源受限，可尝试备用音源"]')?.textContent,
      '· 官方受限'
    )
    assert.equal(rows[1].querySelector('button[aria-label="播放 白夜"]').disabled, false)
    assert.equal(rows[0].querySelector('button[aria-label="播放 Zulu"]').disabled, false)
  })
})

test('loading uses the same table, headers, columns and identity spacing as loaded songs', async () => {
  await harness(async ({ render, tracks }) => {
    const props = { sortable: true, onAppend: () => {}, showLikes: true, externalQuery: '' }
    await render({
      ...props,
      tracks: tracks.map((t) => ({ ...t, source: { kind: 'netease', id: 1 } }))
    })
    const columns = [...document.querySelectorAll('col')].map((c) => c.style.width)
    const headers = document.querySelector('thead').textContent
    const table = document.querySelector('table')
    await render({ ...props, tracks: [], loading: true })
    assert.equal(document.querySelector('table'), table)
    assert.deepEqual(
      [...document.querySelectorAll('col')].map((c) => c.style.width),
      columns
    )
    assert.equal(document.querySelector('thead').textContent, headers)
    assert.equal(document.querySelectorAll('tbody .track-identity-cell').length, 8)
    assert.equal(document.querySelector('tbody tr').children.length, 7)
    assert.equal(document.querySelector('table').getAttribute('aria-busy'), 'true')
    assert.doesNotMatch(document.body.textContent, /没有匹配/)
    assert.equal(
      document.querySelector('tbody .track-identity-cover')?.getAttribute('data-slot'),
      'skeleton'
    )
  })
})

test('track sources have a dedicated column and never appear in the artist text', async () => {
  await harness(async ({ render, tracks }) => {
    await render({
      tracks: tracks.map((track, index) => ({
        ...track,
        source:
          index === 0
            ? {
                kind: 'online',
                track: { reference: { source: 'netease', kind: 'track', id: 'song' } }
              }
            : { kind: 'local', path: 'example.mp3' }
      }))
    })
    assert.match(document.querySelector('thead').textContent, /来源/)
    const rows = [...document.querySelectorAll('tbody tr')]
    assert.equal(rows[0].querySelector('.track-source').textContent, '网易云音乐')
    assert.equal(rows[1].querySelector('.track-source').textContent, 'local')
    for (const row of rows)
      assert.equal(row.querySelector('.track-identity-artist .track-source'), null)
  })
})

test('aggregate row identity, member playback state and source button survive a representative replacement', async () => {
  await harness(async ({ render, tracks, plays }) => {
    const props = {
      tracks: [tracks[0]],
      getRowKey: () => 'stable-group',
      isCurrentTrack: () => true,
      renderSource: () => React.createElement('button', { 'aria-haspopup': 'dialog' }, '来源 ②')
    }
    await render(props)
    const row = document.querySelector('tbody tr[data-index]')
    assert.equal(row.getAttribute('aria-current'), 'true')
    assert.ok(row.querySelector('[aria-label="当前曲目"]'))
    await act(async () => row.querySelector('[aria-haspopup="dialog"]').click())
    assert.equal(plays.length, 0)
    await render({ ...props, tracks: [{ ...tracks[0], key: 'new-source:song' }] })
    assert.equal(document.querySelector('tbody tr[data-index]'), row)
    assert.equal(row.getAttribute('aria-current'), 'true')
  })
})
