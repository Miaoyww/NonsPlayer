import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { errorText, nativeCall, usePlayer, type Track } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { musicClient } from './client'

// Reference-based details never enter the numeric Netease compatibility routes.
interface EntityPageProps {
  collection: MusicCollection
  onError: (cause: unknown) => void
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
          if (!disposed && payload === source) setRevision((value) => value + 1)
        })
          .then((stop) => {
            if (disposed) stop()
            else stops.push(stop)
          })
          .catch((cause) => {
            if (!disposed) props.onError(cause)
          })
      }
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [source, props.onError])
  return (
    <EntityResults key={`${JSON.stringify(props.collection.reference)}:${revision}`} {...props} />
  )
}

function EntityResults({ collection, onError }: EntityPageProps) {
  const reference = collection.reference!
  const [tracks, setTracks] = useState<Track[]>([])
  const [busy, setBusy] = useState(true)
  const [error, setError] = useState<string>()
  const [title, setTitle] = useState(collection.name)
  const [description, setDescription] = useState<string>()
  const player = usePlayer('index', 'queue')
  const active = useRef(false)
  const query = useRef({ cursor: null as string | null, more: true, busy: false })
  const load = useCallback(async () => {
    const state = query.current
    if (!active.current || state.busy || !state.more) return
    state.busy = true
    setBusy(true)
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
      setTracks((previous) => {
        const keys = new Set(previous.map((track) => track.key))
        return [
          ...previous,
          ...next.filter((track) => !keys.has(track.key) && !!keys.add(track.key))
        ]
      })
      state.cursor = response.data.nextCursor
      state.more = state.cursor !== null
    } catch (cause) {
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
            setTitle(response.data.item.name)
            setDescription(response.data.description ?? undefined)
          } else if (response.type === 'entity') setTitle(response.data.name)
        })
        .catch((cause) => {
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
      <MusicPageHeader title={title}>
        <p className="text-sm text-muted-foreground">来源：{reference.source}</p>
      </MusicPageHeader>
      {description && (
        <p className="mb-4 whitespace-pre-wrap text-sm text-muted-foreground">{description}</p>
      )}
      <TrackList
        tracks={tracks}
        hasMore={query.current.more}
        loading={busy && tracks.length === 0}
        busy={busy}
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
