import { reportCaughtError } from '@/lib/diagnostics'
import { errorNotices } from '@/features/playback/error-notices'
import { useListFind } from './list-find'
import { Skeleton } from '@/components/ui/skeleton'
import { favoriteId, onlineReference } from '@/features/music/references'
import { Input } from '@/components/ui/input'
import { TrackAlbum } from '@/components/music/music-links'
import { useVirtualizer } from '@tanstack/react-virtual'
import { scrollParent } from '@/components/music/infinite-load'
import { Heart, ListPlus, Play, Search, ArrowUpDown, ArrowUp, ArrowDown } from 'lucide-react'
import {
  memo,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState
} from 'react'
import { TrackSearchContext } from '@/components/music/track-search-context'
import { errorText, formatTime, type Track } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { PlayingIndicator } from '@/components/music/playing-indicator'
import { useAccount } from '@/features/account/account'
import { useTrackColumnSizing } from './track-column-sizing'
import { SongContextMenu, useSongActions } from '@/components/music/song-actions'
import { TrackIdentity, TrackSourceLabel } from '@/components/music/track-identity'
import type { ReactNode } from 'react'
import { useInterfaceDensity } from '@/features/settings/use-interface-density'
import { isAndroid } from '@/lib/platform'
import { isTauri } from '@tauri-apps/api/core'
import { playTrackOnDoubleClick } from '@/features/playback/track-double-click'

type SortKey = 'default' | 'title' | 'artist' | 'album' | 'duration'

function SortHeader({
  label,
  active,
  descending,
  detail,
  next,
  onClick,
  compact = false
}: {
  label: string
  active: boolean
  descending: boolean
  detail: string
  next: string
  onClick: () => void
  compact?: boolean
}) {
  const Icon = active ? (descending ? ArrowDown : ArrowUp) : ArrowUpDown
  return (
    <button
      type="button"
      className="track-sort-header"
      data-active={active}
      aria-label={`${label}排序：${next}`}
      onClick={onClick}
    >
      <span>{label}</span>
      <span className="track-sort-hint" data-visible={detail === '按序排列'}>
        <Icon aria-hidden="true" />
        <span className={compact ? 'sr-only' : undefined}>{detail}</span>
      </span>
    </button>
  )
}

interface Props {
  loading?: boolean
  skeletonCount?: number
  showLikes?: boolean
  tracks: Track[]
  currentKey?: string
  busy: boolean
  onPlay: (index: number, tracks: Track[]) => void
  /** Candidate pickers and authorized plugin files use their own primary action. */
  doubleClickMode?: 'playback' | 'primary'
  onAppend?: (track: Track) => void
  offset?: number
  currentIndex?: number
  locateRequest?: number
  onRemove?: (track: Track, index: number) => void | Promise<unknown>
  canRemove?: (track: Track) => boolean
  removeLabel?: string
  extraColumns?: {
    label: string
    className?: string
    render: (track: Track, index: number) => ReactNode
  }[]
  selection?: {
    keys: ReadonlySet<string>
    onToggle: (key: string, checked: boolean) => void
    onToggleAll: (keys: string[], checked: boolean) => void
  }
  sortable?: boolean
  searchable?: boolean
  hasMore?: boolean
  showDuration?: boolean
  renderSource?: (track: Track) => ReactNode
  getRowKey?: (track: Track) => string
  isCurrentTrack?: (track: Track) => boolean
  fitContainer?: boolean
}

export const TrackList = memo(function TrackList({
  tracks,
  loading = false,
  skeletonCount = 8,
  showLikes: likesColumn,
  currentKey,
  busy,
  onPlay,
  doubleClickMode = 'playback',
  onAppend,
  offset = 0,
  currentIndex,
  locateRequest,
  onRemove,
  removeLabel,
  canRemove,
  extraColumns = [],
  selection,
  sortable = false,
  searchable = false,
  hasMore = false,
  showDuration: durationColumn = true,
  renderSource,
  getRowKey,
  isCurrentTrack,
  fitContainer = false
}: Props) {
  const mobile = isAndroid()
  const showDuration = !mobile && durationColumn
  const findElement = useRef<HTMLDivElement>(null)
  const [findTarget, setFindTarget] = useState<{ index: number; serial: number }>()
  const [selectedKey, setSelectedKey] = useState<string>()
  const songActions = useSongActions()
  const doubleClickBusy = useRef(false)
  const [sort, setSort] = useState<SortKey>('default')
  const [descending, setDescending] = useState(false)
  const [query, setQuery] = useState('')
  const externalQuery = useContext(TrackSearchContext)
  const searchQuery = externalQuery ?? query
  const sortCycle = (column: 'title' | 'album' | 'duration') => {
    const keys: SortKey[] = column === 'title' ? ['title', 'artist'] : [column]
    const states = [
      { key: 'default' as SortKey, descending: false },
      ...keys.flatMap((key) => [
        { key, descending: false },
        { key, descending: true }
      ])
    ]
    const current = Math.max(
      0,
      states.findIndex((state) => state.key === sort && state.descending === descending)
    )
    return states[(current + 1) % states.length]
  }
  const nextLabel = (column: 'title' | 'album' | 'duration') => {
    const next = sortCycle(column)
    return next.key === 'default'
      ? '恢复默认排序'
      : `${{ title: '标题', artist: '歌手', album: '专辑', duration: '时长' }[next.key]}${next.descending ? '降序' : '升序'}`
  }
  const cycleSort = (column: 'title' | 'album' | 'duration') => {
    const next = sortCycle(column)
    setSort(next.key)
    setDescending(next.descending)
  }
  const ariaSort = (active: boolean) =>
    active ? (descending ? ('descending' as const) : ('ascending' as const)) : undefined
  const titleActive = sort === 'title' || sort === 'artist'
  const entries = useMemo(() => {
    const keyword = searchQuery.trim().toLocaleLowerCase()
    const occurrences = new Map<string, number>()
    const result = tracks
      .map((track, index) => {
        const identity = getRowKey?.(track) ?? track.key
        const occurrence = occurrences.get(identity) ?? 0
        occurrences.set(identity, occurrence + 1)
        return { track, index, rowKey: `${identity}:${occurrence}` }
      })
      .filter(
        ({ track }) =>
          (!searchable && externalQuery === undefined) ||
          !keyword ||
          [track.title, track.artist, track.album, ...(track.aliases ?? [])].some((value) =>
            value.toLocaleLowerCase().includes(keyword)
          )
      )
    if (sortable && sort !== 'default') {
      const collator = new Intl.Collator('zh-CN', { numeric: true, sensitivity: 'base' })
      result.sort((a, b) => {
        const compare =
          sort === 'duration'
            ? a.track.durationMs - b.track.durationMs
            : collator.compare(
                a.track[sort as 'title' | 'artist' | 'album'],
                b.track[sort as 'title' | 'artist' | 'album']
              )
        return (descending ? -compare : compare) || a.index - b.index
      })
    }
    return result
  }, [tracks, searchQuery, searchable, externalQuery, sortable, sort, descending, getRowKey])
  const visibleTracks = useMemo(() => entries.map(({ track }) => track), [entries])
  const doubleClickPlay = (index: number) => {
    if (busy || !isTauri() || doubleClickBusy.current) return
    const track = visibleTracks[index]
    if (!track) return
    if (doubleClickMode === 'primary') {
      onPlay(index, visibleTracks)
      return
    }
    doubleClickBusy.current = true
    songActions.run(
      () =>
        playTrackOnDoubleClick(track, { tracks: visibleTracks, index }).finally(() => {
          doubleClickBusy.current = false
        }),
      (message) => (typeof message === 'string' ? message : undefined)
    )
  }
  const [cardMode] = useInterfaceDensity()
  const { getAccount, likesError, reloadLikes } = useAccount()
  useEffect(() => {
    if (likesError) errorNotices.push(likesError, [{ label: '重试收藏状态', run: reloadLikes }])
  }, [likesError, reloadLikes])
  const body = useRef<HTMLTableSectionElement>(null)
  const [scrollMargin, setScrollMargin] = useState(0)
  const getItemKey = useCallback((index: number) => entries[index].rowKey, [entries])
  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () =>
      body.current
        ? scrollParent(body.current.closest<HTMLElement>('.track-table-container')!)
        : null,
    estimateSize: () => (cardMode === 'compact' ? 52 : 72),
    scrollMargin,
    overscan: 8,
    getItemKey,
    measureElement: (element) => element.getBoundingClientRect().height + 4
  })
  useLayoutEffect(() => {
    virtualizer.measure()
    body.current?.querySelectorAll<HTMLTableRowElement>('tr[data-index]').forEach((row) => {
      virtualizer.measureElement(row)
    })
  }, [cardMode, virtualizer])
  useLayoutEffect(() => {
    const element = body.current
    const parent = element && scrollParent(element.closest<HTMLElement>('.track-table-container')!)
    if (!element || !parent) return
    const measure = () =>
      setScrollMargin(
        element.getBoundingClientRect().top - parent.getBoundingClientRect().top + parent.scrollTop
      )
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(parent)
    observer.observe(element.closest('section') ?? element.closest('table')!)
    return () => observer.disconnect()
  }, [])
  useLayoutEffect(() => {
    if (locateRequest === undefined || currentIndex === undefined || currentIndex < 0) return
    const visibleIndex = entries.findIndex((entry) => entry.index === currentIndex)
    if (visibleIndex >= 0) virtualizer.scrollToIndex(visibleIndex, { align: 'center' })
  }, [locateRequest, scrollMargin, virtualizer])
  useListFind(
    useMemo(
      () => ({
        element: findElement,
        entries: loading ? [] : entries.map((entry, index) => ({ track: entry.track, index })),
        hasMore,
        busy,
        play: (index: number) => onPlay(index, visibleTracks),
        locate: (index: number) => {
          setSelectedKey(entries[index]?.rowKey)
          setFindTarget({ index, serial: Date.now() })
        },
        remove: onRemove
          ? (index: number) => {
              if (canRemove?.(entries[index].track) !== false)
                return onRemove(entries[index].track, entries[index].index)
            }
          : undefined,
        removeLabel
      }),
      [loading, entries, hasMore, busy, onPlay, visibleTracks, onRemove, canRemove, removeLabel]
    )
  )
  useLayoutEffect(() => {
    if (findTarget) virtualizer.scrollToIndex(findTarget.index, { align: 'center' })
  }, [findTarget, scrollMargin, virtualizer])
  useEffect(() => {
    if (!findTarget) return
    const timer = window.setTimeout(() => {
      setSelectedKey(undefined)
      setFindTarget(undefined)
    }, 2500)
    const frame = requestAnimationFrame(() => {
      findElement.current
        ?.querySelector<HTMLElement>(`tr[data-index="${findTarget.index}"]`)
        ?.focus({ preventScroll: true })
    })
    return () => {
      window.clearTimeout(timer)
      cancelAnimationFrame(frame)
    }
  }, [findTarget])
  const showLikes =
    !mobile && (likesColumn ?? tracks.some((track) => onlineReference(track) !== null))
  const sizingSpecs = useMemo(
    () => [
      ...(!mobile || selection ? [{ id: 'index', label: '序号', size: 48, minSize: 36 }] : []),
      { id: 'title', label: '标题', size: 480, minSize: mobile ? 0 : 180 },
      ...(!mobile ? [{ id: 'album', label: '专辑', size: 200, minSize: 100 }] : []),
      { id: 'source', label: '来源', size: mobile ? 80 : 104, minSize: 64 },
      ...(showLikes ? [{ id: 'likes', label: '喜欢', size: 48, minSize: 40 }] : []),
      ...(showDuration ? [{ id: 'duration', label: '时长', size: 80, minSize: 64 }] : []),
      ...extraColumns.map((column, index) => ({
        id: `extra-${index}`,
        label: column.label,
        size: 112,
        minSize: 64
      })),
      ...(onAppend ? [{ id: 'append', label: '加入队列', size: 48, minSize: 40 }] : [])
    ],
    [mobile, selection, showLikes, showDuration, extraColumns, onAppend]
  )
  const columnSizing = useTrackColumnSizing(sizingSpecs)
  const columns =
    (mobile ? 2 + Number(!!selection) : 4) +
    Number(showDuration) +
    extraColumns.length +
    Number(showLikes) +
    Number(!!onAppend)
  const rows = virtualizer.getVirtualItems()
  const top = rows.length ? Math.max(0, rows[0].start - scrollMargin) : 0
  const bottom = rows.length
    ? Math.max(0, virtualizer.getTotalSize() - (rows[rows.length - 1].end - scrollMargin))
    : 0
  return (
    <>
      {searchable && externalQuery === undefined && !mobile && (
        <div className="mb-3 flex flex-wrap items-center justify-end gap-3">
          {hasMore && (
            <span className="text-xs text-muted-foreground">
              搜索已加载歌曲，滚动到底部继续加载
            </span>
          )}
          <label className="relative w-64 max-w-full">
            <span className="sr-only">搜索列表歌曲</span>
            <Search
              aria-hidden="true"
              className="pointer-events-none absolute top-2.5 left-3 size-4 text-muted-foreground"
            />
            <Input
              type="search"
              className="pl-9"
              placeholder="搜索歌曲、艺术家、专辑"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
        </div>
      )}
      <div
        ref={(element) => {
          findElement.current = element
          columnSizing.container.current = element
        }}
        className="track-table-container"
        data-mobile={mobile}
      >
        <table
          className="track-list w-full table-fixed text-left text-sm"
          data-mode={cardMode}
          data-mobile={mobile}
          aria-busy={loading}
          style={{ minWidth: mobile || fitContainer ? undefined : columnSizing.total }}
        >
          <colgroup>
            {columnSizing.columns.map((column) => (
              <col
                key={column.id}
                style={{ width: `${(column.size / columnSizing.total) * 100}%` }}
              />
            ))}
          </colgroup>
          <caption className="sr-only">歌曲列表</caption>
          <thead className="bg-background text-xs text-muted-foreground">
            <tr className="border-b border-border">
              {(!mobile || selection) && (
                <th className="py-3 text-center" scope="col">
                  {selection ? (
                    <input
                      type="checkbox"
                      aria-label="选择全部已加载歌曲"
                      title="选择全部已加载歌曲"
                      disabled={busy || !visibleTracks.length}
                      checked={
                        !!visibleTracks.length &&
                        visibleTracks.every((track) => selection.keys.has(track.key))
                      }
                      ref={(element) => {
                        if (element)
                          element.indeterminate =
                            visibleTracks.some((track) => selection.keys.has(track.key)) &&
                            !visibleTracks.every((track) => selection.keys.has(track.key))
                      }}
                      onChange={(event) =>
                        selection.onToggleAll(
                          visibleTracks.map((track) => track.key),
                          event.target.checked
                        )
                      }
                    />
                  ) : sortable ? (
                    <button
                      type="button"
                      className="track-sequence-header"
                      aria-label="按序排列"
                      title="按序排列"
                      aria-pressed={sort === 'default'}
                      onClick={() => {
                        setSort('default')
                        setDescending(false)
                      }}
                    >
                      #
                    </button>
                  ) : (
                    '#'
                  )}
                  {!mobile && columnSizing.handle('index')}
                </th>
              )}
              <th className="py-3" scope="col" aria-sort={ariaSort(sortable && titleActive)}>
                {sortable ? (
                  <SortHeader
                    label="标题"
                    active={titleActive}
                    descending={descending}
                    detail={
                      titleActive
                        ? `${sort === 'artist' ? '歌手' : '标题'}${descending ? '降序' : '升序'}`
                        : sort === 'default'
                          ? '按序排列'
                          : '未排序'
                    }
                    next={nextLabel('title')}
                    onClick={() => cycleSort('title')}
                  />
                ) : (
                  '标题'
                )}
                {!mobile && columnSizing.handle('title')}
              </th>
              {!mobile && (
                <th className="py-3" scope="col" aria-sort={ariaSort(sortable && sort === 'album')}>
                  {sortable ? (
                    <SortHeader
                      label="专辑"
                      active={sort === 'album'}
                      descending={descending}
                      detail={sort === 'album' ? (descending ? '降序' : '升序') : '默认'}
                      next={nextLabel('album')}
                      onClick={() => cycleSort('album')}
                    />
                  ) : (
                    '专辑'
                  )}
                  {!mobile && columnSizing.handle('album')}
                </th>
              )}
              <th className="py-3" scope="col">
                来源{!mobile && columnSizing.handle('source')}
              </th>
              {showLikes && (
                <th scope="col">
                  <span className="sr-only">喜欢</span>
                  {!mobile && columnSizing.handle('likes')}
                </th>
              )}
              {showDuration && (
                <th
                  className="py-3"
                  scope="col"
                  aria-sort={ariaSort(sortable && sort === 'duration')}
                >
                  {sortable ? (
                    <SortHeader
                      compact
                      label="时长"
                      active={sort === 'duration'}
                      descending={descending}
                      detail={sort === 'duration' ? (descending ? '降序' : '升序') : '默认'}
                      next={nextLabel('duration')}
                      onClick={() => cycleSort('duration')}
                    />
                  ) : (
                    '时长'
                  )}
                  {!mobile && columnSizing.handle('duration')}
                </th>
              )}
              {extraColumns.map((column, index) => (
                <th key={column.label} className="py-3" scope="col">
                  {column.label}
                  {!mobile && columnSizing.handle(`extra-${index}`)}
                </th>
              ))}
              {onAppend && (
                <th scope="col">
                  <span className="sr-only">加入队列</span>
                </th>
              )}
            </tr>
          </thead>
          <tbody ref={body}>
            {loading &&
              Array.from({ length: skeletonCount }, (_, index) => (
                <tr key={`loading-${index}`} className="track-row" aria-hidden="true">
                  {(!mobile || selection) && (
                    <td className="text-center">
                      <Skeleton className="mx-auto h-3 w-3" />
                    </td>
                  )}
                  <td className="track-identity-cell pr-4">
                    <div className="track-identity flex min-w-0 items-center" data-mode={cardMode}>
                      <Skeleton className="track-identity-cover shrink-0 rounded-lg" />
                      <div className="min-w-0 flex-1">
                        <div className="flex h-5 items-center">
                          <Skeleton className="h-3 w-3/4" />
                        </div>
                        <div className="track-identity-artist flex h-4 items-center">
                          <Skeleton className="h-3 w-2/5" />
                        </div>
                      </div>
                    </div>
                  </td>
                  {!mobile && (
                    <td className="pr-4">
                      <Skeleton className="h-3 w-2/3" />
                    </td>
                  )}
                  <td>
                    <Skeleton className="h-3 w-12" />
                  </td>
                  {showLikes && (
                    <td>
                      <Skeleton className="mx-auto size-4 rounded-full" />
                    </td>
                  )}
                  {showDuration && (
                    <td>
                      <Skeleton className="h-3 w-8" />
                    </td>
                  )}
                  {extraColumns.map((column) => (
                    <td key={column.label}>
                      <Skeleton className="h-3 w-2/3" />
                    </td>
                  ))}
                  {onAppend && (
                    <td>
                      <Skeleton className="mx-auto size-4" />
                    </td>
                  )}
                </tr>
              ))}
            {top > 0 && (
              <tr aria-hidden="true">
                <td colSpan={columns} style={{ height: top, padding: 0 }} />
              </tr>
            )}
            {!loading &&
              rows.map((row) => {
                const entry = entries[row.index]
                const { index, track, rowKey } = entry
                const playIndex = entries.indexOf(entry)
                const account = getAccount(onlineReference(track)?.source ?? '')
                const { profile, likedIds, likesReady, pendingLikes, toggleLike, canToggleLike } =
                  account
                const favorite = favoriteId(track, profile?.reference)
                const current = isCurrentTrack
                  ? isCurrentTrack(track)
                  : currentIndex === undefined
                    ? track.key === currentKey
                    : index === currentIndex
                return (
                  <SongContextMenu
                    key={rowKey}
                    track={track}
                    onPlay={() => onPlay(playIndex, visibleTracks)}
                    busy={busy}
                    onRemove={
                      onRemove && canRemove?.(track) !== false
                        ? () => onRemove(track, index)
                        : undefined
                    }
                    removeLabel={removeLabel}
                    render={
                      <tr
                        ref={virtualizer.measureElement}
                        data-index={row.index}
                        data-selected={selectedKey === rowKey}
                        aria-selected={selectedKey === rowKey}
                        onDoubleClick={(event) => {
                          const control = (event.target as HTMLElement).closest(
                            'button, a, input, select, textarea, [role="button"]'
                          )
                          if (event.button !== 0 || (control && control !== event.currentTarget))
                            return
                          event.preventDefault()
                          setSelectedKey(rowKey)
                          doubleClickPlay(playIndex)
                        }}
                        onClick={(event) => {
                          if (
                            !(event.target as HTMLElement).closest(
                              'button, a, input, [role="button"]'
                            )
                          )
                            setSelectedKey(rowKey)
                        }}
                        onKeyDown={(event) => {
                          if (event.target !== event.currentTarget) return
                          if (event.key === 'Enter') {
                            event.preventDefault()
                            setSelectedKey(rowKey)
                          } else if (event.key === 'Escape') setSelectedKey(undefined)
                        }}
                        className="track-row group"
                        data-current={current}
                        aria-current={current ? 'true' : undefined}
                      />
                    }
                  >
                    {(!mobile || selection) && (
                      <td className="text-center tabular-nums text-muted-foreground">
                        {selection ? (
                          <input
                            type="checkbox"
                            aria-label={`选择 ${track.title}`}
                            disabled={busy}
                            checked={selection.keys.has(track.key)}
                            onClick={(event) => event.stopPropagation()}
                            onChange={(event) =>
                              selection.onToggle(track.key, event.target.checked)
                            }
                          />
                        ) : current ? (
                          <PlayingIndicator />
                        ) : (
                          offset + row.index + 1
                        )}
                      </td>
                    )}
                    <td className="track-identity-cell pr-4">
                      <TrackIdentity
                        track={track}
                        mode={cardMode}
                        cover={
                          <button
                            type="button"
                            className="track-cover"
                            disabled={busy}
                            aria-label={`播放 ${track.title}`}
                            onClick={() => onPlay(playIndex, visibleTracks)}
                          >
                            <Cover
                              size="small"
                              cover={track.cover}
                              className="track-identity-cover"
                            />
                            <span className="track-cover-play">
                              <Play aria-hidden="true" />
                            </span>
                          </button>
                        }
                      />
                    </td>
                    {!mobile && (
                      <td className="pr-4 text-muted-foreground" title={track.album}>
                        <div className="track-album-text">
                          <TrackAlbum track={track} />
                        </div>
                      </td>
                    )}
                    <td className="pr-3">
                      {renderSource ? renderSource(track) : <TrackSourceLabel track={track} />}
                    </td>
                    {showLikes && (
                      <td>
                        {favorite !== undefined && (
                          <ActionButton
                            variant="ghost"
                            size="icon-sm"
                            className="track-like"
                            data-liked={likedIds.has(favorite)}
                            aria-pressed={likedIds.has(favorite)}
                            disabled={
                              busy ||
                              !profile ||
                              !likesReady ||
                              pendingLikes.has(favorite) ||
                              canToggleLike === false
                            }
                            aria-label={`${likedIds.has(favorite) ? '取消喜欢' : '喜欢'} ${track.title}`}
                            title={
                              !profile
                                ? '登录后收藏歌曲'
                                : canToggleLike === false
                                  ? '收藏状态（此来源仅支持读取）'
                                  : '喜欢 / 取消喜欢'
                            }
                            onClick={() => {
                              if (favorite !== undefined) {
                                void toggleLike(favorite).catch((cause) => {
                                  reportCaughtError(
                                    'app/src/components/music/track-list.tsx:654',
                                    cause
                                  )
                                  errorNotices.push(errorText(cause))
                                })
                              }
                            }}
                          >
                            <Heart aria-hidden="true" />
                          </ActionButton>
                        )}
                      </td>
                    )}
                    {showDuration && (
                      <td className="tabular-nums text-muted-foreground">
                        {track.durationMs > 0 ? formatTime(track.durationMs) : '—'}
                      </td>
                    )}
                    {extraColumns.map((column) => (
                      <td key={column.label} className="tabular-nums text-muted-foreground">
                        {column.render(track, index)}
                      </td>
                    ))}
                    {onAppend && (
                      <td>
                        <ActionButton
                          variant="ghost"
                          size="icon-sm"
                          disabled={busy}
                          aria-label={`下一首播放 ${track.title}`}
                          title="下一首播放"
                          onClick={() => onAppend(track)}
                        >
                          <ListPlus aria-hidden="true" />
                        </ActionButton>
                      </td>
                    )}
                  </SongContextMenu>
                )
              })}
            {bottom > 0 && (
              <tr aria-hidden="true">
                <td colSpan={columns} style={{ height: bottom, padding: 0 }} />
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  )
})
