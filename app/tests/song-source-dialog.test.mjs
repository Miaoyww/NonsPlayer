import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('song source selection keeps queue identity, sends explicit alternate keys, reports errors and rejects stale playback', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const track = {
    key: 'netease:1',
    title: 'Song',
    artist: 'Artist',
    source: { kind: 'netease', id: 1 }
  }
  const alternate = {
    ...track,
    key: 'qq:2',
    source: { kind: 'online', track: { reference: { source: 'qqmusic', kind: 'track', id: '2' } } }
  }
  const state = {
    revision: 7,
    index: 0,
    queue: [track],
    playbackSource: {
      provider: 'huibq',
      displayName: 'Huibq',
      platform: 'netease',
      title: 'Song',
      artist: 'Artist'
    }
  }
  const statuses = [
    {
      enabled: true,
      descriptor: { source: 'netease', displayName: '网易云音乐', capabilities: ['search'] }
    },
    {
      enabled: true,
      descriptor: {
        source: 'qqmusic',
        displayName: 'QQ 音乐',
        capabilities: ['search'],
        platforms: ['qqmusic', 'netease']
      }
    }
  ]
  const requests = []
  let closes = 0
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    'lucide-react': { Radio: () => null, RotateCcw: () => null },
    '@/components/ui/dialog': Object.fromEntries(
      ['Dialog', 'DialogContent', 'DialogDescription', 'DialogHeader', 'DialogTitle'].map(
        (name) => [name, ({ children }) => React.createElement('div', null, children)]
      )
    ),
    '@/components/music/action-button': {
      ActionButton: ({ variant, ...props }) => React.createElement('button', props)
    },
    '@/features/music/use-music-sources': { useMusicSources: () => ({ statuses }) },
    '@/features/music/resource-providers': {
      useResourceProviders: () => ({
        providers: [
          {
            enabled: true,
            descriptor: {
              id: 'huibq',
              displayName: 'Huibq',
              platforms: ['netease'],
              searchPlatforms: []
            }
          }
        ]
      }),
      providerSearchSources: () => []
    },
    '@/features/music/source-label': {
      sourceLabel: (source) => (source === 'netease' ? '网易云音乐' : source),
      trackSource: () => 'netease'
    },
    '@/features/search/source-track-search': {
      SourceTrackSearch: ({ onSelect }) =>
        React.createElement('button', { onClick: () => onSelect(alternate) }, '选择 QQ 对应歌曲')
    },
    '@/lib/player': {
      usePlayer: () => state,
      getPlayer: () => state,
      errorText: String,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/playback/song-source-dialog.tsx', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  const root = createRoot(document.getElementById('root'))
  const click = async (label) =>
    act(async () => {
      ;[...document.querySelectorAll('button')].find((b) => b.textContent === label).click()
    })
  try {
    await act(async () =>
      root.render(React.createElement(exports.SongSourceDialog, { track, onClose: () => closes++ }))
    )
    assert.match(document.body.textContent, /仅对当前队列条目有效/)
    assert.match(document.body.textContent, /播放音源：Huibq/)
    await click('Huibq')
    assert.equal(requests[0].command, 'player_select_source')
    assert.deepEqual(JSON.parse(JSON.stringify(requests[0].args)), {
      revision: 7,
      provider: 'huibq',
      alternateKey: null
    })
    assert([...document.querySelectorAll('button')].find((b) => b.textContent === 'Huibq').disabled)
    await act(async () => requests[0].reject('音源不可用'))
    assert.match(document.querySelector('[role="alert"]').textContent, /音源不可用/)
    assert.equal(closes, 0)
    await click('选择 QQ 对应歌曲')
    assert.equal(requests[1].args.alternateKey, alternate.key)
    assert.equal(requests[1].args.provider, null)
    await act(async () => requests[1].resolve())
    assert.equal(closes, 1)
    assert.equal(state.queue[0], track)
    await click('QQ 音乐')
    assert.equal(requests[2].args.provider, 'qqmusic')
    assert.equal(requests[2].args.alternateKey, null)
    await act(async () => requests[2].resolve())
    assert.equal(closes, 2)
    state.revision = 8
    await click('Huibq')
    assert.equal(requests.length, 3)
    assert.equal(closes, 3)
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
