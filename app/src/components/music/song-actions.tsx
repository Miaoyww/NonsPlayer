import { reportCaughtError } from '@/lib/diagnostics'
import { trackShareLink } from '@/features/music/share-links'
import { favoriteId, onlineReference } from '@/features/music/references'
import { useMusicSources } from '@/features/music/use-music-sources'
import { PluginSongMenuItems } from '@/plugins/song-menu'
import { AggregatePlaylistPicker } from '@/features/aggregate/aggregate-playlist-picker'
import { ContextMenu } from '@base-ui/react/context-menu'
import {
  ChevronRight,
  Copy,
  Download,
  Ellipsis,
  FolderOpen,
  FolderPlus,
  Heart,
  Info,
  ListPlus,
  LocateFixed,
  Play,
  RefreshCw,
  Share2,
  Tags,
  Trash2
} from 'lucide-react'
import {
  createContext,
  useContext,
  useRef,
  useState,
  type ReactElement,
  type ReactNode
} from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { nativeCall, type Track } from '@/lib/player'
import { isAndroid, isWindows } from '@/lib/platform'
import {
  Dialog,
  DialogDescription,
  DialogContent,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { useAccount } from '@/features/account/account'
import { LocalTagEditorDialog } from '@/features/local/tag-editor-dialog'
import { useDownloads } from '@/features/downloads/download-provider'
import { SongInformationContent } from './song-information'
export function songCopyName(track: Track) {
  return `${track.title} - ${track.artist}`
}

const SongActionsContext = createContext<{
  songLink: (track: Track) => string
  collectAggregate: (track: Track) => void
  collect: (track: Track) => void
  collectGroup: (tracks: Track[]) => void
  details: (track: Track) => void
  edit: (track: Track) => void
  run: (
    operation: () => void | Promise<unknown>,
    notice?: string | ((result: unknown) => string | undefined)
  ) => void
  copy: (text: string) => void
} | null>(null)

export function useSongActions() {
  const actions = useContext(SongActionsContext)
  if (!actions) throw new Error('SongActionsProvider is required')
  return actions
}

export function SongActionsProvider({
  children,
  onError,
  onNotice
}: {
  children: ReactNode
  onError: (error: unknown) => void
  onNotice: (message: string) => void
}) {
  const { statuses } = useMusicSources()
  const sources = statuses.filter((status) => status.enabled).map((status) => status.descriptor)
  const songLink = (track: Track) => trackShareLink(track, sources)
  const [collectTrack, setCollectTrack] = useState<Track>()
  const [aggregateTrack, setAggregateTrack] = useState<Track>()
  const [collectGroup, setCollectGroup] = useState<Track[]>()
  const [track, setTrack] = useState<Track>()
  const [editTrack, setEditTrack] = useState<Track>()
  const copying = useRef(false)
  function run(
    operation: () => void | Promise<unknown>,
    notice?: string | ((result: unknown) => string | undefined)
  ) {
    void Promise.resolve()
      .then(operation)
      .then((result) => {
        const message = typeof notice === 'function' ? notice(result) : notice
        if (message) onNotice(message)
      })
      .catch(onError)
  }
  async function copy(text: string) {
    if (!text || copying.current) return
    copying.current = true
    try {
      await writeText(text)
      onNotice('已复制到剪贴板。')
    } catch (cause) {
      reportCaughtError('app/src/components/music/song-actions.tsx', cause)

      onError(cause)
    } finally {
      copying.current = false
    }
  }
  return (
    <SongActionsContext.Provider
      value={{
        songLink,
        collect: setCollectTrack,
        collectAggregate: setAggregateTrack,
        collectGroup: setCollectGroup,
        details: setTrack,
        edit: setEditTrack,
        run,
        copy: (text) => {
          void copy(text)
        }
      }}
    >
      {children}
      <AggregatePlaylistPicker
        track={aggregateTrack ?? collectTrack}
        tracks={collectGroup}
        onClose={() => {
          setCollectTrack(undefined)
          setCollectGroup(undefined)
          setAggregateTrack(undefined)
        }}
        onNotice={onNotice}
      />
      <Dialog
        open={!!track}
        onOpenChange={(open) => {
          if (!open) setTrack(undefined)
        }}
      >
        <DialogContent className="song-information-dialog" style={{ width: 560 }}>
          <DialogHeader>
            <DialogTitle>歌曲详情</DialogTitle>
            <DialogDescription>查看歌曲信息与文件元数据。</DialogDescription>
          </DialogHeader>
          {track && <SongInformationContent key={track.key} track={track} songLink={songLink} />}
        </DialogContent>
      </Dialog>
      {editTrack && (
        <LocalTagEditorDialog
          key={editTrack.key}
          track={editTrack}
          onClose={() => setEditTrack(undefined)}
          onSaved={(updated) => {
            setEditTrack(undefined)
            onNotice(`已更新「${updated.title}」的标签。`)
          }}
        />
      )}
    </SongActionsContext.Provider>
  )
}

export function SongContextMenu({
  track,
  render,
  children,
  onPlay,
  busy = false,
  onRemove,
  removeLabel = '从列表删除',
  onLocate
}: {
  track: Track
  render: ReactElement
  children: ReactNode
  onPlay: () => void
  busy?: boolean
  onRemove?: () => void | Promise<unknown>
  removeLabel?: string
  onLocate?: () => void
}) {
  const actions = useSongActions()
  const downloads = useDownloads()
  const {
    profile,
    favoriteStatus,
    likesRefreshing,
    likesStale,
    likesError,
    reloadLikes,
    pendingLikes,
    toggleLike,
    canToggleLike
  } = useAccount(onlineReference(track)?.source)
  const id = favoriteId(track, profile?.reference)
  const status = id === undefined ? 'unknown' : favoriteStatus(id)
  const liked = status === 'liked'
  const unavailable = busy || !isTauri()
  const editableLocal =
    track.source.kind === 'local' &&
    !track.source.cue &&
    !track.source.path.toLowerCase().endsWith('.ncm')
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger render={render} tabIndex={0}>
        {children}
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Positioner className="z-[70]" sideOffset={4}>
          <ContextMenu.Popup className="song-context-menu" aria-label={`${track.title} 的歌曲菜单`}>
            {onLocate && (
              <ContextMenu.Item onClick={onLocate}>
                <LocateFixed aria-hidden="true" />
                定位到
              </ContextMenu.Item>
            )}
            {!unavailable && (
              <ContextMenu.Item onClick={() => actions.run(onPlay)}>
                <Play aria-hidden="true" />
                播放
              </ContextMenu.Item>
            )}
            {!unavailable && (
              <ContextMenu.Item
                onClick={() =>
                  actions.run(
                    () => nativeCall('append_queue', { keys: [track.key] }),
                    `已将「${track.title}」设为下一首播放。`
                  )
                }
              >
                <ListPlus aria-hidden="true" />
                下一首播放
              </ContextMenu.Item>
            )}
            {!unavailable && isWindows() && track.source.kind === 'online' && (
              <ContextMenu.Item
                onClick={() =>
                  actions.run(
                    () => downloads.start(track),
                    (result) => (typeof result === 'string' ? result.slice(0, 512) : undefined)
                  )
                }
              >
                <Download aria-hidden="true" />
                下载
              </ContextMenu.Item>
            )}
            {!unavailable && canToggleLike !== false && id !== undefined && profile && (
              <ContextMenu.Item
                disabled={status === 'unknown' || pendingLikes.has(id)}
                title={
                  status === 'unknown' && likesRefreshing
                    ? '正在读取收藏状态'
                    : likesError
                      ? `${status === 'unknown' ? '收藏状态待确认；' : ''}收藏状态读取失败：${likesError}；可重新读取。`
                      : status === 'unknown'
                        ? '收藏状态待确认，可重新读取。'
                        : likesStale
                          ? '收藏状态可能已过期'
                          : undefined
                }
                onClick={() => {
                  if (id !== undefined) actions.run(() => toggleLike(id))
                }}
              >
                {status === 'unknown' ? (
                  <Ellipsis aria-hidden="true" />
                ) : (
                  <Heart aria-hidden="true" />
                )}
                {status === 'unknown' ? '收藏状态待确认' : liked ? '取消收藏' : '收藏'}
              </ContextMenu.Item>
            )}
            {!unavailable &&
              id !== undefined &&
              (status === 'unknown' || likesStale || likesError) && (
                <ContextMenu.Item disabled={likesRefreshing} onClick={reloadLikes}>
                  <RefreshCw aria-hidden="true" />
                  {likesRefreshing ? '正在读取收藏状态…' : '重新读取收藏状态'}
                </ContextMenu.Item>
              )}
            <PluginSongMenuItems track={track} run={(work, notice) => actions.run(work, notice)} />
            {!unavailable && <ContextMenu.Separator />}
            {!unavailable && (
              <ContextMenu.Item onClick={() => actions.collectAggregate(track)}>
                <FolderPlus aria-hidden="true" />
                添加到歌单
              </ContextMenu.Item>
            )}
            {track.source.kind === 'local' ? (
              <>
                {!isAndroid() && (
                  <ContextMenu.Item
                    onClick={() =>
                      actions.run(() => nativeCall('open_local_track_folder', { key: track.key }))
                    }
                  >
                    <FolderOpen aria-hidden="true" />
                    打开文件所在位置
                  </ContextMenu.Item>
                )}
                <ContextMenu.Item
                  onClick={() =>
                    actions.copy(track.source.kind === 'local' ? track.source.path : '')
                  }
                >
                  <Copy aria-hidden="true" />
                  复制文件路径
                </ContextMenu.Item>
                {editableLocal && !isAndroid() && !unavailable && (
                  <ContextMenu.Item onClick={() => actions.edit(track)}>
                    <Tags aria-hidden="true" />
                    编辑标签
                  </ContextMenu.Item>
                )}
                <ContextMenu.Item onClick={() => actions.details(track)}>
                  <Info aria-hidden="true" />
                  更多信息
                </ContextMenu.Item>
              </>
            ) : (
              <ContextMenu.SubmenuRoot>
                <ContextMenu.SubmenuTrigger>
                  <Share2 aria-hidden="true" />
                  分享
                  <ChevronRight className="ml-auto" aria-hidden="true" />
                </ContextMenu.SubmenuTrigger>
                <ContextMenu.Portal>
                  <ContextMenu.Positioner className="z-[71]" sideOffset={4}>
                    <ContextMenu.Popup className="song-context-menu">
                      {actions.songLink(track) && (
                        <ContextMenu.Item onClick={() => actions.copy(actions.songLink(track))}>
                          <Copy aria-hidden="true" />
                          复制歌曲链接
                        </ContextMenu.Item>
                      )}
                      <ContextMenu.Item onClick={() => actions.copy(songCopyName(track))}>
                        <Copy aria-hidden="true" />
                        复制歌曲名称
                      </ContextMenu.Item>
                      <ContextMenu.Item onClick={() => actions.details(track)}>
                        <Info aria-hidden="true" />
                        更多信息
                      </ContextMenu.Item>
                    </ContextMenu.Popup>
                  </ContextMenu.Positioner>
                </ContextMenu.Portal>
              </ContextMenu.SubmenuRoot>
            )}
            {onRemove && !unavailable && (
              <ContextMenu.Item
                className="song-menu-remove"
                onClick={() => {
                  if (onRemove) actions.run(onRemove, '已移除歌曲。')
                }}
              >
                <Trash2 aria-hidden="true" />
                {removeLabel}
              </ContextMenu.Item>
            )}
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  )
}
