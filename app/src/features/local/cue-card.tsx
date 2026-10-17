import { useId } from 'react'
import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { ChevronDown, ChevronUp } from 'lucide-react'
import { Cover } from '@/components/music/cover'
import { ActionButton } from '@/components/music/action-button'
import { CueTrackCard } from './cue-track-card'
import { TrackIdentity, TrackSourceLabel } from '@/components/music/track-identity'
import { CueContextMenu } from './cue-context-menu'
import type { ReactNode } from 'react'
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
  removeLabel,
  columns,
  sequence,
  mobile = false,
  showDuration = true,
  showLikes = false,
  extraCells = [],
  onPlayGroup
}: {
  columns: { id: string; size: number }[]
  sequence: number
  mobile?: boolean
  showDuration?: boolean
  showLikes?: boolean
  extraCells?: ReactNode[]
  onPlayGroup: () => void
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
  const reducedMotion = useReducedMotion()
  const first = entries[0].track
  const label = first.album || '未命名专辑'
  const total = columns.reduce((sum, column) => sum + column.size, 0)
  const toggle = () => onExpandedChange(!expanded)
  const Icon = expanded ? ChevronUp : ChevronDown
  return (
    <article className="cue-card" data-expanded={expanded} data-current={entries.some(isCurrent)}>
      <table className="cue-card-header" aria-label={`${label} CUE`}>
        <colgroup>
          {columns.map((column) => (
            <col key={column.id} style={{ width: `${(column.size / total) * 100}%` }} />
          ))}
        </colgroup>
        <tbody>
          <CueContextMenu
            entries={entries}
            busy={busy}
            onPlay={onPlayGroup}
            onRemove={onRemove}
            removeLabel={removeLabel}
            render={
              <tr
                className="track-row cue-parent-row"
                onClick={(event) => {
                  const control = (event.target as HTMLElement).closest(
                    'button, a, input, [role="button"]'
                  )
                  if (!control || control === event.currentTarget) toggle()
                }}
                onKeyDown={(event) => {
                  if (
                    event.target === event.currentTarget &&
                    (event.key === 'Enter' || event.key === ' ')
                  ) {
                    event.preventDefault()
                    toggle()
                  }
                }}
                aria-expanded={expanded}
                aria-controls={id}
              />
            }
          >
            {!mobile && (
              <td className="text-center tabular-nums text-muted-foreground">{sequence}</td>
            )}
            <td className="track-identity-cell pr-4">
              <TrackIdentity
                track={{ ...first, title: label }}
                cover={
                  <button
                    type="button"
                    className="track-cover cue-card-open"
                    aria-label={`${expanded ? '折叠' : '展开'} ${label} 曲目`}
                    aria-expanded={expanded}
                    aria-controls={id}
                    onClick={toggle}
                  >
                    <Cover cover={first.cover} size="small" className="track-identity-cover" />
                  </button>
                }
              />
            </td>
            {!mobile && (
              <td className="pr-4 text-muted-foreground">
                <span className="cue-card-count">CUE · {entries.length} 首</span>
              </td>
            )}
            <td className="pr-3">
              <TrackSourceLabel track={first} />
            </td>
            {showLikes && <td />}
            {showDuration && (
              <td className="tabular-nums text-muted-foreground">
                {formatTime(entries.reduce((sum, { track }) => sum + track.durationMs, 0))}
              </td>
            )}
            {extraCells.map((cell, index) => (
              <td key={index}>{cell}</td>
            ))}
            {columns.some((column) => column.id === 'append') && (
              <td>
                <ActionButton
                  variant="ghost"
                  size="icon-sm"
                  aria-expanded={expanded}
                  aria-controls={id}
                  aria-label={`${expanded ? '折叠' : '展开'} ${label}`}
                  title={expanded ? '折叠曲目' : '展开曲目'}
                  onClick={toggle}
                >
                  <Icon aria-hidden="true" />
                </ActionButton>
              </td>
            )}
          </CueContextMenu>
        </tbody>
      </table>
      <div id={id} inert={!expanded} aria-hidden={!expanded}>
        <AnimatePresence initial={false}>
          {expanded && (
            <motion.div
              key="tracks"
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: reducedMotion ? 0 : 0.22, ease: 'easeInOut' }}
              style={{ overflow: 'hidden' }}
            >
              <div className="cue-card-children" role="group" aria-label={`${label} 分轨`}>
                {entries.map((entry) => (
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
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </article>
  )
}
