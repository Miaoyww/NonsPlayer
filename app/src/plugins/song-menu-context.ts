import type { Track } from '@/lib/player'
import type { PluginManifest } from './types'

export function songMenuContributions(manifest: PluginManifest, track: Track) {
  return (manifest.contributes.contextMenus ?? []).filter(
    (menu) => menu.target === 'song' && menu.source === track.source.kind
  )
}
export function publicMenuSong(track: Track) {
  return {
    key: track.key,
    title: track.title,
    aliases: track.aliases,
    artists: track.artists,
    albumId: track.albumId,
    artist: track.artist,
    album: track.album,
    durationMs: track.durationMs,
    cover: track.cover.startsWith('http') ? track.cover : '',
    source:
      track.source.kind === 'netease'
        ? { kind: 'netease' as const, id: track.source.id }
        : track.source.kind === 'online'
          ? { kind: 'online' as const, reference: track.source.track.reference }
          : { kind: 'local' as const }
  }
}
