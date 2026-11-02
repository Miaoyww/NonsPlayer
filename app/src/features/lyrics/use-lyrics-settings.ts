import { reportCaughtError } from '@/lib/diagnostics'
import { useSyncExternalStore } from 'react'

const storageKey = 'nons-lyrics-background-speed'
const offsetStorageKey = 'nons-lyrics-offset-ms'
const blurStorageKey = 'nons-lyrics-auto-blur'
const weightStorageKey = 'nons-lyrics-font-weight'
const scaleStorageKey = 'nons-lyrics-font-scale'
const desktopLyricsFontScaleStorageKey = 'nons-desktop-lyrics-font-scale'
const desktopLyricsFontWeightStorageKey = 'nons-desktop-lyrics-font-weight'
const desktopLyricsDoubleLineStorageKey = 'nons-desktop-lyrics-double-line'
const desktopLyricsTranslationStorageKey = 'nons-desktop-lyrics-translation'
const desktopLyricsPronunciationStorageKey = 'nons-desktop-lyrics-pronunciation'
const desktopLyricsSungColorStorageKey = 'nons-desktop-lyrics-sung-color'
export const defaultBackgroundSpeed = 0.5
export const defaultLyricsOffsetMs = -200
export const minLyricsFontScale = 0.7
export const maxLyricsFontScale = 1.6
export const defaultLyricsFontScale = 1
export const minLyricsFontWeight = 100
export const maxLyricsFontWeight = 900
export const defaultLyricsFontWeight = 400
export const defaultDesktopLyricsSungColor = '#87afff'
const colorPattern = /^#[0-9a-f]{6}$/i
const listeners = new Set<() => void>()

function readNumber(key: string, fallback: number, valid: (value: number) => boolean) {
  try {
    const stored = localStorage.getItem(key)
    const value = stored === null ? NaN : Number(stored)
    return valid(value) ? value : fallback
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    return fallback
  }
}

function readBoolean(key: string, fallback: boolean) {
  try {
    const stored = localStorage.getItem(key)
    return stored === 'true' || stored === 'false' ? stored === 'true' : fallback
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    return fallback
  }
}

function readColor(key: string, fallback: string) {
  try {
    const stored = localStorage.getItem(key)
    return stored && colorPattern.test(stored) ? stored.toLowerCase() : fallback
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    return fallback
  }
}

let backgroundSpeed = readNumber(
  storageKey,
  defaultBackgroundSpeed,
  (value) => Number.isFinite(value) && value >= 0 && value <= 4
)
let lyricsOffsetMs = readNumber(
  offsetStorageKey,
  defaultLyricsOffsetMs,
  (value) => Number.isInteger(value) && value >= -60000 && value <= 60000
)
let autoBlur = readBoolean(blurStorageKey, true)
let fontWeight = readNumber(
  weightStorageKey,
  defaultLyricsFontWeight,
  (value) => Number.isInteger(value) && value >= minLyricsFontWeight && value <= maxLyricsFontWeight
)
let fontScale = readNumber(
  scaleStorageKey,
  defaultLyricsFontScale,
  (value) => Number.isFinite(value) && value >= minLyricsFontScale && value <= maxLyricsFontScale
)
let desktopLyricsFontScale = readNumber(
  desktopLyricsFontScaleStorageKey,
  defaultLyricsFontScale,
  (value) => Number.isFinite(value) && value >= minLyricsFontScale && value <= maxLyricsFontScale
)
let desktopLyricsFontWeight = readNumber(
  desktopLyricsFontWeightStorageKey,
  defaultLyricsFontWeight,
  (value) => Number.isInteger(value) && value >= minLyricsFontWeight && value <= maxLyricsFontWeight
)
let showDesktopLyricsDoubleLine = readBoolean(desktopLyricsDoubleLineStorageKey, false)
let showDesktopLyricsTranslation = readBoolean(desktopLyricsTranslationStorageKey, true)
let showDesktopLyricsPronunciation = readBoolean(desktopLyricsPronunciationStorageKey, true)
let desktopLyricsSungColor = readColor(
  desktopLyricsSungColorStorageKey,
  defaultDesktopLyricsSungColor
)

// Settings are stored per origin, so another window's change arrives as a storage event.
function reloadFromStorage() {
  const nextBackgroundSpeed = readNumber(
    storageKey,
    defaultBackgroundSpeed,
    (value) => Number.isFinite(value) && value >= 0 && value <= 4
  )
  const nextLyricsOffsetMs = readNumber(
    offsetStorageKey,
    defaultLyricsOffsetMs,
    (value) => Number.isInteger(value) && value >= -60000 && value <= 60000
  )
  const nextAutoBlur = readBoolean(blurStorageKey, true)
  const nextFontWeight = readNumber(
    weightStorageKey,
    defaultLyricsFontWeight,
    (value) =>
      Number.isInteger(value) && value >= minLyricsFontWeight && value <= maxLyricsFontWeight
  )
  const nextFontScale = readNumber(
    scaleStorageKey,
    defaultLyricsFontScale,
    (value) => Number.isFinite(value) && value >= minLyricsFontScale && value <= maxLyricsFontScale
  )
  const nextDesktopLyricsFontScale = readNumber(
    desktopLyricsFontScaleStorageKey,
    defaultLyricsFontScale,
    (value) => Number.isFinite(value) && value >= minLyricsFontScale && value <= maxLyricsFontScale
  )
  const nextDesktopLyricsFontWeight = readNumber(
    desktopLyricsFontWeightStorageKey,
    defaultLyricsFontWeight,
    (value) =>
      Number.isInteger(value) && value >= minLyricsFontWeight && value <= maxLyricsFontWeight
  )
  const nextShowDesktopLyricsDoubleLine = readBoolean(desktopLyricsDoubleLineStorageKey, false)
  const nextShowDesktopLyricsTranslation = readBoolean(desktopLyricsTranslationStorageKey, true)
  const nextShowDesktopLyricsPronunciation = readBoolean(desktopLyricsPronunciationStorageKey, true)
  const nextDesktopLyricsSungColor = readColor(
    desktopLyricsSungColorStorageKey,
    defaultDesktopLyricsSungColor
  )
  if (
    nextBackgroundSpeed === backgroundSpeed &&
    nextLyricsOffsetMs === lyricsOffsetMs &&
    nextAutoBlur === autoBlur &&
    nextFontWeight === fontWeight &&
    nextFontScale === fontScale &&
    nextDesktopLyricsFontScale === desktopLyricsFontScale &&
    nextDesktopLyricsFontWeight === desktopLyricsFontWeight &&
    nextShowDesktopLyricsDoubleLine === showDesktopLyricsDoubleLine &&
    nextShowDesktopLyricsTranslation === showDesktopLyricsTranslation &&
    nextShowDesktopLyricsPronunciation === showDesktopLyricsPronunciation &&
    nextDesktopLyricsSungColor === desktopLyricsSungColor
  )
    return
  backgroundSpeed = nextBackgroundSpeed
  lyricsOffsetMs = nextLyricsOffsetMs
  autoBlur = nextAutoBlur
  fontWeight = nextFontWeight
  fontScale = nextFontScale
  desktopLyricsFontScale = nextDesktopLyricsFontScale
  desktopLyricsFontWeight = nextDesktopLyricsFontWeight
  showDesktopLyricsDoubleLine = nextShowDesktopLyricsDoubleLine
  showDesktopLyricsTranslation = nextShowDesktopLyricsTranslation
  showDesktopLyricsPronunciation = nextShowDesktopLyricsPronunciation
  desktopLyricsSungColor = nextDesktopLyricsSungColor
  listeners.forEach((notify) => notify())
}

if (typeof window !== 'undefined') {
  const keys = new Set([
    storageKey,
    offsetStorageKey,
    blurStorageKey,
    weightStorageKey,
    scaleStorageKey,
    desktopLyricsFontScaleStorageKey,
    desktopLyricsFontWeightStorageKey,
    desktopLyricsDoubleLineStorageKey,
    desktopLyricsTranslationStorageKey,
    desktopLyricsPronunciationStorageKey,
    desktopLyricsSungColorStorageKey
  ])
  window.addEventListener('storage', (event) => {
    if (event.key === null || keys.has(event.key)) reloadFromStorage()
  })
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function setBackgroundSpeed(value: number) {
  if (!Number.isFinite(value)) return
  backgroundSpeed = Math.max(0, Math.min(4, value))
  try {
    localStorage.setItem(storageKey, String(backgroundSpeed))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setLyricsOffsetMs(value: number) {
  if (!Number.isInteger(value) || value < -60000 || value > 60000 || value === lyricsOffsetMs)
    return
  lyricsOffsetMs = value
  try {
    localStorage.setItem(offsetStorageKey, String(value))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setAutoBlur(value: boolean) {
  if (value === autoBlur) return
  autoBlur = value
  try {
    localStorage.setItem(blurStorageKey, String(value))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setFontWeight(value: number) {
  if (
    !Number.isInteger(value) ||
    value < minLyricsFontWeight ||
    value > maxLyricsFontWeight ||
    value === fontWeight
  )
    return
  fontWeight = value
  try {
    localStorage.setItem(weightStorageKey, String(value))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setFontScale(value: number) {
  if (!Number.isFinite(value)) return
  const clamped =
    Math.round(Math.max(minLyricsFontScale, Math.min(maxLyricsFontScale, value)) * 100) / 100
  if (clamped === fontScale) return
  fontScale = clamped
  try {
    localStorage.setItem(scaleStorageKey, String(clamped))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setDesktopLyricsFontScale(value: number) {
  if (!Number.isFinite(value)) return
  const clamped =
    Math.round(Math.max(minLyricsFontScale, Math.min(maxLyricsFontScale, value)) * 100) / 100
  if (clamped === desktopLyricsFontScale) return
  desktopLyricsFontScale = clamped
  try {
    localStorage.setItem(desktopLyricsFontScaleStorageKey, String(clamped))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setDesktopLyricsFontWeight(value: number) {
  if (
    !Number.isInteger(value) ||
    value < minLyricsFontWeight ||
    value > maxLyricsFontWeight ||
    value === desktopLyricsFontWeight
  )
    return
  desktopLyricsFontWeight = value
  try {
    localStorage.setItem(desktopLyricsFontWeightStorageKey, String(value))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setShowDesktopLyricsDoubleLine(value: boolean) {
  if (value === showDesktopLyricsDoubleLine) return
  showDesktopLyricsDoubleLine = value
  try {
    localStorage.setItem(desktopLyricsDoubleLineStorageKey, String(value))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setShowDesktopLyricsTranslation(value: boolean) {
  if (value === showDesktopLyricsTranslation) return
  showDesktopLyricsTranslation = value
  try {
    localStorage.setItem(desktopLyricsTranslationStorageKey, String(value))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setShowDesktopLyricsPronunciation(value: boolean) {
  if (value === showDesktopLyricsPronunciation) return
  showDesktopLyricsPronunciation = value
  try {
    localStorage.setItem(desktopLyricsPronunciationStorageKey, String(value))
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

function setDesktopLyricsSungColor(value: string) {
  const normalized = value.trim().toLowerCase()
  if (!colorPattern.test(normalized) || normalized === desktopLyricsSungColor) return
  desktopLyricsSungColor = normalized
  try {
    localStorage.setItem(desktopLyricsSungColorStorageKey, normalized)
  } catch (cause) {
    reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

    /* Changes still work for this session. */
  }
  listeners.forEach((notify) => notify())
}

export function useLyricsSettings() {
  const speed = useSyncExternalStore(subscribe, () => backgroundSpeed)
  const offset = useSyncExternalStore(subscribe, () => lyricsOffsetMs)
  const blur = useSyncExternalStore(subscribe, () => autoBlur)
  const weight = useSyncExternalStore(subscribe, () => fontWeight)
  const scale = useSyncExternalStore(subscribe, () => fontScale)
  const desktopScale = useSyncExternalStore(subscribe, () => desktopLyricsFontScale)
  const desktopWeight = useSyncExternalStore(subscribe, () => desktopLyricsFontWeight)
  const desktopDoubleLine = useSyncExternalStore(subscribe, () => showDesktopLyricsDoubleLine)
  const desktopTranslation = useSyncExternalStore(subscribe, () => showDesktopLyricsTranslation)
  const desktopPronunciation = useSyncExternalStore(subscribe, () => showDesktopLyricsPronunciation)
  const desktopSungColor = useSyncExternalStore(subscribe, () => desktopLyricsSungColor)
  return {
    backgroundSpeed: speed,
    setBackgroundSpeed,
    lyricsOffsetMs: offset,
    setLyricsOffsetMs,
    autoBlur: blur,
    setAutoBlur,
    fontWeight: weight,
    setFontWeight,
    fontScale: scale,
    setFontScale,
    desktopLyricsFontScale: desktopScale,
    setDesktopLyricsFontScale,
    desktopLyricsFontWeight: desktopWeight,
    setDesktopLyricsFontWeight,
    showDesktopLyricsDoubleLine: desktopDoubleLine,
    setShowDesktopLyricsDoubleLine,
    showDesktopLyricsTranslation: desktopTranslation,
    setShowDesktopLyricsTranslation,
    showDesktopLyricsPronunciation: desktopPronunciation,
    setShowDesktopLyricsPronunciation,
    desktopLyricsSungColor: desktopSungColor,
    setDesktopLyricsSungColor
  }
}
