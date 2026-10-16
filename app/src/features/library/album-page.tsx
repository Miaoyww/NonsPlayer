import { ErrorNotice } from '@/components/error-notice'
import { TrackListSkeleton } from '@/components/music/loading'
import { tracksForCollection } from '@/features/music/queries'
import { CollectionTabs } from '@/features/library/collection-tabs'
import { DetailHeader } from '@/features/library/detail-header'
import { DetailDescription } from '@/features/library/detail-description'
import { MusicPage } from '@/components/music/music-page'
// Independent album detail, adapted from the playlist detail in music-library.tsx.
// Layout originally adapted from YesPlayMusic src/views/library.vue.
// Copyright (c) 2020-2023 qier222, MIT. See notices/YesPlayMusic-LICENSE.txt.
import { useCallback, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Play, RefreshCw } from 'lucide-react'
import { nativeCall, usePlayer, type Track } from '@/lib/player'
import { playLibraryCollection, type CollectionTracks } from '@/features/library/library-api'
import { useEntityDetail } from '@/features/library/music-entities'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { usePagedList } from '@/lib/use-paged-list'
import { ActionButton } from '@/components/music/action-button'
import { ArtistLinks, formatReleaseDate } from '@/components/music/music-links'
import { Cover } from '@/components/music/cover'
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
  const list = usePagedList<Track, CollectionTracks>(loader, 100, isTauri())
  async function play() {
    if (playing) return
    setPlaying(true)
    try {
      if (await playLibraryCollection(album)) onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      onError(cause)
    } finally {
      setPlaying(false)
    }
  }
  return (
    <MusicPage aria-label="专辑详情">
      <DetailHeader>
        <Cover size="large" cover={album.cover} className="library-detail-cover rounded-xl" />
        <div className="library-detail-info">
          <h1 className="library-detail-title">{album.name}</h1>
          <div className="library-detail-meta">
            <p className="font-semibold">
              专辑 · <ArtistLinks artists={album.artists} name={album.subtitle} />
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              {list.metadata?.total ?? album.trackCount} 首 · {formatReleaseDate(album.publishedAt)}
            </p>
          </div>
          <DetailDescription description={detail?.description} />
          <div className="library-detail-actions">
            <ActionButton
              variant="secondary"
              className="library-detail-play"
              disabled={playing || !isTauri() || !list.items.length}
              onClick={() => void play()}
            >
              <Play aria-hidden="true" />
              {playing ? '正在加载…' : '播放'}
            </ActionButton>
          </div>
        </div>
      </DetailHeader>
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
      <CollectionTabs
        key={`${collection.kind}:${collection.id}`}
        collection={album}
        hasMore={list.more}
      >
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
      </CollectionTabs>
    </MusicPage>
  )
}
