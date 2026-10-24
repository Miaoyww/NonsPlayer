import { reportCaughtError } from '@/lib/diagnostics'
import { validCoverInput } from '@/features/music/cover-image'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export const recentPlaylistLimit = 20
export const recentPlaylistKey = (userId: number | string) => `nons-recent-playlists:${userId}`

export function parseRecentPlaylists(saved: string | null): MusicCollection[] {
  try {
    const items: unknown = JSON.parse(saved ?? '[]')
    if (!Array.isArray(items)) return []
    const ids = new Set<string>()
    return items
      .filter((item): item is MusicCollection => {
        if (
          !item ||
          item.kind !== 'playlist' ||
          typeof item.id !== 'string' ||
          !item.id.length ||
          item.id.length > 1024 ||
          item.reference?.id !== item.id ||
          item.reference?.kind !== 'playlist' ||
          typeof item.reference?.source !== 'string' ||
          !item.reference.source.length ||
          typeof item.name !== 'string' ||
          !validCoverInput(item.cover) ||
          typeof item.subtitle !== 'string' ||
          !Number.isFinite(item.trackCount) ||
          item.trackCount < 0 ||
          ids.has(JSON.stringify(item.reference))
        )
          return false
        ids.add(JSON.stringify(item.reference))
        return true
      })
      .slice(0, recentPlaylistLimit)
  } catch (cause) {
    reportCaughtError('app/src/features/library/recent-playlists.ts', cause)

    return []
  }
}

export function rememberPlaylist(items: MusicCollection[], item: MusicCollection) {
  return [
    item,
    ...items.filter(
      (previous) => JSON.stringify(previous.reference) !== JSON.stringify(item.reference)
    )
  ].slice(0, recentPlaylistLimit)
}
