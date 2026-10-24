import { reportCaughtError } from '@/lib/diagnostics'
import { useSyncExternalStore } from 'react'
import { isAndroid } from '@/lib/platform'

export type InterfaceDensity = 'standard' | 'compact'
const storageKey = 'nons-interface-density'
const listeners = new Set<() => void>()
let mode: InterfaceDensity = isAndroid() ? 'compact' : 'standard'
function isMode(value: unknown): value is InterfaceDensity {
  return value === 'standard' || value === 'compact'
}
try {
  const saved = localStorage.getItem(storageKey)
  if (isMode(saved)) mode = saved
} catch (cause) {
  reportCaughtError('app/src/features/settings/use-interface-density.ts', cause)

  /* Storage is optional. */
}
const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
function setMode(value: InterfaceDensity) {
  if (!isMode(value) || value === mode) return
  mode = value
  try {
    localStorage.setItem(storageKey, value)
  } catch (cause) {
    reportCaughtError('app/src/features/settings/use-interface-density.ts', cause)

    /* Keep the current session usable. */
  }
  listeners.forEach((notify) => notify())
}
export function useInterfaceDensity() {
  return [useSyncExternalStore(subscribe, () => mode), setMode] as const
}
