import { presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { useCallback, useEffect, useRef, useState } from 'react'
import { Popover } from '@base-ui/react/popover'
import { isTauri } from '@tauri-apps/api/core'
import { ArrowRight, ListVideo, Search } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Cover } from '@/components/music/cover'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { Skeleton } from '@/components/ui/skeleton'
import { useAccount } from '@/features/account/account'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'
import { usePagedList } from '@/lib/use-paged-list'
import { getLibraryCollections } from './library-api'
import { CreatePlaylist } from './create-playlist'
import { parseRecentPlaylists, recentPlaylistKey, rememberPlaylist } from './recent-playlists'

export function PlaylistSidebar({ hidden = false }: { hidden?: boolean }) {
  const { profile, loading } = useAccount()
  const { page, navigate } = useMusicNavigation()
  const [open, setOpen] = useState(false)
  const [keyword, setKeyword] = useState('')
  const [refresh, setRefresh] = useState(0)
  const [recent, setRecent] = useState<{ userId?: string; items: MusicCollection[] }>({ items: [] })
  const userId = profile ? JSON.stringify(profile.reference) : undefined
  const history = useRef(recent)
  const previousVisit = useRef({ page, userId })
  useEffect(() => {
    if (hidden) setOpen(false)
  }, [hidden])
  useEffect(() => {
    setOpen(false)
    setKeyword('')
  }, [userId])
  useEffect(() => {
    const visit = previousVisit.current
    previousVisit.current = { page, userId }
    if (!userId) return
    const previous = history.current
    let items = previous.userId === userId ? previous.items : []
    if (previous.userId !== userId) {
      try {
        items = parseRecentPlaylists(localStorage.getItem(recentPlaylistKey(userId)))
      } catch (cause) {
        reportCaughtError('app/src/features/library/playlist-sidebar.tsx', cause)

        /* Storage may be unavailable. */
      }
    }
    if (
      visit.userId === userId &&
      visit.page !== page &&
      page.view === 'collection' &&
      page.collection?.kind === 'playlist'
    ) {
      items = rememberPlaylist(items, page.collection)
      try {
        localStorage.setItem(recentPlaylistKey(userId), JSON.stringify(items))
      } catch (cause) {
        reportCaughtError('app/src/features/library/playlist-sidebar.tsx', cause)

        /* Keep session history. */
      }
    }
    history.current = { userId, items }
    setRecent(history.current)
  }, [page, userId])
  const loader = useCallback(
    (offset: number) => getLibraryCollections('playlist', offset, 'all'),
    [userId]
  )
  const playlists = usePagedList(loader, 30, open && !!profile && isTauri(), {
    cacheKey: presentationKey('sidebar-playlists', userId),
    refreshKey: refresh
  })
  const matches = (item: MusicCollection) =>
    item.name.toLocaleLowerCase().includes(keyword.trim().toLocaleLowerCase())
  const recentItems = recent.userId === userId ? recent.items.filter(matches).slice(0, 2) : []
  function openPlaylist(item: MusicCollection) {
    navigate('collection', '', item)
    setOpen(false)
  }
  function row(item: MusicCollection) {
    const selected = page.view === 'collection' && page.collection?.id === item.id
    return (
      <Button
        key={item.id}
        variant={selected ? 'secondary' : 'ghost'}
        className="h-auto w-full justify-start gap-3 rounded-xl p-2 text-left"
        aria-current={selected ? 'page' : undefined}
        onClick={() => openPlaylist(item)}
      >
        <Cover size="small" cover={item.cover} className="size-11" />
        <span className="min-w-0 flex-1">
          <span className="block truncate font-medium">{item.name}</span>
          <span className="block text-xs text-muted-foreground">{item.trackCount} 首</span>
        </span>
      </Button>
    )
  }
  return (
    <Popover.Root open={open && !hidden} onOpenChange={setOpen}>
      <Popover.Trigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            className={hidden ? 'hidden' : ''}
            aria-label="打开歌单侧栏"
            title="我的歌单"
          />
        }
      >
        <ListVideo aria-hidden="true" />
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner side="bottom" align="start" sideOffset={12} className="z-40">
          <Popover.Popup className="playlist-sidebar glass-surface flex w-72 max-w-[calc(100vw-24px)] flex-col gap-3 overflow-hidden rounded-2xl border border-border p-3 shadow-xl outline-none">
            <Popover.Title className="sr-only">歌单侧栏</Popover.Title>
            <div className="flex shrink-0 items-center gap-2">
              <label className="flex min-w-0 flex-1 items-center gap-2 rounded-xl bg-muted px-3 py-2">
                <Search aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
                <input
                  aria-label="搜索我的歌单"
                  placeholder="搜索我的歌单"
                  className="min-w-0 w-full bg-transparent text-sm outline-none"
                  value={keyword}
                  onChange={(event) => setKeyword(event.target.value)}
                />
              </label>
              <CreatePlaylist
                iconOnly
                disabled={!profile}
                onCreated={() => setRefresh((value) => value + 1)}
              />
            </div>
            {!profile ? (
              <div className="flex flex-1 items-center justify-center py-5 text-center text-sm text-muted-foreground">
                {loading ? (
                  '正在读取账号…'
                ) : (
                  <Button
                    variant="ghost"
                    onClick={() => {
                      navigate('accounts')
                      setOpen(false)
                    }}
                  >
                    登录后查看我的歌单
                  </Button>
                )}
              </div>
            ) : (
              <>
                <section aria-label="我的歌单" className="flex min-h-0 flex-1 flex-col">
                  <h2 className="px-2 pb-2 text-sm font-semibold">我的歌单</h2>
                  <div className="playlist-sidebar-scroll min-h-0 flex-1 overflow-y-auto">
                    {playlists.items.filter(matches).map(row)}
                    {playlists.busy && playlists.items.length === 0 && (
                      <div aria-label="正在加载歌单" role="status" className="space-y-2">
                        {[0, 1, 2].map((index) => (
                          <Skeleton key={index} className="h-14 rounded-xl" />
                        ))}
                      </div>
                    )}
                    {!playlists.busy &&
                      !playlists.error &&
                      !playlists.more &&
                      !playlists.items.some(matches) && (
                        <p className="px-2 py-3 text-xs text-muted-foreground">
                          {keyword ? '没有匹配的歌单' : '暂无歌单'}
                        </p>
                      )}
                    <InfiniteLoad
                      more={playlists.more}
                      busy={playlists.busy}
                      error={playlists.error}
                      onLoad={playlists.loadMore}
                    />
                  </div>
                </section>
                <section
                  aria-label="最近打开"
                  className="flex max-h-[40%] min-h-0 shrink flex-col border-t border-border pt-3"
                >
                  <h2 className="px-2 pb-2 text-sm font-semibold">最近打开</h2>
                  <div className="playlist-sidebar-scroll min-h-0 overflow-y-auto">
                    {recentItems.map(row)}
                    {!recentItems.length && (
                      <p className="px-2 py-3 text-xs text-muted-foreground">
                        {keyword ? '没有匹配的最近歌单' : '打开的歌单会显示在这里'}
                      </p>
                    )}
                  </div>
                </section>
              </>
            )}
            <Button
              variant="secondary"
              className="w-full shrink-0"
              onClick={() => {
                navigate('library')
                setOpen(false)
              }}
            >
              查看全部歌单 <ArrowRight aria-hidden="true" />
            </Button>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
