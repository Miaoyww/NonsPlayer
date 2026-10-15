import { FindSongGrid } from '@/components/music/list-find'
import { onlineReference, collectionReference } from '@/features/music/legacy-reference'
import { musicClient } from '@/features/music/client'
import { Skeleton } from '@/components/ui/skeleton'
import {
  TrackListSkeleton,
  CollectionGridSkeleton,
  SongGridSkeleton,
  TextSkeleton
} from '@/components/music/loading'
import AccountLoginPage from '@/features/account/account-login-page'
import { CollectionTabs } from '@/features/library/collection-tabs'
import { MusicPage } from '@/components/music/music-page'
import { CollectionContextMenu, useCollectionActions } from '@/features/library/collection-actions'
import { TrackArtists } from '@/components/music/music-links'
import { AlbumCard } from '@/features/library/album-card'
import { PlaylistCard } from '@/features/library/playlist-card'
import { CollectionHeader } from '@/features/library/collection-header'
import { SongContextMenu } from '@/components/music/song-actions'
import { TrackTitle } from '@/components/music/track-title'
// Layout and interaction adapted from YesPlayMusic src/views/library.vue.
// Copyright (c) 2020-2023 qier222, MIT. See notices/YesPlayMusic-LICENSE.txt.
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Play, RefreshCw, UserRound } from 'lucide-react'
import { errorText, nativeCall, usePlayer, type Track } from '@/lib/player'
import {
  getLibraryCollections,
  getLibraryHistory,
  getLibraryTracks,
  getMusicLibrary,
  peekMusicLibrary,
  peekLibraryCollections,
  invalidateMusicLibrary,
  playLibraryCollection,
  type CollectionTracks,
  type LibrarySummary,
  type LibraryTab,
  type PlaylistFilter
} from '@/features/library/library-api'
import { CreatePlaylist } from '@/features/library/create-playlist'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import { ActionButton } from '@/components/music/action-button'
import { useAccount } from '@/features/account/account'
import { Cover } from '@/components/music/cover'
import { PlayingIndicator } from '@/components/music/playing-indicator'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { usePagedList } from '@/lib/use-paged-list'
import { LyricExcerpt } from '@/features/lyrics/lyric-excerpt'
import { reconcileTracks } from '@/features/library/reconcile-tracks'

const filters = [
  { value: 'all', label: '全部歌单' },
  { value: 'mine', label: '创建的歌单' },
  { value: 'liked', label: '收藏的歌单' }
]
const tabs = [
  { value: 'playlist', label: '歌单' },
  { value: 'album', label: '专辑' },
  { value: 'artist', label: '艺人' },
  { value: 'history', label: '听歌记录' }
] as const

function CollectionCards({
  items,
  busy,
  onOpen,
  onPlay
}: {
  items: MusicCollection[]
  busy: boolean
  onOpen: (item: MusicCollection) => void
  onPlay: (item: MusicCollection) => void
}) {
  return (
    <div className={items[0]?.kind === 'artist' ? 'library-cover-grid' : 'discover-playlist-grid'}>
      {items.map((item) =>
        item.kind === 'album' ? (
          <AlbumCard
            key={`${item.kind}:${item.id}`}
            item={item}
            busy={busy}
            onOpen={onOpen}
            onPlay={onPlay}
          />
        ) : (
          <PlaylistCard
            key={`${item.kind}:${item.id}`}
            item={item}
            busy={busy}
            onOpen={onOpen}
            onPlay={onPlay}
          />
        )
      )}
    </div>
  )
}

export default function MusicLibrary({
  onError,
  onNotice,
  readOnly = false
}: {
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
  readOnly?: boolean
}) {
  const {
    profile,
    loading: accountLoading,
    error: accountError,
    likesRevision,
    reloadLikes
  } = useAccount()
  const account = profile?.reference
  const { revision: collectionRevision } = useCollectionActions()
  const { page, navigate } = useMusicNavigation()
  const player = usePlayer('index', 'queue')
  const currentKey = player.index !== null ? player.queue[player.index]?.key : undefined
  const [summaryState, setSummary] = useState<LibrarySummary | undefined>(() =>
    profile ? peekMusicLibrary(profile.userId, profile.reference?.source) : undefined
  )
  const summary =
    summaryState?.profile.userId === profile?.userId &&
    summaryState?.profile.reference?.source === profile?.reference?.source
      ? summaryState
      : undefined
  const [summaryBusy, setSummaryBusy] = useState(false)
  const [summaryError, setSummaryError] = useState<string>()
  const [tab, setTab] = useState<LibraryTab>('playlist')
  const [filter, setFilter] = useState<PlaylistFilter>('all')
  const [week, setWeek] = useState(true)
  const collection = page.collection
  const [playing, setPlaying] = useState(false)
  const playRequest = useRef(0)
  const [refresh, setRefresh] = useState(0)
  const showingDetail = page.view === 'collection' && !!collection

  useEffect(() => {
    let disposed = false
    setSummaryError(undefined)
    setSummaryBusy(false)
    if (!account || !isTauri()) {
      setSummary(undefined)
      return
    }
    if (showingDetail) return
    const cached = peekMusicLibrary(account.id, account.source)
    const current = cached ?? (summary?.profile.userId === account.id ? summary : undefined)
    setSummary(current)
    setSummaryBusy(!current)
    void getMusicLibrary(account.id)
      .then((value) => {
        if (!disposed)
          setSummary((previous) => ({
            ...value,
            likedTracks: reconcileTracks(previous?.likedTracks ?? [], value.likedTracks)
          }))
      })
      .catch((cause) => {
        if (!disposed) setSummaryError(errorText(cause))
      })
      .finally(() => {
        if (!disposed) setSummaryBusy(false)
      })
    return () => {
      disposed = true
    }
  }, [account, refresh, likesRevision, collectionRevision, showingDetail])

  const listLoader = useCallback(
    async (offset: number) => {
      if (tab === 'history') {
        const value = await getLibraryHistory(week, offset)
        return { items: value.tracks, more: value.more }
      }
      const value = await getLibraryCollections(tab, offset, filter, account?.id)
      return { items: value.items, more: value.more }
    },
    [account, tab, filter, week, refresh, showingDetail, collectionRevision]
  )
  const list = usePagedList<Track | MusicCollection>(
    listLoader,
    tab === 'history' ? 100 : 30,
    !!profile && isTauri() && !showingDetail
  )
  const preview =
    profile &&
    tab === 'playlist' &&
    filter === 'all' &&
    !list.items.length &&
    (list.busy || list.error)
      ? peekLibraryCollections(profile.userId, profile.reference?.source)
      : undefined
  const collections = {
    items: (preview?.items ?? list.items) as MusicCollection[],
    more: list.more
  }
  const history = { tracks: list.items as Track[], more: list.more }
  const listBusy = list.busy
  const detailLoader = useCallback(
    async (offset: number) => {
      const value = await getLibraryTracks(collection!, offset, account!.id)
      return { items: value.tracks, more: value.more, metadata: value }
    },
    [collection, account, showingDetail]
  )
  const detailList = usePagedList<Track, CollectionTracks>(
    detailLoader,
    100,
    !!collection && !!profile && showingDetail && isTauri(),
    { refreshKey: `${refresh}:${likesRevision}:${collectionRevision}`, reconcile: reconcileTracks }
  )
  const detail = {
    ...detailList.metadata,
    tracks: detailList.items,
    total: detailList.metadata?.total ?? 0,
    more: detailList.more
  }
  const detailBusy = detailList.busy

  function openCollection(item: MusicCollection) {
    navigate('collection', '', item)
  }
  async function playCollection(item: MusicCollection, key?: string) {
    if (playing && !key) return
    const request = ++playRequest.current
    setPlaying(true)
    try {
      const truncated = await playLibraryCollection(item, key)
      if (request === playRequest.current && truncated)
        onNotice('已将前 1000 首歌曲加入播放队列。其余歌曲可在收藏详情中继续浏览并播放。')
    } catch (cause) {
      if (request === playRequest.current) onError(cause)
    } finally {
      if (request === playRequest.current) setPlaying(false)
    }
  }
  function playPage(tracks: Track[], index: number) {
    void nativeCall('play_queue', { keys: tracks.map((track) => track.key), index }).catch(onError)
  }
  function append(track: Track) {
    void nativeCall('append_queue', { keys: [track.key] })
      .then(() => onNotice(`已将「${track.title}」设为下一首播放。`))
      .catch(onError)
  }
  async function removeFromPlaylist(item: MusicCollection, track: Track) {
    const reference = onlineReference(track),
      playlist = collectionReference(item)
    if (!reference || reference.source !== playlist.source) return
    await musicClient.write(playlist.source, {
      operation: 'playlistTrack',
      playlist,
      track: reference,
      add: false
    })
    reloadLikes()
    setRefresh((value) => value + 1)
  }
  const liked = summary?.likedPlaylist
  const featuredTracks = summary?.likedTracks.slice(0, 12) ?? []
  const retry = (
    <ActionButton
      size="sm"
      variant="ghost"
      onClick={() => {
        invalidateMusicLibrary()
        setRefresh((value) => value + 1)
      }}
    >
      <RefreshCw aria-hidden="true" />
      重试
    </ActionButton>
  )

  if (!showingDetail && !profile && !accountLoading) return <AccountLoginPage />

  return (
    <MusicPage aria-label="音乐库">
      {showingDetail ? (
        <>
          <CollectionHeader
            key={`header:${collection.kind}:${collection.id}`}
            collection={collection}
            description={detail.description}
            total={detailList.metadata?.total ?? collection.trackCount}
            busy={playing}
            disabled={playing || !profile || detailBusy || !detail.tracks.length}
            onPlay={() => void playCollection(collection)}
          />
          <CollectionTabs
            key={`${collection.kind}:${collection.id}`}
            collection={collection}
            hasMore={detail.more}
          >
            {!profile ? (
              <div className="library-empty">
                <p>登录对应音乐来源后查看这个收藏。</p>
                <ActionButton variant="secondary" onClick={() => navigate('accounts')}>
                  登录账号
                </ActionButton>
              </div>
            ) : detailBusy || detail.tracks.length ? (
              <TrackList
                key={`${collection.kind}:${collection.id}`}
                tracks={detail.tracks}
                loading={detailBusy && !detail.tracks.length}
                showLikes
                sortable
                searchable
                hasMore={detail.more}
                currentKey={currentKey}
                busy={playing}
                onPlay={(index, tracks) => playPage(tracks, index)}
                onAppend={append}
                removeLabel={collection.kind === 'playlist' ? '从歌单删除' : '从列表删除'}
                onRemove={
                  !readOnly &&
                  collection.kind === 'playlist' &&
                  collection.creatorId === profile.userId
                    ? (track) => removeFromPlaylist(collection, track)
                    : undefined
                }
              />
            ) : (
              <p className="library-empty">这里还没有歌曲。</p>
            )}
            {profile && (
              <InfiniteLoad
                more={detail.more}
                busy={detailBusy}
                error={detailList.error}
                onLoad={detailList.loadMore}
              />
            )}
          </CollectionTabs>
        </>
      ) : (
        <>
          <header className="library-profile">
            {profile?.avatarUrl ? (
              <Cover
                size="small"
                cover={profile.avatarUrl}
                className="library-avatar rounded-full"
              />
            ) : (
              <UserRound className="library-avatar bg-muted p-2" aria-hidden="true" />
            )}
            <h1 className="library-heading">
              {profile ? `${profile.nickname}的音乐库` : '我的音乐库'}
            </h1>
          </header>
          {(accountLoading || accountError) && (
            <div
              role={accountError ? 'alert' : 'status'}
              className="mt-4 text-sm text-muted-foreground"
            >
              {accountError ?? <Skeleton className="h-4 w-32" />}
            </div>
          )}
          <div className="library-featured">
            <CollectionContextMenu
              item={liked ? { ...liked, name: '我喜欢的音乐', liked: true } : undefined}
              busy={playing}
              render={<div className="library-liked-card" />}
            >
              <button
                className="library-liked-open"
                aria-label="打开我喜欢的音乐"
                disabled={!liked}
                onClick={() => {
                  if (liked) openCollection({ ...liked, name: '我喜欢的音乐' })
                }}
              >
                <div className="library-liked-top">
                  {summaryBusy || accountLoading ? (
                    <TextSkeleton />
                  ) : (
                    <LyricExcerpt
                      tracks={featuredTracks}
                      enabled={!!profile && !summaryBusy && !!(summary || summaryError)}
                    />
                  )}
                </div>
                <div>
                  <h2>我喜欢的音乐</h2>
                  <div className="library-liked-count">
                    {summaryBusy || accountLoading ? (
                      <Skeleton className="h-4 w-20" />
                    ) : summaryError || summary?.likedError ? (
                      '暂时无法读取'
                    ) : liked ? (
                      `${liked.trackCount} 首歌`
                    ) : profile ? (
                      '还没有喜欢的音乐'
                    ) : (
                      '登录后收藏你的音乐'
                    )}
                  </div>
                </div>
              </button>
              <ActionButton
                size="icon-lg"
                className="library-liked-play"
                aria-label="播放我喜欢的音乐"
                disabled={playing || !liked || !summary?.likedTracks.length}
                onClick={() => {
                  if (liked) void playCollection(liked)
                }}
                title="播放收藏（最多 1000 首）"
              >
                <Play aria-hidden="true" />
              </ActionButton>
            </CollectionContextMenu>
            <div className="library-featured-songs">
              {!profile && !accountLoading ? (
                <div className="library-empty">
                  <p>登录音乐账号，找回你喜欢的旋律。</p>
                  <ActionButton variant="secondary" onClick={() => navigate('accounts')}>
                    登录账号
                  </ActionButton>
                </div>
              ) : summaryError || summary?.likedError ? (
                <div role="alert" className="library-empty text-destructive">
                  <p>{summaryError ?? summary?.likedError}</p>
                  {retry}
                </div>
              ) : summaryBusy || accountLoading ? (
                <SongGridSkeleton />
              ) : featuredTracks.length ? (
                <FindSongGrid
                  tracks={featuredTracks}
                  busy={playing}
                  onPlay={(track) => {
                    if (liked) void playCollection(liked, track.key)
                  }}
                >
                  {featuredTracks.map((track) => (
                    <SongContextMenu
                      key={track.key}
                      track={track}
                      onPlay={() => {
                        if (liked) void playCollection(liked, track.key)
                      }}
                      removeLabel="从歌单删除"
                      onRemove={
                        !readOnly && liked && liked.creatorId === profile?.userId
                          ? () => removeFromPlaylist(liked, track)
                          : undefined
                      }
                      render={
                        <div
                          role="group"
                          onClick={(event) => {
                            if (liked && !(event.target as HTMLElement).closest('button'))
                              void playCollection(liked, track.key)
                          }}
                          onKeyDown={(event) => {
                            if (
                              liked &&
                              event.target === event.currentTarget &&
                              ['Enter', ' '].includes(event.key)
                            ) {
                              event.preventDefault()
                              void playCollection(liked, track.key)
                            }
                          }}
                          className="library-song"
                          data-current={track.key === currentKey}
                          title={`${track.title} · ${track.artist}`}
                        />
                      }
                    >
                      <button
                        type="button"
                        aria-label={`播放 ${track.title}`}
                        onClick={() => {
                          if (liked) void playCollection(liked, track.key)
                        }}
                      >
                        <Cover size="small" cover={track.cover} className="size-10" />
                      </button>
                      <div className="min-w-0 flex-1">
                        <button
                          type="button"
                          className="block max-w-full truncate text-left font-semibold"
                          onClick={() => {
                            if (liked) void playCollection(liked, track.key)
                          }}
                        >
                          <TrackTitle track={track} />
                        </button>
                        <p className="truncate text-xs opacity-75">
                          <TrackArtists track={track} />
                        </p>
                      </div>
                      {track.key === currentKey && <PlayingIndicator />}
                    </SongContextMenu>
                  ))}
                </FindSongGrid>
              ) : (
                <p className="library-empty text-muted-foreground">喜欢的歌曲会出现在这里。</p>
              )}
            </div>
          </div>
          <div className="library-tabs-row">
            <div className="library-tabs" aria-label="音乐收藏分类">
              <div className="library-playlist-tab" data-active={tab === 'playlist'}>
                <ActionButton
                  variant="ghost"
                  className="library-tab"
                  aria-pressed={tab === 'playlist'}
                  onClick={() => {
                    setTab('playlist')
                  }}
                >
                  {filters.find((item) => item.value === filter)?.label}
                </ActionButton>
                <Select
                  items={filters}
                  value={filter}
                  onValueChange={(value) => {
                    if (value) {
                      setFilter(value as PlaylistFilter)
                      setTab('playlist')
                    }
                  }}
                >
                  <SelectTrigger size="sm" aria-label="筛选歌单" className="library-filter-trigger">
                    <span className="sr-only">
                      <SelectValue />
                    </span>
                  </SelectTrigger>
                  <SelectContent alignItemWithTrigger={false}>
                    <SelectGroup>
                      {filters.map((item) => (
                        <SelectItem key={item.value} value={item.value}>
                          {item.label}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </div>
              {tabs
                .slice(1)
                .filter((item) => !readOnly || item.value !== 'history')
                .map((item) => (
                  <ActionButton
                    key={item.value}
                    variant="ghost"
                    className="library-tab"
                    data-active={tab === item.value}
                    aria-pressed={tab === item.value}
                    onClick={() => {
                      setTab(item.value)
                    }}
                  >
                    {item.label}
                  </ActionButton>
                ))}
            </div>
            {tab === 'playlist' && profile && !readOnly && (
              <CreatePlaylist
                onCreated={() => {
                  setRefresh((value) => value + 1)
                }}
              />
            )}
          </div>
          {tab === 'history' && (
            <div className="mb-5 flex gap-2">
              <ActionButton
                variant={week ? 'secondary' : 'ghost'}
                size="sm"
                aria-pressed={week}
                onClick={() => {
                  setWeek(true)
                }}
              >
                最近一周
              </ActionButton>
              <ActionButton
                variant={!week ? 'secondary' : 'ghost'}
                size="sm"
                aria-pressed={!week}
                onClick={() => {
                  setWeek(false)
                }}
              >
                所有时间
              </ActionButton>
            </div>
          )}
          {accountLoading && !profile ? (
            <CollectionGridSkeleton />
          ) : !profile ? (
            <p className="library-empty text-muted-foreground">
              登录后查看收藏的歌单、专辑和艺人。
            </p>
          ) : listBusy && !list.items.length && !collections.items.length ? (
            tab === 'history' ? (
              <TrackListSkeleton sortable={false} />
            ) : (
              <CollectionGridSkeleton artist={tab === 'artist'} />
            )
          ) : tab === 'history' ? (
            history.tracks.length ? (
              <TrackList
                tracks={history.tracks}
                busy={playing}
                currentKey={currentKey}
                onPlay={(index) => playPage(history.tracks, index)}
                onAppend={append}
              />
            ) : (
              <p className="library-empty text-muted-foreground">这段时间还没有听歌记录。</p>
            )
          ) : collections.items.length ? (
            <CollectionCards
              items={collections.items}
              busy={playing}
              onOpen={openCollection}
              onPlay={(item) => void playCollection(item)}
            />
          ) : (
            <p className="library-empty text-muted-foreground">
              {tab === 'playlist'
                ? '没有符合筛选条件的歌单。'
                : `还没有收藏${tab === 'album' ? '专辑' : '艺人'}。`}
            </p>
          )}
          {profile && (
            <InfiniteLoad
              more={list.more}
              busy={listBusy}
              error={list.error}
              onLoad={list.loadMore}
            />
          )}
        </>
      )}
    </MusicPage>
  )
}
