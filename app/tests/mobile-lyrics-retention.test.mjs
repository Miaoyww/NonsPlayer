import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'
import React, { act, useEffect, useState } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'

test('reopening lyrics retains the same mounted renderer and pauses it while the cover is visible', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const oldWindow = globalThis.window,
    oldDocument = globalThis.document
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let mounts = 0,
    unmounts = 0
  function Probe({ active }) {
    useEffect(() => {
      mounts++
      return () => {
        unmounts++
      }
    }, [])
    return React.createElement('span', { 'data-renderer-active': String(active) }, 'lyrics')
  }
  const modules = {
    'react/jsx-runtime': jsx,
    react: React,
    '@tauri-apps/api/core': { isTauri: () => true },
    'lucide-react': {
      Info: () => null,
      Repeat: () => null,
      Repeat1: () => null,
      ChevronDown: () => null
    },
    '@/components/music/action-button': { ActionButton: 'button' },
    '@/components/music/cover': { Cover: () => null },
    '@/components/music/music-links': { TrackArtists: () => null },
    '@/components/music/song-actions': { useSongActions: () => ({ details() {} }) },
    '@/features/lyrics/use-mobile-lyrics-page': {
      useMobileLyricsPage() {
        const [lyricsPage, set] = useState(false)
        return { lyricsPage, showLyricsPage: () => set(true), showOverview: () => set(false) }
      }
    },
    '@/lib/player': { usePlayer: () => ({ repeatMode: 'off', shuffle: false }) },
    './current-track-like': { CurrentTrackLike: () => null },
    './music-options': { QualitySelect: () => null },
    './now-playing-menu': { NowPlayingMenu: () => null },
    './playback-timeline': { PlaybackTimeline: () => null },
    './playback-transport': { PlaybackTransport: ({ leadingControl }) => leadingControl ?? null }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/playback/mobile-fullscreen-player.tsx', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    { exports, require: (name) => modules[name] ?? {} }
  )
  const root = createRoot(dom.window.document.getElementById('root'))
  try {
    await act(async () =>
      root.render(
        React.createElement(exports.MobileFullscreenPlayer, {
          track: { key: 'test', title: 'test' },
          present: true,
          loading: false,
          onError() {},
          onClose() {},
          onRefresh() {},
          onQueue() {},
          renderLyrics: (active) => React.createElement(Probe, { active })
        })
      )
    )
    const tap = async (label) =>
      act(async () =>
        dom.window.document
          .querySelector(`[aria-label="${label}"]`)
          .dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }))
      )
    assert.equal(mounts, 0)
    await tap('查看歌词')
    assert.equal(mounts, 1)
    await tap('返回封面与播放控制')
    assert.equal(unmounts, 0, 'switching to the cover must not dispose the lyric player')
    assert.equal(
      dom.window.document.querySelector('[data-renderer-active]').dataset.rendererActive,
      'false'
    )
    await tap('查看歌词')
    assert.equal(mounts, 1)
    assert.equal(
      dom.window.document.querySelector('[data-renderer-active]').dataset.rendererActive,
      'true'
    )
  } finally {
    await act(async () => root.unmount())
    globalThis.window = oldWindow
    globalThis.document = oldDocument
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
    dom.window.close()
  }
})
