import type { Track } from '@/lib/player'
export function sourceLabel(
  source: string,
  descriptors: import('./types').SourceDescriptor[] = []
) {
  return source === 'local'
    ? '本地音乐'
    : (descriptors.find((value) => value.source === source)?.displayName ?? source)
}
export function trackSource(track: Track) {
  return track.source.kind === 'online' ? track.source.track.reference.source : track.source.kind
}
