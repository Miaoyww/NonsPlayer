import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function load(storage = {}) {
  const values = new Map(Object.entries(storage))
  const exports = {}
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
      require: (name) =>
        name === 'react'
          ? { useSyncExternalStore: (_subscribe, getSnapshot) => getSnapshot() }
          : undefined
    }
  )
  return { ...exports, values }
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
  assert.equal(initial.showDesktopLyricsDoubleLine, false)
  assert.equal(initial.showDesktopLyricsTranslation, true)
  assert.equal(initial.showDesktopLyricsPronunciation, true)
  assert.equal(initial.desktopLyricsSungColor, '#87afff')

  initial.setDesktopLyricsFontScale(1.25)
  initial.setDesktopLyricsFontWeight(700)
  initial.setShowDesktopLyricsDoubleLine(true)
  initial.setShowDesktopLyricsTranslation(false)
  initial.setShowDesktopLyricsPronunciation(false)
  initial.setDesktopLyricsSungColor('#AABBCC')

  const updated = settings.useLyricsSettings()
  assert.equal(settings.values.get('nons-desktop-lyrics-font-scale'), '1.25')
  assert.equal(settings.values.get('nons-desktop-lyrics-font-weight'), '700')
  assert.equal(settings.values.get('nons-desktop-lyrics-double-line'), 'true')
  assert.equal(settings.values.get('nons-desktop-lyrics-translation'), 'false')
  assert.equal(settings.values.get('nons-desktop-lyrics-pronunciation'), 'false')
  assert.equal(settings.values.get('nons-desktop-lyrics-sung-color'), '#aabbcc')
  assert.equal(updated.desktopLyricsFontScale, 1.25)
  assert.equal(updated.desktopLyricsFontWeight, 700)
  assert.equal(updated.showDesktopLyricsDoubleLine, true)
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
  assert.equal(invalid.showDesktopLyricsDoubleLine, false)
  assert.equal(invalid.desktopLyricsSungColor, '#87afff')
})
