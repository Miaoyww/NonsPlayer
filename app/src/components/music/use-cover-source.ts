import { reportCaughtError } from '@/lib/diagnostics'
import { afterFirstPaint } from '@/lib/after-first-paint'
import { coverCandidates, type CoverInput, type CoverSize } from '@/features/music/cover-image'
import { useEffect, useRef, useState } from 'react'
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
  pending?: boolean
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
  const animation = useRef<{ url: string; cached: boolean } | undefined>(undefined)
  if (animation.current?.url !== url)
    animation.current = {
      url,
      cached: remote && coverCache.peekFresh(requestKey('runtime_cover', { url })) !== undefined
    }
  const [loaded, setLoaded] = useState<LoadedCover>()
  const localRecovery = useRef<
    { identity: string; url: string; status?: 'pending' | 'done' } | undefined
  >(undefined)
  useEffect(() => {
    const recovery = { identity, url }
    localRecovery.current = visible ? recovery : undefined
    return () => {
      if (localRecovery.current === recovery) localRecovery.current = undefined
    }
  }, [identity, url, visible])
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
        .catch((cause) => {
          reportCaughtError('app/src/components/music/use-cover-source.ts', cause)

          if (disposed) return
          if (index + 1 < candidates.length) setAttempt({ identity, index: index + 1 })
          else setLoaded({ identity, url, source: undefined, cached: false })
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
    if (!remote && isTauri()) {
      const recovery = localRecovery.current
      if (!recovery || recovery.identity !== identity || recovery.url !== url) return
      if (recovery.status === 'pending') return
      if (!recovery.status) {
        recovery.status = 'pending'
        setLoaded({ identity, url, source: undefined, cached: false, pending: true })
        void nativeCall<string>('local_cover_repair', { url })
          .then((repaired) => {
            if (localRecovery.current !== recovery) return
            recovery.status = 'done'
            const repairedSource = coverSource(repaired)
            setLoaded({
              identity,
              url,
              source: repairedSource
                ? `${repairedSource}?local-cover-repair=${Date.now()}`
                : undefined,
              cached: false
            })
          })
          .catch((cause) => {
            reportCaughtError('app/src/components/music/use-cover-source.ts', cause)
            if (localRecovery.current !== recovery) return
            recovery.status = 'done'
            setLoaded({ identity, url, source: undefined, cached: false })
          })
        return
      }
    }
    if (remote && isTauri() && current?.cached) {
      coverCache.delete(requestKey('runtime_cover', { url }))
      void nativeCall('runtime_cover_invalidate', { url }).catch((cause) => {
        reportCaughtError('app/src/components/music/use-cover-source.ts', cause)
      })
    }
    if (index + 1 < candidates.length) setAttempt({ identity, index: index + 1 })
    else setLoaded({ identity, url, source: undefined, cached: false })
  }
  return {
    source,
    onError,
    skipAnimation: animation.current.cached,
    pending: visible && ((remote && isTauri() && !current) || !!current?.pending)
  }
}
