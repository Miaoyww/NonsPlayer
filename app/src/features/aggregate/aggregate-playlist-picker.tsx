import { CreatePlaylist } from '@/features/library/create-playlist'
import { onlineReference } from '@/features/music/legacy-reference'
import { ownedPlaylistTargets, addToPlaylist, type PlaylistTarget } from './playlist-targets'
import { useMusicSources } from '@/features/music/use-music-sources'
import { useAggregateReplacement } from './aggregate-library'
import { writeAggregate, useAggregateLibrary } from '@/features/aggregate/aggregate-library'
import { presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useCollectionActions } from '@/features/library/collection-actions'
import { nativeCall, errorText, type Track } from '@/lib/player'
import type { LocalEntity } from '@/features/local/local-library'
import { usePagedList } from '@/lib/use-paged-list'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { InfiniteLoad } from '@/components/music/infinite-load'
export function AggregatePlaylistPicker({
  track,
  tracks,
  onClose,
  onNotice
}: {
  track?: Track
  tracks?: Track[]
  onClose: () => void
  onNotice: (message: string) => void
}) {
  const { reload } = useAggregateLibrary()
  const { changed } = useCollectionActions()
  const replacement = useAggregateReplacement()
  const { statuses, revision, preferences } = useMusicSources()
  const [query, setQuery] = useState('')
  const [retry, setRetry] = useState(0)
  const [targets, setTargets] = useState<PlaylistTarget[]>([])
  const [targetError, setTargetError] = useState<string>()
  const [loadingTargets, setLoadingTargets] = useState(false)
  const selected = tracks ?? (track ? [track] : [])
  useEffect(() => {
    let disposed = false
    setTargets([])
    setTargetError(undefined)
    if (!track && !tracks?.length) return
    setLoadingTargets(true)
    void (async () => {
      const results = await Promise.allSettled(
        statuses
          .filter(
            (s) =>
              s.enabled &&
              s.descriptor.capabilities.includes('userLibrary') &&
              s.descriptor.capabilities.includes('browse')
          )
          .map((s) =>
            ownedPlaylistTargets(
              s.descriptor.source,
              s.descriptor.capabilities.includes('playlistWrite')
            )
          )
      )
      if (disposed) return
      setTargets(results.flatMap((result) => (result.status === 'fulfilled' ? result.value : [])))
      setTargetError(
        results
          .filter((result) => result.status === 'rejected')
          .map((result) => errorText((result as PromiseRejectedResult).reason))
          .join('；') || undefined
      )
      setLoadingTargets(false)
    })()
    return () => {
      disposed = true
    }
  }, [track, tracks, revision, retry])
  async function addPlatform(target: PlaylistTarget) {
    if (busy) return
    setBusy(true)
    setError(undefined)
    try {
      await addToPlaylist(target, selected, replacement(target.item)?.id)
      await reload()
      changed()
      onNotice(`已添加到「${target.item.name}」。`)
      onClose()
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  const [name, setName] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  useEffect(() => {
    setName('')
    setQuery('')
    setError(undefined)
  }, [track, tracks])
  const loader = useCallback(
    (offset: number) =>
      nativeCall<{ items: LocalEntity[]; more: boolean }>('local_entities', {
        kind: 'playlist',
        keyword: '',
        offset
      }),
    [track, tracks]
  )
  const list = usePagedList<LocalEntity>(loader, 30, !!track || !!tracks?.length, {
    cacheKey: presentationKey('aggregate-playlist-picker')
  })
  const rows = [
    ...targets
      .filter(
        (target) =>
          !target.item.liked ||
          selected.every((t) => onlineReference(t)?.source === target.account.source)
      )
      .map((target) => ({
        id: `${target.account.source}:${target.item.id}`,
        name: target.item.name,
        cover: target.item.cover,
        subtitle: replacement(target.item)
          ? '聚合歌单'
          : (statuses.find((s) => s.descriptor.source === target.account.source)?.descriptor
              .displayName ?? target.account.source),
        run: () => addPlatform(target)
      })),
    ...list.items
      .filter((item) => !targets.some((target) => replacement(target.item)?.id === item.id))
      .map((item) => ({
        id: item.id,
        name: item.name,
        cover: item.cover,
        subtitle: `${item.trackCount} 首`,
        run: () => add(item.id)
      }))
  ].filter((item) => item.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
  const scroll = useRef<HTMLDivElement>(null)
  const virtual = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scroll.current,
    estimateSize: () => 64,
    overscan: 5
  })
  useEffect(() => {
    scroll.current?.scrollTo({ top: 0 })
  }, [query])
  async function add(id?: string) {
    if ((!track && !tracks?.length) || busy) return
    setBusy(true)
    setError(undefined)
    try {
      const keys = (tracks ?? (track ? [track] : [])).map((item) => item.key)
      if (id) await writeAggregate({ operation: 'add', id, keys })
      else await writeAggregate({ operation: 'create', name, keys })
      await reload()
      onNotice('已添加到聚合歌单。')
      onClose()
    } catch (cause) {
      reportCaughtError('app/src/features/aggregate/aggregate-playlist-picker.tsx', cause)

      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      open={!!track || !!tracks?.length}
      onOpenChange={(open) => {
        if (!open && !busy) onClose()
      }}
    >
      <DialogContent className="max-h-[80dvh] overflow-auto">
        <DialogHeader>
          <DialogTitle>添加到歌单</DialogTitle>
          <DialogDescription>
            {tracks?.length ? `${tracks[0].album} · ${tracks.length} 首` : track?.title}
          </DialogDescription>
        </DialogHeader>
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault()
            void add()
          }}
        >
          <Input
            aria-label="新歌单名称"
            placeholder="新聚合歌单名称"
            value={name}
            maxLength={100}
            disabled={busy}
            onChange={(event) => setName(event.target.value)}
          />
          <ActionButton type="submit" variant="outline" disabled={busy || !name.trim()}>
            创建并添加
          </ActionButton>
        </form>
        <div className="flex items-center gap-2">
          <Input
            aria-label="搜索歌单"
            placeholder="搜索歌单"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          {preferences &&
            targets.some(
              (target) => target.account.source === preferences.current && target.writable
            ) && (
              <CreatePlaylist
                iconOnly
                source={preferences.current}
                disabled={busy}
                onCreated={() => {
                  setRetry((v) => v + 1)
                  changed()
                }}
              />
            )}
        </div>
        {loadingTargets && <p role="status">正在读取平台歌单…</p>}
        {busy && <p role="status">正在添加；首次加入其他来源的歌曲时会先完整读取原歌单…</p>}
        {targetError && (
          <div className="flex gap-2">
            <ErrorNotice error={targetError} />
            <ActionButton
              variant="ghost"
              disabled={busy || loadingTargets}
              onClick={() => setRetry((v) => v + 1)}
            >
              重试
            </ActionButton>
          </div>
        )}
        <div ref={scroll} className="h-80 overflow-auto" aria-label="选择目标歌单" aria-busy={busy}>
          <div style={{ height: virtual.getTotalSize(), position: 'relative' }}>
            {virtual.getVirtualItems().map((row) => {
              const item = rows[row.index]
              return (
                <ActionButton
                  key={item.id}
                  variant="ghost"
                  className="absolute left-0 w-full justify-start gap-3"
                  style={{ top: row.start, height: row.size }}
                  disabled={busy}
                  onClick={() => void item.run()}
                >
                  <Cover cover={item.cover} className="size-10 shrink-0" />
                  <span className="min-w-0 truncate">{item.name}</span>
                  <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                    {item.subtitle}
                  </span>
                </ActionButton>
              )
            })}
          </div>
          <InfiniteLoad
            more={list.more}
            busy={list.busy}
            error={list.error}
            onLoad={list.loadMore}
          />
        </div>
        {!list.busy && !list.items.length && (
          <p className="text-sm text-muted-foreground">还没有本机歌单，可以在上方创建。</p>
        )}
        {error && <ErrorNotice error={error} />}
      </DialogContent>
    </Dialog>
  )
}
