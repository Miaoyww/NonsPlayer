import { ChevronLeft, ChevronRight, Pause, Play, Repeat, Repeat1, Shuffle } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { adjacentIndex, nativeCall, statusLabels, usePlayer } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'
import { trackDisplayTitle } from '@/components/music/track-title'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/playback-notice'
import { QueuePopover } from '@/features/queue/queue-popover'
import { VolumeControl } from '@/features/playback/volume-control'
import { PlaybackTimeline } from '@/features/playback/playback-timeline'
import { CurrentTrackLike } from '@/features/playback/current-track-like'

export function PlaybackBar({
  onLyrics,
  onQueue,
  onError,
  qualityControl,
  notice
}: {
  notice?: PlaybackNoticeMessage
  qualityControl?: ReactNode
  onLyrics: () => void
  onQueue: () => void
  onError: (error: unknown) => void
}) {
  const state = usePlayer()
  const track = state.index !== null ? state.queue[state.index] : undefined
  const playing = ['playing', 'buffering', 'loading'].includes(state.status)
  const [preview, setPreview] = useState<'previous' | 'next' | null>(null)
  const [hoverExpanded, setHoverExpanded] = useState(false)
  const looping = !state.shuffle && (state.repeatMode === 'all' || state.repeatMode === 'one')
  const previousIndex = adjacentIndex(state, 'previous')
  const nextIndex = adjacentIndex(state, 'next')
  const previous = previousIndex === null ? undefined : state.queue[previousIndex]
  const next = nextIndex === null ? undefined : state.queue[nextIndex]
  const previewTrack = preview === 'previous' ? previous : preview === 'next' ? next : undefined
  const title = previewTrack
    ? trackDisplayTitle(previewTrack)
    : track
      ? trackDisplayTitle(track)
      : '选择一首音乐'
  const action = (action: string) => void nativeCall('player_action', { action }).catch(onError)
  const repeatLabel =
    state.repeatMode === 'one' ? '单曲循环' : state.repeatMode === 'all' ? '列表循环' : '顺序播放'
  return (
    <footer className="floating-playback" aria-label="播放控制">
      <PlaybackNotice notice={notice} />
      <div
        className="playback-capsule"
        data-hover-expanded={hoverExpanded}
        onMouseLeave={() => setHoverExpanded(false)}
      >
        <div className="capsule-glass glass-surface" aria-hidden="true" />
        <div className="capsule-main" onMouseEnter={() => setHoverExpanded(true)}>
          <ActionButton
            size="icon-lg"
            className="capsule-play"
            aria-label={playing ? '暂停' : '播放'}
            disabled={!track || !isTauri()}
            onClick={() => action('toggle')}
          >
            {playing ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
          </ActionButton>
          <div className="capsule-center">
            <div className="capsule-navigation">
              <ActionButton
                variant="ghost"
                size="icon-sm"
                aria-label={previous ? `上一首：${trackDisplayTitle(previous)}` : '上一首'}
                disabled={!track || !isTauri()}
                onMouseEnter={() => setPreview('previous')}
                onMouseLeave={() => setPreview(null)}
                onFocus={() => setPreview('previous')}
                onBlur={() => setPreview(null)}
                onClick={() => action('previous')}
              >
                <ChevronLeft aria-hidden="true" />
              </ActionButton>
              <button
                type="button"
                className="capsule-title"
                data-preview={!!previewTrack}
                aria-label="打开正在播放"
                title={title}
                onClick={onLyrics}
              >
                <span key={title}>{title}</span>
              </button>
              <ActionButton
                variant="ghost"
                size="icon-sm"
                aria-label={next ? `下一首：${trackDisplayTitle(next)}` : '下一首'}
                disabled={!next || !isTauri()}
                onMouseEnter={() => setPreview('next')}
                onMouseLeave={() => setPreview(null)}
                onFocus={() => setPreview('next')}
                onBlur={() => setPreview(null)}
                onClick={() => action('next')}
              >
                <ChevronRight aria-hidden="true" />
              </ActionButton>
            </div>
            <PlaybackTimeline layout="inline" onError={onError} />
          </div>
          <div className="capsule-playback-options">
            {qualityControl}
            <ActionButton
              variant="ghost"
              size="icon-sm"
              aria-label="随机播放"
              className="playback-mode-button"
              title={state.shuffle ? '关闭随机播放' : '开启随机播放'}
              aria-pressed={state.shuffle}
              disabled={!isTauri()}
              onClick={() => action('shuffle')}
            >
              <Shuffle aria-hidden="true" />
            </ActionButton>
            <ActionButton
              variant="ghost"
              size="icon-sm"
              aria-label={`播放模式：${repeatLabel}，点击切换`}
              className="playback-mode-button"
              title={repeatLabel}
              disabled={!isTauri()}
              aria-pressed={looping}
              onClick={() => action('repeat')}
            >
              {state.repeatMode === 'one' ? (
                <Repeat1 aria-hidden="true" />
              ) : (
                <Repeat aria-hidden="true" />
              )}
            </ActionButton>
          </div>
        </div>
        <div className="capsule-options">
          <CurrentTrackLike track={track} onError={onError} />
          <QueuePopover onPage={onQueue} onError={onError} />
        </div>
        <VolumeControl onError={onError} />
        <span className="sr-only">
          {statusLabels[state.status]}
          {state.actualQuality && ` · 实际音质 ${state.actualQuality}`}
        </span>
      </div>
    </footer>
  )
}
