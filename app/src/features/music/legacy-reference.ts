import type { Track } from '@/lib/player'
import type { EntityRef } from './types'
// Historical queue data is interpreted once at the boundary, never in plugin DTOs.
export function onlineReference(track: Track): EntityRef | null {
  if (track.source.kind === 'online') return track.source.track.reference
  if (track.source.kind === 'netease')
    return { source: 'netease', kind: 'track', id: String(track.source.id) }
  return null
}
export function favoriteId(
  track: Track,
  account?: import('./types').AccountRef
): string | undefined {
  const reference = onlineReference(track)
  return reference && reference.source === account?.source ? reference.id : undefined
}
export function collectionReference(
  value: import('@/features/workspace/music-navigation').MusicCollection
): EntityRef {
  if (value.reference) return value.reference
  if (
    value.localId ||
    typeof value.id !== 'number' ||
    !Number.isSafeInteger(value.id) ||
    value.id <= 0
  )
    throw new Error('收藏缺少来源引用')
  return { source: 'netease', kind: value.kind, id: String(value.id) }
}
