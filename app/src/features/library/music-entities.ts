import { requestCache, presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useState } from 'react'
import { queryMusic } from '@/features/music/queries'
import { collectionReference } from '@/features/music/legacy-reference'
import { musicCollection } from '@/features/music/entities'
import { isTauri } from '@tauri-apps/api/core'
import { errorText } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export interface EntityDetail {
  item: MusicCollection
  description?: string | null
  albumCount?: number | null
}
async function readDetail(collection: MusicCollection): Promise<EntityDetail> {
  const entity = collectionReference(collection)
  const response = await queryMusic({ operation: 'detail', entity }, entity.source)
  if (response.type !== 'detail') throw new Error('详情响应无效')
  return { ...response.data, item: musicCollection(response.data.item) }
}

export function useEntityDetail(collection: MusicCollection, refresh: number, enabled = true) {
  const cacheKey = presentationKey('entity-detail', collectionReference(collection))
  const cached = enabled ? requestCache.peekFresh<EntityDetail>(cacheKey) : undefined
  const [detail, setDetail] = useState<EntityDetail | undefined>(cached)
  const [error, setError] = useState<string>()
  const [busy, setBusy] = useState(false)
  useEffect(() => {
    let disposed = false
    const revision = requestCache.revision
    setDetail(cached)
    setError(undefined)
    setBusy(enabled && isTauri() && !cached)
    if (enabled && isTauri())
      void readDetail(collection)
        .then((value) => {
          if (!disposed) {
            if (requestCache.revision === revision) requestCache.set(cacheKey, value)
            setDetail(value)
          }
        })
        .catch((cause) => {
          reportCaughtError('app/src/features/library/music-entities.ts', cause)

          if (!disposed) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed) setBusy(false)
        })
    return () => {
      disposed = true
    }
  }, [cacheKey, refresh, enabled])
  return { detail, error, busy }
}
