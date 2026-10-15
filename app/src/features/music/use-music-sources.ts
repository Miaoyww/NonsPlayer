import { useSyncExternalStore } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { errorText, nativeCall } from '@/lib/player'
import { sourcePreferences } from './source-preferences'
import { createSourceStore, type SourceStatus } from './source-store'
export type { SourceStatus } from './source-store'
const sources = createSourceStore(
  () =>
    isTauri()
      ? Promise.all([sourcePreferences.get(), nativeCall<SourceStatus[]>('adapter_list')])
      : Promise.resolve([undefined, []]),
  (event, changed) =>
    isTauri() ? listen(event, ({ payload }) => changed(payload)) : Promise.resolve(() => {}),
  errorText
)
export function useMusicSources() {
  return {
    ...useSyncExternalStore(sources.subscribe, sources.getSnapshot),
    refresh: sources.refresh
  }
}
