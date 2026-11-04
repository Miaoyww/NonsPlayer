import { nativeCall, type Track } from '@/lib/player'
import { getTrackDoubleClickSettings } from './use-track-double-click'

export async function playTrackOnDoubleClick(
  track: Track,
  queue?: { tracks: Track[]; index: number }
): Promise<string | undefined> {
  const { action, scope } = getTrackDoubleClickSettings()
  if (scope === 'queue' && queue?.tracks[queue.index] === track) {
    await nativeCall('play_queue', {
      keys: queue.tracks.map((item) => item.key),
      index: queue.index
    })
    return
  }
  await nativeCall('append_queue', { keys: [track.key], play: action === 'play' })
  if (action === 'next') return `已将「${track.title}」设为下一首播放。`
}
