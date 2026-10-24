import type { Track } from '@/lib/player'
import type { EntityRef } from './types'
export function onlineReference(track: Track): EntityRef | null {
  if (track.source.kind === 'online') return track.source.track.reference
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
  const reference = optionalCollectionReference(value)
  if (!reference) throw new Error('收藏缺少来源引用')
  return reference
}
export function optionalCollectionReference(
  value: import('@/features/workspace/music-navigation').MusicCollection
): EntityRef | null {
  return value.reference
}
