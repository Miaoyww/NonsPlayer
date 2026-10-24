import { presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { TrackListSkeleton } from '@/components/music/loading'
import { tracksForCollection } from '@/features/music/queries'
import { CollectionContent } from '@/features/library/collection-content'
import { CollectionHeader } from '@/features/library/collection-header'
import { MusicPage } from '@/components/music/music-page'
// Independent album detail, adapted from the playlist detail in music-library.tsx.
// Layout originally adapted from YesPlayMusic src/views/library.vue.
// Copyright (c) 2020-2023 qier222, MIT. See notices/YesPlayMusic-LICENSE.txt.
import { useCallback, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { RefreshCw } from 'lucide-react'
import { nativeCall, usePlayer, type Track } from '@/lib/player'
import { playLibraryCollection, type CollectionTracks } from '@/features/library/library-api'
import { useEntityDetail } from '@/features/library/music-entities'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { usePagedList } from '@/lib/use-paged-list'
import { ActionButton } from '@/components/music/action-button'
import { formatReleaseDate } from '@/components/music/music-links'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export default function AlbumPage({
  collection,
  onError,
  onNotice
}: {
  collection: MusicCollection
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const [refresh, setRefresh] = useState(0)
  const [playing, setPlaying] = useState(false)
  const { detail, error, busy } = useEntityDetail(collection, refresh)
  const album = detail?.item ?? collection
  const player = usePlayer('index', 'queue')
  const currentKey = player.index !== null ? player.queue[player.index]?.key : undefined
  const loader = useCallback(
    async (offset: number) => {
      const value = await tracksForCollection(collection, offset)
      return { items: value.tracks, more: value.more, metadata: value }
    },
    [collection.id, refresh]
  )
  const list = usePagedList<Track, CollectionTracks>(loader, 100, isTauri(), {
    cacheKey: presentationKey('album-tracks', collection.reference ?? collection)
  })
  async function play() {
    if (playing) return
    setPlaying(true)
    try {
      if (await playLibraryCollection(album, list.items[0]?.key))
        onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      reportCaughtError('app/src/features/library/album-page.tsx', cause)

      onError(cause)
    } finally {
      setPlaying(false)
    }
  }
  return (
    <MusicPage aria-label="专辑详情">
      <CollectionContent key={`${collection.kind}:${collection.id}`} hasMore={list.more}>
        <CollectionHeader
          collection={album}
          description={detail?.description}
          total={list.metadata?.total ?? album.trackCount}
          metadata={<span>{formatReleaseDate(album.publishedAt)}</span>}
          busy={playing}
          disabled={playing || !isTauri() || !list.items.length}
          onPlay={() => void play()}
        />
        {error && (
          <div className="mb-5 flex items-center gap-3 text-sm text-destructive">
            <ErrorNotice error={error} />
            <ActionButton
              variant="ghost"
              size="sm"
              onClick={() => {
                invalidateNativeCache(['music_query'])
                setRefresh((value) => value + 1)
              }}
            >
              <RefreshCw aria-hidden="true" />
              重试
            </ActionButton>
          </div>
        )}
        {(busy || list.busy) && !list.items.length ? (
          <TrackListSkeleton />
        ) : list.items.length ? (
          <TrackList
            key={`${collection.kind}:${collection.id}`}
            tracks={list.items}
            sortable
            searchable
            hasMore={list.more}
            currentKey={currentKey}
            busy={playing}
            onPlay={(index, tracks) => {
              void nativeCall('play_queue', {
                keys: tracks.map((track) => track.key),
                index
              }).catch(onError)
            }}
            onAppend={(track) => {
              void nativeCall('append_queue', { keys: [track.key] })
                .then(() => onNotice(`已将「${track.title}」设为下一首播放。`))
                .catch(onError)
            }}
          />
        ) : (
          <p className="library-empty">这里还没有歌曲。</p>
        )}
        <InfiniteLoad more={list.more} busy={list.busy} error={list.error} onLoad={list.loadMore} />
      </CollectionContent>
    </MusicPage>
  )
}
