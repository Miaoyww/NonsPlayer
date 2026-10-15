import { ArtistLinks } from '@/components/music/music-links'
import { CurrentTrackLike } from '@/features/playback/current-track-like'
import { TrackIdentity } from '@/components/music/track-identity'
import type { PluginMenuSong } from './sdk'
import type { Track } from '@/lib/player'
import { useScope, checkScope } from './scope'
import type { PluginSong } from './types'

export function SongArtists({ song }: { song: PluginSong }) {
  useScope('ui')
  return <ArtistLinks artists={song.artists} name={song.artist} />
}

export function SongIdentity({ song }: { song: PluginMenuSong }) {
  useScope('ui')
  const track: Track = {
    ...song,
    source:
      song.source.kind === 'online'
        ? {
            kind: 'online',
            track: {
              reference: song.source.reference,
              title: song.title,
              aliases: song.aliases ?? [],
              artist: song.artist,
              artists: [],
              album: song.album,
              albumReference: null,
              durationMs: song.durationMs,
              cover: song.cover,
              associations: []
            }
          }
        : { kind: 'local', path: '', neteaseId: null }
  }
  return <TrackIdentity track={track} />
}

export function SongLikeButton({
  song,
  onError
}: {
  song: PluginSong
  onError: (error: unknown) => void
}) {
  const scope = useScope('ui')
  const track: Track = {
    ...song,
    key: JSON.stringify([song.reference.source, song.reference.kind, song.reference.id]),
    source: { kind: 'online', track: song }
  }
  return (
    <span onClickCapture={() => checkScope(scope, 'ui')}>
      <CurrentTrackLike track={track} showLabel onError={onError} />
    </span>
  )
}
