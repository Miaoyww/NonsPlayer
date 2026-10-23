import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

const local = {
  key: 'local:disney',
  title: 'Mad at Disney',
  durationMs: 135000,
  source: { kind: 'local', path: 'disney.mp3', neteaseId: null }
}
const streamed = {
  key: 'ncm:disney',
  title: 'Mad at Disney',
  durationMs: 136000,
  source: { kind: 'netease', id: 1 }
}
const localLyrics = { source: 'local', format: 'lrc', content: 'local line lyrics' }
const streamLyrics = {
  source: 'netease',
  format: 'yrc',
  content: 'stream word lyrics with translation'
}

async function harness(track, settle = true, localSource = 'local') {
  let sources = { localSource, amll: true, qq: true }
  let present = true
  let player = { queue: [track], index: 0, status: 'playing', revision: 0 }
  const states = []
  const effects = []
  const events = []
  const pending = []
  const parsed = []
  const requests = []
  let rendererLoads = 0
  let cursor = 0
  const jsx = (type, props) => ({ type, props })
  const modules = {
    '@/lib/platform': { isAndroid: () => false },
    react: {
      useState(initial) {
        const index = cursor++
        states[index] ??= { value: typeof initial === 'function' ? initial() : initial }
        return [
          states[index].value,
          (value) => {
            states[index].value = typeof value === 'function' ? value(states[index].value) : value
          }
        ]
      },
      useRef(value) {
        const index = cursor++
        states[index] ??= { value: { current: value } }
        return states[index].value
      },
      useEffect: (fn) => effects.push(fn),
      useCallback: (fn) => fn,
      useMemo: (fn) => fn(),
      lazy: () => 'DeferredLyricRenderer',
      Suspense: 'Suspense'
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    './correction': {
      offsetLyrics: (lines) => lines,
      automaticLyrics: { candidate: null, offsetMs: 0 }
    },
    '@/lib/player': {
      usePlayer: () => player,
      getPlayer: () => player,
      errorText: String,
      nativeCall: async () => ({ candidate: null, offsetMs: 0 })
    },
    '@/features/lyrics/load-lyrics': {
      lyricTrackVersion: (track) => JSON.stringify(track),
      loadLyrics: (_track, _refresh, _sources, apply, isCurrent) =>
        new Promise((resolve) => {
          requests.push({ key: _track.key, refresh: _refresh })
          pending.push((value) => {
            if (isCurrent()) apply(value)
            resolve(null)
          })
        })
    },
    '@/features/playback/now-playing-menu': { NowPlayingMenu: 'NowPlayingMenu' },
    '@/features/lyrics/parse-lyrics': {
      EmptyLyricsError: class extends Error {},
      parseLyrics: (value) => {
        parsed.push(value)
        return [{ words: [], translatedLyric: '', romanLyric: '' }]
      }
    },
    '@/features/local/use-local-preferences': { useLocalPreferences: () => ({ options: {} }) },
    '@/features/lyrics/use-lyric-sources': {
      useLyricSources: () => ({ sources }),
      getLyricSources: () => sources
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (_name, fn) => {
        events.push(fn)
        return () => {}
      }
    },
    'motion/react': {
      motion: { section: 'section' },
      useIsPresent: () => present,
      useReducedMotion: () => false
    }
  }
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/lyrics-view.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, {
    exports,
    require: (name) => {
      if (name === './lyric-renderer') rendererLoads++
      return name === '@/components/error-notice' ? errorNoticeModule : (modules[name] ?? {})
    }
  })
  const render = () => {
    cursor = 0
    return exports.default({ onQueue() {}, onNotice() {} })
  }
  const initial = render()
  if (settle) initial.props.onAnimationComplete?.()
  effects.forEach((fn) => fn())
  await new Promise(setImmediate)
  return {
    initial,
    setPresent: (value) => {
      present = value
    },
    parsed,
    pending,
    requests,
    rendererLoads: () => rendererLoads,
    events,
    render,
    selectSource: (localSource) => {
      sources = { ...sources, localSource }
    },
    switchTo: (next) => {
      player = { ...player, queue: [next], revision: player.revision + 1 }
    },
    updateMetadata: (next) => {
      player = { ...player, queue: [next] }
    }
  }
}

function menu(node) {
  if (!node || typeof node !== 'object') return
  if (node.type === 'NowPlayingMenu') return node.props
  for (const child of [node.props?.children].flat()) {
    const found = menu(child)
    if (found) return found
  }
}

test('menu refresh bypasses lyric caches, preserves lyrics during loading and rejects late delivery after a switch', async () => {
  const app = await harness(streamed)
  app.pending[0](streamLyrics)
  await Promise.resolve()
  menu(app.render()).onRefresh()
  await new Promise(setImmediate)
  assert.deepEqual(app.requests, [
    { key: streamed.key, refresh: false },
    { key: streamed.key, refresh: true }
  ])
  assert.equal(menu(app.render()).source, '网易云音乐')
  assert.equal(menu(app.render()).loading, true)
  app.switchTo(local)
  app.pending[1]({ ...streamLyrics, source: 'qq' })
  await Promise.resolve()
  assert.deepEqual(app.parsed, [streamLyrics])
  assert.equal(menu(app.render()).source, undefined)
})

test('a stale menu refresh cannot start a request for the previous playback instance', async () => {
  const app = await harness(local)
  const oldMenu = menu(app.render())
  app.switchTo(streamed)
  oldMenu.onRefresh()
  assert.equal(app.requests.length, 1)
})

test('a stale menu cannot refresh the previous lyrics selection', async () => {
  const app = await harness(local)
  const oldMenu = menu(app.render())
  app.selectSource('online')
  oldMenu.onRefresh()
  assert.equal(app.requests.length, 1)
})

test('new refresh results win over an older request for the same song', async () => {
  const app = await harness(streamed)
  menu(app.render()).onRefresh()
  await new Promise(setImmediate)
  const refreshed = { ...streamLyrics, source: 'amll', format: 'ttml' }
  app.pending[1](refreshed)
  await Promise.resolve()
  app.pending[0](streamLyrics)
  await Promise.resolve()
  assert.deepEqual(app.parsed, [refreshed])
  assert.equal(menu(app.render()).source, 'AMLL DB')
})

test('opening a song reserves the same lyrics layout before and after loading starts', async () => {
  const app = await harness(streamed)
  const layout = (tree) => {
    if (!tree || typeof tree !== 'object') return
    if (tree.props?.className?.includes('now-playing-layout')) return tree.props.className
    for (const child of [tree.props?.children].flat()) {
      const found = layout(child)
      if (found) return found
    }
  }
  assert.equal(layout(app.initial), layout(app.render()))
})

test('lyrics and renderer load during entry and ready lyrics render before animation completes', async () => {
  const app = await harness(streamed, false)
  assert.equal(app.rendererLoads(), 1, 'renderer module starts loading while lyrics are pending')
  assert.equal(app.requests.length, 1)
  app.pending[0](streamLyrics)
  await Promise.resolve()
  const count = (tree) => {
    if (!tree || typeof tree !== 'object') return 0
    if (tree.type?.name === 'LyricRenderer') return 1
    return [tree.props?.children].flat().reduce((sum, child) => sum + count(child), 0)
  }
  assert.equal(count(app.render()), 1, 'ready lyrics render without waiting for entry animation')
  app.initial.props.onAnimationComplete()
  assert.equal(count(app.render()), 1)
  app.setPresent(false)
  assert.equal(count(app.render()), 0, 'AMLL stops as soon as exit starts')
})

test('a local lyrics response cannot replace stream lyrics after playback switches, before React effects clean up', async () => {
  const app = await harness(local)
  app.switchTo(streamed)
  app.pending[0](localLyrics)
  await Promise.resolve()
  assert.equal(app.parsed.length, 0, 'late local lyrics were applied while the stream was active')
})

test('local songs online background updates are rejected immediately when playback switches to streaming', async () => {
  const app = await harness(local, true, 'online')
  const lyrics = { ...localLyrics, source: 'qq' }
  app.pending[0](lyrics)
  await Promise.resolve()
  assert.equal(app.parsed.length, 1)
  app.switchTo(streamed)
  app.events[0]({ payload: { key: local.key, lyrics } })
  assert.equal(
    app.parsed.length,
    1,
    'old local listener replaced the stream before its cleanup ran'
  )
})

test('source selection changes reject online background updates before effect cleanup', async () => {
  const app = await harness(local, true, 'online')
  app.pending[0](streamLyrics)
  await Promise.resolve()
  app.selectSource('local')
  app.events[0]({ payload: { key: local.key, lyrics: streamLyrics } })
  assert.deepEqual(app.parsed, [streamLyrics])
})

test('selection changes reject pending lyrics before effect cleanup', async () => {
  const app = await harness(local)
  app.selectSource('online')
  app.pending[0](localLyrics)
  await Promise.resolve()
  assert.equal(app.parsed.length, 0)
})

test('local selection rejects online background updates even with the same song key', async () => {
  const app = await harness(local)
  app.pending[0](localLyrics)
  await Promise.resolve()
  app.events[0]({ payload: { key: local.key, lyrics: { ...streamLyrics, source: 'local' } } })
  app.events[0]({ payload: { key: local.key, lyrics: streamLyrics } })
  assert.deepEqual(app.parsed, [localLyrics])
})

test('stream lyrics responses are likewise rejected after switching to a local source', async () => {
  const app = await harness(streamed)
  app.switchTo(local)
  app.pending[0](streamLyrics)
  await Promise.resolve()
  assert.equal(app.parsed.length, 0)
})

test('returning to the same song rejects responses from its previous playback instance', async () => {
  const app = await harness(streamed)
  app.switchTo(local)
  app.switchTo(streamed)
  app.pending[0](streamLyrics)
  await Promise.resolve()
  assert.equal(app.parsed.length, 0)
})

test('the first stream render hides local lyrics even before the new loading effect runs', async () => {
  const app = await harness(local)
  app.pending[0](localLyrics)
  await Promise.resolve()
  const renderedLines = (tree) => {
    if (!tree || typeof tree !== 'object') return 0
    if (tree.type?.name === 'LyricRenderer') return tree.props.lines.length
    return [tree.props?.children].flat().reduce((count, child) => count + renderedLines(child), 0)
  }
  assert.equal(renderedLines(app.render()), 1)
  app.switchTo(streamed)
  assert.equal(renderedLines(app.render()), 0)
})

test('active stream lyrics and translations still apply normally', async () => {
  const app = await harness(streamed)
  app.pending[0](streamLyrics)
  await Promise.resolve()
  assert.deepEqual(app.parsed, [streamLyrics])
})

test('same-song metadata changes reject pending responses before effect cleanup', async () => {
  const app = await harness(streamed)
  app.updateMetadata({ ...streamed, title: 'Corrected title' })
  app.pending[0](streamLyrics)
  await Promise.resolve()
  assert.equal(app.parsed.length, 0)
})

test('background refresh metadata must match the current song even with the same key', async () => {
  const app = await harness(streamed)
  app.pending[0](streamLyrics)
  await Promise.resolve()
  app.events[0]({
    payload: {
      key: streamed.key,
      track: { ...streamed, title: 'Previous title' },
      lyrics: streamLyrics
    }
  })
  assert.deepEqual(app.parsed, [streamLyrics])
})
