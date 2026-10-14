import { afterFirstPaint } from '@/lib/after-first-paint'
import { coverCandidates, type CoverInput, type CoverSize } from '@/features/music/cover-image'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { coverSource, nativeCall } from '@/lib/player'
import { coverCache, requestKey } from '@/lib/runtime-cache'
import { useLocalOptions } from '@/features/local/use-local-options'

export function useCoverSource(cover?: CoverInput, enabled = true, size: CoverSize = 'normal') {
  return useCoverImageSource(cover, enabled, size).source
}
interface LoadedCover {
  identity: string
  url: string
  source?: string
  cached: boolean
}
export function useCoverImageSource(
  cover?: CoverInput,
  enabled = true,
  size: CoverSize = 'normal'
) {
  const candidates = coverCandidates(cover, size)
  const identity = JSON.stringify(candidates)
  const [attempt, setAttempt] = useState<{ identity: string; index: number }>()
  const index = attempt?.identity === identity ? attempt.index : 0
  const url = candidates[index] ?? ''
  const { showCovers } = useLocalOptions()
  const remote = /^https?:\/\//.test(url)
  const visible = enabled && !!url && (remote || showCovers)
  const [loaded, setLoaded] = useState<LoadedCover>()
  useEffect(() => {
    if (!visible) {
      setAttempt(undefined)
      setLoaded(undefined)
      return
    }
    if (!remote || !isTauri()) return
    let disposed = false
    const cancel = afterFirstPaint(() => {
      void nativeCall<string>('runtime_cover', { url })
        .then((source) => {
          if (!disposed) setLoaded({ identity, url, source, cached: true })
        })
        .catch(() => {
          if (disposed) return
          if (index + 1 < candidates.length) setAttempt({ identity, index: index + 1 })
          else setLoaded({ identity, url, source: coverSource(url), cached: false })
        })
    })
    return () => {
      cancel()
      disposed = true
    }
  }, [identity, url, index, visible, remote])
  const current = loaded?.identity === identity && loaded.url === url ? loaded : undefined
  const source = !visible
    ? undefined
    : current
      ? current.source
      : remote && isTauri()
        ? undefined
        : coverSource(url)
  const onError = () => {
    if (!source || !url) return
    if (remote && isTauri() && current?.cached) {
      coverCache.delete(requestKey('runtime_cover', { url }))
      void nativeCall('runtime_cover_invalidate', { url }).catch(() => {})
    }
    if (index + 1 < candidates.length) setAttempt({ identity, index: index + 1 })
    else if (current?.cached) setLoaded({ identity, url, source: coverSource(url), cached: false })
    else setLoaded({ identity, url, source: undefined, cached: false })
  }
  return { source, onError, pending: visible && remote && isTauri() && !current }
}
