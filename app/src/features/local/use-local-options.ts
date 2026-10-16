import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useSyncExternalStore } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { nativeCall } from '@/lib/player'

const listeners = new Set<() => void>()
let showCovers = true
let initial: Promise<void> | undefined
const subscribe = (fn: () => void) => {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}
function update(value: boolean) {
  showCovers = value
  listeners.forEach((fn) => fn())
}
export async function setLocalCovers(value: boolean) {
  if (isTauri()) await nativeCall('set_local_options', { showCovers: value })
  update(value)
}
export function useLocalOptions() {
  const value = useSyncExternalStore(subscribe, () => showCovers)
  useEffect(() => {
    if (!isTauri()) return
    initial ??= nativeCall<{ showCovers: boolean }>('local_options')
      .then((options) => update(options.showCovers))
      .catch((cause) => {
        reportCaughtError('app/src/features/local/use-local-options.ts', cause)

        initial = undefined
      })
  }, [])
  return { showCovers: value, setShowCovers: setLocalCovers }
}
