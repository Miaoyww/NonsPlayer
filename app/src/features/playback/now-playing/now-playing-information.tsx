import { SongInformationContent } from '@/components/music/song-information'
import { useSongActions } from '@/components/music/song-actions'
import type { Track } from '@/lib/player'

export function NowPlayingInformation({ track }: { track?: Track }) {
  const { songLink } = useSongActions()
  if (!track) return null
  return (
    <section className="now-playing-information" aria-label="歌曲信息">
      <article className="now-playing-information-card">
        <SongInformationContent key={track.key} track={track} songLink={songLink} inlineError />
      </article>
    </section>
  )
}
