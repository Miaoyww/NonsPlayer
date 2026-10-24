import { useCallback, useEffect, useMemo, useState } from 'react'
import { Plus, Trash2, ArrowUpDown, Check, ListChecks } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { MusicPage } from '@/components/music/music-page'
import { CollectionContent } from '@/features/library/collection-content'
import { ActionButton } from '@/components/music/action-button'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { ErrorNotice } from '@/components/error-notice'
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { useMusicSources } from '@/features/music/use-music-sources'
import { useSongActions } from '@/components/music/song-actions'
import { usePagedList } from '@/lib/use-paged-list'
import { nativeCall, usePlayer } from '@/lib/player'
import { AggregateHeader } from './aggregate-header'
import { PlaylistCard } from '@/features/library/playlist-card'
import {
  aggregateCollection,
  allAggregateTracks,
  playAggregate,
  useAggregateLibrary,
  writeAggregate,
  type AggregateMember
} from './aggregate-library'

export function CreateAggregatePlaylist({ onError }: { onError: (cause: unknown) => void }) {
  const { reload } = useAggregateLibrary()
  const { navigate } = useMusicNavigation()
  const [create, setCreate] = useState(false)
  const [name, setName] = useState('')
  const [busy, setBusy] = useState(false)
  return (
    <>
      <ActionButton variant="ghost" disabled={!isTauri()} onClick={() => setCreate(true)}>
        <Plus aria-hidden="true" />
        创建聚合歌单
      </ActionButton>
      <Dialog
        open={create}
        onOpenChange={(open) => {
          if (!busy) setCreate(open)
        }}
      >
        <DialogContent>
          <DialogTitle>创建聚合歌单</DialogTitle>
          <DialogDescription>保存在这台设备，可添加本地及不同平台的歌曲。</DialogDescription>
          <form
            className="flex flex-col gap-3"
            onSubmit={(event) => {
              event.preventDefault()
              if (busy) return
              setBusy(true)
              void writeAggregate({ operation: 'create', name })
                .then(async (id) => {
                  await reload()
                  setCreate(false)
                  setName('')
                  navigate('local-playlist', '', {
                    id,
                    localId: id,
                    reference: null,
                    kind: 'playlist',
                    name,
                    cover: '',
                    subtitle: '聚合歌单',
                    trackCount: 0
                  })
                })
                .catch(onError)
                .finally(() => setBusy(false))
            }}
          >
            <Input
              aria-label="歌单名称"
              maxLength={100}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <ActionButton type="submit" disabled={busy || !name.trim()}>
              创建
            </ActionButton>
          </form>
        </DialogContent>
      </Dialog>
    </>
  )
}
// Used when the selected source cannot display its own library.
export function AggregateSection({ onError }: { onError: (cause: unknown) => void }) {
  const { playlists, error } = useAggregateLibrary()
  const { navigate } = useMusicNavigation()
  return (
    <section aria-label="全部歌单">
      <div className="library-tabs-row">
        <h2>全部歌单</h2>
        <CreateAggregatePlaylist onError={onError} />
      </div>
      <ErrorNotice error={error} />
      <div className="discover-playlist-grid">
        {playlists.map((p) => (
          <PlaylistCard
            key={p.id}
            item={aggregateCollection(p)}
            busy={false}
            onOpen={(item) => navigate('local-playlist', '', item)}
            onPlay={() => void playAggregate(p.id).catch(onError)}
          />
        ))}
      </div>
    </section>
  )
}
export default function AggregatePage({
  id,
  onError,
  onNotice
}: {
  id: string
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { playlists, revision, reload } = useAggregateLibrary()
  const playlist = playlists.find((p) => p.id === id)
  const { navigate } = useMusicNavigation()
  const { statuses } = useMusicSources()
  const actions = useSongActions()
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [batch, setBatch] = useState(false)
  const [busy, setBusy] = useState(false)
  const [edit, setEdit] = useState<'rename' | 'delete' | 'unlink' | 'order'>()
  const [name, setName] = useState('')
  const [position, setPosition] = useState('1')
  const player = usePlayer('queue', 'index')
  const currentKey = player.index === null ? undefined : player.queue[player.index]?.key
  const loader = useCallback(
    (offset: number) =>
      nativeCall<{ items: AggregateMember[]; more: boolean }>('aggregate_tracks', {
        id,
        offset
      }),
    [id, revision]
  )
  const list = usePagedList(loader, 100, isTauri())
  const members = useMemo(() => new Map(list.items.map((m) => [m.track.key, m])), [list.items])
  useEffect(() => {
    setSelected(new Set())
  }, [id, revision])
  useEffect(() => {
    setBatch(false)
  }, [id])
  const toggleBatch = () => {
    setBatch((value) => !value)
    setSelected(new Set())
  }
  async function run(work: () => Promise<unknown>, message?: string) {
    if (busy) return
    setBusy(true)
    try {
      await work()
      await reload()
      if (message) onNotice(message)
    } catch (cause) {
      onError(cause)
    } finally {
      setBusy(false)
    }
  }
  const mutate = (operation: 'remove', keys = [...selected]) =>
    run(() => writeAggregate({ operation, id, keys }))
  if (!playlist)
    return (
      <MusicPage>
        <p role="status">歌单不存在或正在读取。</p>
        <ActionButton onClick={() => navigate('library')}>返回音乐库</ActionButton>
      </MusicPage>
    )
  const sourceName = (m: AggregateMember) =>
    m.track.source.kind === 'local'
      ? '本地'
      : (statuses.find(
          (s) =>
            s.descriptor.source ===
            (m.track.source.kind === 'online' ? m.track.source.track.reference.source : '')
        )?.descriptor.displayName ?? '在线来源')
  const reason = (m: AggregateMember) => {
    if (m.unavailable) return m.unavailable
    const source = m.track.source.kind === 'online' ? m.track.source.track.reference.source : null
    if (source && !statuses.some((s) => s.enabled && s.descriptor.source === source))
      return '来源不可用'
    if (m.track.source.kind === 'online' && m.track.source.track.unavailableReason)
      return `不可用：${m.track.source.track.unavailableReason}`
    return ''
  }
  return (
    <MusicPage aria-label="聚合歌单">
      <CollectionContent key={id} hasMore={list.more} searchable={!batch}>
        <AggregateHeader
          key={id}
          playlist={playlist}
          batch={batch}
          onBatch={toggleBatch}
          busy={busy}
          onPlay={() =>
            void run(async () => {
              const skipped = await playAggregate(id)
              onNotice(
                skipped ? `已开始播放，跳过 ${skipped} 首已退出曲库的歌曲。` : '已开始播放。'
              )
            })
          }
          onEdit={(operation) => {
            setName(playlist.name)
            setEdit(operation)
          }}
        />
        {batch && (
          <div
            className="mb-4 flex flex-wrap items-center gap-2"
            role="toolbar"
            aria-label="批量操作"
          >
            <span className="mr-2 inline-flex items-center gap-2 text-sm text-muted-foreground">
              <ListChecks aria-hidden="true" className="size-4" />
              已选 {selected.size} 首
            </span>
            <ActionButton
              disabled={
                busy ||
                !selected.size ||
                list.items.some((m) => selected.has(m.track.key) && m.remote)
              }
              variant="outline"
              title={
                list.items.some((m) => selected.has(m.track.key) && m.remote)
                  ? '原歌单歌曲始终跟随平台，请在原歌单中删除'
                  : undefined
              }
              onClick={() => void mutate('remove')}
            >
              <Trash2 aria-hidden="true" />
              移除
            </ActionButton>
            <ActionButton
              disabled={busy || !selected.size}
              variant="outline"
              onClick={() =>
                actions.collectGroup(
                  list.items.filter((m) => selected.has(m.track.key)).map((m) => m.track)
                )
              }
            >
              <Plus aria-hidden="true" />
              添加到其他聚合歌单
            </ActionButton>
            <ActionButton
              disabled={busy || !selected.size}
              variant="outline"
              onClick={() => setEdit('order')}
            >
              <ArrowUpDown aria-hidden="true" />
              移动到位置
            </ActionButton>
            <ActionButton
              variant="outline"
              className="ml-auto"
              disabled={busy}
              onClick={toggleBatch}
            >
              <Check aria-hidden="true" />
              完成
            </ActionButton>
          </div>
        )}
        <TrackList
          tracks={list.items.map((m) => m.track)}
          busy={busy}
          currentKey={currentKey}
          searchable={!batch}
          hasMore={list.more}
          selection={
            batch
              ? {
                  keys: selected,
                  onToggle: (key, checked) =>
                    setSelected((old) => {
                      const next = new Set(old)
                      if (checked) next.add(key)
                      else next.delete(key)
                      return next
                    }),
                  onToggleAll: (keys, checked) =>
                    setSelected((old) => {
                      const next = new Set(old)
                      for (const key of keys) {
                        if (checked) next.add(key)
                        else next.delete(key)
                      }
                      return next
                    })
                }
              : undefined
          }
          onPlay={(_, tracks) => {
            void run(() => playAggregate(id, tracks[_]?.key))
          }}
          onRemove={(track) => mutate('remove', [track.key])}
          canRemove={(track) => !members.get(track.key)?.remote}
          removeLabel="从聚合歌单移除"
          renderSource={(track) => {
            const m = members.get(track.key)!
            return (
              <span title={reason(m)}>
                {sourceName(m)}
                {reason(m) ? ` · ${reason(m)}` : ''}
              </span>
            )
          }}
        />
        <InfiniteLoad more={list.more} busy={list.busy} error={list.error} onLoad={list.loadMore} />
        {!list.busy && !list.items.length && (
          <p className="py-8 text-muted-foreground">还没有歌曲，可从歌曲菜单添加。</p>
        )}
      </CollectionContent>
      <Dialog
        open={!!edit}
        onOpenChange={(open) => {
          if (!open && !busy) setEdit(undefined)
        }}
      >
        <DialogContent>
          <DialogTitle>
            {
              {
                rename: '修改歌单名称',
                delete: '删除本机歌单',
                unlink: '解除同步关联',
                order: '移动选中歌曲'
              }[edit ?? 'rename']
            }
          </DialogTitle>
          <DialogDescription>
            {edit === 'delete'
              ? '只删除本机歌单和同步关联，原平台歌单及音频文件保留。'
              : edit === 'unlink'
                ? '保留当前歌曲，转为独立聚合歌单并恢复原平台入口。'
                : edit === 'order'
                  ? '选中歌曲按当前歌单顺序整体移动，位置从 1 开始。'
                  : '名称需要 1–100 个字符。'}
          </DialogDescription>
          <form
            className="flex flex-col gap-3"
            onSubmit={(e) => {
              e.preventDefault()
              void run(async () => {
                if (edit === 'order') {
                  const all = (await allAggregateTracks(id)).map((m) => m.track.key)
                  const moving = all.filter((k) => selected.has(k))
                  const rest = all.filter((k) => !selected.has(k))
                  const target = Number(position)
                  if (!Number.isInteger(target) || target < 1 || target > rest.length + 1)
                    throw new Error(`位置需要为 1–${rest.length + 1}`)
                  rest.splice(target - 1, 0, ...moving)
                  await writeAggregate({ operation: 'reorder', id, keys: rest })
                } else if (edit === 'rename')
                  await writeAggregate({ operation: 'rename', id, name })
                else if (edit) await writeAggregate({ operation: edit, id })
                setEdit(undefined)
                if (edit === 'delete') navigate('library')
              })
            }}
          >
            {edit === 'rename' && (
              <Input
                aria-label="歌单名称"
                maxLength={100}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            )}
            {edit === 'order' && (
              <Input
                aria-label="目标位置"
                type="number"
                min={1}
                max={playlist.trackCount}
                value={position}
                onChange={(e) => setPosition(e.target.value)}
              />
            )}
            <ActionButton type="submit" disabled={busy || (edit === 'rename' && !name.trim())}>
              {busy ? '正在保存…' : '确认'}
            </ActionButton>
          </form>
        </DialogContent>
      </Dialog>
    </MusicPage>
  )
}
