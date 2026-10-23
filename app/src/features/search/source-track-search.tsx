import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useMemo, useState, useSyncExternalStore } from 'react'
import { musicClient } from '@/features/music/client'
import { searchResourceProvider } from '@/features/music/resource-providers'
import { AggregateQuery, type SearchSource } from './aggregate-query'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { ActionButton } from '@/components/music/action-button'
import { Input } from '@/components/ui/input'
import { errorText, type Track } from '@/lib/player'
import { Search } from 'lucide-react'

export function SourceTrackSearch({
  initialKeyword,
  sources,
  busy,
  onSelect
}: {
  initialKeyword: string
  sources: SearchSource[]
  busy: boolean
  onSelect: (track: Track, provider?: string) => void
}) {
  const [input, setInput] = useState(initialKeyword)
  const [keyword, setKeyword] = useState(initialKeyword)
  const query = useMemo(
    () =>
      new AggregateQuery<Track>(
        sources,
        async (key, cursor) => {
          const source = sources.find((s) => s.source === key)!
          if (source.provider) return searchResourceProvider(source, keyword, cursor)
          const response = await musicClient.query(key, {
            operation: 'search',
            keyword,
            kind: 'track',
            page: { cursor, limit: 50 }
          })
          if (response.type !== 'tracks') throw new Error('搜索结果类型错误')
          return {
            items: await musicClient.queueTracks(key, response.data.items),
            nextCursor: response.data.nextCursor
          }
        },
        (track) => track.key,
        errorText,
        300
      ),
    [sources, keyword]
  )
  const state = useSyncExternalStore(query.subscribe, query.getSnapshot)
  useEffect(() => {
    query.start()
    if (keyword.trim()) query.loadVisible('all')
    return () => query.dispose()
  }, [query, keyword])
  const tracks = state.items.map((s) => s.item)
  const origins = useMemo(
    () => new Map(state.items.map(({ source, item }) => [item, source])),
    [state.items]
  )
  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col" aria-label="搜索其它平台歌曲">
      <form
        className="mb-3 flex shrink-0 gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          if (input.trim()) setKeyword(input.trim())
        }}
      >
        <Input
          className="min-w-0 flex-1"
          aria-label="歌曲或歌手"
          value={input}
          maxLength={85}
          onChange={(event) => setInput(event.target.value)}
        />
        <ActionButton type="submit" variant="secondary" disabled={busy || !input.trim()}>
          <Search aria-hidden="true" />
          搜索
        </ActionButton>
      </form>
      <div className="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain">
        <TrackList
          doubleClickMode="primary"
          fitContainer
          tracks={tracks}
          busy={busy}
          showLikes={false}
          searchable={false}
          sortable={false}
          getRowKey={(track) => `${origins.get(track)}:${track.key}`}
          loading={!tracks.length && state.sources.some((s) => s.busy)}
          renderSource={(track) => (
            <span className="text-xs text-muted-foreground">
              {sources.find((s) => s.source === origins.get(track))?.name}
            </span>
          )}
          onPlay={(_index, visible) => {
            const track = visible[_index]
            const origin = state.items.find((item) => item.item === track)?.source
            onSelect(track, sources.find((s) => s.source === origin)?.provider)
          }}
        />
        <InfiniteLoad
          more={state.sources.some((s) => s.more && !s.error)}
          busy={state.sources.some((s) => s.busy)}
          onLoad={() => query.loadVisible('all')}
        />
        {state.sources
          .filter((s) => s.error)
          .map((s) => (
            <div key={s.source} className="flex items-center gap-2 py-2 text-sm">
              <ErrorNotice error={`${s.name}：${s.error}`} />
              <ActionButton
                size="sm"
                variant="ghost"
                onClick={() => void query.load(s.source, true)}
              >
                重试
              </ActionButton>
            </div>
          ))}
        {!sources.length && <p className="py-3 text-muted-foreground">没有已启用的可搜索来源。</p>}
        {!!sources.length && !tracks.length && !state.sources.some((s) => s.busy || s.error) && (
          <p className="py-3 text-muted-foreground">没有找到对应歌曲，请尝试其它关键词。</p>
        )}
      </div>
    </section>
  )
}
