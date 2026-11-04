import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function lyricControl({ failure, desktop = true, android = false } = {}) {
  const calls = []
  const errors = []
  const stateSets = []
  const jsx = (type, props) => ({ type, props })
  const lyricCopy = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/lyrics/lyric-copy.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    { exports: lyricCopy }
  )
  const modules = {
    './lyric-copy': lyricCopy,
    '@/components/music/song-actions': {
      useSongActions: () => ({ copy: (text) => calls.push({ command: 'copy', text }) })
    },
    '@base-ui/react/context-menu': {
      ContextMenu: Object.fromEntries(
        ['Root', 'Portal', 'Positioner', 'Popup', 'Item'].map((key) => [key, key])
      )
    },
    '@/lib/platform': { isAndroid: () => android },
    '@/features/local/use-local-preferences': {
      useLocalPreferences: () => ({ options: { artistSeparators: [] } })
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    react: {
      useRef: (value) => ({ current: value }),
      useState: (initial) => [
        initial,
        (next) => stateSets.push(typeof next === 'function' ? next(initial) : next)
      ],
      useEffect() {},
      useCallback: (fn) => fn,
      useMemo: (fn) => fn()
    },
    '@applemusic-like-lyrics/react': { LyricPlayer: 'lyric-player' },
    './manual-lyric-player': { ManualLyricPlayer: 'manual-lyric-player' },
    '@/features/settings/appearance/use-font-settings': {
      useFontSettings: () => ({ fonts: { app: '', lyrics: '' } })
    },
    './use-lyrics-settings': {
      useLyricsSettings: () => ({ autoBlur: true, fontWeight: 400 })
    },
    '@applemusic-like-lyrics/lyric': {},
    '@applemusic-like-lyrics/core/style.css': {},
    '@tauri-apps/api/event': {},
    '@tauri-apps/api/core': { isTauri: () => desktop },
    '@/features/lyrics/use-lyric-sources': {},
    '@/features/lyrics/load-lyrics': {},
    '@/features/lyrics/parse-lyrics': {},
    'motion/react': { useReducedMotion: () => false },
    'lucide-react': {},
    '@/lib/player': {
      useProgress: () => ({ status: 'paused', positionMs: 0 }),
      nativeCall: (command, args) => {
        calls.push({ command, ...args })
        return failure ? Promise.reject(failure) : Promise.resolve()
      }
    },
    '@/components/music/music-links': {},
    '@/components/music/action-button': {},
    '@/components/music/cover': {},
    '@/features/lyrics/album-background': {},
    '@/features/playback/controls/now-playing-controls': {},
    '@/features/playback/now-playing/now-playing-menu': {}
  }
  const source = readFileSync(
    new URL('../src/features/lyrics/lyric-renderer.tsx', import.meta.url),
    'utf8'
  )
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  const exports = {}
  runInNewContext(`${outputText}\nexports.testRenderer = exports.default;`, {
    exports,
    document: { visibilityState: 'visible' },
    require: (name) => {
      assert.ok(Object.hasOwn(modules, name), `unexpected dependency: ${name}`)
      return modules[name]
    }
  })
  const tree = exports.testRenderer({ lines: [], onError: (error) => errors.push(error) })
  const player = tree.props.children.find((node) => node?.type === 'lyric-player')
  const click = (startTime) =>
    player.props.onLyricLineClick({ line: { getLine: () => ({ startTime }) } })
  const contextMenu = (line, event = {}) => {
    let prevented = false
    player.props.onLyricLineContextMenu({
      clientX: 12,
      clientY: 34,
      preventDefault: () => {
        prevented = true
      },
      line: { getLine: () => line },
      ...event
    })
    return { prevented, states: stateSets }
  }
  return { click, contextMenu, calls, errors }
}

test('clicking a lyric line seeks native playback to its start time in milliseconds', () => {
  const control = lyricControl()
  control.click(75250)
  assert.equal(control.calls[0].command, 'player_seek')
  assert.equal(control.calls[0].positionMs, 75250)
})

test('lyric seek errors reach the visible playback error handler', async () => {
  const failure = new Error('seek unavailable')
  const control = lyricControl({ failure })
  control.click(25000)
  await Promise.resolve()
  assert.deepEqual(control.errors, [failure])
})

test('browser preview does not invoke native seek', () => {
  const control = lyricControl({ desktop: false })
  control.click(25000)
  assert.equal(control.calls.length, 0)
})

test('Android lyric taps never seek native playback', () => {
  const control = lyricControl({ android: true })
  control.click(75250)
  assert.equal(control.calls.length, 0)
})

test('right-clicking a lyric line opens a copy menu with its visible text', () => {
  const control = lyricControl()
  const { prevented, states } = control.contextMenu({
    words: [{ word: 'Hello' }, { word: ' world' }],
    translatedLyric: '你好，世界',
    romanLyric: 'Hello world'
  })
  assert.equal(prevented, true)
  assert.equal(states.at(-1).x, 12)
  assert.equal(states.at(-1).y, 34)
  assert.equal(states.at(-1).text, 'Hello world\n你好，世界\nHello world')
})

test('right-clicking a line without text shows no menu', () => {
  const control = lyricControl()
  const { prevented, states } = control.contextMenu({
    words: [],
    translatedLyric: '',
    romanLyric: ''
  })
  assert.equal(prevented, true)
  assert.equal(states.length, 0)
})
