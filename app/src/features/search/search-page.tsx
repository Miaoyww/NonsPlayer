import { useEffect, useMemo, useState, useSyncExternalStore } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Disc3, ListMusic, Mic2, Music2, RefreshCw, Search } from 'lucide-react'
import { Tabs, TabsList, TabsTab } from '@/components/animate-ui/components/base/tabs'
import { errorText, nativeCall, usePlayer, type Track } from '@/lib/player'
import { playLibraryCollection } from '@/features/library/library-api'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'
import { TrackList } from '@/components/music/track-list'
import { TrackListSkeleton, CollectionGridSkeleton } from '@/components/music/loading'
import { PlaylistCard } from '@/features/library/playlist-card'
import { AlbumCard } from '@/features/library/album-card'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { ActionButton } from '@/components/music/action-button'
import { musicClient } from '@/features/music/client'
import { musicCollection } from '@/features/music/entities'
import { useMusicSources } from '@/features/music/use-music-sources'
import { AggregateQuery, type SearchSource } from './aggregate-query'

const categories = [
  { value: 'track', label: '单曲', icon: Music2 },
  { value: 'playlist', label: '歌单', icon: ListMusic },
  { value: 'artist', label: '歌手', icon: Mic2 },
  { value: 'album', label: '专辑', icon: Disc3 }
] as const
type SearchKind = (typeof categories)[number]['value']
type Result = Track | MusicCollection

function Results({
  kind,
  keyword,
  sources,
  onError,
  onNotice
}: {
  kind: SearchKind
  keyword: string
  sources: SearchSource[]
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { navigate } = useMusicNavigation()
  const player = usePlayer('index', 'queue')
  const [filter, setFilter] = useState('all')
  const [playing, setPlaying] = useState(false)
  const query = useMemo(
    () =>
      new AggregateQuery<Result>(
        sources,
        async (source, cursor) => {
          const response = await musicClient.query(source, {
            operation: 'search',
            keyword,
            kind,
            page: { cursor, limit: kind === 'track' ? 50 : 30 }
          })
          if (kind === 'track' && response.type === 'tracks')
            return {
              items: await musicClient.queueTracks(source, response.data.items),
              nextCursor: response.data.nextCursor
            }
          if (kind !== 'track' && response.type === 'entities')
            return {
              items: response.data.items.map(musicCollection),
              nextCursor: response.data.nextCursor
            }
          throw new Error('音乐来源返回了错误的搜索结果')
        },
        (item) => ('key' in item ? item.key : JSON.stringify(item.reference)),
        errorText
      ),
    [sources, keyword, kind]
  )
  const state = useSyncExternalStore(query.subscribe, query.getSnapshot)
  useEffect(() => {
    query.start()
    if (isTauri() && keyword) query.loadVisible('all')
    return () => query.dispose()
  }, [query, keyword])
  const visibleSources = state.sources.filter(
    (value) => filter === 'all' || filter === value.source
  )
  const items = state.items
    .filter((value) => filter === 'all' || value.source === filter)
    .map((value) => value.item)
  const busy = visibleSources.some((value) => value.busy)
  const more = visibleSources.some((value) => value.more && !value.error)
  const tracks = items as Track[]
  async function playCollection(item: MusicCollection) {
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
  const open = (item: MusicCollection) => navigate('collection', '', item)
  return (
    <>
      <div className="mb-3 flex flex-wrap gap-2" role="group" aria-label="搜索来源筛选">
        {[{ source: 'all', name: '综合' }, ...sources].map((value) => (
          <ActionButton
            key={value.source}
            size="sm"
            variant={filter === value.source ? 'secondary' : 'ghost'}
            aria-pressed={filter === value.source}
            onClick={() => {
              setFilter(value.source)
              query.loadVisible(value.source)
            }}
          >
            {value.name}
          </ActionButton>
        ))}
      </div>
      <div
        className="mb-5 flex flex-wrap gap-x-5 gap-y-2 text-sm text-muted-foreground"
        role="status"
      >
        {state.sources.map((value) => (
          <div key={value.source}>
            <span>
              {value.name} ·{' '}
              {value.unavailable ??
                (value.busy
                  ? '正在搜索…'
                  : value.error
                    ? `请求失败${value.count ? `，已保留 ${value.count} 项` : ''}`
                    : value.count
                      ? `已加载 ${value.count} 项`
                      : '暂无结果')}
              {value.capped ? '（已达本次上限）' : ''}
            </span>
            {value.error && (
              <ActionButton
                size="sm"
                variant="ghost"
                aria-label={`重试${value.name}搜索`}
                title={value.error}
                onClick={() => void query.load(value.source, true)}
              >
                <RefreshCw aria-hidden="true" />
                重试
              </ActionButton>
            )}
          </div>
        ))}
      </div>
      <p className="mb-3 text-xs text-muted-foreground">来源推荐顺序</p>
      {busy && !items.length ? (
        kind === 'track' ? (
          <TrackListSkeleton sortable={false} />
        ) : (
          <CollectionGridSkeleton artist={kind === 'artist'} />
        )
      ) : items.length ? (
        kind === 'track' ? (
          <TrackList
            tracks={tracks}
            hasMore={more}
            busy={false}
            currentKey={player.index === null ? undefined : player.queue[player.index]?.key}
            onPlay={(index, values) => {
              const start = Math.max(0, index - 999)
              void nativeCall('play_queue', {
                keys: values.slice(start, start + 1000).map((track) => track.key),
                index: index - start
              }).catch(onError)
            }}
            onAppend={(track) =>
              void nativeCall('append_queue', { keys: [track.key] })
                .then(() => onNotice(`已将「${track.title}」设为下一首播放。`))
                .catch(onError)
            }
          />
        ) : (
          <div className="discover-playlist-grid">
            {(items as MusicCollection[]).map((item) => {
              const sourceName = sources.find(
                (value) => value.source === item.reference?.source
              )?.name
              return kind === 'album' ? (
                <AlbumCard
                  key={JSON.stringify(item.reference)}
                  item={item}
                  sourceName={sourceName}
                  busy={playing || !isTauri()}
                  onOpen={open}
                  onPlay={(item) => void playCollection(item)}
                />
              ) : (
                <PlaylistCard
                  key={JSON.stringify(item.reference)}
                  item={item}
                  sourceName={sourceName}
                  busy={playing || !isTauri()}
                  onOpen={open}
                  onPlay={(item) => void playCollection(item)}
                />
              )
            })}
          </div>
        )
      ) : (
        <div className="search-empty">
          <Search className="size-9" aria-hidden="true" />
          <p>
            {!isTauri()
              ? '在桌面应用中搜索音乐'
              : visibleSources.some((value) => value.error || value.unavailable)
                ? '部分来源不可用，请查看来源状态并重试。'
                : '没有找到匹配结果'}
          </p>
          <span>试试其他关键词，或切换搜索类型。</span>
        </div>
      )}
      <InfiniteLoad more={more} busy={busy} onLoad={() => query.loadVisible(filter)} />
      {visibleSources
        .filter((value) => value.error)
        .map((value) => (
          <div
            key={value.source}
            className="flex items-center justify-center gap-2 py-3 text-sm"
            role="alert"
          >
            <span>
              {value.name}：{value.error}
            </span>
            <ActionButton
              size="sm"
              variant="ghost"
              onClick={() => void query.load(value.source, true)}
            >
              <RefreshCw aria-hidden="true" />
              重试
            </ActionButton>
          </div>
        ))}
    </>
  )
}

export default function SearchPage({
  onError,
  onNotice
}: {
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { page } = useMusicNavigation()
  const [kind, setKind] = useState<SearchKind>('track')
  const { preferences, statuses, revision, error } = useMusicSources()
  const sources = useMemo(
    () =>
      preferences
        ? [preferences.current, ...preferences.supplementary].map((source) => {
            const status = statuses.find((value) => value.descriptor.source === source)
            return {
              source,
              name: status?.descriptor.displayName ?? source,
              unavailable: !status
                ? '来源未安装'
                : !status.enabled
                  ? '来源已停用'
                  : !status.descriptor.capabilities.includes('search')
                    ? '不支持搜索'
                    : undefined
            }
          })
        : [],
    [preferences, statuses]
  )
  return (
    <MusicPage className="search-page" aria-label="搜索音乐">
      <MusicPageHeader title={page.query || '搜索音乐'}>
        <p>
          {page.query
            ? `聚合搜索 · ${sources.map((value) => value.name).join('、')}`
            : '输入歌曲、歌手、歌单或专辑名称开始搜索。'}
        </p>
      </MusicPageHeader>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      <Tabs value={kind} onValueChange={(value) => setKind(value as SearchKind)} className="gap-0">
        <div className="music-page-section-bar">
          <h2>搜索结果</h2>
          <TabsList className="music-tabs" aria-label="搜索结果类型">
            {categories.map(({ value, label, icon: Icon }) => (
              <TabsTab key={value} value={value}>
                <Icon aria-hidden="true" />
                {label}
              </TabsTab>
            ))}
          </TabsList>
        </div>
      </Tabs>
      {sources.length > 0 && page.query ? (
        <Results
          key={`${page.query}:${kind}:${revision}`}
          kind={kind}
          keyword={page.query}
          sources={sources}
          onError={onError}
          onNotice={onNotice}
        />
      ) : (
        <div className="search-empty">
          <p>
            {page.query && isTauri() ? '正在读取音乐来源…' : '在顶部搜索框输入关键词开始搜索。'}
          </p>
        </div>
      )}
    </MusicPage>
  )
}
