import { useMusicSources } from '@/features/music/use-music-sources'
import { Menu } from '@base-ui/react/menu'
import { favoriteId, onlineReference } from '@/features/music/references'
import {
  Copy,
  Ellipsis,
  FolderPlus,
  Heart,
  Info,
  RefreshCw,
  Share2,
  Radio,
  FilePenLine
} from 'lucide-react'
import { LyricCorrectionDialog } from '@/features/lyrics/correction-dialog'
import { useState } from 'react'
import { SongSourceDialog } from './song-source-dialog'
import { ActionButton } from '@/components/music/action-button'
import { songCopyName, useSongActions } from '@/components/music/song-actions'
import { useAccount } from '@/features/account/account'
import { isTauri } from '@tauri-apps/api/core'
import type { Track } from '@/lib/player'

export function NowPlayingMenu({
  track,
  source,
  loading,
  onRefresh,
  showLike = true
}: {
  track?: Track
  source?: string
  loading: boolean
  onRefresh: () => void
  showLike?: boolean
}) {
  const actions = useSongActions()
  const [sourcesOpen, setSourcesOpen] = useState(false)
  const [lyricsOpen, setLyricsOpen] = useState(false)
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
  } = useAccount(track ? onlineReference(track)?.source : undefined)
  const id = track ? favoriteId(track, profile?.reference) : undefined
  const status = id === undefined ? 'unknown' : favoriteStatus(id)
  const liked = status === 'liked'
  const { statuses } = useMusicSources()
  const reference = track ? onlineReference(track) : null
  const canCollect =
    reference?.source === profile?.reference.source &&
    statuses.some(
      (value) =>
        value.enabled &&
        value.descriptor.source === reference?.source &&
        value.descriptor.capabilities.includes('playlistWrite')
    )
  const desktop = isTauri()
  const link = track ? actions.songLink(track) : ''
  return (
    <>
      <Menu.Root>
        <Menu.Trigger
          disabled={!track}
          render={
            <ActionButton variant="ghost" size="icon-sm" aria-label="歌曲菜单" title="歌曲菜单" />
          }
        >
          <Ellipsis aria-hidden="true" />
        </Menu.Trigger>
        <Menu.Portal>
          <Menu.Positioner className="z-[70]" side="bottom" align="end" sideOffset={6}>
            <Menu.Popup className="song-context-menu" aria-label="歌曲菜单">
              {showLike && id !== undefined && (
                <Menu.Item
                  disabled={
                    !desktop ||
                    !profile ||
                    status === 'unknown' ||
                    pendingLikes.has(id) ||
                    canToggleLike === false
                  }
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
                  onClick={() => actions.run(() => toggleLike(id))}
                >
                  {status === 'unknown' ? (
                    <Ellipsis aria-hidden="true" />
                  ) : (
                    <Heart aria-hidden="true" />
                  )}
                  {status === 'unknown' ? '收藏状态待确认' : liked ? '取消收藏歌曲' : '收藏歌曲'}
                </Menu.Item>
              )}
              {showLike &&
                id !== undefined &&
                (status === 'unknown' || likesStale || likesError) && (
                  <Menu.Item disabled={!desktop || likesRefreshing} onClick={reloadLikes}>
                    <RefreshCw aria-hidden="true" />
                    {likesRefreshing ? '正在读取收藏状态…' : '重新读取收藏状态'}
                  </Menu.Item>
                )}
              {track && (track.source.kind === 'local' || canCollect) && (
                <Menu.Item disabled={!desktop} onClick={() => actions.collect(track)}>
                  <FolderPlus aria-hidden="true" />
                  {track.source.kind === 'local' ? '添加到本地歌单' : '收藏到歌单'}
                </Menu.Item>
              )}
              {track && (
                <Menu.Item onClick={() => actions.copy(link || songCopyName(track))}>
                  {link ? <Share2 aria-hidden="true" /> : <Copy aria-hidden="true" />}
                  {link ? '复制歌曲链接' : '复制歌曲名称'}
                </Menu.Item>
              )}
              {track?.source.kind === 'local' && (
                <Menu.Item
                  onClick={() => {
                    if (track.source.kind === 'local') actions.copy(track.source.path)
                  }}
                >
                  <Copy aria-hidden="true" />
                  复制文件路径
                </Menu.Item>
              )}
              <Menu.Item
                disabled={!track || !desktop}
                onClick={() => {
                  if (track) actions.details(track)
                }}
              >
                <Info aria-hidden="true" />
                歌曲详情
              </Menu.Item>
              <Menu.Separator />
              {track && track.source.kind !== 'local' && (
                <Menu.Item disabled={!desktop} onClick={() => setSourcesOpen(true)}>
                  <Radio aria-hidden="true" />
                  歌曲来源
                </Menu.Item>
              )}
              <Menu.Group>
                <Menu.Item disabled={!track || !desktop} onClick={() => setLyricsOpen(true)}>
                  <FilePenLine aria-hidden="true" />
                  歌词来源
                  <span className="ml-auto text-xs text-muted-foreground">
                    {source ?? (loading ? '正在获取…' : '暂无歌词')}
                  </span>
                </Menu.Item>
                <Menu.Item disabled={!track || !desktop || loading} onClick={onRefresh}>
                  <RefreshCw aria-hidden="true" />
                  {loading ? '正在获取歌词…' : '刷新歌词'}
                </Menu.Item>
              </Menu.Group>
            </Menu.Popup>
          </Menu.Positioner>
        </Menu.Portal>
      </Menu.Root>
      {sourcesOpen && track && (
        <SongSourceDialog key={track.key} track={track} onClose={() => setSourcesOpen(false)} />
      )}
      {lyricsOpen && track && (
        <LyricCorrectionDialog key={track.key} track={track} onClose={() => setLyricsOpen(false)} />
      )}
    </>
  )
}
