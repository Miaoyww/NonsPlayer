import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { Fragment, type MouseEvent } from 'react'
import { type MusicCredit, type Track } from '@/lib/player'
import { useMusicNavigation } from '@/features/workspace/music-navigation'

export function ArtistLinks({
  artists,
  name = '',
  separator = '/'
}: {
  artists?: MusicCredit[]
  name?: string
  separator?: string
}) {
  const { navigate } = useMusicNavigation()
  const credits = artists?.length
    ? artists
    : name.split(' / ').map((name) => ({ name }) as MusicCredit)
  return (
    <span className="music-credits">
      {credits.map((artist, index) => (
        <Fragment key={`${artist.reference?.id ?? artist.name}:${index}`}>
          {index > 0 && <span className="music-credit-separator"> {separator} </span>}
          {artist.reference ? (
            <button
              type="button"
              className="music-entity-link"
              onClick={(event: MouseEvent) => {
                event.stopPropagation()
                navigate('artist', '', {
                  id: artist.reference!.id,
                  reference: artist.reference!,
                  kind: 'artist',
                  name: artist.name,
                  cover: '',
                  subtitle: '',
                  trackCount: 0
                })
              }}
            >
              {artist.name}
            </button>
          ) : (
            <span>{artist.name}</span>
          )}
        </Fragment>
      ))}
    </span>
  )
}

export function AlbumLink({
  reference,
  name,
  cover = ''
}: {
  reference?: import('@/features/music/types').EntityRef | null
  name: string
  cover?: import('@/features/music/types').CoverInput
}) {
  const { navigate } = useMusicNavigation()
  return reference ? (
    <button
      type="button"
      className="music-entity-link"
      onClick={(event) => {
        event.stopPropagation()
        navigate('album', '', {
          id: reference.id,
          reference,
          kind: 'album',
          name,
          cover,
          subtitle: '',
          trackCount: 0
        })
      }}
    >
      {name}
    </button>
  ) : (
    <span>{name}</span>
  )
}

export function TrackArtists({ track }: { track: Track }) {
  if (track.source.kind === 'local') return <LocalTrackArtists track={track} />
  return <ArtistLinks artists={track.source.track.artists} name={track.artist} />
}

export function TrackAlbum({ track }: { track: Track }) {
  const { navigate } = useMusicNavigation()
  if (track.source.kind === 'online')
    return (
      <AlbumLink
        reference={track.source.track.albumReference}
        name={track.album}
        cover={track.cover}
      />
    )
  if (track.source.kind === 'local')
    return (
      <button
        type="button"
        className="music-entity-link"
        onClick={(event) => {
          event.stopPropagation()
          navigate('local-album', '', {
            id: 0,
            localId: track.key,
            reference: null,
            kind: 'album',
            name: track.album,
            cover: track.cover,
            subtitle: track.artist,
            trackCount: 0
          })
        }}
      >
        {track.album}
      </button>
    )
}

export function formatReleaseDate(value?: number | null) {
  return value
    ? new Date(value).toLocaleDateString('sv-SE', { timeZone: 'Asia/Shanghai' })
    : '发行日期未知'
}

function LocalTrackArtists({ track }: { track: Track }) {
  const { navigate } = useMusicNavigation()
  const { options } = useLocalPreferences()
  let names = [track.artist]
  for (const separator of options.artistSeparators)
    names = names.flatMap((name) => name.split(separator))
  names = [...new Set(names.map((name) => name.trim()).filter(Boolean))]
  return (
    <span className="music-credits">
      {names.map((name, index) => (
        <Fragment key={name}>
          {index > 0 && <span className="music-credit-separator"> / </span>}
          <button
            type="button"
            className="music-entity-link"
            onClick={(event) => {
              event.stopPropagation()
              navigate('local-artist', '', {
                id: 0,
                localId: name,
                reference: null,
                kind: 'artist',
                name,
                cover: track.cover,
                subtitle: '',
                trackCount: 0
              })
            }}
          >
            {name}
          </button>
        </Fragment>
      ))}
    </span>
  )
}
