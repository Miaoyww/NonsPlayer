import { useAccount } from '@/features/account/account'
import { invalidateNativeCache, requestCache, presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { PlaylistCollectionHeader } from '@/features/library/playlist/playlist-collection-header'
import { PlaylistPageShell } from '@/features/library/playlist/playlist-page-shell'
import { PlatformPlaylistMenu } from '@/features/aggregate/platform-playlist-menu'
import { playLibraryCollection } from '@/features/library/shared/library-api'
import { TrackList } from '@/components/music/track-list'
import { BatchToolbar } from '@/components/music/batch-toolbar'
import { ActionButton } from '@/components/music/action-button'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { errorText, nativeCall, usePlayer, type Track } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { useDownloads } from '@/features/downloads/download-provider'
import { isWindows } from '@/lib/platform'
import { Download } from 'lucide-react'
import { musicCollection } from './entities'
import { musicClient } from './client'

interface StreamingPlaylistPageProps {
  collection: MusicCollection
  onError: (cause: unknown) => void
  onNotice?: (message: string) => void
}

export default function StreamingPlaylistPage(props: StreamingPlaylistPageProps) {
  const [revision, setRevision] = useState(0)
  const source = props.collection.reference!.source
  useEffect(() => {
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
            reportCaughtError('app/src/features/music/streaming-playlist-page.tsx', cause)

            if (!disposed) props.onError(cause)
          })
      }
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [source, props.onError])
  return (
    <StreamingPlaylistContent
      key={`${JSON.stringify(props.collection.reference)}:${revision}`}
      {...props}
    />
  )
}

function StreamingPlaylistContent({ collection, onError, onNotice }: StreamingPlaylistPageProps) {
  const reference = collection.reference!
  const { profile } = useAccount(reference.source)
  const cacheKey = presentationKey('entity-tracks', reference)
  const cached = requestCache.peekFresh<{ tracks: Track[]; total: number | null }>(cacheKey)
  const [tracks, setTracks] = useState<Track[]>(cached?.tracks ?? [])
  const [busy, setBusy] = useState(!cached)
  const [error, setError] = useState<string>()
  const [detail, setDetail] = useState(collection)
  const [total, setTotal] = useState(cached?.total ?? collection.trackCount)
  const [playing, setPlaying] = useState(false)
  const [batch, setBatch] = useState(false)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [downloading, setDownloading] = useState(false)
  const [description, setDescription] = useState<string>()
  const player = usePlayer('index', 'queue')
  const downloads = useDownloads()
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
      const response = await musicClient.query(reference.source, {
        operation: 'tracks',
        entity: reference,
        page: { cursor: state.cursor, limit: 100 }
      })
      if (query.current !== state || !active.current) return
      if (response.type !== 'tracks') throw new Error('音乐来源返回了错误的歌曲列表')
      const next = await musicClient.storedTracks(reference.source, response.data.items)
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
      reportCaughtError('app/src/features/music/streaming-playlist-page.tsx', cause)

      if (query.current === state && active.current) {
        if (
          typeof cause === 'object' &&
          cause !== null &&
          'code' in cause &&
          cause.code === 'staleContext'
        ) {
          state.cursor = null
          state.more = true
          setError('歌单已更新，请重试加载最新列表。')
        } else setError(errorText(cause))
      }
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
          reportCaughtError('app/src/features/music/streaming-playlist-page.tsx', cause)

          if (active.current && query.current === state) onError(cause)
        })
    } else setBusy(false)
    return () => {
      active.current = false
      query.current = { ...state }
    }
  }, [load, reference, onError])
  const toggleBatch = () => {
    setBatch((value) => !value)
    setSelected(new Set())
  }
  async function downloadSelected() {
    const items = tracks.filter(
      (track) => selected.has(track.key) && track.source.kind === 'online'
    )
    if (!items.length || downloading) return
    setDownloading(true)
    let queued = 0
    let skipped = 0
    let failed = 0
    try {
      for (const track of items) {
        try {
          const message = await downloads.start(track)
          if (message === '该歌曲已下载') skipped++
          else queued++
        } catch (cause) {
          failed++
          reportCaughtError('app/src/features/music/streaming-playlist-page.tsx', cause)
        }
      }
      const skippedText = skipped ? `，跳过 ${skipped} 首已下载` : ''
      const failedText = failed ? `，${failed} 首失败` : ''
      onNotice?.(`已加入下载 ${queued} 首${skippedText}${failedText}。`)
    } finally {
      setDownloading(false)
    }
  }
  return (
    <PlaylistPageShell
      hasMore={query.current.more}
      searchable={!batch}
      header={
        <PlaylistCollectionHeader
          collection={detail}
          description={description}
          actions={
            <PlatformPlaylistMenu
              key={`${reference.source}:${profile?.reference?.id}:${reference.id}`}
              collection={detail}
              account={profile?.reference}
              onError={onError}
              batch={batch}
              onBatch={isWindows() ? toggleBatch : undefined}
            />
          }
          total={total || detail.trackCount}
          busy={playing}
          disabled={playing || !isTauri() || !tracks.length}
          onPlay={() => {
            if (playing) return
            setPlaying(true)
            void playLibraryCollection(detail, tracks[0]?.key)
              .then((truncated) => {
                if (truncated) onNotice?.('已将前 1000 首歌曲加入播放队列。')
              })
              .catch(onError)
              .finally(() => setPlaying(false))
          }}
        />
      }
    >
      {batch && (
        <BatchToolbar selected={selected.size} busy={busy || downloading} onDone={toggleBatch}>
          <ActionButton
            variant="outline"
            disabled={busy || downloading || !selected.size}
            onClick={() => void downloadSelected()}
          >
            <Download aria-hidden="true" />
            下载
          </ActionButton>
        </BatchToolbar>
      )}
      <TrackList
        tracks={tracks}
        hasMore={query.current.more}
        loading={busy && tracks.length === 0 && !cached}
        busy={busy || downloading}
        showLikes
        searchable={!batch}
        sortable
        currentKey={player.index === null ? undefined : player.queue[player.index]?.key}
        selection={
          batch
            ? {
                keys: selected,
                onToggle: (key, checked) =>
                  setSelected((old) => {
                    const next = new Set(old)
                    if (checked) next.add(key)
                    else next.delete(key)
                    return next
                  }),
                onToggleAll: (keys, checked) =>
                  setSelected((old) => {
                    const next = new Set(old)
                    for (const key of keys) {
                      if (checked) next.add(key)
                      else next.delete(key)
                    }
                    return next
                  })
              }
            : undefined
        }
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
    </PlaylistPageShell>
  )
}
