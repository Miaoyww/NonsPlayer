import { useState } from 'react'
import { ChevronDown } from 'lucide-react'
import { TrackList } from '@/components/music/track-list'
import { TrackSourceLabel } from '@/components/music/track-identity'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { isTauri } from '@tauri-apps/api/core'
import { formatTime, nativeCall, type Track } from '@/lib/player'
import { isAndroid } from '@/lib/platform'
import type { SearchTrackGroup } from './track-aggregation'

export function AggregateTrackResults({
  groups,
  currentKey,
  hasMore,
  onError,
  onNotice
}: {
  groups: SearchTrackGroup[]
  currentKey?: string
  hasMore: boolean
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const [selected, setSelected] = useState<string>()
  const selectedGroup = groups.find((group) => group.id === selected)
  const byTrack = new Map(groups.map((group) => [group.representative.key, group]))
  const play = (index: number, values: Track[]) => {
    const start = Math.max(0, index - 999)
    void nativeCall('play_queue', {
      keys: values.slice(start, start + 1000).map((track) => track.key),
      index: index - start
    }).catch(onError)
  }
  const append = (track: Track) =>
    void nativeCall('append_queue', { keys: [track.key] })
      .then(() => onNotice(`已将「${track.title}」设为下一首播放。`))
      .catch(onError)
  return (
    <>
      <TrackList
        tracks={groups.map((group) => group.representative)}
        hasMore={hasMore}
        busy={!isTauri()}
        getRowKey={(track) => byTrack.get(track.key)!.id}
        isCurrentTrack={(track) =>
          byTrack.get(track.key)!.members.some((member) => member.track.key === currentKey)
        }
        onPlay={play}
        onAppend={append}
        renderSource={(track) => {
          const group = byTrack.get(track.key)!
          if (group.members.length === 1) return <TrackSourceLabel track={track} />
          return (
            <button
              type="button"
              className="flex min-h-8 w-full cursor-pointer items-center gap-1 rounded-md px-1 text-left hover:bg-accent focus-visible:outline-2 focus-visible:outline-ring"
              aria-label={`查看「${track.title}」的 ${group.sourceCount} 个来源，共 ${group.members.length} 首歌曲`}
              aria-haspopup="dialog"
              title="查看已加载的来源歌曲"
              onClick={(event) => {
                event.stopPropagation()
                setSelected(group.id)
              }}
            >
              <span className="min-w-0 flex-1">
                <TrackSourceLabel track={track} />
              </span>
              <span className="flex size-5 shrink-0 items-center justify-center rounded-full border text-[11px] tabular-nums">
                {group.sourceCount}
              </span>
              <ChevronDown className="size-3 shrink-0 text-muted-foreground" aria-hidden="true" />
            </button>
          )
        }}
      />
      <Dialog
        open={!!selectedGroup}
        onOpenChange={(open) => {
          if (!open) setSelected(undefined)
        }}
      >
        <DialogContent className="max-h-[calc(100dvh-2rem)] grid-rows-[auto_minmax(0,1fr)] sm:max-w-6xl">
          <DialogHeader className="min-w-0 pr-6">
            <DialogTitle>{selectedGroup?.representative.title} · 来源歌曲</DialogTitle>
            <DialogDescription>
              已加载 {selectedGroup?.sourceCount ?? 0} 个来源的 {selectedGroup?.members.length ?? 0}{' '}
              首歌曲。播放仅播放所选单曲，收藏按歌曲所属来源操作。
            </DialogDescription>
          </DialogHeader>
          <div className="min-h-0 min-w-0 overflow-y-auto px-3">
            {selectedGroup && (
              <TrackList
                fitContainer
                tracks={selectedGroup.members.map((member) => member.track)}
                busy={!isTauri()}
                currentKey={currentKey}
                onPlay={(index, values) => play(0, [values[index]])}
                onAppend={append}
                renderSource={(track) => (
                  <div className="space-y-1">
                    <TrackSourceLabel track={track} />
                    {isAndroid() && (
                      <span className="block text-xs text-muted-foreground">
                        {track.album || '未知专辑'} · {formatTime(track.durationMs)}
                      </span>
                    )}
                    {track.key === selectedGroup.representative.key && (
                      <span className="text-xs text-muted-foreground">主列表展示</span>
                    )}
                  </div>
                )}
              />
            )}
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
}
