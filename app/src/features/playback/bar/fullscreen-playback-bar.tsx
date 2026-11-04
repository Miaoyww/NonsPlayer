import { CurrentTrackLike } from '@/features/playback/now-playing/current-track-like'
import { CurrentTrackCollect } from '@/features/playback/now-playing/current-track-collect'
import { statusLabels, usePlayer } from '@/lib/player'
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
        <div className="fullscreen-playback-left">
          <CurrentTrackLike track={track} onError={onError} />
          <CurrentTrackCollect track={track} />
        </div>
        <div className="fullscreen-playback-main">
          <PlaybackTransport withModes onError={onError} />
          <PlaybackTimeline layout="inline" onError={onError} />
        </div>
        <div className="persistent-options">
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
