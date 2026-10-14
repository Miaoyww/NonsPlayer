import { Skeleton } from '@/components/ui/skeleton'
import { useAccount } from '@/features/account/account'
import { Cover } from '@/components/music/cover'
import { TrackList } from '@/components/music/track-list'
import {
  Play,
  Plus,
  ChevronDown,
  CalendarDays,
  Radio,
  UserRound,
  Disc3,
  ThumbsDown,
  SkipForward,
  Sparkles,
  LayoutGrid
} from 'lucide-react'
import { CollectionHeader } from '@/features/library/collection-header'
import { CollectionTabs } from '@/features/library/collection-tabs'
import { ActionButton } from '@/components/music/action-button'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'

export function TextSkeleton() {
  return (
    <div role="status" aria-label="正在加载" className="flex w-full flex-col gap-2">
      <Skeleton className="h-4 w-3/4" />
      <Skeleton className="h-3 w-1/2" />
      <Skeleton className="h-3 w-2/3" />
    </div>
  )
}

export function SongGridSkeleton() {
  return (
    <div className="library-song-grid" role="status" aria-label="正在加载歌曲">
      {Array.from({ length: 12 }, (_, i) => (
        <div key={i} className="library-song">
          <Skeleton className="size-10 shrink-0 rounded-lg" />
          <div className="min-w-0 flex-1">
            <div className="flex h-5 items-center">
              <Skeleton className="h-3 w-4/5" />
            </div>
            <div className="flex h-4 items-center">
              <Skeleton className="h-3 w-3/5" />
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}

export function CollectionGridSkeleton({
  count = 12,
  artist = false
}: {
  count?: number
  artist?: boolean
}) {
  return (
    <div
      className={artist ? 'library-cover-grid' : 'discover-playlist-grid'}
      role="status"
      aria-label="正在加载收藏"
    >
      {Array.from({ length: count }, (_, i) => (
        <article key={i} className="library-cover-card">
          <div className="library-cover-art" data-round={artist}>
            <Skeleton
              className={
                artist ? 'aspect-square w-full rounded-full' : 'aspect-square w-full rounded-xl'
              }
            />
          </div>
          <div className="library-cover-title flex h-[1.3em] items-center">
            <Skeleton className="h-3 w-4/5" />
          </div>
          <div className="library-cover-subtitle flex h-[1.5em] items-center">
            <Skeleton className="h-3 w-3/5" />
          </div>
        </article>
      ))}
    </div>
  )
}

const noOp = () => {}
export function TrackListSkeleton({
  count = 8,
  online = true,
  sortable = true,
  searchable = false
}: {
  count?: number
  online?: boolean
  sortable?: boolean
  searchable?: boolean
}) {
  return (
    <TrackList
      tracks={[]}
      loading
      skeletonCount={count}
      showLikes={online}
      sortable={sortable}
      searchable={searchable}
      busy
      onPlay={noOp}
      onAppend={noOp}
    />
  )
}

export function LibraryPageSkeleton() {
  const { profile } = useAccount()
  return (
    <MusicPage aria-label="正在加载音乐库" aria-busy="true">
      <header className="library-profile">
        {profile?.avatarUrl ? (
          <Cover size="small" cover={profile.avatarUrl} className="library-avatar rounded-full" />
        ) : (
          <Skeleton className="library-avatar" />
        )}
        <h1 className="library-heading">
          {profile ? (
            `${profile.nickname}的音乐库`
          ) : (
            <Skeleton className="h-[1.25em] w-[10em] max-w-full" />
          )}
        </h1>
      </header>
      <div className="library-featured">
        <div className="library-liked-card">
          <div className="library-liked-open">
            <div className="library-liked-top">
              <TextSkeleton />
            </div>
            <div>
              <h2>我喜欢的音乐</h2>
              <div className="library-liked-count flex h-5 items-center">
                <Skeleton className="h-3 w-20" />
              </div>
            </div>
          </div>
          <div className="library-liked-play flex items-center justify-center">
            <Play aria-hidden="true" />
          </div>
        </div>
        <div className="library-featured-songs">
          <SongGridSkeleton />
        </div>
      </div>
      <div className="library-tabs-row">
        <div className="library-tabs">
          {['全部歌单', '专辑', '艺人', '听歌记录'].map((label, i) =>
            i === 0 ? (
              <div key={label} className="library-playlist-tab" data-active="true">
                <ActionButton variant="ghost" className="library-tab" disabled>
                  {label}
                </ActionButton>
                <span className="library-filter-trigger flex h-8 items-center">
                  <ChevronDown className="size-4" aria-hidden="true" />
                </span>
              </div>
            ) : (
              <ActionButton key={label} variant="ghost" className="library-tab" disabled>
                {label}
              </ActionButton>
            )
          )}
        </div>
        <ActionButton variant="ghost" disabled>
          <Plus aria-hidden="true" />
          新建歌单
        </ActionButton>
      </div>
      <CollectionGridSkeleton />
    </MusicPage>
  )
}

export function MusicPageSkeleton({
  tracks = false,
  view = 'search',
  collection,
  title
}: {
  tracks?: boolean
  view?: 'search' | 'local' | 'discover' | 'collection' | 'accounts'
  collection?: MusicCollection
  title?: string
}) {
  if (view === 'collection' && collection)
    return (
      <MusicPage aria-busy="true">
        <CollectionHeader
          collection={collection}
          total={collection.trackCount}
          busy={false}
          disabled
          onPlay={noOp}
        />
        <CollectionTabs collection={collection} hasMore={false}>
          <TrackListSkeleton />
        </CollectionTabs>
      </MusicPage>
    )
  if (view === 'discover' && !tracks)
    return (
      <MusicPage className="discovery" aria-busy="true">
        <MusicPageHeader title={<Skeleton className="h-[1.25em] w-[10em] max-w-full" />}>
          <Skeleton className="my-1 h-4 w-64" />
        </MusicPageHeader>
        <div className="discover-featured">
          <div className="discover-shortcuts">
            {['每日推荐', '私人雷达'].map((name, i) => (
              <div className="discover-shortcut" key={name}>
                <Skeleton className="discover-shortcut-cover rounded-lg" />
                <div className="discover-shortcut-copy">
                  <div className="discover-shortcut-title">
                    {i === 0 ? <CalendarDays /> : <Radio />}
                    <strong>{name}</strong>
                  </div>
                  <div className="discover-shortcut-desc">
                    {i === 0 ? '根据你的音乐口味 · 每日更新' : '发现你独特的音乐品味'}
                  </div>
                </div>
                <Skeleton className="size-8 rounded-full" />
              </div>
            ))}
          </div>
          <article className="discover-fm">
            <Skeleton className="discover-fm-cover shrink-0" />
            <div className="discover-fm-info">
              <h2>
                <Skeleton className="h-[1.3em] w-56 max-w-full" />
              </h2>
              <div className="discover-fm-meta">
                <UserRound />
                <Skeleton className="h-4 w-36" />
              </div>
              <div className="discover-fm-meta">
                <Disc3 />
                <Skeleton className="h-4 w-24" />
              </div>
              <div className="discover-fm-bottom">
                <div className="discover-fm-controls">
                  <ActionButton variant="ghost" size="icon-lg" disabled aria-label="不喜欢这首歌">
                    <ThumbsDown />
                  </ActionButton>
                  <ActionButton
                    className="discover-fm-play"
                    size="icon-lg"
                    disabled
                    aria-label="播放私人 FM"
                  >
                    <Play />
                  </ActionButton>
                  <ActionButton variant="ghost" size="icon-lg" disabled aria-label="下一首">
                    <SkipForward />
                  </ActionButton>
                </div>
                <span className="discover-fm-label">
                  <Radio />
                  私人 FM
                </span>
              </div>
            </div>
          </article>
        </div>
        <div className="music-page-section-bar">
          <h2>发现更多</h2>
          <div className="music-tabs flex items-center gap-3">
            <span className="flex items-center gap-2">
              <Sparkles className="size-4" />
              推荐歌单
            </span>
            <span className="flex items-center gap-2">
              <LayoutGrid className="size-4" />
              歌单广场
            </span>
          </div>
        </div>
        <CollectionGridSkeleton />
      </MusicPage>
    )
  if (view === 'accounts')
    return (
      <MusicPage aria-busy="true">
        <MusicPageHeader title="音乐账号" />
        <div className="mx-auto flex max-w-lg flex-col items-center gap-6 py-10">
          <Skeleton className="size-48" />
          <TextSkeleton />
        </div>
      </MusicPage>
    )
  return (
    <MusicPage aria-busy="true">
      <MusicPageHeader title={title ?? (view === 'local' ? '本地音乐' : '搜索音乐')}>
        <p>{view === 'local' ? '熟悉的收藏，随时聆听。' : '网易云音乐搜索结果'}</p>
      </MusicPageHeader>
      <div className="music-page-section-bar">
        <Skeleton className="h-9 w-64" />
      </div>
      <TrackListSkeleton online={view !== 'local'} sortable={view === 'local'} />
    </MusicPage>
  )
}
