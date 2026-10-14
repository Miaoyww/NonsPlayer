import { Skeleton } from '@/components/ui/skeleton'
import { TrackListSkeleton, CollectionGridSkeleton, TextSkeleton } from '@/components/music/loading'
import { Tabs, TabsList, TabsTab, TabsPanel } from '@/components/animate-ui/components/base/tabs'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'
import { CollectionContextMenu } from '@/features/library/collection-actions'
import { TrackArtists, TrackAlbum } from '@/components/music/music-links'
import { GreetingQuote } from '@/features/discovery/greeting-quote'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { isAndroid } from '@/lib/platform'
import {
  CalendarDays,
  ChevronDown,
  ChevronRight,
  Disc3,
  LayoutGrid,
  ListFilter,
  Pause,
  Play,
  Radio,
  RefreshCw,
  SkipForward,
  Sparkles,
  Tags,
  ThumbsDown,
  UserRound
} from 'lucide-react'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import {
  nativeCall,
  errorText,
  usePlayer,
  getPlayer,
  retryPrivateFm,
  type Track
} from '@/lib/player'
import { usePagedList } from '@/lib/use-paged-list'
import { playLibraryCollection } from '@/features/library/library-api'
import { createDiscoveryPlaylistLoader } from '@/features/discovery/discovery-api'
import { useAccount } from '@/features/account/account'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { PlaylistCard } from '@/features/library/playlist-card'
import { TrackList } from '@/components/music/track-list'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'

interface Category {
  name: string
  group: string
}
const radar: MusicCollection = {
  id: 3136952023,
  kind: 'playlist',
  name: '私人雷达',
  cover: '',
  subtitle: '根据听歌记录为你打造',
  trackCount: 0
}

function CategoryPicker({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [open, setOpen] = useState(false)
  const [categories, setCategories] = useState<Category[]>([])
  const [error, setError] = useState<string>()
  const [revision, setRevision] = useState(0)
  useEffect(() => {
    let disposed = false
    if (!open || !isTauri()) return
    setError(undefined)
    void nativeCall<Category[]>('discovery_categories', { refresh: revision > 0 })
      .then((items) => {
        if (!disposed) setCategories(items)
      })
      .catch((cause) => {
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
    }
  }, [open, revision])
  const select = (name: string) => {
    onChange(name)
    setOpen(false)
  }
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger render={<Button variant="outline" className="rounded-full" />}>
        <ListFilter aria-hidden="true" />
        {value === '全部' ? '全部歌单' : value}
        <ChevronDown aria-hidden="true" />
      </DialogTrigger>
      <DialogContent className="discover-category-dialog">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Tags aria-hidden="true" />
            歌单分类
          </DialogTitle>
          <DialogDescription>选择你喜欢的音乐风格</DialogDescription>
        </DialogHeader>
        <div className="discover-category-scroll">
          <Button
            className="rounded-full"
            variant={value === '全部' ? 'default' : 'secondary'}
            aria-pressed={value === '全部'}
            onClick={() => select('全部')}
          >
            全部歌单
          </Button>
          {error ? (
            <div role="alert" className="mt-5 text-sm text-destructive">
              {error}
              <Button variant="ghost" onClick={() => setRevision((v) => v + 1)}>
                重试
              </Button>
            </div>
          ) : !categories.length ? (
            isTauri() ? (
              <TextSkeleton />
            ) : (
              <p className="mt-5 text-sm text-muted-foreground">在桌面应用中查看歌单分类。</p>
            )
          ) : (
            [...new Set(categories.map((item) => item.group))].map((group) => (
              <section key={group} className="mt-6">
                <h3 className="mb-3 font-semibold">{group}</h3>
                <div className="flex flex-wrap gap-2">
                  {categories
                    .filter((item) => item.group === group)
                    .map((item) => (
                      <Button
                        key={item.name}
                        className="rounded-full"
                        variant={value === item.name ? 'default' : 'secondary'}
                        aria-pressed={value === item.name}
                        onClick={() => select(item.name)}
                      >
                        {item.name}
                      </Button>
                    ))}
                </div>
              </section>
            ))
          )}
        </div>
        <DialogFooter>
          <DialogClose render={<Button variant="outline" />}>关闭</DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function PrivateFM({ onError }: { onError: (cause: unknown) => void }) {
  const { navigate } = useMusicNavigation()
  const { profile } = useAccount()
  const player = usePlayer('index', 'privateFmError', 'privateFmSession', 'queue', 'status')
  const [tracks, setTracks] = useState<Track[]>([])
  const [error, setError] = useState<string>()
  const [busy, setBusy] = useState(false)
  const [revision, setRevision] = useState(0)
  const generation = useRef(0)
  const pending = useRef(false)
  useEffect(() => {
    const serial = ++generation.current
    pending.current = false
    setTracks([])
    setError(undefined)
    setBusy(false)
    if (!profile || !isTauri()) return
    setBusy(true)
    void nativeCall<Track[]>('discovery_tracks', { kind: 'fm', refresh: true })
      .then((items) => {
        if (serial === generation.current) setTracks(items)
      })
      .catch((cause) => {
        if (serial === generation.current) setError(errorText(cause))
      })
      .finally(() => {
        if (serial === generation.current) setBusy(false)
      })
    return () => {
      generation.current++
    }
  }, [profile, revision])
  const current = player.index !== null ? player.queue[player.index] : undefined
  const active = !!current && player.privateFmSession != null
  const track = active ? current : tracks[0]
  const playing = active && player.status === 'playing'
  async function play() {
    if (pending.current || !track) return
    pending.current = true
    setBusy(true)
    const serial = generation.current
    try {
      if (active) await nativeCall('player_action', { action: playing ? 'pause' : 'resume' })
      else {
        await nativeCall('play_private_fm', { keys: tracks.map((item) => item.key) })
      }
    } catch (cause) {
      if (serial === generation.current) onError(cause)
    } finally {
      if (serial === generation.current) {
        pending.current = false
        setBusy(false)
      }
    }
  }
  async function next(dislike = false) {
    if (pending.current || !track) return
    pending.current = true
    setBusy(true)
    setError(undefined)
    const serial = generation.current
    try {
      if (dislike && track.source.kind === 'netease')
        await nativeCall('discovery_dislike', { id: track.source.id })
      if (serial !== generation.current) return
      if (active) {
        if (getPlayer().privateFmSession !== player.privateFmSession) return
        await nativeCall('player_action', { action: 'next' })
        return
      }
      const remaining = tracks.slice(tracks.findIndex((item) => item.key === track.key) + 1)
      const items = remaining.length
        ? remaining
        : await nativeCall<Track[]>('discovery_tracks', { kind: 'fm', refresh: true })
      if (serial !== generation.current) return
      const fresh = items.filter((item) => item.key !== track.key)
      if (!fresh.length) throw new Error('暂时没有新的 FM 歌曲，请重试。')
      setTracks(fresh)
      await nativeCall('play_private_fm', { keys: fresh.map((item) => item.key) })
    } catch (cause) {
      if (serial === generation.current) setError(errorText(cause))
    } finally {
      if (serial === generation.current) {
        pending.current = false
        setBusy(false)
      }
    }
  }
  return (
    <article className="discover-fm" aria-busy={busy && !track}>
      {busy && !track ? (
        <Skeleton className="discover-fm-cover" />
      ) : (
        <Cover size="large" cover={track?.cover} className="discover-fm-cover" />
      )}
      <div className="discover-fm-info">
        <h2 title={track?.title}>
          {busy && !track ? <Skeleton className="h-7 w-3/4" /> : (track?.title ?? '你的下一首心动')}
        </h2>
        <div className="discover-fm-meta">
          <UserRound aria-hidden="true" />
          <div className="min-w-0">
            {busy && !track ? (
              <Skeleton className="h-4 w-36" />
            ) : track ? (
              <TrackArtists track={track} />
            ) : (
              '随你的音乐口味探索'
            )}
          </div>
        </div>
        <div className="discover-fm-meta">
          <Disc3 aria-hidden="true" />
          <div className="min-w-0">
            {busy && !track ? (
              <Skeleton className="h-4 w-24" />
            ) : track ? (
              <TrackAlbum track={track} />
            ) : (
              '私人 FM'
            )}
          </div>
        </div>
        {(error || (active && player.privateFmError)) && (
          <div role="alert" className="text-xs text-destructive">
            {error || player.privateFmError}
            <ActionButton
              variant="ghost"
              size="sm"
              onClick={() => {
                setError(undefined)
                if (active) retryPrivateFm()
                else setRevision((v) => v + 1)
              }}
            >
              重试
            </ActionButton>
          </div>
        )}
        <div className="discover-fm-bottom">
          <div className="discover-fm-controls">
            <ActionButton
              variant="ghost"
              size="icon-lg"
              aria-label="不喜欢这首歌"
              disabled={busy || !track}
              onClick={() => void next(true)}
            >
              <ThumbsDown aria-hidden="true" />
            </ActionButton>
            <ActionButton
              className="discover-fm-play"
              size="icon-lg"
              aria-label={playing ? '暂停私人 FM' : '播放私人 FM'}
              disabled={busy || !track}
              onClick={() => void play()}
            >
              {playing ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
            </ActionButton>
            <ActionButton
              variant="ghost"
              size="icon-lg"
              aria-label="下一首私人 FM"
              disabled={busy || !track}
              onClick={() => void next()}
            >
              <SkipForward aria-hidden="true" />
            </ActionButton>
          </div>
          <span className="discover-fm-label">
            <Radio aria-hidden="true" />
            私人 FM
          </span>
        </div>
        {profile && !track && !error && !busy && (
          <p role="status" className="mt-2 text-sm text-muted-foreground">
            {
              <>
                暂时没有推荐歌曲。
                <ActionButton variant="ghost" size="sm" onClick={() => setRevision((v) => v + 1)}>
                  重试
                </ActionButton>
              </>
            }
          </p>
        )}
        {!profile && (
          <div className="mt-2">
            <ActionButton variant="secondary" onClick={() => navigate('accounts', 'netease')}>
              登录账号
            </ActionButton>
          </div>
        )}
      </div>
    </article>
  )
}

export default function Discovery({
  onError,
  onNotice
}: {
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { profile } = useAccount()
  const { page, navigate } = useMusicNavigation()
  const player = usePlayer('index', 'privateFmError', 'privateFmSession', 'queue', 'status')
  const current = player.index !== null ? player.queue[player.index] : undefined
  const [radarInfo, setRadarInfo] = useState<MusicCollection>(radar)
  useEffect(() => {
    let disposed = false
    setRadarInfo(radar)
    if (!profile || !isTauri()) return
    void nativeCall<MusicCollection>('discovery_radar')
      .then((item) => {
        if (!disposed) setRadarInfo({ ...item, name: '私人雷达' })
      })
      .catch(() => {
        /* The shortcut remains available if its optional cover fails. */
      })
    return () => {
      disposed = true
    }
  }, [profile])
  const [category, setCategory] = useState('全部')
  const [order, setOrder] = useState('hot')
  const [revision, setRevision] = useState(0)
  const [playing, setPlaying] = useState(false)
  const pending = useRef(false)
  const section = page.query === 'square' ? 'square' : 'recommended'
  const daily = page.query === 'daily'
  const loader = useMemo(
    () =>
      createDiscoveryPlaylistLoader({
        section,
        category: section === 'square' ? category : '全部',
        order: section === 'square' ? order : 'hot',
        refresh: revision > 0
      }),
    [section, category, order, revision, profile]
  )
  const list = usePagedList(loader, 30, isTauri() && !daily)
  const dailyLoader = useCallback(
    async () => ({
      items: await nativeCall<Track[]>('discovery_tracks', {
        kind: 'daily',
        refresh: revision > 0
      }),
      more: false
    }),
    [profile, revision]
  )
  const songs = usePagedList(dailyLoader, 100, isTauri() && daily && !!profile)
  async function play(item: MusicCollection) {
    if (pending.current) return
    pending.current = true
    setPlaying(true)
    try {
      if (await playLibraryCollection(item)) onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      onError(cause)
    } finally {
      pending.current = false
      setPlaying(false)
    }
  }
  async function playDaily(next: boolean) {
    if (pending.current || !profile || !isTauri()) return
    pending.current = true
    setPlaying(true)
    try {
      const tracks = await nativeCall<Track[]>('discovery_tracks', { kind: 'daily' })
      if (!tracks.length) throw new Error('今天还没有推荐歌曲，请稍后重试。')
      await nativeCall(next ? 'append_queue' : 'play_queue', {
        keys: tracks.map((track) => track.key),
        ...(next ? {} : { index: 0 })
      })
      onNotice(next ? '已将每日推荐按顺序加入下一首播放。' : '已播放每日推荐并替换播放列表。')
    } catch (cause) {
      onError(cause)
    } finally {
      pending.current = false
      setPlaying(false)
    }
  }
  const hour = new Date().getHours()
  const greeting = hour < 6 ? '夜深了' : hour < 12 ? '上午好' : hour < 18 ? '下午好' : '晚上好'
  return (
    <MusicPage className="discovery" aria-label="发现音乐">
      {daily ? (
        <>
          <header className="mb-8">
            <h1 className="library-heading">每日推荐</h1>
            <p className="mt-2 text-sm text-muted-foreground">根据你的音乐口味，每日更新。</p>
          </header>
          {!profile ? (
            <div className="library-empty">
              <p>登录后发现今天为你推荐的音乐。</p>
              <ActionButton variant="secondary" onClick={() => navigate('accounts', 'netease')}>
                登录账号
              </ActionButton>
            </div>
          ) : (
            <>
              {songs.busy && !songs.items.length ? (
                <TrackListSkeleton />
              ) : (
                <TrackList
                  tracks={songs.items}
                  sortable
                  busy={playing}
                  currentKey={current?.key}
                  onPlay={(index, tracks) =>
                    void nativeCall('play_queue', {
                      keys: tracks.map((t) => t.key),
                      index
                    }).catch(onError)
                  }
                  onAppend={(track) =>
                    void nativeCall('append_queue', { keys: [track.key] })
                      .then(() => onNotice(`已将「${track.title}」设为下一首播放。`))
                      .catch(onError)
                  }
                />
              )}
              <InfiniteLoad
                more={songs.more}
                busy={songs.busy}
                error={songs.error}
                onLoad={songs.loadMore}
              />
            </>
          )}
        </>
      ) : (
        <>
          <MusicPageHeader title={`${greeting}${profile ? `，${profile.nickname}` : '，音乐相伴'}`}>
            <GreetingQuote />
          </MusicPageHeader>
          <div className="discover-featured">
            <div className="discover-shortcuts">
              <CollectionContextMenu
                name="每日推荐"
                busy={playing}
                onPlay={profile ? () => void playDaily(false) : undefined}
                onNext={profile ? () => void playDaily(true) : undefined}
                render={
                  <button
                    type="button"
                    className="discover-shortcut"
                    onClick={() => navigate('discover', 'daily')}
                  />
                }
              >
                {!isAndroid() && (
                  <Cover
                    size="small"
                    cover={profile?.avatarUrl}
                    className="discover-shortcut-cover"
                  />
                )}
                <span className="discover-shortcut-copy">
                  <span className="discover-shortcut-title">
                    <CalendarDays aria-hidden="true" />
                    <strong>每日推荐</strong>
                  </span>
                  <span className="discover-shortcut-desc">根据你的音乐口味 · 每日更新</span>
                </span>
                <ChevronRight className="discover-shortcut-arrow" aria-hidden="true" />
              </CollectionContextMenu>
              <CollectionContextMenu
                item={radarInfo}
                busy={playing || !profile}
                render={
                  <button
                    type="button"
                    className="discover-shortcut"
                    onClick={() => navigate('collection', '', radarInfo)}
                  />
                }
              >
                {!isAndroid() && (
                  <Cover cover={radarInfo.cover} className="discover-shortcut-cover" />
                )}
                <span className="discover-shortcut-copy">
                  <span className="discover-shortcut-title">
                    <Radio aria-hidden="true" />
                    <strong>私人雷达</strong>
                  </span>
                  <span className="discover-shortcut-desc">发现你独特的音乐品味</span>
                </span>
                <ChevronRight className="discover-shortcut-arrow" aria-hidden="true" />
              </CollectionContextMenu>
            </div>
            <PrivateFM onError={onError} />
          </div>
          <Tabs
            value={section}
            onValueChange={(value) => navigate('discover', value === 'square' ? 'square' : '')}
            className="gap-0"
          >
            <div className="music-page-section-bar">
              <h2>发现更多</h2>
              <TabsList className="music-tabs" aria-label="发现分类">
                <TabsTab value="recommended">
                  <Sparkles aria-hidden="true" />
                  推荐歌单
                </TabsTab>
                <TabsTab value="square">
                  <LayoutGrid aria-hidden="true" />
                  歌单广场
                </TabsTab>
              </TabsList>
              <ActionButton
                variant="ghost"
                size="icon"
                aria-label="刷新歌单"
                disabled={list.busy}
                onClick={() => setRevision((v) => v + 1)}
              >
                <RefreshCw aria-hidden="true" />
              </ActionButton>
            </div>
            <TabsPanel key={section} value={section} transition={{ duration: 0.15 }}>
              {section === 'square' && (
                <div className="discover-filters">
                  <CategoryPicker value={category} onChange={setCategory} />
                  <div className="flex gap-2">
                    <ActionButton
                      className="rounded-full"
                      variant={order === 'hot' ? 'default' : 'ghost'}
                      aria-pressed={order === 'hot'}
                      onClick={() => setOrder('hot')}
                    >
                      热门
                    </ActionButton>
                    <ActionButton
                      className="rounded-full"
                      variant={order === 'new' ? 'default' : 'ghost'}
                      aria-pressed={order === 'new'}
                      onClick={() => setOrder('new')}
                    >
                      最新
                    </ActionButton>
                  </div>
                </div>
              )}
              {list.busy && !list.items.length ? (
                <CollectionGridSkeleton />
              ) : list.items.length ? (
                <div className="discover-playlist-grid">
                  {list.items.map((item) => (
                    <PlaylistCard
                      key={item.id}
                      item={item}
                      busy={playing || !isTauri()}
                      onOpen={(value) => navigate('collection', '', value)}
                      onPlay={(value) => void play(value)}
                    />
                  ))}
                </div>
              ) : (
                !list.error && (
                  <p className="library-empty text-muted-foreground">
                    {isTauri() ? '暂时没有歌单。' : '在桌面应用中获取推荐歌单与歌单广场。'}
                  </p>
                )
              )}
              <InfiniteLoad
                more={list.more}
                busy={list.busy}
                error={list.error}
                onLoad={list.loadMore}
              />
            </TabsPanel>
          </Tabs>
        </>
      )}
    </MusicPage>
  )
}
