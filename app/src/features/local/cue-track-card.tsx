import { ListPlus, Play } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { PlayingIndicator } from '@/components/music/playing-indicator'
import { SongContextMenu } from '@/components/music/song-actions'
import { formatTime, type Track } from '@/lib/player'

export function CueTrackCard({
  track,
  current,
  selected,
  busy,
  onPlay,
  onAppend,
  onRemove,
  removeLabel
}: {
  track: Track
  current: boolean
  selected: boolean
  busy: boolean
  onPlay: () => void
  onAppend?: () => void
  onRemove?: () => void | Promise<unknown>
  removeLabel?: string
}) {
  const number = track.source.kind === 'local' ? track.source.cue?.trackNumber : undefined
  return (
    <SongContextMenu
      track={track}
      busy={busy}
      onPlay={onPlay}
      onRemove={onRemove}
      removeLabel={removeLabel}
      render={
        <div
          className="cue-track-card"
          data-current={current}
          data-selected={selected}
          aria-current={current ? 'true' : undefined}
        />
      }
    >
      <span className="cue-track-number">{number}</span>
      <button
        type="button"
        className="cue-track-play"
        disabled={busy}
        onClick={onPlay}
        aria-label={`播放 ${track.title}`}
      >
        {current ? <PlayingIndicator /> : <Play aria-hidden="true" className="size-3.5" />}
      </button>
      <button type="button" className="cue-track-text" disabled={busy} onClick={onPlay}>
        <span className="cue-track-title">{track.title}</span>
        <span className="cue-track-artist">{track.artist}</span>
      </button>
      <span className="cue-track-duration">{formatTime(track.durationMs)}</span>
      {onAppend && (
        <ActionButton
          variant="ghost"
          size="icon-sm"
          disabled={busy}
          onClick={onAppend}
          aria-label={`下一首播放 ${track.title}`}
          title="下一首播放"
        >
          <ListPlus aria-hidden="true" />
        </ActionButton>
      )}
    </SongContextMenu>
  )
}
