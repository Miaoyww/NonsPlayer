import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { errorText, nativeCall } from '@/lib/player'
import { sourcePreferences, type SourcePreferences } from './source-preferences'
import type { SourceDescriptor } from './types'

export interface SourceStatus {
  descriptor: SourceDescriptor
  enabled: boolean
}

export function useMusicSources() {
  const [preferences, setPreferences] = useState<SourcePreferences>()
  const [statuses, setStatuses] = useState<SourceStatus[]>([])
  const [revision, setRevision] = useState(0)
  const [error, setError] = useState('')
  const generation = useRef(0)
  const refresh = useCallback(async () => {
    const serial = ++generation.current
    if (!isTauri()) return
    try {
      const [next, adapters] = await Promise.all([
        sourcePreferences.get(),
        nativeCall<SourceStatus[]>('adapter_list')
      ])
      if (serial !== generation.current) return
      setPreferences(next)
      setStatuses(adapters)
      setRevision((value) => value + 1)
      setError('')
    } catch (cause) {
      if (serial === generation.current) setError(errorText(cause))
    }
  }, [])
  useEffect(() => {
    let disposed = false
    const stops: (() => void)[] = []
    if (isTauri())
      void (async () => {
        // Subscribe before the initial read so a change cannot be missed at startup.
        for (const event of [
          'music-source-preferences-changed',
          'adapters-changed',
          'music-account-changed',
          'music-changed'
        ]) {
          const stop = await listen(event, () => {
            if (!disposed) void refresh()
          })
          if (disposed) stop()
          else stops.push(stop)
        }
        if (!disposed) await refresh()
      })().catch((cause) => {
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
      generation.current++
      stops.forEach((stop) => stop())
    }
  }, [refresh])
  return { preferences, statuses, revision, error, refresh }
}
