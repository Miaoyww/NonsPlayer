import type { Track } from '@/lib/player'
import { NowPlayingPanelCard } from './now-playing-panel-card'
import { SongSourceContent } from './song-source-dialog'

export function NowPlayingSource({ track, onClose }: { track: Track; onClose: () => void }) {
  return (
    <NowPlayingPanelCard
      label="歌曲来源"
      className="now-playing-panel-card-section"
      cardClassName="now-playing-panel-card"
    >
      <SongSourceContent key={track.key} track={track} onClose={onClose} />
    </NowPlayingPanelCard>
  )
}
