import { coverOriginal } from '@/features/music/cover-image'
import type { Track } from '@/lib/player'
import type { PluginManifest } from './types'
import { onlineReference } from '@/features/music/legacy-reference'

export function songMenuContributions(manifest: PluginManifest, track: Track) {
  return (manifest.contributes.contextMenus ?? []).filter(
    (menu) => menu.target === 'song' && menu.source === (onlineReference(track)?.source ?? 'local')
  )
}
export function publicMenuSong(track: Track) {
  const cover = coverOriginal(track.cover)
  return {
    key: track.key,
    title: track.title,
    aliases: track.aliases,
    artists: track.artists,
    albumId: track.albumId,
    artist: track.artist,
    album: track.album,
    durationMs: track.durationMs,
    cover: cover.startsWith('http') ? cover : '',
    source: onlineReference(track)
      ? { kind: 'online' as const, reference: onlineReference(track)! }
      : { kind: 'local' as const }
  }
}
