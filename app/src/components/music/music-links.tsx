import { reportCaughtError } from '@/lib/diagnostics'
import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { readTrackInformation } from '@/features/music/queries'
import { Fragment, useState, type MouseEvent } from 'react'
import { errorText, type MusicCredit, type Track } from '@/lib/player'
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
        <Fragment key={`${artist.id ?? artist.name}:${index}`}>
          {index > 0 && <span className="music-credit-separator"> {separator} </span>}
          {artist.id || artist.reference ? (
            <button
              type="button"
              className="music-entity-link"
              onClick={(event: MouseEvent) => {
                event.stopPropagation()
                navigate('artist', '', {
                  id: artist.reference?.id ?? artist.id ?? 0,
                  reference: artist.reference ?? undefined,
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
  id,
  reference,
  name,
  cover = ''
}: {
  id?: number | null
  reference?: import('@/features/music/types').EntityRef | null
  name: string
  cover?: import('@/features/music/types').CoverInput
}) {
  const { navigate } = useMusicNavigation()
  return id || reference ? (
    <button
      type="button"
      className="music-entity-link"
      onClick={(event) => {
        event.stopPropagation()
        navigate('album', '', {
          id: reference?.id ?? id ?? 0,
          reference: reference ?? undefined,
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
  const { navigate } = useMusicNavigation()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  if (track.source.kind === 'local') return <LocalTrackArtists track={track} />
  if (track.source.kind === 'online')
    return <ArtistLinks artists={track.source.track.artists} name={track.artist} />
  const onlineId = track.source.id
  if (track.artists?.length || !onlineId)
    return <ArtistLinks artists={track.artists} name={track.artist} />
  // Older persisted tracks have names only. Resolve their credits on demand, never per row on mount.
  return (
    <span>
      {track.artist.split(' / ').map((name, index) => (
        <Fragment key={`${name}:${index}`}>
          {index > 0 && <span className="music-credit-separator"> / </span>}
          <button
            type="button"
            className="music-entity-link"
            disabled={busy}
            title={error ?? name}
            onClick={(event) => {
              event.stopPropagation()
              if (busy) return
              setBusy(true)
              setError(undefined)
              void readTrackInformation(track)
                .then((value) => {
                  const artist = value.artists.find((artist) => artist.name === name)
                  if (!artist?.reference) throw new Error('暂无可跳转的歌手信息')
                  navigate('artist', '', {
                    id: artist.reference.id,
                    reference: artist.reference,
                    kind: 'artist',
                    name: artist.name,
                    cover: '',
                    subtitle: '',
                    trackCount: 0
                  })
                })
                .catch((cause) => {
                  reportCaughtError('app/src/components/music/music-links.tsx', cause)
                  return setError(errorText(cause))
                })
                .finally(() => setBusy(false))
            }}
          >
            {name}
          </button>
        </Fragment>
      ))}
      {error && (
        <span role="status" className="sr-only">
          {error}
        </span>
      )}
    </span>
  )
}

export function TrackAlbum({ track }: { track: Track }) {
  const { navigate } = useMusicNavigation()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
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
  const onlineId = track.source.id
  if (track.albumId || !onlineId)
    return <AlbumLink id={track.albumId} name={track.album} cover={track.cover} />
  return (
    <span>
      <button
        type="button"
        className="music-entity-link"
        disabled={busy}
        title={error ?? track.album}
        onClick={(event) => {
          event.stopPropagation()
          if (busy) return
          setBusy(true)
          setError(undefined)
          void readTrackInformation(track)
            .then((value) => {
              if (!value.albumReference) throw new Error('暂无可跳转的专辑信息')
              navigate('album', '', {
                id: value.albumReference.id,
                reference: value.albumReference,
                kind: 'album',
                name: track.album,
                cover: track.cover,
                subtitle: '',
                trackCount: 0
              })
            })
            .catch((cause) => {
              reportCaughtError('app/src/components/music/music-links.tsx', cause)
              return setError(errorText(cause))
            })
            .finally(() => setBusy(false))
        }}
      >
        {track.album}
      </button>
      {error && (
        <span role="status" className="sr-only">
          {error}
        </span>
      )}
    </span>
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
