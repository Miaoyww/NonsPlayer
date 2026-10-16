import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useSyncExternalStore } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { nativeCall } from '@/lib/player'
export interface LocalPreferences {
  artistSeparators: string[]
}
let options: LocalPreferences = { artistSeparators: ['/', '、', ';'] }
let initial: Promise<void> | undefined
const listeners = new Set<() => void>()
const subscribe = (fn: () => void) => {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}
function update(value: LocalPreferences) {
  options = value
  listeners.forEach((fn) => fn())
}
export async function setLocalPreferences(value: LocalPreferences) {
  await nativeCall('set_local_preferences', { options: value })
  update(value)
}
export function useLocalPreferences() {
  const value = useSyncExternalStore(subscribe, () => options)
  useEffect(() => {
    if (!isTauri()) return
    initial ??= nativeCall<LocalPreferences>('local_preferences')
      .then(update)
      .catch((cause) => {
        reportCaughtError('app/src/features/local/use-local-preferences.ts', cause)

        initial = undefined
      })
  }, [])
  return { options: value, setOptions: setLocalPreferences }
}
