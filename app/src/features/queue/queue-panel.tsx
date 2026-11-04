import { useVirtualizer } from '@tanstack/react-virtual'
import { ArrowRight, Play } from 'lucide-react'
import { memo, useLayoutEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { formatTime, nativeCall, usePlayer, type Track } from '@/lib/player'
import { Cover } from '@/components/music/cover'
import { PlayingIndicator } from '@/components/music/playing-indicator'
import { SongContextMenu } from '@/components/music/song-actions'
import { trackDisplayTitle } from '@/components/music/track-title'
import { TrackIdentity } from '@/components/music/track-identity'
import { useInterfaceDensity } from '@/features/settings/appearance/use-interface-density'
import { QueueControls } from './queue-controls'

const QueuePanelTrackCard = memo(function QueuePanelTrackCard({
  track,
  current,
  onPlay,
  onRemove
}: {
  track: Track
  current: boolean
  onPlay: () => void
  onRemove: () => Promise<unknown>
}) {
  const [density] = useInterfaceDensity()
  return (
    <SongContextMenu
      track={track}
      onPlay={onPlay}
      onRemove={onRemove}
      render={
        <div
          role="group"
          onClick={(event) => {
            if (isTauri() && !(event.target as HTMLElement).closest('button')) onPlay()
          }}
          onKeyDown={(event) => {
            if (
              isTauri() &&
              event.target === event.currentTarget &&
              ['Enter', ' '].includes(event.key)
            ) {
              event.preventDefault()
              onPlay()
            }
          }}
          className="queue-track-card"
          data-mode={density}
          data-current={current}
          aria-current={current ? 'true' : undefined}
          aria-label={`${current ? '当前播放：' : '播放：'}${trackDisplayTitle(track)}，${track.artist}`}
        />
      }
    >
      <div className="min-w-0 flex-1 text-left text-sm">
        <TrackIdentity
          track={track}
          mode={density}
          showSource
          cover={
            <button
              type="button"
              disabled={!isTauri()}
              aria-label={`播放 ${track.title}`}
              onClick={onPlay}
              className="queue-track-cover"
            >
              <Cover size="small" cover={track.cover} className="track-identity-cover" />
              <span className="queue-track-cover-play">
                <Play aria-hidden="true" />
              </span>
            </button>
          }
        />
      </div>
      {current && <PlayingIndicator />}
      <span className="shrink-0 text-sm tabular-nums text-muted-foreground">
        {formatTime(track.durationMs)}
      </span>
    </SongContextMenu>
  )
})

function QueuePanelTrackList({
  onError,
  locateRequest
}: {
  onError: (error: unknown) => void
  locateRequest: number
}) {
  const state = usePlayer('index', 'queue')
  const [density] = useInterfaceDensity()
  const parent = useRef<HTMLDivElement>(null)
  const virtualizer = useVirtualizer({
    count: state.queue.length,
    getScrollElement: () => parent.current,
    estimateSize: () => (density === 'compact' ? 52 : 72),
    overscan: 6
  })
  useLayoutEffect(() => {
    virtualizer.measure()
  }, [density, virtualizer])
  useLayoutEffect(() => {
    if (state.index !== null) virtualizer.scrollToIndex(state.index, { align: 'center' })
  }, [locateRequest, density, virtualizer, state.index])
  return (
    <div ref={parent} className="queue-card-scroll">
      {state.queue.length ? (
        <div role="list" style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
          {virtualizer.getVirtualItems().map((row) => (
            <div
              key={row.index}
              role="listitem"
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: row.size,
                transform: `translateY(${row.start}px)`
              }}
            >
              <QueuePanelTrackCard
                track={state.queue[row.index]}
                current={row.index === state.index}
                onRemove={() =>
                  nativeCall('remove_queue_track', {
                    index: row.index,
                    key: state.queue[row.index].key
                  })
                }
                onPlay={() => void nativeCall('player_jump', { index: row.index }).catch(onError)}
              />
            </div>
          ))}
        </div>
      ) : (
        <p className="py-16 text-center text-sm text-muted-foreground">播放列表还是空的</p>
      )}
    </div>
  )
}

export function QueuePanel({
  onError,
  onPage
}: {
  onError: (error: unknown) => void
  onPage: () => void
}) {
  const state = usePlayer('index', 'queue')
  const [locate, setLocate] = useState(0)
  return (
    <section className="queue-panel" aria-label="播放列表">
      <header className="queue-panel-header">
        <div className="queue-panel-summary">
          <div className="queue-panel-title-row">
            <h2>播放列表</h2>
            <button
              type="button"
              className="queue-panel-open"
              aria-label="打开完整播放列表"
              title="打开完整播放列表"
              onClick={onPage}
            >
              <ArrowRight aria-hidden="true" />
            </button>
          </div>
          <p>{state.queue.length} 首歌</p>
        </div>
        <QueueControls
          count={state.queue.length}
          canLocate={state.index !== null}
          onLocate={() => setLocate((value) => value + 1)}
          onError={onError}
        />
      </header>
      <QueuePanelTrackList onError={onError} locateRequest={locate} />
    </section>
  )
}
