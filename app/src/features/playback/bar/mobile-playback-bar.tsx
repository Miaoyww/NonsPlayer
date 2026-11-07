import { Pause, Play } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import type { CSSProperties } from 'react'
import { nativeCall, statusLabels, usePlayer, useProgress } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { trackDisplayTitle } from '@/components/music/track-title'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/bar/playback-notice'

function QueueIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M3 4.5v6l5.5-3z" fill="currentColor" stroke="none" />
      <path d="M12 5h9" />
      <path d="M12 12.5h9" />
      <path d="M3 19.5h18" />
    </svg>
  )
}

export function MobilePlaybackBar({
  onLyrics,
  onQueue,
  onError,
  notice
}: {
  notice?: PlaybackNoticeMessage
  onLyrics: () => void
  onQueue: () => void
  onError: (error: unknown) => void
}) {
  const state = usePlayer('actualQuality', 'index', 'queue', 'status')
  const progress = useProgress()
  const track = state.index === null ? undefined : state.queue[state.index]
  const playing = ['playing', 'buffering', 'loading'].includes(state.status)
  const ratio = progress.durationMs > 0 ? Math.min(1, progress.positionMs / progress.durationMs) : 0
  const title = track ? trackDisplayTitle(track) : '选择一首音乐'
  const label = track ? `${title} - ${track.artist}` : title
  const action = (value: string) =>
    void nativeCall('player_action', { action: value }).catch(onError)
  return (
    <footer className="persistent-playback" data-mobile-expanded aria-label="播放控制">
      <PlaybackNotice notice={notice} />
      <div className="persistent-playback-surface mobile-playback-surface">
        <div className="capsule-glass glass-surface" aria-hidden="true" />
        <button
          type="button"
          className="mobile-playback-track"
          aria-label="打开正在播放"
          title={label}
          onClick={onLyrics}
        >
          <Cover
            size="small"
            loading="eager"
            cover={track?.cover}
            className="mobile-playback-cover"
          />
          <span className="mobile-playback-title">
            {title}
            {track && <span className="mobile-playback-artist"> - {track.artist}</span>}
          </span>
        </button>
        <button
          type="button"
          className="mobile-playback-toggle"
          style={{ '--playback-progress': `${ratio * 100}%` } as CSSProperties}
          aria-label={playing ? '暂停' : '播放'}
          disabled={!track || !isTauri()}
          onClick={() => action('toggle')}
        >
          {playing ? (
            <Pause className="size-4 fill-current" aria-hidden="true" />
          ) : (
            <Play className="size-4 fill-current" aria-hidden="true" />
          )}
        </button>
        <ActionButton
          variant="ghost"
          className="mobile-playback-queue"
          aria-label="打开播放队列"
          disabled={!isTauri()}
          onClick={onQueue}
        >
          <QueueIcon />
        </ActionButton>
        <span className="sr-only">
          {statusLabels[state.status]}
          {state.actualQuality && ` · 实际音质 ${state.actualQuality}`}
        </span>
      </div>
    </footer>
  )
}
