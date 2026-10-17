import { useId } from 'react'
import { ChevronDown, ChevronUp } from 'lucide-react'
import { Cover } from '@/components/music/cover'
import { ActionButton } from '@/components/music/action-button'
import { CueTrackCard } from './cue-track-card'
import { formatTime, type Track } from '@/lib/player'
import './cue-card.css'

export interface CueCardEntry {
  track: Track
  index: number
  rowKey: string
}

export function CueCard({
  entries,
  expanded,
  onExpandedChange,
  busy,
  isCurrent,
  selectedKey,
  onPlay,
  onAppend,
  onRemove,
  removeLabel
}: {
  entries: CueCardEntry[]
  expanded: boolean
  onExpandedChange: (expanded: boolean) => void
  busy: boolean
  isCurrent: (entry: CueCardEntry) => boolean
  selectedKey?: string
  onPlay: (entry: CueCardEntry) => void
  onAppend?: (track: Track) => void
  onRemove?: (track: Track, index: number) => void | Promise<unknown>
  removeLabel?: string
}) {
  const id = useId()
  const first = entries[0].track
  const label = first.album || '未命名专辑'
  const artists = [...new Set(entries.map(({ track }) => track.artist))].join(' / ')
  const Icon = expanded ? ChevronUp : ChevronDown
  return (
    <article className="cue-card" data-expanded={expanded} data-current={entries.some(isCurrent)}>
      <div className="cue-card-header">
        <button
          type="button"
          className="cue-card-open"
          aria-expanded={expanded}
          aria-controls={id}
          onClick={() => onExpandedChange(true)}
        >
          <Cover cover={first.cover} size="small" className="cue-card-cover" />
          <span className="cue-card-text">
            <span className="cue-card-title">{label}</span>
            <span className="cue-card-subtitle">{artists}</span>
          </span>
          <span className="cue-card-count">CUE · {entries.length} 首</span>
          <span className="cue-card-duration">
            {formatTime(entries.reduce((sum, { track }) => sum + track.durationMs, 0))}
          </span>
        </button>
        <ActionButton
          variant="ghost"
          size="icon-sm"
          aria-expanded={expanded}
          aria-controls={id}
          aria-label={`${expanded ? '折叠' : '展开'} ${label}`}
          title={expanded ? '折叠曲目' : '展开曲目'}
          onClick={() => onExpandedChange(!expanded)}
        >
          <Icon aria-hidden="true" />
        </ActionButton>
      </div>
      <div
        id={id}
        hidden={!expanded}
        className="cue-card-children"
        role="group"
        aria-label={`${label} 分轨`}
      >
        {expanded &&
          entries.map((entry) => (
            <CueTrackCard
              key={entry.rowKey}
              track={entry.track}
              selected={selectedKey === entry.rowKey}
              current={isCurrent(entry)}
              busy={busy}
              onPlay={() => onPlay(entry)}
              onAppend={onAppend ? () => onAppend(entry.track) : undefined}
              onRemove={onRemove ? () => onRemove(entry.track, entry.index) : undefined}
              removeLabel={removeLabel}
            />
          ))}
      </div>
    </article>
  )
}
