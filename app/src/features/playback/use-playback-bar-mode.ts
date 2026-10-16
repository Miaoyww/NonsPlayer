import { reportCaughtError } from '@/lib/diagnostics'
import { useSyncExternalStore } from 'react'
import { isAndroid } from '@/lib/platform'

export type PlaybackBarMode = 'collapsible' | 'persistent' | 'off'
const storageKey = 'nons-playback-bar-mode'
const listeners = new Set<() => void>()
let mode: PlaybackBarMode = 'collapsible'
function isMode(value: unknown): value is PlaybackBarMode {
  return value === 'collapsible' || value === 'persistent' || value === 'off'
}
try {
  const saved = localStorage.getItem(storageKey)
  if (isMode(saved)) mode = saved
} catch (cause) {
  reportCaughtError('app/src/features/playback/use-playback-bar-mode.ts', cause)

  /* Storage is optional. */
}
const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
function setMode(value: PlaybackBarMode) {
  if (isAndroid()) return
  if (!isMode(value) || value === mode) return
  mode = value
  try {
    localStorage.setItem(storageKey, value)
  } catch (cause) {
    reportCaughtError('app/src/features/playback/use-playback-bar-mode.ts', cause)

    /* Keep the current session usable. */
  }
  listeners.forEach((notify) => notify())
}
export function usePlaybackBarMode() {
  return [
    useSyncExternalStore(subscribe, () => (isAndroid() ? 'persistent' : mode)),
    setMode
  ] as const
}
