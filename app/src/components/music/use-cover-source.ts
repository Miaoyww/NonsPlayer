import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { coverSource, nativeCall } from '@/lib/player'
import { coverCache, requestKey } from '@/lib/runtime-cache'
import { useLocalOptions } from '@/features/local/use-local-options'

export function useCoverSource(cover?: string, enabled = true) {
  return useCoverImageSource(cover, enabled).source
}

export function useCoverImageSource(cover?: string, enabled = true) {
  const { showCovers } = useLocalOptions()
  const remote = !!cover && /^https?:\/\//.test(cover)
  const visible = enabled && !!cover && (remote || showCovers)
  const [loaded, setLoaded] = useState<{ cover: string; source?: string }>()
  useEffect(() => {
    if (!visible || !remote || !isTauri()) return
    let disposed = false
    const sized = new URL(cover!)
    sized.searchParams.set('param', '512y512')
    void nativeCall<string>('runtime_cover', { url: sized.toString() })
      .then((source) => {
        if (!disposed) setLoaded({ cover: cover!, source })
      })
      .catch(() => {
        if (!disposed) {
          const source = coverSource(cover!)
          setLoaded({ cover: cover!, source })
        }
      })
    return () => {
      disposed = true
    }
  }, [cover, visible, remote])
  const original = cover ? coverSource(cover) : undefined
  const source = !visible
    ? undefined
    : loaded?.cover === cover
      ? loaded.source
      : remote && isTauri()
        ? undefined
        : original
  const onError = () => {
    if (!source || !cover) return
    if (remote && isTauri() && source !== original) {
      const sized = new URL(cover)
      sized.searchParams.set('param', '512y512')
      coverCache.delete(requestKey('runtime_cover', { url: sized.toString() }))
      void nativeCall('runtime_cover_invalidate', { url: sized.toString() }).catch(() => {})
      setLoaded({ cover, source: original })
    } else setLoaded({ cover, source: undefined })
  }
  return { source, onError, pending: visible && remote && isTauri() && loaded?.cover !== cover }
}
