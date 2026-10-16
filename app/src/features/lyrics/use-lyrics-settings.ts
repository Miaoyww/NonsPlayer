import { reportCaughtError } from '@/lib/diagnostics'
import { useSyncExternalStore } from 'react'

const storageKey = 'nons-lyrics-background-speed'
export const defaultBackgroundSpeed = 0.5
const listeners = new Set<() => void>()
let backgroundSpeed = defaultBackgroundSpeed
try {
  const stored = localStorage.getItem(storageKey)
  const value = stored === null ? NaN : Number(stored)
  if (Number.isFinite(value) && value >= 0 && value <= 4) backgroundSpeed = value
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

export function useLyricsSettings() {
  const speed = useSyncExternalStore(subscribe, () => backgroundSpeed)
  return { backgroundSpeed: speed, setBackgroundSpeed }
}
