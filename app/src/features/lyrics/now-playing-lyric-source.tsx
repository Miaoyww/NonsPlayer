import type { Track } from '@/lib/player'
import { NowPlayingPanelCard } from '@/features/playback/now-playing/now-playing-panel-card'
import { LyricCorrectionContent } from './correction-dialog'

export function NowPlayingLyricSource({ track, onClose }: { track: Track; onClose: () => void }) {
  return (
    <NowPlayingPanelCard
      label="歌词来源与纠错"
      className="now-playing-panel-card-section"
      cardClassName="now-playing-panel-card"
    >
      <LyricCorrectionContent key={track.key} track={track} onClose={onClose} />
    </NowPlayingPanelCard>
  )
}
