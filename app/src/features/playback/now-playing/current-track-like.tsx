import { Ellipsis, Heart } from 'lucide-react'
import { favoriteId, onlineReference } from '@/features/music/references'
import { isTauri } from '@tauri-apps/api/core'
import type { Track } from '@/lib/player'
import { Button } from '@/components/ui/button'
import { useAccount } from '@/features/account/account'

export function CurrentTrackLike({
  track,
  onError,
  showLabel = false
}: {
  track?: Track
  onError: (error: unknown) => void
  showLabel?: boolean
}) {
  const {
    profile,
    favoriteStatus,
    likesRefreshing,
    likesStale,
    likesError,
    pendingLikes,
    toggleLike,
    canToggleLike
  } = useAccount(track ? onlineReference(track)?.source : undefined)
  const songId = track ? favoriteId(track, profile?.reference) : undefined
  const status = songId === undefined ? 'unknown' : favoriteStatus(songId)
  const liked = status === 'liked'
  const unknown = songId !== undefined && status === 'unknown'
  const likePending = songId !== undefined && pendingLikes.has(songId)
  return (
    <Button
      variant="ghost"
      size={showLabel ? 'sm' : 'icon-sm'}
      className="playback-like"
      aria-label={
        unknown
          ? '收藏状态待确认'
          : showLabel
            ? liked
              ? '取消收藏歌曲'
              : '收藏歌曲'
            : liked
              ? '取消收藏当前歌曲'
              : '收藏当前歌曲'
      }
      aria-pressed={unknown ? undefined : liked}
      aria-busy={likePending || likesRefreshing}
      title={
        !track
          ? '选择一首音乐后收藏'
          : songId === undefined
            ? '歌曲所属平台账号不支持收藏这首歌曲'
            : !profile
              ? '登录后收藏歌曲'
              : unknown && likesRefreshing
                ? '正在读取收藏状态'
                : likesError
                  ? `${unknown ? '收藏状态待确认；' : ''}收藏状态读取失败：${likesError}；可在歌曲菜单重新读取。`
                  : unknown
                    ? '收藏状态待确认，可在歌曲菜单重新读取。'
                    : likesStale
                      ? '收藏状态可能已过期，可在歌曲菜单重新读取。'
                      : canToggleLike === false
                        ? '收藏状态（此来源仅支持读取）'
                        : liked
                          ? '取消收藏'
                          : '收藏'
      }
      disabled={
        !isTauri() ||
        songId === undefined ||
        !profile ||
        status === 'unknown' ||
        likePending ||
        canToggleLike === false
      }
      onClick={() => {
        if (songId !== undefined) void toggleLike(songId).catch(onError)
      }}
    >
      {unknown ? <Ellipsis aria-hidden="true" /> : <Heart aria-hidden="true" />}
      {showLabel && (likePending ? '处理中…' : unknown ? '待确认' : liked ? '已收藏' : '收藏')}
    </Button>
  )
}
