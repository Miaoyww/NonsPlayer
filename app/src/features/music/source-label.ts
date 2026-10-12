import type { Track } from '@/lib/player'
export function sourceLabel(source: string) {
  return (
    ({ netease: '网易云音乐', qqmusic: 'QQ 音乐', local: '本地音乐' } as Record<string, string>)[
      source
    ] ?? source
  )
}
export function trackSource(track: Track) {
  return track.source.kind === 'online' ? track.source.track.reference.source : track.source.kind
}
