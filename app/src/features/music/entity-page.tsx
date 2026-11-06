import { invalidateNativeCache } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import ArtistPage from '@/features/library/artist/artist-page'
import AlbumPage from '@/features/library/album/album-page'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import StreamingPlaylistPage from './streaming-playlist-page'

// Details use the full source and entity reference.
interface EntityPageProps {
  collection: MusicCollection
  onError: (cause: unknown) => void
  onNotice?: (message: string) => void
}

export default function EntityPage(props: EntityPageProps) {
  const [revision, setRevision] = useState(0)
  const source = props.collection.reference!.source
  useEffect(() => {
    if (props.collection.kind === 'playlist') return
    let disposed = false
    const stops: (() => void)[] = []
    if (isTauri())
      for (const event of ['music-account-changed', 'adapters-changed', 'music-changed']) {
        void listen<string>(event, ({ payload }) => {
          if (!disposed && payload === source) {
            invalidateNativeCache(['music_query'], 'event', event)
            setRevision((value) => value + 1)
          }
        })
          .then((stop) => {
            if (disposed) stop()
            else stops.push(stop)
          })
          .catch((cause) => {
            reportCaughtError('app/src/features/music/entity-page.tsx', cause)

            if (!disposed) props.onError(cause)
          })
      }
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [source, props.collection.kind, props.onError])
  return (
    <EntityContent key={`${JSON.stringify(props.collection.reference)}:${revision}`} {...props} />
  )
}
const ignoreNotice = () => {}
function EntityContent(props: EntityPageProps) {
  if (props.collection.kind === 'artist')
    return <ArtistPage {...props} onNotice={props.onNotice ?? ignoreNotice} />
  if (props.collection.kind === 'album')
    return <AlbumPage {...props} onNotice={props.onNotice ?? ignoreNotice} />
  return <StreamingPlaylistPage {...props} />
}
