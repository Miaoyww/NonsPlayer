import { CurrentTrackLike } from '@/features/playback/now-playing/current-track-like'
import { CurrentTrackCollect } from '@/features/playback/now-playing/current-track-collect'
import { statusLabels, usePlayer } from '@/lib/player'
import { ListMusic } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { PlaybackTimeline } from '@/features/playback/controls/playback-timeline'
import { PlaybackTransport } from '@/features/playback/controls/playback-transport'
import { VolumeControl } from '@/features/playback/controls/volume-control'
import { DesktopLyricsButton } from '@/features/desktop-lyrics/desktop-lyrics-button'
import type { ReactNode } from 'react'

export function FullscreenPlaybackBar({
  onQueuePanel,
  queueActive = false,
  onError,
  pluginActions
}: {
  onQueuePanel: () => void
  queueActive?: boolean
  onError: (error: unknown) => void
  pluginActions?: ReactNode
}) {
  const state = usePlayer('actualQuality', 'index', 'queue', 'status')
  const track = state.index === null ? undefined : state.queue[state.index]
  return (
    <footer className="fullscreen-playback-bar" aria-label="全屏播放控制">
      <div className="fullscreen-playback-bar-surface">
        <div className="fullscreen-playback-left">
          <CurrentTrackLike track={track} onError={onError} />
          <CurrentTrackCollect track={track} />
          {pluginActions}
        </div>
        <div className="fullscreen-playback-main">
          <PlaybackTransport withModes onError={onError} />
          <PlaybackTimeline layout="inline" onError={onError} />
        </div>
        <div className="persistent-options">
          <DesktopLyricsButton onError={onError} />
          <VolumeControl onError={onError} />
          <ActionButton
            variant="ghost"
            size="icon-sm"
            aria-label={queueActive ? '隐藏播放列表' : '显示播放列表'}
            aria-pressed={queueActive}
            title={queueActive ? '隐藏播放列表' : '显示播放列表'}
            onClick={onQueuePanel}
          >
            <ListMusic aria-hidden="true" />
          </ActionButton>
        </div>
        <span className="sr-only">
          {statusLabels[state.status]}
          {state.actualQuality && ` · 实际音质 ${state.actualQuality}`}
        </span>
      </div>
    </footer>
  )
}
