import { TextSkeleton } from '@/components/music/loading'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Copy, ListPlus, Pencil, Play, Trash2 } from 'lucide-react'
import {
  cloneElement,
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactElement,
  type ReactNode
} from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { nativeCall, errorText } from '@/lib/player'
import { getLibraryTracks, playMusicCollection } from '@/features/library/library-api'
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogContent,
  DialogTitle
} from '@/components/ui/dialog'
import { Button as ActionButton } from '@/components/ui/button'
import { useAccount } from '@/features/account/account'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'

const CollectionActions = createContext<{
  revision: number
  changed: () => void
  edit: (item: MusicCollection) => void
  remove: (item: MusicCollection) => void
  play: (item: MusicCollection, next?: boolean) => void
  copy: (item: MusicCollection) => void
  busy: boolean
} | null>(null)
export function useCollectionActions() {
  const value = useContext(CollectionActions)
  if (!value) throw new Error('CollectionActionsProvider is required')
  return value
}
export function CollectionActionsProvider({
  children,
  onError,
  onNotice
}: {
  children: ReactNode
  onError: (error: unknown) => void
  onNotice: (message: string) => void
}) {
  const { profile } = useAccount()
  const { page, navigate } = useMusicNavigation()
  const [revision, setRevision] = useState(0)
  const [target, setTarget] = useState<{ item: MusicCollection; deleting: boolean }>()
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [ready, setReady] = useState(false)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const pending = useRef(false)
  const [error, setError] = useState<string>()
  const changed = () => setRevision((v) => v + 1)
  useEffect(() => {
    setTarget(undefined)
  }, [profile?.userId])
  useEffect(() => {
    let disposed = false
    setReady(false)
    setError(undefined)
    setName(target?.item.name ?? '')
    setDescription('')
    setLoading(!!target && !target.deleting)
    if (target && !target.deleting && profile)
      void getLibraryTracks(target.item, 0, profile.userId)
        .then((value) => {
          if (!disposed) {
            setDescription(value.description ?? '')
            setReady(true)
          }
        })
        .catch((cause) => {
          if (!disposed) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed) setLoading(false)
        })
    return () => {
      disposed = true
    }
  }, [target, profile?.userId])
  async function play(item: MusicCollection, next = false) {
    if (pending.current) return
    pending.current = true
    setBusy(true)
    try {
      const truncated = await playMusicCollection(item, next)
      onNotice(
        `${next ? '已按顺序加入下一首播放' : '已替换播放列表'}${truncated ? '（前 1000 首）' : ''}。`
      )
    } catch (cause) {
      onError(cause)
    } finally {
      pending.current = false
      setBusy(false)
    }
  }
  async function save() {
    if (!target || pending.current || !profile) return
    pending.current = true
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall(
        target.deleting ? 'delete_library_playlist' : 'update_library_playlist',
        target.deleting
          ? { id: target.item.id }
          : { id: target.item.id, name: name.trim(), description }
      )
      changed()
      setTarget(undefined)
      if (page.collection?.id === target.item.id && page.collection.kind === 'playlist')
        navigate(
          target.deleting ? 'library' : 'collection',
          '',
          target.deleting ? undefined : { ...target.item, name: name.trim() }
        )
      onNotice(target.deleting ? '已删除歌单。' : '已保存歌单信息。')
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      pending.current = false
      setBusy(false)
    }
  }
  return (
    <CollectionActions.Provider
      value={{
        revision,
        changed,
        edit: (item) => setTarget({ item, deleting: false }),
        remove: (item) => setTarget({ item, deleting: true }),
        play: (item, next) => {
          void play(item, next)
        },
        busy,
        copy: (item) => {
          void navigator.clipboard
            .writeText(`https://music.163.com/#/${item.kind}?id=${item.id}`)
            .then(() => onNotice('已复制分享链接。'), onError)
        }
      }}
    >
      {children}
      <Dialog
        open={!!target}
        onOpenChange={(open) => {
          if (!open && !busy) setTarget(undefined)
        }}
      >
        <DialogContent showCloseButton={!busy}>
          <DialogTitle>{target?.deleting ? '删除歌单' : '编辑歌单信息'}</DialogTitle>
          <DialogDescription>
            {target?.deleting
              ? `确定删除「${target.item.name}」吗？删除后无法恢复。`
              : '修改歌单名称和简介。'}
          </DialogDescription>
          <form
            className="flex flex-col gap-4"
            onSubmit={(event) => {
              event.preventDefault()
              void save()
            }}
          >
            {!target?.deleting && (
              <>
                <label className="flex flex-col gap-2 text-sm">
                  歌单名称
                  <input
                    className="music-input"
                    value={name}
                    maxLength={40}
                    required
                    disabled={busy || loading}
                    onChange={(event) => setName(event.target.value)}
                  />
                </label>
                <label className="flex flex-col gap-2 text-sm">
                  简介
                  <textarea
                    className="music-input min-h-28"
                    value={description}
                    maxLength={1000}
                    disabled={busy || loading}
                    onChange={(event) => setDescription(event.target.value)}
                  />
                </label>
              </>
            )}
            {loading && <TextSkeleton />}
            {error && (
              <div role="alert" className="text-sm text-destructive">
                {error}
                {!ready && !target?.deleting && (
                  <ActionButton
                    variant="ghost"
                    size="sm"
                    onClick={() => setTarget((value) => (value ? { ...value } : value))}
                  >
                    重试读取
                  </ActionButton>
                )}
              </div>
            )}
            <div className="flex justify-end gap-2">
              <DialogClose render={<ActionButton variant="outline" disabled={busy} />}>
                取消
              </DialogClose>
              <ActionButton
                type="submit"
                variant={target?.deleting ? 'destructive' : 'default'}
                disabled={busy || loading || (!target?.deleting && (!name.trim() || !ready))}
              >
                {busy ? '正在处理…' : target?.deleting ? '删除' : '保存'}
              </ActionButton>
            </div>
          </form>
        </DialogContent>
      </Dialog>
    </CollectionActions.Provider>
  )
}
export function CollectionContextMenu({
  item,
  name,
  busy,
  render,
  children,
  onPlay,
  onNext
}: {
  item?: MusicCollection
  name?: string
  busy: boolean
  render: ReactElement
  children: ReactNode
  onPlay?: () => void
  onNext?: () => void
}) {
  const actions = useCollectionActions()
  const { profile } = useAccount()
  const unavailable = busy || actions.busy || !isTauri()
  if (item?.kind === 'artist' || (!item && ((!onPlay && !onNext) || unavailable)))
    return cloneElement(render, undefined, children)
  const editable =
    !!item && !item.reference && !!profile && item.creatorId === profile.userId && !item.liked
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger render={render} tabIndex={0}>
        {children}
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Positioner className="z-[70]" sideOffset={4}>
          <ContextMenu.Popup
            className="song-context-menu"
            aria-label={`${name ?? item?.name} 的菜单`}
          >
            {!unavailable && (item || onPlay) && (
              <ContextMenu.Item
                onClick={() => {
                  if (onPlay) onPlay()
                  else if (item) actions.play(item)
                }}
              >
                <Play aria-hidden="true" />
                播放
              </ContextMenu.Item>
            )}
            {!unavailable && (item || onNext) && (
              <ContextMenu.Item
                onClick={() => {
                  if (onNext) onNext()
                  else if (item) actions.play(item, true)
                }}
              >
                <ListPlus aria-hidden="true" />
                下一首播放
              </ContextMenu.Item>
            )}
            {!unavailable && item && <ContextMenu.Separator />}
            {item && (
              <ContextMenu.Item onClick={() => actions.copy(item)}>
                <Copy aria-hidden="true" />
                复制链接
              </ContextMenu.Item>
            )}
            {item?.kind === 'playlist' && editable && !unavailable && (
              <>
                <ContextMenu.Item onClick={() => actions.edit(item)}>
                  <Pencil aria-hidden="true" />
                  编辑歌单信息
                </ContextMenu.Item>
                <ContextMenu.Item className="song-menu-remove" onClick={() => actions.remove(item)}>
                  <Trash2 aria-hidden="true" />
                  删除歌单
                </ContextMenu.Item>
              </>
            )}
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  )
}
