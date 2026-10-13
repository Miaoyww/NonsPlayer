import { CurrentTrackLike } from '@/features/playback/current-track-like'
import { statusLabels, usePlayer } from '@/lib/player'
import { Cover } from '@/components/music/cover'
import { QualitySelect } from '@/features/playback/music-options'
import { PlaybackTimeline } from '@/features/playback/playback-timeline'
import { PlaybackTransport } from '@/features/playback/playback-transport'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/playback-notice'
import { QueuePopover } from '@/features/queue/queue-popover'
import { trackDisplayTitle } from '@/components/music/track-title'
import { VolumeControl } from '@/features/playback/volume-control'
import { isAndroid } from '@/lib/platform'

export function PersistentPlaybackBar({
  onLyrics,
  onQueue,
  onError,
  notice
}: {
  onLyrics: () => void
  onQueue: () => void
  onError: (error: unknown) => void
  notice?: PlaybackNoticeMessage
}) {
  const state = usePlayer('actualQuality', 'index', 'queue', 'status')
  const track = state.index === null ? undefined : state.queue[state.index]
  const title = track ? trackDisplayTitle(track) : '选择一首音乐'
  return (
    <footer className="persistent-playback" aria-label="常驻播放栏">
      <PlaybackNotice notice={notice} />
      <div className="persistent-playback-surface">
        <div className="capsule-glass glass-surface" aria-hidden="true" />
        {!isAndroid() && <PlaybackTimeline layout="edge" showTimeOnHover onError={onError} />}
        <div className="persistent-track">
          <button
            type="button"
            className="persistent-cover"
            aria-label="打开正在播放"
            title="打开全屏播放器"
            onClick={onLyrics}
          >
            <Cover cover={track?.cover} className="size-12" />
          </button>
          <div className="persistent-track-text">
            <p className="persistent-track-title" title={title}>
              {title}
            </p>
            <p className="persistent-track-artist" title={track?.artist}>
              {track?.artist || 'Nons'}
            </p>
          </div>
        </div>
        <PlaybackTransport withModes onError={onError} />
        <div className="persistent-options">
          {!isAndroid() && <QualitySelect />}
          <CurrentTrackLike track={track} onError={onError} />
          {!isAndroid() && <VolumeControl onError={onError} />}
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
