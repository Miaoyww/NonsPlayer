import { reportCaughtError } from '@/lib/diagnostics'
import { useSyncExternalStore } from 'react'

const storageKey = 'nons-lyrics-background-speed'
const blurStorageKey = 'nons-lyrics-auto-blur'
const weightStorageKey = 'nons-lyrics-font-weight'
export const defaultBackgroundSpeed = 0.5
const listeners = new Set<() => void>()
let backgroundSpeed = defaultBackgroundSpeed
let autoBlur = true
let fontWeight = 400
try {
  const stored = localStorage.getItem(storageKey)
  const value = stored === null ? NaN : Number(stored)
  if (Number.isFinite(value) && value >= 0 && value <= 4) backgroundSpeed = value
} catch (cause) {
  reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

  /* Keep the default when storage is unavailable. */
}

try {
  const stored = localStorage.getItem(blurStorageKey)
  if (stored === 'true' || stored === 'false') autoBlur = stored === 'true'
} catch (cause) {
  reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

  /* Keep the default when storage is unavailable. */
}

try {
  const value = Number(localStorage.getItem(weightStorageKey))
  if (Number.isInteger(value) && value >= 100 && value <= 900) fontWeight = value
} catch (cause) {
  reportCaughtError('app/src/features/lyrics/use-lyrics-settings.ts', cause)

  /* Keep the default when storage is unavailable. */
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

export function useLyricsSettings() {
  const speed = useSyncExternalStore(subscribe, () => backgroundSpeed)
  const blur = useSyncExternalStore(subscribe, () => autoBlur)
  const weight = useSyncExternalStore(subscribe, () => fontWeight)
  return {
    backgroundSpeed: speed,
    setBackgroundSpeed,
    autoBlur: blur,
    setAutoBlur,
    fontWeight: weight,
    setFontWeight
  }
}
