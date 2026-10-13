import { TrackListSkeleton, CollectionGridSkeleton, TextSkeleton } from '@/components/music/loading'
import { DetailHeader } from '@/features/library/detail-header'
import { DetailDescription } from '@/features/library/detail-description'
import { MusicPage } from '@/components/music/music-page'
import { useCallback, useRef, useState, type KeyboardEvent } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Play, RefreshCw } from 'lucide-react'
import { nativeCall, usePlayer, type Track } from '@/lib/player'
import {
  playLibraryCollection,
  type CollectionPage,
  type CollectionTracks
} from '@/features/library/library-api'
import { useEntityDetail } from '@/features/library/music-entities'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { usePagedList } from '@/lib/use-paged-list'
import { ActionButton } from '@/components/music/action-button'
import { AlbumCard } from '@/features/library/album-card'
import { Cover } from '@/components/music/cover'
import { formatReleaseDate } from '@/components/music/music-links'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'

export default function ArtistPage({
  collection,
  onError,
  onNotice
}: {
  collection: MusicCollection
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { navigate } = useMusicNavigation()
  const [tab, setTab] = useState<'songs' | 'albums'>('songs')
  const [refresh, setRefresh] = useState(0)
  const [playing, setPlaying] = useState(false)
  const tabButtons = useRef<(HTMLButtonElement | null)[]>([])
  const { detail, error, busy } = useEntityDetail(collection, refresh)
  const artist = detail?.item ?? collection
  const player = usePlayer('index', 'queue')
  const currentKey = player.index !== null ? player.queue[player.index]?.key : undefined
  const songLoader = useCallback(
    async (offset: number) => {
      const page = await nativeCall<CollectionTracks>('artist_tracks', {
        id: collection.id,
        offset
      })
      return { items: page.tracks, more: page.more }
    },
    [collection.id, refresh]
  )
  const albumLoader = useCallback(
    async (offset: number) =>
      nativeCall<CollectionPage>('artist_albums', { id: collection.id, offset }),
    [collection.id, refresh]
  )
  const songs = usePagedList<Track>(songLoader, 100, isTauri())
  // The first album page also supplies the two latest releases; further pages load in the album tab.
  const albums = usePagedList<MusicCollection>(albumLoader, 30, isTauri())
  const latest = [...albums.items.slice(0, 30)]
    .sort((a, b) => (b.publishedAt ?? 0) - (a.publishedAt ?? 0))
    .slice(0, 2)
  async function play(item: MusicCollection) {
    if (playing) return
    setPlaying(true)
    try {
      if (await playLibraryCollection(item)) onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      onError(cause)
    } finally {
      setPlaying(false)
    }
  }
  const openAlbum = (item: MusicCollection) => navigate('album', '', item)
  function tabKey(event: KeyboardEvent, index: number) {
    const next =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? 1
          : ['ArrowLeft', 'ArrowRight'].includes(event.key)
            ? 1 - index
            : undefined
    if (next === undefined) return
    event.preventDefault()
    setTab(next === 0 ? 'songs' : 'albums')
    tabButtons.current[next]?.focus()
  }
  return (
    <MusicPage className="artist-page" aria-label="歌手详情">
      <DetailHeader>
        <Cover cover={artist.cover} className="library-detail-cover rounded-full" />
        <div className="library-detail-info">
          <h1 className="library-detail-title">{artist.name}</h1>
          <div className="library-detail-meta">
            <p className="font-semibold">歌手</p>
            <p className="mt-1 text-xs text-muted-foreground">
              {artist.trackCount} 首歌曲
              {detail?.albumCount != null && ` · ${detail.albumCount} 张专辑`}
            </p>
          </div>
          <DetailDescription description={detail?.description} />
          <div className="library-detail-actions">
            <ActionButton
              variant="secondary"
              className="library-detail-play"
              disabled={playing || !songs.items.length || !isTauri()}
              onClick={() => {
                void nativeCall('play_queue', {
                  keys: songs.items.map((track) => track.key),
                  index: 0
                }).catch(onError)
              }}
            >
              <Play aria-hidden="true" />
              播放
            </ActionButton>
          </div>
        </div>
      </DetailHeader>
      {busy && <TextSkeleton />}
      {error && (
        <div role="alert" className="mb-5 flex items-center gap-3 text-sm text-destructive">
          {error}
          <ActionButton
            variant="ghost"
            size="sm"
            onClick={() => {
              invalidateNativeCache(['music_entity_detail'])
              setRefresh((value) => value + 1)
            }}
          >
            <RefreshCw aria-hidden="true" />
            重试
          </ActionButton>
        </div>
      )}
      <section className="artist-releases" aria-labelledby="artist-releases-title">
        <h2 id="artist-releases-title" className="mb-5 text-xl font-semibold">
          最新发布
        </h2>
        <div className="artist-release-grid">
          {latest.map((album) => (
            <button
              type="button"
              className="artist-release"
              key={album.id}
              onClick={() => openAlbum(album)}
            >
              <Cover cover={album.cover} className="artist-release-cover rounded-xl" />
              <span className="min-w-0 text-left">
                <span className="block truncate text-lg font-semibold">{album.name}</span>
                <span className="mt-2 block text-sm text-muted-foreground">
                  {formatReleaseDate(album.publishedAt)}
                </span>
                <span className="mt-1 block text-xs text-muted-foreground">
                  专辑 · {album.trackCount} 首
                </span>
              </span>
            </button>
          ))}
        </div>
        {albums.busy && !latest.length && <CollectionGridSkeleton count={2} />}
        {!albums.busy && !albums.error && !latest.length && (
          <p className="py-6 text-sm text-muted-foreground">还没有发布专辑。</p>
        )}
        {albums.error && tab === 'songs' && (
          <InfiniteLoad
            more={false}
            busy={albums.busy}
            error={albums.error}
            onLoad={albums.loadMore}
          />
        )}
      </section>
      <div className="library-tabs-row">
        <div className="library-tabs" role="tablist" aria-label="歌手作品">
          {(['songs', 'albums'] as const).map((value, index) => (
            <button
              type="button"
              key={value}
              ref={(element) => {
                tabButtons.current[index] = element
              }}
              role="tab"
              id={`artist-tab-${value}`}
              aria-controls={`artist-panel-${value}`}
              aria-selected={tab === value}
              tabIndex={tab === value ? 0 : -1}
              className="library-tab"
              data-active={tab === value}
              onKeyDown={(event) => tabKey(event, index)}
              onClick={() => setTab(value)}
            >
              {value === 'songs' ? '歌曲' : '专辑'}
            </button>
          ))}
        </div>
      </div>
      <div
        role="tabpanel"
        id="artist-panel-songs"
        aria-labelledby="artist-tab-songs"
        hidden={tab !== 'songs'}
        tabIndex={0}
      >
        {songs.items.length ? (
          <TrackList
            key={`${collection.kind}:${collection.id}`}
            tracks={songs.items}
            sortable
            searchable
            hasMore={songs.more}
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
        ) : songs.busy ? (
          <TrackListSkeleton />
        ) : (
          <p className="library-empty">这里还没有歌曲。</p>
        )}
        {tab === 'songs' && (
          <InfiniteLoad
            more={songs.more}
            busy={songs.busy}
            error={songs.error}
            onLoad={songs.loadMore}
          />
        )}
      </div>
      <div
        role="tabpanel"
        id="artist-panel-albums"
        aria-labelledby="artist-tab-albums"
        hidden={tab !== 'albums'}
        tabIndex={0}
      >
        <div className="discover-playlist-grid">
          {albums.items.map((album) => (
            <AlbumCard
              key={album.id}
              item={album}
              busy={playing}
              onOpen={openAlbum}
              onPlay={(item) => void play(item)}
            />
          ))}
        </div>
        {!albums.items.length &&
          (albums.busy ? (
            <CollectionGridSkeleton />
          ) : (
            <p className="library-empty">这里还没有专辑。</p>
          ))}
        {tab === 'albums' && (
          <InfiniteLoad
            more={albums.more}
            busy={albums.busy}
            error={albums.error}
            onLoad={albums.loadMore}
          />
        )}
      </div>
    </MusicPage>
  )
}
