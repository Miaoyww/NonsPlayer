import { useSyncExternalStore } from 'react'

let hidden = false
const listeners = new Set<() => void>()

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function setPanelHidden(value: boolean) {
  if (hidden === value) return
  hidden = value
  listeners.forEach((listener) => listener())
}

export function usePanelHidden() {
  return [useSyncExternalStore(subscribe, () => hidden), setPanelHidden] as const
}
