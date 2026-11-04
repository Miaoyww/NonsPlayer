import { CurrentTrackLike } from '@/features/playback/now-playing/current-track-like'
import { statusLabels, usePlayer } from '@/lib/player'
import { QualitySelect } from '@/features/playback/now-playing/music-options'
import { PlaybackTimeline } from '@/features/playback/controls/playback-timeline'
import { PlaybackTransport } from '@/features/playback/controls/playback-transport'
import { QueuePopover } from '@/features/queue/queue-popover'
import { VolumeControl } from '@/features/playback/controls/volume-control'
import { DesktopLyricsButton } from '@/features/desktop-lyrics/desktop-lyrics-button'

export function FullscreenPlaybackBar({
  onQueue,
  onError
}: {
  onQueue: () => void
  onError: (error: unknown) => void
}) {
  const state = usePlayer('actualQuality', 'index', 'queue', 'status')
  const track = state.index === null ? undefined : state.queue[state.index]
  return (
    <footer className="fullscreen-playback-bar" aria-label="全屏播放控制">
      <div className="fullscreen-playback-bar-surface">
        <PlaybackTimeline layout="edge" showTimeOnHover onError={onError} />
        <PlaybackTransport withModes onError={onError} />
        <div className="persistent-options">
          <QualitySelect />
          <CurrentTrackLike track={track} onError={onError} />
          <DesktopLyricsButton onError={onError} />
          <VolumeControl onError={onError} />
          <QueuePopover onPage={onQueue} onError={onError} />
        </div>
        <span className="sr-only">
          {statusLabels[state.status]}
          {state.actualQuality && ` · 实际音质 ${state.actualQuality}`}
        </span>
      </div>
    </footer>
  )
}
