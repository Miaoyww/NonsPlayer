import { useEffect, useSyncExternalStore } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall } from '@/lib/player'

export interface LyricSources {
  localSource: 'online' | 'local'
  amll: boolean
  qq: boolean
}
const listeners = new Set<() => void>()
let state = {
  sources: { localSource: 'online', amll: true, qq: true } as LyricSources,
  ready: false,
  busy: false,
  error: ''
}
let initial: Promise<void> | undefined
export const getLyricSources = () => state.sources
const subscribe = (fn: () => void) => {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}
function update(value: Partial<typeof state>) {
  state = { ...state, ...value }
  listeners.forEach((fn) => fn())
}

export async function setLyricSources(sources: LyricSources) {
  if (state.busy || !state.ready || !isTauri()) return
  update({ busy: true, error: '' })
  try {
    await nativeCall('set_lyric_sources', { sources })
    update({ sources })
  } catch (cause) {
    update({ error: errorText(cause) })
  } finally {
    update({ busy: false })
  }
}

export function useLyricSources() {
  const value = useSyncExternalStore(subscribe, () => state)
  useEffect(() => {
    if (!isTauri()) return
    initial ??= nativeCall<LyricSources>('lyric_sources')
      .then((sources) => update({ sources, ready: true, error: '' }))
      .catch((cause) => {
        initial = undefined
        update({ error: errorText(cause) })
      })
  }, [])
  return { ...value, setSources: setLyricSources }
}
