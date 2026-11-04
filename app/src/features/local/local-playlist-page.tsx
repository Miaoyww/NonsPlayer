import { presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { TrackListSkeleton } from '@/components/music/loading'
import { CollectionHeader } from '@/features/library/collection/collection-header'
import { PlaylistPageShell } from '@/features/library/playlist/playlist-page-shell'
import { useCallback, useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Pencil, Trash2 } from 'lucide-react'
import { errorText, nativeCall, usePlayer, type Track } from '@/lib/player'
import { localCollection, playLocalEntity, type LocalEntity } from '@/features/local/local-library'
import { usePagedList } from '@/lib/use-paged-list'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription
} from '@/components/ui/dialog'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'

export default function LocalPlaylistPage({
  collection: initial,
  refresh,
  onError,
  onNotice
}: {
  collection: MusicCollection
  refresh: number
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { navigate } = useMusicNavigation()
  const id = initial.localId!
  const identity = `playlist:${id}`
  const [revision, setRevision] = useState(0)
  const [detail, setDetail] = useState<{ identity: string; item: LocalEntity }>()
  const [editing, setEditing] = useState<'rename' | 'delete'>()
  const [name, setName] = useState('')
  const [saving, setSaving] = useState(false)
  const [dialogError, setDialogError] = useState<string>()
  const [playing, setPlaying] = useState(false)
  useEffect(() => setDialogError(undefined), [editing])
  useEffect(() => {
    if (!isTauri() || editing === 'delete') return
    let disposed = false
    void nativeCall<LocalEntity>('local_entity_detail', { kind: 'playlist', id })
      .then((item) => {
        if (!disposed) setDetail({ identity, item })
      })
      .catch(onError)
    return () => {
      disposed = true
    }
  }, [editing, id, identity, refresh, revision, onError])
  const collection = detail?.identity === identity ? localCollection(detail.item) : initial
  const loader = useCallback(
    (offset: number) =>
      nativeCall<{ items: Track[]; more: boolean }>('local_entity_tracks', {
        kind: 'playlist',
        id,
        keyword: '',
        offset
      }),
    [id, refresh, revision]
  )
  const list = usePagedList<Track>(loader, 100, isTauri(), {
    cacheKey: presentationKey('local-detail', 'playlist', id)
  })
  const player = usePlayer('index', 'queue')
  async function play() {
    if (playing) return
    setPlaying(true)
    try {
      if (await playLocalEntity('playlist', id)) onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      reportCaughtError('app/src/features/local/local-playlist-page.tsx', cause)

      onError(cause)
    } finally {
      setPlaying(false)
    }
  }
  async function save() {
    if (saving || !editing) return
    setSaving(true)
    setDialogError(undefined)
    try {
      if (editing === 'rename') {
        await nativeCall('rename_local_playlist', { id, name })
        setRevision((value) => value + 1)
        navigate('local-playlist', '', { ...collection, name: name.trim() })
      } else {
        await nativeCall('delete_local_playlist', { id })
        navigate('local')
      }
      setEditing(undefined)
    } catch (cause) {
      reportCaughtError('app/src/features/local/local-playlist-page.tsx', cause)

      setDialogError(errorText(cause))
    } finally {
      setSaving(false)
    }
  }
  return (
    <>
      <PlaylistPageShell
        hasMore={list.more}
        header={
          <CollectionHeader
            collection={collection}
            total={collection.trackCount}
            busy={playing}
            disabled={playing || !isTauri() || !list.items.length}
            onPlay={() => void play()}
            cover={
              <Cover
                cover={collection.cover || list.items[0]?.cover}
                className="library-detail-cover rounded-xl"
              />
            }
            actions={
              <>
                <ActionButton
                  variant="ghost"
                  disabled={!isTauri()}
                  onClick={() => {
                    setName(collection.name)
                    setEditing('rename')
                  }}
                >
                  <Pencil aria-hidden="true" />
                  改名
                </ActionButton>
                <ActionButton
                  variant="ghost"
                  disabled={!isTauri()}
                  onClick={() => setEditing('delete')}
                >
                  <Trash2 aria-hidden="true" />
                  删除歌单
                </ActionButton>
              </>
            }
          />
        }
      >
        {list.busy && !list.items.length ? (
          <TrackListSkeleton online={false} />
        ) : list.items.length > 0 ? (
          <TrackList
            tracks={list.items}
            searchable
            sortable
            hasMore={list.more}
            busy={!isTauri()}
            currentKey={player.index === null ? undefined : player.queue[player.index]?.key}
            onPlay={(index, tracks) => {
              const start = index >= 1000 ? index : 0
              void nativeCall('play_queue', {
                keys: tracks.slice(start, start + 1000).map((t) => t.key),
                index: index - start
              }).catch(onError)
            }}
            onAppend={(track) =>
              void nativeCall('append_queue', { keys: [track.key] }).catch(onError)
            }
            onRemove={async (track) => {
              await nativeCall('remove_local_playlist_track', { id, key: track.key })
              setRevision((value) => value + 1)
            }}
            removeLabel="从歌单移除"
          />
        ) : (
          !list.error && (
            <div className="library-empty">
              {list.busy ? '正在读取…' : '还没有歌曲，在本地歌曲的右键菜单中添加到此歌单。'}
            </div>
          )
        )}
        <InfiniteLoad more={list.more} busy={list.busy} error={list.error} onLoad={list.loadMore} />
      </PlaylistPageShell>
      <Dialog
        open={!!editing}
        onOpenChange={(open) => {
          if (!open && !saving) setEditing(undefined)
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editing === 'rename' ? '修改歌单名称' : '删除歌单'}</DialogTitle>
            <DialogDescription>
              {editing === 'delete' ? '删除歌单会保留曲库中的歌曲和原始文件。' : '改变歌单的名称。'}
            </DialogDescription>
          </DialogHeader>
          <form
            onSubmit={(event) => {
              event.preventDefault()
              void save()
            }}
            className="flex flex-col gap-4"
          >
            {editing === 'rename' && (
              <Input
                autoFocus
                aria-label="歌单名称"
                value={name}
                maxLength={100}
                onChange={(event) => setName(event.target.value)}
              />
            )}
            {dialogError && <ErrorNotice error={dialogError} />}
            <ActionButton type="submit" disabled={saving || (editing === 'rename' && !name.trim())}>
              {saving ? '正在保存…' : editing === 'delete' ? '删除歌单' : '保存'}
            </ActionButton>
          </form>
        </DialogContent>
      </Dialog>
    </>
  )
}
