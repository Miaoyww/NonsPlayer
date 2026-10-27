import { nativeCall } from '@/lib/player'
import { coverCache, isOtherQueryCacheKey, requestCache } from '@/lib/runtime-cache'
import { clearLibraryPreviews, libraryPreviewCacheStatus } from '@/features/library/library-preview'
import { clearContinuations } from '@/features/music/queries'

export type CacheUsage = { usedBytes: number; entries: number }
export type NativeCacheKind = 'covers' | 'collections' | 'metadata' | 'compiled' | 'logs'
export type OtherCacheKind = NativeCacheKind | 'queries'
export type OtherCacheStatus = Record<OtherCacheKind, CacheUsage>

export async function otherCacheStatus(): Promise<OtherCacheStatus> {
  const native = await nativeCall<Record<NativeCacheKind, CacheUsage>>('other_cache_status')
  const queries = requestCache.stats(isOtherQueryCacheKey)
  const preview = libraryPreviewCacheStatus()
  return {
    ...native,
    queries: {
      usedBytes: queries.usedBytes + preview.usedBytes,
      entries: queries.entries + preview.entries
    }
  }
}

export async function clearOtherCache(kind: OtherCacheKind) {
  if (kind !== 'queries') await nativeCall('clear_other_cache', { kind })
  if (kind === 'covers') coverCache.invalidate()
  if (kind === 'queries' || kind === 'collections') {
    requestCache.invalidateMatching(isOtherQueryCacheKey)
    clearContinuations()
    clearLibraryPreviews(true)
  }
}
