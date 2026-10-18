import { requestCache, presentationKey, invalidateNativeCache } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { MusicPage } from '@/components/music/music-page'
import { CollectionHeader } from '@/features/library/collection-header'
import { CollectionTabs } from '@/features/library/collection-tabs'
import ArtistPage from '@/features/library/artist-page'
import AlbumPage from '@/features/library/album-page'
import { playLibraryCollection } from '@/features/library/library-api'
import { musicCollection } from './entities'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { errorText, nativeCall, usePlayer, type Track } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { musicClient } from './client'

// Reference-based details never enter the numeric Netease compatibility routes.
interface EntityPageProps {
  collection: MusicCollection
  onError: (cause: unknown) => void
  onNotice?: (message: string) => void
}

export default function EntityPage(props: EntityPageProps) {
  const [revision, setRevision] = useState(0)
  const source = props.collection.reference!.source
  useEffect(() => {
    let disposed = false
    const stops: (() => void)[] = []
    if (isTauri())
      for (const event of ['music-account-changed', 'adapters-changed', 'music-changed']) {
        void listen<string>(event, ({ payload }) => {
          if (!disposed && payload === source) {
            invalidateNativeCache(['music_query'])
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
  }, [source, props.onError])
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
  return <EntityResults {...props} />
}

function EntityResults({ collection, onError, onNotice }: EntityPageProps) {
  const reference = collection.reference!
  const cacheKey = presentationKey('entity-tracks', reference)
  const cached = requestCache.peekFresh<{ tracks: Track[]; total: number | null }>(cacheKey)
  const [tracks, setTracks] = useState<Track[]>(cached?.tracks ?? [])
  const [busy, setBusy] = useState(!cached)
  const [error, setError] = useState<string>()
  const [detail, setDetail] = useState(collection)
  const [total, setTotal] = useState(cached?.total ?? collection.trackCount)
  const [playing, setPlaying] = useState(false)
  const [description, setDescription] = useState<string>()
  const player = usePlayer('index', 'queue')
  const active = useRef(false)
  const query = useRef({ cursor: null as string | null, more: true, busy: false })
  const load = useCallback(async () => {
    const state = query.current
    if (!active.current || state.busy || !state.more) return
    state.busy = true
    const revision = requestCache.revision
    setBusy(!cached || tracks.length > 0)
    setError(undefined)
    try {
      const response = await musicClient.query(
        reference.source,
        reference.kind === 'artist'
          ? {
              operation: 'artistTracks',
              artist: reference,
              page: { cursor: state.cursor, limit: 100 }
            }
          : { operation: 'tracks', entity: reference, page: { cursor: state.cursor, limit: 100 } }
      )
      if (query.current !== state || !active.current) return
      if (response.type !== 'tracks') throw new Error('音乐来源返回了错误的歌曲列表')
      const next = await musicClient.queueTracks(reference.source, response.data.items)
      if (query.current !== state || !active.current) return
      if (state.cursor === null && requestCache.revision === revision)
        requestCache.set(cacheKey, { tracks: next, total: response.data.total })
      const firstPage = state.cursor === null
      setTracks((previous) => {
        if (firstPage) return next
        const keys = new Set(previous.map((track) => track.key))
        return [
          ...previous,
          ...next.filter((track) => !keys.has(track.key) && !!keys.add(track.key))
        ]
      })
      state.cursor = response.data.nextCursor
      if (response.data.total !== null) setTotal(response.data.total)
      state.more = state.cursor !== null
    } catch (cause) {
      reportCaughtError('app/src/features/music/entity-page.tsx', cause)

      if (query.current === state && active.current) setError(errorText(cause))
    } finally {
      if (query.current === state && active.current) {
        state.busy = false
        setBusy(false)
      }
    }
  }, [reference])
  useEffect(() => {
    active.current = true
    const state = { cursor: null, more: true, busy: false }
    query.current = state
    if (isTauri()) {
      void load()
      void musicClient
        .query(reference.source, { operation: 'detail', entity: reference })
        .then((response) => {
          if (!active.current || query.current !== state) return
          if (response.type === 'detail') {
            setDetail(musicCollection(response.data.item))
            setDescription(response.data.description ?? undefined)
          } else if (response.type === 'entity') setDetail(musicCollection(response.data))
        })
        .catch((cause) => {
          reportCaughtError('app/src/features/music/entity-page.tsx', cause)

          if (active.current && query.current === state) onError(cause)
        })
    } else setBusy(false)
    return () => {
      active.current = false
      query.current = { ...state }
    }
  }, [load, reference, onError])
  return (
    <MusicPage aria-label="音乐详情">
      <CollectionHeader
        collection={detail}
        description={description}
        total={total || detail.trackCount}
        busy={playing}
        disabled={playing || !isTauri() || !tracks.length}
        onPlay={() => {
          if (playing) return
          setPlaying(true)
          void playLibraryCollection(detail)
            .then((truncated) => {
              if (truncated) onNotice?.('已将前 1000 首歌曲加入播放队列。')
            })
            .catch(onError)
            .finally(() => setPlaying(false))
        }}
      />
      <CollectionTabs collection={detail} hasMore={query.current.more}>
        <TrackList
          tracks={tracks}
          hasMore={query.current.more}
          loading={busy && tracks.length === 0 && !cached}
          busy={busy}
          showLikes
          searchable
          sortable
          currentKey={player.index === null ? undefined : player.queue[player.index]?.key}
          onPlay={(index, items) => {
            const start = Math.max(0, index - 999)
            void nativeCall('play_queue', {
              keys: items.slice(start, start + 1000).map((track) => track.key),
              index: index - start
            }).catch(onError)
          }}
          onAppend={(track) => {
            void nativeCall('append_queue', { keys: [track.key] }).catch(onError)
          }}
        />
      </CollectionTabs>
      <InfiniteLoad
        more={query.current.more}
        busy={busy}
        error={error}
        onLoad={() => {
          void load()
        }}
      />
    </MusicPage>
  )
}
