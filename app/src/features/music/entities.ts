import type { MusicEntity } from './types'
import type { MusicCollection } from '@/features/workspace/music-navigation'
export function musicCollection(item: MusicEntity): MusicCollection {
  if (item.reference.kind === 'track') throw new Error('歌曲不能作为收藏卡片显示')
  return {
    reference: item.reference,
    id: item.reference.id,
    kind: item.reference.kind,
    name: item.name,
    cover: item.cover,
    subtitle: item.subtitle,
    trackCount: item.trackCount,
    creatorId: item.creator?.id,
    liked: item.liked,
    playCount: item.playCount,
    publishedAt: item.publishedAt,
    artists: item.artists
  }
}
