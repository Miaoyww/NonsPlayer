import { reportCaughtError } from '@/lib/diagnostics'
import { useSyncExternalStore } from 'react'

const storageKey = 'nons-glass-effect'
const listeners = new Set<() => void>()
let enabled = true

function readEnabled() {
  try {
    return localStorage.getItem(storageKey) !== 'off'
  } catch (cause) {
    reportCaughtError('app/src/features/settings/appearance/use-glass-effect.ts', cause)

    /* Storage is optional. */
    return true
  }
}

function apply() {
  if (enabled) delete document.documentElement.dataset.glassEffect
  else document.documentElement.dataset.glassEffect = 'off'
}

export function initializeGlassEffect() {
  enabled = readEnabled()
  apply()
}

// The setting is stored per origin, so another window's change arrives as a storage event.
if (typeof window !== 'undefined') {
  window.addEventListener('storage', (event) => {
    if (event.key !== null && event.key !== storageKey) return
    const next = readEnabled()
    if (next === enabled) return
    enabled = next
    apply()
    listeners.forEach((notify) => notify())
  })
}

const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function setEnabled(value: boolean) {
  if (value === enabled) return
  enabled = value
  try {
    localStorage.setItem(storageKey, value ? 'on' : 'off')
  } catch (cause) {
    reportCaughtError('app/src/features/settings/appearance/use-glass-effect.ts', cause)

    /* Keep the current session usable. */
  }
  apply()
  listeners.forEach((notify) => notify())
}

export function useGlassEffect() {
  return [useSyncExternalStore(subscribe, () => enabled), setEnabled] as const
}
