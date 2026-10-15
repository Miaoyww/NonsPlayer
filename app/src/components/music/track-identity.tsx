import { useMusicSources } from '@/features/music/use-music-sources'
import type { ReactNode } from 'react'
import type { Track } from '@/lib/player'
import { Cover } from './cover'
import { TrackArtists } from './music-links'
import { TrackTitle, trackDisplayTitle } from './track-title'
import { sourceLabel, trackSource } from '@/features/music/source-label'
import {
  useInterfaceDensity,
  type InterfaceDensity
} from '@/features/settings/use-interface-density'

export function TrackIdentity({
  track,
  cover,
  showSource = false,
  mode,
  onTitleCopy
}: {
  track: Track
  cover?: ReactNode
  showSource?: boolean
  mode?: InterfaceDensity
  onTitleCopy?: () => void
}) {
  const [preferredMode] = useInterfaceDensity()
  const displayMode = mode ?? preferredMode
  return (
    <div className="track-identity flex min-w-0 items-center" data-mode={displayMode}>
      {cover ?? <Cover size="small" cover={track.cover} className="track-identity-cover" />}
      <div className="min-w-0 flex-1">
        {onTitleCopy ? (
          <button
            type="button"
            className="track-title track-title-copy truncate font-medium"
            aria-label={`复制歌曲名称 ${track.title}`}
            title={trackDisplayTitle(track)}
            onClick={(event) => {
              event.stopPropagation()
              onTitleCopy()
            }}
          >
            <TrackTitle track={track} />
          </button>
        ) : (
          <p className="track-title truncate font-medium" title={trackDisplayTitle(track)}>
            <TrackTitle track={track} />
          </p>
        )}
        <p
          className="track-identity-artist truncate text-xs text-muted-foreground"
          title={track.artist}
        >
          <TrackArtists track={track} />
          {track.source.kind === 'online' && track.source.track.unavailableReason && (
            <span className="ml-2" aria-label="官方音源受限，可尝试备用音源">
              · 官方受限
            </span>
          )}
        </p>
      </div>
      {showSource && (
        <span className="ml-3 max-w-24 shrink-0">
          <TrackSourceLabel track={track} />
        </span>
      )}
    </div>
  )
}

export function TrackSourceLabel({ track }: { track: Track }) {
  const { statuses } = useMusicSources()
  const name = sourceLabel(
    trackSource(track),
    statuses.map((value) => value.descriptor)
  )
  return (
    <span
      className="track-source block truncate text-xs text-muted-foreground"
      title={name}
      aria-label={`歌曲来源：${name}`}
    >
      {name}
    </span>
  )
}
