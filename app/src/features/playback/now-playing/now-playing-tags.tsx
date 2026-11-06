import { useSongActions } from '@/components/music/song-actions'
import { trackSource } from '@/features/music/source-label'
import type { PlayerSnapshot, Track } from '@/lib/player'

const SOURCE_BADGES: Record<string, string> = {
  netease: 'NETEASE',
  qq: 'QQ',
  qqmusic: 'QQ',
  local: 'LOCAL'
}

const LOSSLESS_FORMATS = new Set(['flac', 'alac', 'ape', 'wav', 'aiff', 'wavpack', 'tta'])

export function qualityBadgeLabel(quality?: string | null) {
  const value = quality?.trim().toLowerCase()
  if (!value) return '—'
  if (value === 'hires' || value === 'hi-res') return 'Hi-Res'
  if (value === 'lossless' || LOSSLESS_FORMATS.has(value)) return 'Lossless'
  if (value === 'exhigh') return 'HQ'
  if (value === 'higher') return 'SQ'
  if (value === 'standard') return 'LQ'
  const kbps = Number(value.match(/^(\d{3,4})\s*k(?:bps)?$/)?.[1])
  if (Number.isFinite(kbps)) return kbps >= 320 ? 'HQ' : kbps >= 192 ? 'SQ' : 'LQ'
  return value.toUpperCase()
}

export function NowPlayingTags({
  track,
  quality,
  playbackMode,
  lyricFormat,
  onSource,
  sourceActive = false,
  onLyrics,
  lyricsActive = false,
  onInformation,
  informationActive = false
}: {
  track?: Track
  quality?: string | null
  playbackMode?: PlayerSnapshot['playbackMode']
  lyricFormat?: string
  onSource?: () => void
  sourceActive?: boolean
  onLyrics: () => void
  lyricsActive?: boolean
  onInformation?: () => void
  informationActive?: boolean
}) {
  const actions = useSongActions()
  if (!track) return null
  const source = trackSource(track)
  const sourceBadge = SOURCE_BADGES[source] ?? source.toUpperCase()
  const lyricBadge = lyricFormat?.trim().toUpperCase() || 'NO-LRC'
  const qualityBadge =
    track.source.kind === 'local' || playbackMode === 'local' ? 'Local' : qualityBadgeLabel(quality)
  return (
    <div className="now-playing-tags" aria-label="歌曲来源、音质与歌词格式">
      {track.source.kind === 'online' ? (
        <button
          type="button"
          className="now-playing-tag"
          data-active={sourceActive || undefined}
          title={sourceActive ? '返回歌词' : `选择歌曲来源：${sourceBadge}`}
          aria-label={sourceActive ? '返回歌词' : `选择歌曲来源，当前平台 ${sourceBadge}`}
          aria-pressed={sourceActive}
          onClick={onSource}
        >
          {sourceBadge}
        </button>
      ) : (
        <span className="now-playing-tag" title={`歌曲来源：${sourceBadge}`}>
          {sourceBadge}
        </span>
      )}
      <button
        type="button"
        className="now-playing-tag"
        data-active={informationActive || undefined}
        title={informationActive ? '返回歌词' : '查看歌曲详情'}
        aria-label={informationActive ? '返回歌词' : `查看歌曲详情，当前音质 ${qualityBadge}`}
        aria-pressed={informationActive}
        onClick={() => (onInformation ? onInformation() : actions.details(track))}
      >
        {qualityBadge}
      </button>
      <button
        type="button"
        className="now-playing-tag"
        data-active={lyricsActive || undefined}
        title={lyricsActive ? '返回歌词' : '选择歌词来源'}
        aria-label={lyricsActive ? '返回歌词' : `选择歌词来源，当前格式 ${lyricBadge}`}
        aria-pressed={lyricsActive}
        onClick={onLyrics}
      >
        {lyricBadge}
      </button>
    </div>
  )
}
