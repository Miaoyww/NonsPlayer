import { validCoverInput } from '@/features/music/cover-image'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export const recentPlaylistLimit = 20
export const recentPlaylistKey = (userId: number) => `nons-recent-playlists:${userId}`

export function parseRecentPlaylists(saved: string | null): MusicCollection[] {
  try {
    const items: unknown = JSON.parse(saved ?? '[]')
    if (!Array.isArray(items)) return []
    const ids = new Set<number>()
    return items
      .filter((item): item is MusicCollection => {
        if (
          !item ||
          item.kind !== 'playlist' ||
          !Number.isSafeInteger(item.id) ||
          item.id <= 0 ||
          typeof item.name !== 'string' ||
          !validCoverInput(item.cover) ||
          typeof item.subtitle !== 'string' ||
          !Number.isFinite(item.trackCount) ||
          item.trackCount < 0 ||
          ids.has(item.id)
        )
          return false
        ids.add(item.id)
        return true
      })
      .slice(0, recentPlaylistLimit)
  } catch {
    return []
  }
}

export function rememberPlaylist(items: MusicCollection[], item: MusicCollection) {
  return [item, ...items.filter((previous) => previous.id !== item.id)].slice(
    0,
    recentPlaylistLimit
  )
}
