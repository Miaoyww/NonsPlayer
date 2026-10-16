import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { TextSkeleton } from '@/components/music/loading'
import { onlineReference, collectionReference } from '@/features/music/legacy-reference'
import { musicClient } from '@/features/music/client'
import { useEffect, useRef, useState } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { Search } from 'lucide-react'
import { errorText, type Track } from '@/lib/player'
import { musicCollection } from '@/features/music/entities'
import { Dialog, DialogDescription, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { useAccount } from '@/features/account/account'
import { useCollectionActions } from '@/features/library/collection-actions'
import { CreatePlaylist } from '@/features/library/create-playlist'
import { Cover } from '@/components/music/cover'
import { Button as ActionButton } from '@/components/ui/button'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export function PlaylistPicker({
  track,
  onClose,
  onNotice
}: {
  track?: Track
  onClose: () => void
  onNotice: (text: string) => void
}) {
  const { profile, reloadLikes } = useAccount(track ? onlineReference(track)?.source : undefined)
  const { changed } = useCollectionActions()
  const [items, setItems] = useState<MusicCollection[]>([])
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const pending = useRef(false)
  const [error, setError] = useState<string>()
  const [revision, setRevision] = useState(0)
  const account = useRef(profile?.userId)
  account.current = profile?.userId
  const previousAccount = useRef(profile?.userId)
  useEffect(() => {
    if (previousAccount.current !== profile?.userId) {
      previousAccount.current = profile?.userId
      onClose()
    }
  }, [profile?.userId, onClose])
  useEffect(() => {
    let disposed = false
    setItems([])
    setQuery('')
    setError(undefined)
    setLoading(!!track && !!profile)
    if (track && profile)
      void (async () => {
        const all = new Map<number | string, MusicCollection>()
        let cursor: string | null = null
        const seen = new Set<string>()
        for (let offset = 0; offset <= 100000; offset += 30) {
          const result = await musicClient.query(profile.reference.source, {
            operation: 'libraryCollections',
            kind: 'playlist',
            filter: 'created',
            page: { cursor, limit: 30 }
          })
          if (result.type !== 'entities') throw new Error('歌单列表响应无效')
          const page = {
            items: result.data.items.map(musicCollection),
            more: result.data.nextCursor !== null
          }
          cursor = result.data.nextCursor
          if (cursor && seen.has(cursor)) throw new Error('歌单返回重复游标')
          if (cursor) seen.add(cursor)
          if (disposed) return
          page.items
            .filter((item) => item.creatorId === profile.userId)
            .forEach((item) => all.set(item.id, item))
          if (!page.more) {
            setItems([...all.values()])
            return
          }
        }
        throw new Error('歌单数量过多，请在音乐库中整理后重试。')
      })()
        .catch((cause) => {
          reportCaughtError('app/src/features/library/playlist-picker.tsx', cause)

          if (!disposed) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed) setLoading(false)
        })
    return () => {
      disposed = true
    }
  }, [track, profile?.userId, revision])
  const filtered = items.filter((item) =>
    item.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())
  )
  const scroll = useRef<HTMLDivElement>(null)
  const virtual = useVirtualizer({
    count: filtered.length,
    getScrollElement: () => scroll.current,
    estimateSize: () => 64,
    overscan: 5
  })
  useEffect(() => {
    scroll.current?.scrollTo({ top: 0 })
  }, [query])
  async function collect(item: MusicCollection) {
    const reference = track ? onlineReference(track) : null
    if (!reference || pending.current || !profile) return
    const userId = profile.userId
    pending.current = true
    setSaving(true)
    setError(undefined)
    try {
      const playlist = collectionReference(item)
      if (playlist.source !== reference.source) throw new Error('曲目与歌单来源不一致')
      await musicClient.write(playlist.source, {
        operation: 'playlistTrack',
        playlist,
        track: reference,
        add: true
      })
      if (account.current !== userId) return
      reloadLikes()
      changed()
      onNotice(`已收藏到「${item.name}」。`)
      onClose()
    } catch (cause) {
      reportCaughtError('app/src/features/library/playlist-picker.tsx', cause)

      if (account.current === userId) setError(errorText(cause))
    } finally {
      pending.current = false
      setSaving(false)
    }
  }
  return (
    <Dialog
      open={!!track && !!profile}
      onOpenChange={(open) => {
        if (!open && !saving) onClose()
      }}
    >
      <DialogContent className="playlist-picker" showCloseButton={!saving}>
        <div className="playlist-picker-header">
          <div className="shrink-0">
            <DialogTitle className="text-left text-xl">收藏到歌单</DialogTitle>
            <DialogDescription className="mt-2 text-xs">
              {loading ? '正在读取歌单…' : `共 ${items.length} 个歌单`}
            </DialogDescription>
          </div>
          <div className="playlist-picker-tools">
            <label className="playlist-picker-search">
              <Search className="size-4 shrink-0" aria-hidden="true" />
              <input
                aria-label="搜索自己的歌单"
                placeholder="搜索歌单"
                value={query}
                disabled={loading || saving}
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>
            <CreatePlaylist
              iconOnly
              source={profile?.reference.source}
              disabled={saving}
              onCreated={() => {
                setRevision((v) => v + 1)
                changed()
              }}
            />
          </div>
        </div>
        {error && (
          <div className="flex items-center gap-2 text-sm text-destructive">
            <ErrorNotice error={error} />
            {!items.length && (
              <ActionButton variant="ghost" size="sm" onClick={() => setRevision((v) => v + 1)}>
                重试
              </ActionButton>
            )}
          </div>
        )}
        {saving && (
          <p role="status" className="text-sm text-muted-foreground">
            正在收藏…
          </p>
        )}
        <div
          ref={scroll}
          className="playlist-picker-list"
          tabIndex={0}
          aria-label="选择收藏目标歌单"
          aria-busy={loading || saving}
        >
          {loading ? (
            <TextSkeleton />
          ) : filtered.length ? (
            <div style={{ height: virtual.getTotalSize(), position: 'relative' }}>
              {virtual.getVirtualItems().map((row) => {
                const item = filtered[row.index]
                return (
                  <button
                    type="button"
                    key={item.id}
                    className="playlist-picker-row"
                    disabled={saving}
                    onClick={() => void collect(item)}
                    style={{
                      position: 'absolute',
                      top: row.start,
                      left: 0,
                      width: '100%',
                      height: row.size
                    }}
                  >
                    <Cover
                      size="small"
                      cover={item.cover}
                      className="size-10 shrink-0 rounded-lg"
                    />
                    <span className="min-w-0 text-left">
                      <span className="block truncate text-sm font-medium">{item.name}</span>
                      <span className="mt-1 block text-xs text-muted-foreground">
                        {item.trackCount} 首
                      </span>
                    </span>
                  </button>
                )
              })}
            </div>
          ) : (
            !error && (
              <p className="p-6 text-sm text-muted-foreground">
                {query ? '没有匹配的歌单。' : '还没有自己的歌单，点击右上角加号创建。'}
              </p>
            )
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
