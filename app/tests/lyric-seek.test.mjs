import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function lyricControl({ failure, desktop = true, android = false } = {}) {
  const calls = []
  const errors = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    '@/lib/platform': { isAndroid: () => android },
    '@/features/local/use-local-preferences': {
      useLocalPreferences: () => ({ options: { artistSeparators: [] } })
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    react: {
      useRef: (value) => ({ current: value }),
      useState: (initial) => [initial, () => {}],
      useEffect() {},
      useCallback: (fn) => fn,
      useMemo: (fn) => fn()
    },
    '@applemusic-like-lyrics/react': { LyricPlayer: 'lyric-player' },
    '@/features/settings/use-font-settings': {
      useFontSettings: () => ({ fonts: { app: '', lyrics: '' } })
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
    '@/features/playback/now-playing-controls': {},
    '@/features/playback/now-playing-menu': {}
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
  const click = (startTime) =>
    tree.props.onLyricLineClick({ line: { getLine: () => ({ startTime }) } })
  return { click, calls, errors }
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
