import { presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useCallback, useEffect, useState } from 'react'
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
export function LocalPlaylistPicker({
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
  const [name, setName] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  useEffect(() => {
    setName('')
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
    cacheKey: presentationKey('local-playlist-picker')
  })
  async function add(id?: string) {
    if ((!track && !tracks?.length) || busy) return
    setBusy(true)
    setError(undefined)
    try {
      id ??= await nativeCall<string>('create_local_playlist', { name })
      for (const item of tracks ?? (track ? [track] : [])) {
        await nativeCall('add_local_playlist_track', { id, key: item.key })
      }
      onNotice('已添加到本地歌单。')
      onClose()
    } catch (cause) {
      reportCaughtError('app/src/features/local/local-playlist-picker.tsx', cause)

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
          <DialogTitle>添加到本地歌单</DialogTitle>
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
            placeholder="新歌单名称"
            value={name}
            maxLength={100}
            disabled={busy}
            onChange={(event) => setName(event.target.value)}
          />
          <ActionButton type="submit" variant="outline" disabled={busy || !name.trim()}>
            创建并添加
          </ActionButton>
        </form>
        <div className="flex flex-col gap-2">
          {list.items.map((item) => (
            <ActionButton
              key={item.id}
              variant="ghost"
              className="h-auto justify-start gap-3 py-2"
              disabled={busy}
              onClick={() => void add(item.id)}
            >
              <Cover cover={item.cover} className="size-10" />
              <span className="min-w-0 truncate">{item.name}</span>
              <span className="ml-auto text-xs text-muted-foreground">{item.trackCount} 首</span>
            </ActionButton>
          ))}
        </div>
        {!list.busy && !list.items.length && (
          <p className="text-sm text-muted-foreground">还没有本地歌单，可以在上方创建。</p>
        )}
        <InfiniteLoad more={list.more} busy={list.busy} error={list.error} onLoad={list.loadMore} />
        {error && <ErrorNotice error={error} />}
      </DialogContent>
    </Dialog>
  )
}
