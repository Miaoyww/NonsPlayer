import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { trackShareLink } from '@/features/music/share-links'
import { favoriteId, onlineReference } from '@/features/music/references'
import { readTrackInformation } from '@/features/music/queries'
import { useMusicSources } from '@/features/music/use-music-sources'
import { TextSkeleton } from '@/components/music/loading'
import { PluginSongMenuItems } from '@/plugins/song-menu'
import { AggregatePlaylistPicker } from '@/features/aggregate/aggregate-playlist-picker'
import { ContextMenu } from '@base-ui/react/context-menu'
import {
  ChevronRight,
  Copy,
  Ellipsis,
  FolderPlus,
  Heart,
  Info,
  ListPlus,
  LocateFixed,
  Play,
  RefreshCw,
  Share2,
  Trash2
} from 'lucide-react'
import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactElement,
  type ReactNode
} from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { nativeCall, formatTime, errorText, type Track } from '@/lib/player'
import {
  Dialog,
  DialogDescription,
  DialogContent,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { ActionButton } from '@/components/music/action-button'
import { useAccount } from '@/features/account/account'

interface LocalInformation {
  trackNumber: number | null
  discNumber: number | null
  bitrate: number | null
  sampleRate: number | null
  bitDepth: number | null
  channels: number | null
  fileSize: number
  format: string
}
interface SongInformation {
  artists: import('@/features/music/types').MusicCredit[]
  albumReference: import('@/features/music/types').EntityRef | null
  publishedAt: number | null
}
export function songCopyName(track: Track) {
  return `${track.title} - ${track.artist}`
}

const SongActionsContext = createContext<{
  songLink: (track: Track) => string
  collectAggregate: (track: Track) => void
  collect: (track: Track) => void
  collectGroup: (tracks: Track[]) => void
  details: (track: Track) => void
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
  const [localInformation, setLocalInformation] = useState<LocalInformation>()
  const [information, setInformation] = useState<SongInformation>()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const [retry, setRetry] = useState(0)
  const [copyMessage, setCopyMessage] = useState<string>()
  const copying = useRef(false)
  useEffect(() => {
    let disposed = false
    setInformation(undefined)
    setLocalInformation(undefined)
    setError(undefined)
    setCopyMessage(undefined)
    setBusy(!!track && isTauri())
    if (track?.source.kind === 'local' && isTauri())
      void nativeCall<LocalInformation>('local_track_information', {
        key: track.key,
        refresh: retry > 0
      })
        .then((value) => {
          if (!disposed) setLocalInformation(value)
        })
        .catch((cause) => {
          reportCaughtError('app/src/components/music/song-actions.tsx', cause)

          if (!disposed) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed) setBusy(false)
        })
    else if (track && isTauri())
      void readTrackInformation(track, retry > 0)
        .then((value) => {
          if (!disposed) setInformation(value)
        })
        .catch((cause) => {
          reportCaughtError('app/src/components/music/song-actions.tsx', cause)

          if (!disposed) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed) setBusy(false)
        })
    return () => {
      disposed = true
    }
  }, [track, retry])
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
      await navigator.clipboard.writeText(text)
      setCopyMessage('已复制')
      onNotice('已复制到剪贴板。')
    } catch (cause) {
      reportCaughtError('app/src/components/music/song-actions.tsx', cause)

      setCopyMessage('复制失败，请重试')
      onError(cause)
    } finally {
      copying.current = false
    }
  }
  const local = track?.source.kind === 'local'
  const fields =
    track && local
      ? [
          ['歌曲名称', track.title],
          ['艺术家', track.artist],
          ['专辑', track.album],
          ['时长', formatTime(track.durationMs)],
          ['音轨', localInformation?.trackNumber?.toString() ?? ''],
          ['碟号', localInformation?.discNumber?.toString() ?? ''],
          ['码率', localInformation?.bitrate ? `${localInformation.bitrate} kbps` : ''],
          ['采样率', localInformation?.sampleRate ? `${localInformation.sampleRate} Hz` : ''],
          ['位深', localInformation?.bitDepth ? `${localInformation.bitDepth} bit` : ''],
          ['声道', localInformation?.channels?.toString() ?? ''],
          ['格式', localInformation?.format ?? ''],
          [
            '文件大小',
            localInformation?.fileSize
              ? `${(localInformation.fileSize / 1024 / 1024).toFixed(2)} MB`
              : ''
          ],
          ['路径', track.source.kind === 'local' ? track.source.path : '']
        ]
      : track
        ? [
            ['歌曲名称', track.title],
            ['歌手', information?.artists.map((artist) => artist.name).join(' / ') || track.artist],
            [
              '歌手 ID',
              information?.artists.map((artist) => artist.reference?.id ?? '未知').join(' / ') ?? ''
            ],
            ['专辑', track.album],
            ['专辑 ID', information?.albumReference?.id ?? ''],
            [
              '歌曲 ID',
              (track.source.kind === 'local'
                ? track.source.neteaseId
                : track.source.track.reference.id
              )?.toString() ?? ''
            ],
            ['时长', formatTime(track.durationMs)],
            [
              '发行时间',
              information?.publishedAt
                ? new Date(information.publishedAt).toLocaleDateString('sv-SE', {
                    timeZone: 'Asia/Shanghai'
                  })
                : ''
            ],
            ['歌曲链接', songLink(track)]
          ]
        : []
  function field(index: number) {
    const [label, value] = fields[index]
    return (
      <div className="song-information-field" key={label}>
        <label htmlFor={`song-information-${index}`}>{label}</label>
        <div>
          <input id={`song-information-${index}`} readOnly value={value || '未知'} />
          <ActionButton
            variant="ghost"
            size="icon-sm"
            aria-label={`复制${label}`}
            disabled={!value}
            onClick={() => void copy(value)}
          >
            <Copy aria-hidden="true" />
          </ActionButton>
        </div>
      </div>
    )
  }
  return (
    <SongActionsContext.Provider
      value={{
        songLink,
        collect: setCollectTrack,
        collectAggregate: setAggregateTrack,
        collectGroup: setCollectGroup,
        details: (value) => {
          setRetry(0)
          setTrack(value)
        },
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
            <DialogTitle>歌曲详情复制</DialogTitle>
            <DialogDescription>查看歌曲信息，点击复制按钮复制单项或全部信息。</DialogDescription>
          </DialogHeader>
          {track && (
            <>
              {local ? (
                <>
                  {field(0)}
                  <div className="song-information-grid">{[1, 2, 3].map(field)}</div>
                  <h3 className="song-information-divider">文件信息</h3>
                  <div className="song-information-grid">
                    {[4, 5, 6, 7, 8, 9, 10, 11].map(field)}
                  </div>
                  {field(12)}
                </>
              ) : (
                <>
                  {field(0)}
                  <h3 className="song-information-divider">制作人员</h3>
                  <div className="song-information-grid">{[1, 2, 3, 4].map(field)}</div>
                  <h3 className="song-information-divider">歌曲信息</h3>
                  <div className="song-information-grid">{[5, 6].map(field)}</div>
                  {field(7)}
                  {field(8)}
                </>
              )}
              {busy && <TextSkeleton />}
              {error && (
                <div className="flex items-center gap-2 text-sm text-destructive">
                  <ErrorNotice error={error} />
                  <ActionButton
                    variant="ghost"
                    size="sm"
                    onClick={() => setRetry((value) => value + 1)}
                  >
                    重试
                  </ActionButton>
                </div>
              )}
              <p role="status" className="sr-only">
                {copyMessage}
              </p>
              <ActionButton
                variant="secondary"
                className="w-full"
                onClick={() =>
                  void copy(
                    fields.map(([label, value]) => `${label}：${value || '未知'}`).join('\n')
                  )
                }
              >
                复制全部信息
              </ActionButton>
            </>
          )}
        </DialogContent>
      </Dialog>
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
                <ContextMenu.Item
                  onClick={() =>
                    actions.copy(track.source.kind === 'local' ? track.source.path : '')
                  }
                >
                  <Copy aria-hidden="true" />
                  复制文件路径
                </ContextMenu.Item>
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
