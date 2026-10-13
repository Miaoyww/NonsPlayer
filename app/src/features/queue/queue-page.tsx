import { useMemo, useState } from 'react'
import { Search } from 'lucide-react'
import { nativeCall, usePlayer } from '@/lib/player'
import { QueueControls } from '@/features/queue/queue-controls'
import { TrackList } from '@/components/music/track-list'

export function QueuePage({ onError }: { onError: (error: unknown) => void }) {
  const state = usePlayer('index', 'queue')
  const [query, setQuery] = useState('')
  const [locate, setLocate] = useState(0)
  const matches = useMemo(() => {
    const keyword = query.trim().toLocaleLowerCase()
    return state.queue
      .map((track, index) => ({ track, index }))
      .filter(
        ({ track }) =>
          !keyword ||
          [track.title, ...(track.aliases ?? []), track.artist].some((text) =>
            text.toLocaleLowerCase().includes(keyword)
          )
      )
  }, [state.queue, query])
  const tracks = useMemo(() => matches.map(({ track }) => track), [matches])
  const currentIndex = matches.findIndex(({ index }) => index === state.index)
  return (
    <>
      <header className="queue-page-header flex flex-wrap shrink-0 items-center justify-between gap-4 px-8 pb-6 pt-8">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">播放列表</h1>
          <p className="mt-2 text-sm text-muted-foreground">{state.queue.length} 首歌</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 rounded-lg border border-input px-3 py-2 focus-within:ring-2 focus-within:ring-ring">
            <Search aria-hidden="true" className="size-4 text-muted-foreground" />
            <input
              type="search"
              aria-label="搜索当前播放列表"
              placeholder="搜索播放列表"
              className="w-44 bg-transparent text-sm outline-none"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <QueueControls
            count={state.queue.length}
            canLocate={state.index !== null}
            onLocate={() => {
              setQuery('')
              setLocate((value) => value + 1)
            }}
            onError={onError}
          />
        </div>
      </header>
      <div className="queue-page-scroll relative isolate min-h-0 flex-1 overflow-auto px-8">
        {tracks.length ? (
          <TrackList
            tracks={tracks}
            currentIndex={currentIndex}
            busy={false}
            locateRequest={locate}
            onRemove={(track, index) =>
              nativeCall('remove_queue_track', { index: matches[index].index, key: track.key })
            }
            onPlay={(index) =>
              void nativeCall('player_jump', { index: matches[index].index }).catch(onError)
            }
          />
        ) : (
          <p className="py-20 text-center text-muted-foreground">
            {state.queue.length ? '没有找到匹配的歌曲' : '播放列表还是空的'}
          </p>
        )}
      </div>
    </>
  )
}
