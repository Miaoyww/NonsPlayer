import { CurrentTrackLike } from '@/features/playback/current-track-like'
import { statusLabels, usePlayer } from '@/lib/player'
import { Cover } from '@/components/music/cover'
import { QualitySelect } from '@/features/playback/music-options'
import { PlaybackTimeline } from '@/features/playback/playback-timeline'
import { PlaybackTransport } from '@/features/playback/playback-transport'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/playback-notice'
import { QueuePopover } from '@/features/queue/queue-popover'
import { TrackIdentity } from '@/components/music/track-identity'
import { VolumeControl } from '@/features/playback/volume-control'
import { isAndroid } from '@/lib/platform'
import { useState } from 'react'
import { ChevronDown, ChevronUp } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'

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
  const mobile = isAndroid()
  const [expanded, setExpanded] = useState(false)
  const cover = (
    <button
      type="button"
      className="persistent-cover"
      aria-label="打开正在播放"
      title="打开全屏播放器"
      onClick={onLyrics}
    >
      <Cover size="small" cover={track?.cover} className="size-12" />
    </button>
  )
  const identity = (
    <div className="persistent-track">
      {track ? (
        <TrackIdentity track={track} cover={cover} />
      ) : (
        <>
          {cover}
          <div className="persistent-track-text">
            <p className="persistent-track-title">选择一首音乐</p>
            <p className="persistent-track-artist">Nons</p>
          </div>
        </>
      )}
    </div>
  )
  return (
    <footer
      className="persistent-playback"
      data-mobile-expanded={mobile ? expanded : undefined}
      aria-label="常驻播放栏"
    >
      <PlaybackNotice notice={notice} />
      {mobile && (
        <div
          id="mobile-playback-details"
          className="mobile-playback-details glass-surface"
          inert={!expanded}
          aria-hidden={!expanded}
        >
          {identity}
          <div className="persistent-options">
            <CurrentTrackLike track={track} onError={onError} />
            <QueuePopover onPage={onQueue} onError={onError} />
          </div>
        </div>
      )}
      <div className="persistent-playback-surface">
        <div className="capsule-glass glass-surface" aria-hidden="true" />
        {!isAndroid() && <PlaybackTimeline layout="edge" showTimeOnHover onError={onError} />}
        {!mobile && identity}
        <PlaybackTransport withModes onError={onError} />
        {mobile ? (
          <ActionButton
            variant="ghost"
            size="icon-lg"
            className="mobile-playback-expand"
            aria-label={expanded ? '收起播放栏' : '展开播放栏'}
            aria-expanded={expanded}
            aria-controls="mobile-playback-details"
            onClick={() => setExpanded((value) => !value)}
          >
            {expanded ? <ChevronDown aria-hidden="true" /> : <ChevronUp aria-hidden="true" />}
          </ActionButton>
        ) : (
          <div className="persistent-options">
            {!isAndroid() && <QualitySelect />}
            <CurrentTrackLike track={track} onError={onError} />
            {!isAndroid() && <VolumeControl onError={onError} />}
            <QueuePopover onPage={onQueue} onError={onError} />
          </div>
        )}
        <span className="sr-only">
          {statusLabels[state.status]}
          {state.actualQuality && ` · 实际音质 ${state.actualQuality}`}
        </span>
      </div>
    </footer>
  )
}
