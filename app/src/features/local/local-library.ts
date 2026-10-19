import { nativeCall, type Track } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'
export interface LocalEntity {
  id: string
  kind: 'artist' | 'album' | 'playlist'
  name: string
  cover: string
  subtitle: string
  trackCount: number
}
export function localCollection(item: LocalEntity): MusicCollection {
  return { ...item, id: 0, localId: item.id }
}
export async function playLocalEntity(kind: string, id: string) {
  if (kind === 'playlist') {
    await nativeCall('aggregate_play', { id })
    return false
  }
  const tracks: Track[] = []
  let more = true
  for (let offset = 0; more && tracks.length < 1000; offset += 100) {
    const page = await nativeCall<{ items: Track[]; more: boolean }>('local_entity_tracks', {
      kind,
      id,
      keyword: '',
      offset
    })
    tracks.push(...page.items)
    more = page.more
  }
  if (!tracks.length) throw new Error('这里还没有可播放的音乐。')
  await nativeCall('play_queue', {
    keys: tracks.slice(0, 1000).map((track) => track.key),
    index: 0
  })
  return more
}
