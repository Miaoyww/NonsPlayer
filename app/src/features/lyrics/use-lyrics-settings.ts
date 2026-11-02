import { reportCaughtError } from '@/lib/diagnostics'
import { useSyncExternalStore } from 'react'

const storageKey = 'nons-lyrics-background-speed'
const offsetStorageKey = 'nons-lyrics-offset-ms'
const blurStorageKey = 'nons-lyrics-auto-blur'
const weightStorageKey = 'nons-lyrics-font-weight'
const scaleStorageKey = 'nons-lyrics-font-scale'
export const defaultBackgroundSpeed = 0.5
export const defaultLyricsOffsetMs = -200
export const minLyricsFontScale = 0.7
export const maxLyricsFontScale = 1.6
export const defaultLyricsFontScale = 1
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
  400,
  (value) => Number.isInteger(value) && value >= 100 && value <= 900
)
let fontScale = readNumber(
  scaleStorageKey,
  defaultLyricsFontScale,
  (value) => Number.isFinite(value) && value >= minLyricsFontScale && value <= maxLyricsFontScale
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
    400,
    (value) => Number.isInteger(value) && value >= 100 && value <= 900
  )
  const nextFontScale = readNumber(
    scaleStorageKey,
    defaultLyricsFontScale,
    (value) => Number.isFinite(value) && value >= minLyricsFontScale && value <= maxLyricsFontScale
  )
  if (
    nextBackgroundSpeed === backgroundSpeed &&
    nextLyricsOffsetMs === lyricsOffsetMs &&
    nextAutoBlur === autoBlur &&
    nextFontWeight === fontWeight &&
    nextFontScale === fontScale
  )
    return
  backgroundSpeed = nextBackgroundSpeed
  lyricsOffsetMs = nextLyricsOffsetMs
  autoBlur = nextAutoBlur
  fontWeight = nextFontWeight
  fontScale = nextFontScale
  listeners.forEach((notify) => notify())
}

if (typeof window !== 'undefined') {
  const keys = new Set([
    storageKey,
    offsetStorageKey,
    blurStorageKey,
    weightStorageKey,
    scaleStorageKey
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
  if (!Number.isInteger(value) || value < 100 || value > 900 || value === fontWeight) return
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

export function useLyricsSettings() {
  const speed = useSyncExternalStore(subscribe, () => backgroundSpeed)
  const offset = useSyncExternalStore(subscribe, () => lyricsOffsetMs)
  const blur = useSyncExternalStore(subscribe, () => autoBlur)
  const weight = useSyncExternalStore(subscribe, () => fontWeight)
  const scale = useSyncExternalStore(subscribe, () => fontScale)
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
    setFontScale
  }
}
