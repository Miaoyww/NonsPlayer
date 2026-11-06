import { SongInformationContent } from '@/components/music/song-information'
import { useSongActions } from '@/components/music/song-actions'
import type { Track } from '@/lib/player'
import { NowPlayingPanelCard } from './now-playing-panel-card'

export function NowPlayingInformation({ track }: { track?: Track }) {
  const { songLink } = useSongActions()
  if (!track) return null
  return (
    <NowPlayingPanelCard label="歌曲信息">
      <SongInformationContent key={track.key} track={track} songLink={songLink} inlineError />
    </NowPlayingPanelCard>
  )
}
