import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

const require = createRequire(import.meta.url)

test('offset saves keep AMLL lines and its player intact while updating paused and playing clocks', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const frames = new Map()
  let nextFrame = 0
  const previous = Object.fromEntries(
    ['window', 'document', 'IS_REACT_ACT_ENVIRONMENT'].map((key) => [
      key,
      Object.getOwnPropertyDescriptor(globalThis, key)
    ])
  )
  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    IS_REACT_ACT_ENVIRONMENT: true
  })
  document.fonts = { ready: Promise.resolve() }
  const counts = { constructed: 0, setLines: 0, rebuilds: 0, requests: 0, parses: 0 }
  const calls = []
  const listeners = new Map()
  const settings = { lyricsOffsetMs: -200, autoBlur: false, fontWeight: 400, fontScale: 1 }
  const progress = { positionMs: 5500, status: 'paused' }
  const track = { key: 'song:1', durationMs: 20000, source: { kind: 'online' } }
  const player = { queue: [track], index: 0, revision: 1, status: 'paused' }
  const sources = { localSource: 'online', amll: true, qq: true }
  const localPreferences = {}
  const rawLines = [
    {
      startTime: 1000,
      endTime: 9000,
      words: [{ word: 'Hello', startTime: 1000, endTime: 9000 }],
      translatedLyric: '你好',
      romanLyric: 'ni hao'
    }
  ]
  let core
  class ProbePlayer extends EventTarget {
    constructor() {
      super()
      counts.constructed++
      this.element = document.createElement('div')
      this.lines = []
      core = this
      return new Proxy(this, {
        get(target, key) {
          const value = Reflect.get(target, key)
          return typeof value === 'function'
            ? value.bind(target)
            : value === undefined
              ? () => {}
              : value
        }
      })
    }
    getElement() {
      return this.element
    }
    getBottomLineElement() {
      return null
    }
    getLyricLines() {
      return this.lines
    }
    setLyricLines(lines) {
      counts.setLines++
      this.lines = lines
      this.rebuildLyricView()
    }
    rebuildLyricView() {
      counts.rebuilds++
    }
    setCurrentTime(value) {
      this.time = value
    }
  }
  const empty = () => null
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    'react-dom': require('react-dom'),
    '@applemusic-like-lyrics/core': {
      LyricPlayer: ProbePlayer,
      MaskObsceneWordsMode: { Disabled: 0 }
    },
    '@applemusic-like-lyrics/core/style.css': {},
    './manual-lyric-player': { ManualLyricPlayer: ProbePlayer },
    'lucide-react': new Proxy({}, { get: () => empty }),
    'motion/react': {
      useReducedMotion: () => false,
      useIsPresent: () => true,
      motion: {
        section: ({ children, className, style }) =>
          React.createElement('section', { className, style }, children)
      }
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => {
        listeners.set(name, callback)
        return () => listeners.delete(name)
      }
    },
    '@/lib/platform': { isAndroid: () => false },
    '@/lib/player': {
      usePlayer: () => player,
      getPlayer: () => player,
      useProgress: () => progress,
      currentPosition: () => progress.positionMs + 250,
      errorText: String,
      nativeCall: async (command, args) => {
        calls.push({ command, ...args })
        if (command === 'lyric_correction')
          return { source: null, candidate: null, localTtml: null, offsetMs: 0 }
        if (command === 'set_lyric_offset') {
          listeners.get('lyric-offset-changed')?.({
            payload: { key: args.key, offsetMs: args.offsetMs }
          })
          // Deliver the native response separately from its event.
          await new Promise(setImmediate)
        }
      }
    },
    './use-lyrics-settings': { useLyricsSettings: () => settings },
    '@/features/settings/appearance/use-font-settings': {
      useFontSettings: () => ({ fonts: { lyrics: '', app: '' } })
    },
    '@/components/music/song-actions': { useSongActions: () => ({ copy() {} }) },
    '@base-ui/react/context-menu': { ContextMenu: {} },
    '@/features/local/use-local-preferences': {
      useLocalPreferences: () => ({ options: localPreferences })
    },
    '@/features/lyrics/use-lyric-sources': {
      useLyricSources: () => ({ sources }),
      getLyricSources: () => sources
    },
    '@/features/lyrics/load-lyrics': {
      lyricTrackVersion: (value) => JSON.stringify(value),
      loadLyrics: async (_track, _refresh, _sources, parse) => {
        counts.requests++
        parse({ source: 'netease', format: 'lrc' })
      }
    },
    '@/features/lyrics/parse-lyrics': {
      EmptyLyricsError: class extends Error {},
      parseLyrics: () => {
        counts.parses++
        return rawLines
      }
    },
    '@/lib/runtime-cache': { requestCache: { revision: 0 }, RUNTIME_TTL: 600000 },
    '@/components/ui/panel': {
      Panel: ({ value, pages }) => pages.find((page) => page.value === value)?.content
    },
    '@/components/music/action-button': {
      ActionButton: ({ children, variant, size, ...props }) =>
        React.createElement('button', props, children)
    },
    '@/features/mv/use-local-mv': { useLocalMv: () => ({ mode: 'lyrics' }) },
    './panel-visibility': { usePanelHidden: () => React.useState(false) },
    '@/plugins/host': { usePluginViews: () => [], PluginSlot: empty },
    '@/features/playback/now-playing/cover-context-menu': {
      CoverContextMenu: ({ children }) => children
    }
  }
  for (const [path, names] of Object.entries({
    '@/components/error-notice': ['ErrorNotice'],
    '@/components/music/music-links': ['TrackArtists', 'TrackAlbum'],
    '@/components/music/cover': ['Cover'],
    '@/features/lyrics/album-background': ['AlbumBackground'],
    '@/features/playback/bar/fullscreen-playback-bar': ['FullscreenPlaybackBar'],
    '@/features/playback/now-playing/now-playing-menu': ['NowPlayingMenu'],
    '@/features/queue/queue-panel': ['QueuePanel'],
    '@/features/playback/now-playing/now-playing-tags': ['NowPlayingTags'],
    '@/features/playback/now-playing/now-playing-source': ['NowPlayingSource'],
    '@/features/lyrics/now-playing-lyric-source': ['NowPlayingLyricSource'],
    './lyric-loading': ['LyricLoading'],
    '@/features/playback/mobile/mobile-fullscreen-player': ['MobileFullscreenPlayer'],
    '@/features/playback/bar/playback-notice': ['PlaybackNotice'],
    '@/features/playback/now-playing/now-playing-information': ['NowPlayingInformation'],
    '@/components/ui/spinner': ['Spinner'],
    '@/features/mv/local-mv-stage': ['LocalMvStage'],
    '@/features/mv/mv-alignment-popover': ['MvAlignmentPopover'],
    '@/features/mv/mv-panel': ['MvPanel'],
    '@/features/mv/mv-lyric-card': ['MvLyricCard']
  }))
    modules[path] = Object.fromEntries(names.map((name) => [name, empty]))
  function load(path) {
    const exports = {}
    runInNewContext(
      ts.transpileModule(readFileSync(path, 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        document,
        performance,
        ResizeObserver: class {
          observe() {}
          disconnect() {}
        },
        requestAnimationFrame: (callback) => {
          frames.set(++nextFrame, callback)
          return nextFrame
        },
        cancelAnimationFrame: (id) => frames.delete(id),
        require: (name) => {
          assert.ok(Object.hasOwn(modules, name), `unexpected dependency: ${name}`)
          return modules[name]
        }
      }
    )
    return exports
  }
  modules['@applemusic-like-lyrics/react'] = load(require.resolve('@applemusic-like-lyrics/react'))
  modules['./correction'] = load(new URL('../src/features/lyrics/correction.ts', import.meta.url))
  modules['./lyric-copy'] = load(new URL('../src/features/lyrics/lyric-copy.ts', import.meta.url))
  modules['./lyric-renderer'] = load(
    new URL('../src/features/lyrics/lyric-renderer.tsx', import.meta.url)
  )
  modules['./lyric-offset-control'] = load(
    new URL('../src/features/lyrics/lyric-offset-control.tsx', import.meta.url)
  )
  const LyricsView = load(
    new URL('../src/features/lyrics/lyrics-view.tsx', import.meta.url)
  ).default
  const root = createRoot(document.getElementById('root'))
  const render = async () =>
    React.act(async () =>
      root.render(React.createElement(LyricsView, { onClose() {}, onQueue() {} }))
    )
  const flushFrames = async () =>
    React.act(async () => {
      const pending = [...frames.values()]
      frames.clear()
      pending.forEach((callback) => callback(performance.now()))
    })
  try {
    await render()
    await flushFrames()
    const baseline = { ...counts }
    const firstCore = core
    const firstLines = core.lines
    const initialTime = core.time
    await React.act(async () => {
      document.querySelector('[aria-label="歌词延后 0.1 秒"]').click()
      await new Promise(setImmediate)
    })
    await flushFrames()
    assert.equal(core.lines, firstLines, 'offset-only changes must retain AMLL lyric lines')
    assert.equal(counts.setLines, baseline.setLines, 'AMLL must not reload its lyrics')
    assert.equal(counts.rebuilds, baseline.rebuilds, 'AMLL must not rebuild its view')
    assert.equal(initialTime, 5700)
    assert.equal(core.time, 5600)
    for (const offsetMs of [100, -300, 0]) {
      await React.act(async () =>
        listeners.get('lyric-offset-changed')({ payload: { key: track.key, offsetMs } })
      )
      await flushFrames()
      assert.equal(core.time, progress.positionMs - (settings.lyricsOffsetMs + offsetMs))
      assert.equal(core.lines, firstLines)
    }
    settings.lyricsOffsetMs = 350
    await render()
    await flushFrames()
    assert.equal(core.time, 5150)
    assert.equal(counts.requests, baseline.requests)
    assert.equal(counts.parses, baseline.parses)
    progress.status = 'playing'
    await render()
    await flushFrames()
    assert.equal(core.time, 5400, 'the interpolated clock also applies the offset')
    await React.act(async () => {
      core.dispatchEvent(
        Object.assign(new Event('line-click'), { line: { getLine: () => rawLines[0] } })
      )
    })
    assert.equal(calls.at(-1).command, 'player_seek')
    assert.equal(calls.at(-1).positionMs, 1350)
    assert.equal(core, firstCore)
    assert.equal(core.lines, firstLines)
    assert.equal(counts.setLines, baseline.setLines)
    assert.equal(counts.rebuilds, baseline.rebuilds)
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    for (const [key, descriptor] of Object.entries(previous)) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor)
      else delete globalThis[key]
    }
  }
})
