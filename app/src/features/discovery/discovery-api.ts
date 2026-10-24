import { queryMusic, queryPage, currentSource } from '@/features/music/queries'
import { musicClient } from '@/features/music/client'
import { musicCollection } from '@/features/music/entities'
import { onlineReference } from '@/features/music/references'
import type { Track } from '@/lib/player'
import type { CollectionPage } from '@/features/library/library-api'

interface PlaylistQuery {
  section: 'recommended' | 'square'
  category: string
  order: string
  refresh: boolean
}

// Keep cached pages intact; deduplicate only this query's displayed collections.
export function createDiscoveryPlaylistLoader(query: PlaylistQuery) {
  const seen = new Set<string>()
  return async (offset: number): Promise<CollectionPage> => {
    const response = await queryPage(
      {
        operation: 'recommendedPlaylists',
        section: query.section,
        category: query.category,
        order: query.order,
        page: { cursor: null, limit: 30 }
      },
      offset,
      30,
      undefined,
      query.refresh
    )
    if (response.type !== 'entities') throw new Error('推荐歌单响应无效')
    const page = {
      items: response.data.items.map(musicCollection),
      more: response.data.nextCursor !== null
    }
    if (offset === 0) seen.clear()
    return {
      ...page,
      items: page.items.filter((item) => {
        const id = JSON.stringify(item.reference)
        if (seen.has(id)) return false
        seen.add(id)
        return true
      })
    }
  }
}
export async function discoveryCategories(refresh = false) {
  const response = await queryMusic({ operation: 'categories' }, undefined, refresh)
  if (response.type !== 'categories') throw new Error('分类响应无效')
  return response.data
}
export async function discoveryTracks(
  kind: string,
  refresh = false,
  source?: string
): Promise<Track[]> {
  source ??= await currentSource()
  const response = await queryMusic({ operation: 'recommendedTracks', kind }, source, refresh)
  if (response.type !== 'tracks') throw new Error('推荐曲目响应无效')
  return musicClient.queueTracks(source, response.data.items)
}
export async function discoveryRadar() {
  const response = await queryMusic({ operation: 'radar' })
  if (response.type !== 'entity') throw new Error('雷达歌单响应无效')
  return musicCollection(response.data)
}
export async function dislikeTrack(track: Track) {
  const reference = onlineReference(track)
  if (!reference) throw new Error('曲目缺少来源引用')
  return musicClient.write(reference.source, { operation: 'dislike', track: reference })
}
