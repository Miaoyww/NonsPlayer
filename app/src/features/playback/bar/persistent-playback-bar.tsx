import { CurrentTrackLike } from '@/features/playback/now-playing/current-track-like'
import { statusLabels, usePlayer } from '@/lib/player'
import { Cover } from '@/components/music/cover'
import { PlaybackTimeline } from '@/features/playback/controls/playback-timeline'
import { PlaybackTransport } from '@/features/playback/controls/playback-transport'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/bar/playback-notice'
import { QueuePopover } from '@/features/queue/queue-popover'
import { TrackIdentity } from '@/components/music/track-identity'
import { VolumeControl } from '@/features/playback/controls/volume-control'
import { isAndroid } from '@/lib/platform'
import { DesktopLyricsButton } from '@/features/desktop-lyrics/desktop-lyrics-button'
import { MobilePlaybackBar } from '@/features/playback/bar/mobile-playback-bar'

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
  if (mobile) {
    return (
      <MobilePlaybackBar notice={notice} onLyrics={onLyrics} onQueue={onQueue} onError={onError} />
    )
  }
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
    <footer className="persistent-playback" aria-label="常驻播放栏">
      <PlaybackNotice notice={notice} />
      <div className="persistent-playback-surface">
        <div className="capsule-glass glass-surface" aria-hidden="true" />
        <PlaybackTimeline layout="edge" showTimeOnHover onError={onError} />
        {identity}
        <PlaybackTransport withModes onError={onError} />
        <div className="persistent-options">
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
