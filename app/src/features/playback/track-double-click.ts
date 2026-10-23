import { nativeCall, type Track } from '@/lib/player'
import { getTrackDoubleClickSettings } from './use-track-double-click'

export function playTrackOnDoubleClick(
  track: Track,
  queue?: { tracks: Track[]; index: number }
): Promise<void> {
  const { action, scope } = getTrackDoubleClickSettings()
  if (scope === 'queue' && queue?.tracks[queue.index] === track) {
    return nativeCall('play_queue', {
      keys: queue.tracks.map((item) => item.key),
      index: queue.index
    })
  }
  return nativeCall('append_queue', { keys: [track.key], play: action === 'play' })
}
