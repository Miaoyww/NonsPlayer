import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function deferred() {
  let resolve
  let reject
  const promise = new Promise((done, fail) => {
    resolve = done
    reject = fail
  })
  return { promise, resolve, reject }
}

function load(storage = {}, { tauri = false, android = false, readOffset, writeOffset } = {}) {
  const values = new Map(Object.entries(storage))
  const exports = {}
  const calls = []
  const nativeListeners = new Map()
  const windowListeners = new Map()
  const source = readFileSync(
    new URL('../src/features/lyrics/use-lyrics-settings.ts', import.meta.url),
    'utf8'
  )
  runInNewContext(
    ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    {
      exports,
      localStorage: {
        getItem: (key) => values.get(key) ?? null,
        setItem: (key, value) => values.set(key, value)
      },
      window: {
        addEventListener: (name, listener) => {
          const listeners = windowListeners.get(name) ?? new Set()
          listeners.add(listener)
          windowListeners.set(name, listeners)
        }
      },
      require: (name) => {
        if (name === 'react')
          return { useSyncExternalStore: (_subscribe, getSnapshot) => getSnapshot() }
        if (name === '@tauri-apps/api/core') return { isTauri: () => tauri }
        if (name === '@tauri-apps/api/event')
          return {
            listen: async (event, listener) => {
              calls.push(['listen', event])
              nativeListeners.set(event, listener)
              return () => nativeListeners.delete(event)
            }
          }
        if (name === '@/lib/platform') return { isAndroid: () => android }
        if (name === '@/lib/player')
          return {
            nativeCall: (command, args) => {
              calls.push([command, args])
              if (command === 'lyric_global_offset')
                return readOffset ? readOffset() : Promise.resolve(-200)
              if (command === 'set_lyric_global_offset')
                return writeOffset ? writeOffset(args.offsetMs) : Promise.resolve(args.offsetMs)
              throw new Error(`Unexpected command: ${command}`)
            }
          }
      }
    }
  )
  return {
    ...exports,
    values,
    calls,
    nativeListeners,
    dispatchWindow: (name, event) => {
      for (const listener of windowListeners.get(name) ?? []) listener(event)
    }
  }
}

test('global lyric offset defaults to -200ms, persists valid values and ignores invalid ones', () => {
  const settings = load()
  assert.equal(settings.defaultLyricsOffsetMs, -200)
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, -200)
  settings.useLyricsSettings().setLyricsOffsetMs(350)
  assert.equal(settings.values.get('nons-lyrics-offset-ms'), '350')
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 350)
  settings.useLyricsSettings().setLyricsOffsetMs(60001)
  settings.useLyricsSettings().setLyricsOffsetMs(100.5)
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 350)
})

test('global lyric offset restores a valid saved value and falls back for invalid storage', () => {
  assert.equal(load({ 'nons-lyrics-offset-ms': '-450' }).useLyricsSettings().lyricsOffsetMs, -450)
  assert.equal(load({ 'nons-lyrics-offset-ms': '60001' }).useLyricsSettings().lyricsOffsetMs, -200)
})

test('desktop lyric offset registers its listener before reading the native value', async () => {
  const settings = load({}, { tauri: true })
  await settings.nativeLyricsOffsetReady
  assert.deepEqual(
    settings.calls.map(([command]) => command),
    ['listen', 'lyric_global_offset']
  )
})

test('desktop lyric offset ignores an initial read superseded by a native event', async () => {
  const read = deferred()
  const settings = load({}, { tauri: true, readOffset: () => read.promise })
  while (!settings.calls.some(([command]) => command === 'lyric_global_offset'))
    await new Promise((resolve) => setImmediate(resolve))
  settings.nativeListeners.get('lyric-global-offset-changed')({ payload: -300 })
  read.resolve(-100)
  await settings.nativeLyricsOffsetReady
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, -300)
})

test('desktop lyric offset ignores an initial read superseded by a successful save', async () => {
  const read = deferred()
  const settings = load({}, { tauri: true, readOffset: () => read.promise })
  while (!settings.calls.some(([command]) => command === 'lyric_global_offset'))
    await new Promise((resolve) => setImmediate(resolve))
  settings.useLyricsSettings().setLyricsOffsetMs(250)
  while (settings.useLyricsSettings().lyricsOffsetMs !== 250)
    await new Promise((resolve) => setImmediate(resolve))
  read.resolve(-100)
  await settings.nativeLyricsOffsetReady
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 250)
})

test('desktop lyric offset starts from the default instead of stale local storage', async () => {
  const read = deferred()
  const settings = load(
    { 'nons-lyrics-offset-ms': '900' },
    { tauri: true, readOffset: () => read.promise }
  )
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, -200)
  read.resolve(50)
  await settings.nativeLyricsOffsetReady
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 50)
})

test('desktop lyric offset sends the legacy local value only on the first native read', async () => {
  const read = deferred()
  const settings = load(
    { 'nons-lyrics-offset-ms': '450' },
    { tauri: true, readOffset: () => read.promise }
  )
  while (!settings.calls.some(([command]) => command === 'lyric_global_offset'))
    await new Promise((resolve) => setImmediate(resolve))
  const initialRead = settings.calls.find(([command]) => command === 'lyric_global_offset')
  assert.equal(JSON.stringify(initialRead[1]), JSON.stringify({ legacyOffsetMs: 450 }))
  read.resolve(450)
  await settings.nativeLyricsOffsetReady
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 450)
})

test('desktop lyric offset serializes writes and keeps the newest target', async () => {
  const first = deferred()
  const second = deferred()
  const writes = [first, second]
  const settings = load({}, { tauri: true, writeOffset: () => writes.shift().promise })
  await settings.nativeLyricsOffsetReady
  settings.useLyricsSettings().setLyricsOffsetMs(100)
  settings.useLyricsSettings().setLyricsOffsetMs(300)
  while (!settings.calls.some(([, args]) => args?.offsetMs === 100))
    await new Promise((resolve) => setImmediate(resolve))
  first.resolve(100)
  while (!settings.calls.some(([, args]) => args?.offsetMs === 300))
    await new Promise((resolve) => setImmediate(resolve))
  settings.nativeListeners.get('lyric-global-offset-changed')({ payload: 100 })
  settings.useLyricsSettings().setLyricsOffsetMs(300)
  second.resolve(300)
  while (settings.useLyricsSettings().lyricsOffsetMs !== 300)
    await new Promise((resolve) => setImmediate(resolve))
  assert.equal(
    settings.calls.filter(([command]) => command === 'set_lyric_global_offset').length,
    2
  )
  assert.equal(settings.values.get('nons-lyrics-offset-ms'), '300')
})

test('desktop lyric offset restores native authority after a failed write', async () => {
  const write = deferred()
  let reads = 0
  const settings = load(
    {},
    {
      tauri: true,
      readOffset: () => Promise.resolve(++reads === 1 ? -200 : 50),
      writeOffset: () => write.promise
    }
  )
  await settings.nativeLyricsOffsetReady
  settings.useLyricsSettings().setLyricsOffsetMs(100)
  while (!settings.calls.some(([command]) => command === 'set_lyric_global_offset'))
    await new Promise((resolve) => setImmediate(resolve))
  write.reject(new Error('save failed'))
  while (settings.useLyricsSettings().lyricsOffsetMs !== 50)
    await new Promise((resolve) => setImmediate(resolve))
  settings.useLyricsSettings().setLyricsOffsetMs(50)
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(
    settings.calls.filter(([command]) => command === 'set_lyric_global_offset').length,
    1
  )
  const recoveryRead = settings.calls.filter(([command]) => command === 'lyric_global_offset')[1]
  assert.equal(recoveryRead[1], undefined)
})

test('desktop lyric offset follows native events and ignores offset storage events', async () => {
  const settings = load({}, { tauri: true })
  await settings.nativeLyricsOffsetReady
  settings.nativeListeners.get('lyric-global-offset-changed')({ payload: 250 })
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 250)
  assert.equal(settings.values.get('nons-lyrics-offset-ms'), '250')
  settings.values.set('nons-lyrics-offset-ms', '900')
  settings.dispatchWindow('storage', { key: 'nons-lyrics-offset-ms' })
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 250)
})

test('Android uses the local preview offset without native IPC', async () => {
  const settings = load({}, { tauri: true, android: true })
  await settings.nativeLyricsOffsetReady
  assert.equal(settings.calls.length, 0)
  settings.useLyricsSettings().setLyricsOffsetMs(125)
  assert.equal(settings.useLyricsSettings().lyricsOffsetMs, 125)
  assert.equal(settings.values.get('nons-lyrics-offset-ms'), '125')
})

test('lyric font scale defaults to 100%, persists and clamps adjustments', () => {
  const settings = load()
  assert.equal(settings.defaultLyricsFontScale, 1)
  assert.equal(settings.useLyricsSettings().fontScale, 1)
  settings.useLyricsSettings().setFontScale(1.25)
  assert.equal(settings.values.get('nons-lyrics-font-scale'), '1.25')
  assert.equal(settings.useLyricsSettings().fontScale, 1.25)
  settings.useLyricsSettings().setFontScale(2)
  assert.equal(settings.useLyricsSettings().fontScale, 1.6)
  settings.useLyricsSettings().setFontScale(Number.NaN)
  assert.equal(settings.useLyricsSettings().fontScale, 1.6)
})

test('lyric font scale restores a valid saved value and falls back for invalid storage', () => {
  assert.equal(load({ 'nons-lyrics-font-scale': '1.35' }).useLyricsSettings().fontScale, 1.35)
  assert.equal(load({ 'nons-lyrics-font-scale': '2' }).useLyricsSettings().fontScale, 1)
  assert.equal(load({ 'nons-lyrics-font-scale': 'abc' }).useLyricsSettings().fontScale, 1)
})

test('desktop lyric settings default, persist and clamp their values', () => {
  const settings = load()
  const initial = settings.useLyricsSettings()
  assert.equal(initial.desktopLyricsFontScale, 1)
  assert.equal(initial.desktopLyricsFontWeight, 400)
  assert.equal(initial.showDesktopLyricsDoubleLine, true)
  assert.equal(initial.showDesktopLyricsTranslation, true)
  assert.equal(initial.showDesktopLyricsPronunciation, true)
  assert.equal(initial.desktopLyricsSungColor, '#87afff')

  initial.setDesktopLyricsFontScale(1.25)
  initial.setDesktopLyricsFontWeight(700)
  initial.setShowDesktopLyricsDoubleLine(false)
  initial.setShowDesktopLyricsTranslation(false)
  initial.setShowDesktopLyricsPronunciation(false)
  initial.setDesktopLyricsSungColor('#AABBCC')

  const updated = settings.useLyricsSettings()
  assert.equal(settings.values.get('nons-desktop-lyrics-font-scale'), '1.25')
  assert.equal(settings.values.get('nons-desktop-lyrics-font-weight'), '700')
  assert.equal(settings.values.get('nons-desktop-lyrics-double-line'), 'false')
  assert.equal(settings.values.get('nons-desktop-lyrics-translation'), 'false')
  assert.equal(settings.values.get('nons-desktop-lyrics-pronunciation'), 'false')
  assert.equal(settings.values.get('nons-desktop-lyrics-sung-color'), '#aabbcc')
  assert.equal(updated.desktopLyricsFontScale, 1.25)
  assert.equal(updated.desktopLyricsFontWeight, 700)
  assert.equal(updated.showDesktopLyricsDoubleLine, false)
  assert.equal(updated.showDesktopLyricsTranslation, false)
  assert.equal(updated.showDesktopLyricsPronunciation, false)
  assert.equal(updated.desktopLyricsSungColor, '#aabbcc')

  updated.setDesktopLyricsFontScale(2)
  updated.setDesktopLyricsFontWeight(1000)
  updated.setDesktopLyricsSungColor('red')
  const clamped = settings.useLyricsSettings()
  assert.equal(clamped.desktopLyricsFontScale, 1.6)
  assert.equal(clamped.desktopLyricsFontWeight, 700)
  assert.equal(clamped.desktopLyricsSungColor, '#aabbcc')
})

test('desktop lyric settings restore valid storage and fall back for invalid values', () => {
  const settings = load({
    'nons-desktop-lyrics-font-scale': '1.35',
    'nons-desktop-lyrics-font-weight': '500',
    'nons-desktop-lyrics-double-line': 'true',
    'nons-desktop-lyrics-sung-color': '#123ABC'
  }).useLyricsSettings()
  assert.equal(settings.desktopLyricsFontScale, 1.35)
  assert.equal(settings.desktopLyricsFontWeight, 500)
  assert.equal(settings.showDesktopLyricsDoubleLine, true)
  assert.equal(settings.desktopLyricsSungColor, '#123abc')

  const invalid = load({
    'nons-desktop-lyrics-font-scale': '2',
    'nons-desktop-lyrics-font-weight': '950',
    'nons-desktop-lyrics-double-line': 'yes',
    'nons-desktop-lyrics-sung-color': 'purple'
  }).useLyricsSettings()
  assert.equal(invalid.desktopLyricsFontScale, 1)
  assert.equal(invalid.desktopLyricsFontWeight, 400)
  assert.equal(invalid.showDesktopLyricsDoubleLine, true)
  assert.equal(invalid.desktopLyricsSungColor, '#87afff')
})
