import { Heart } from 'lucide-react'
import { favoriteId } from '@/features/music/legacy-reference'
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
  const { profile, likedIds, likesReady, pendingLikes, toggleLike, canToggleLike } = useAccount()
  const songId = track ? favoriteId(track, profile?.reference) : undefined
  const liked = songId !== undefined && likedIds.has(songId)
  const likePending = songId !== undefined && pendingLikes.has(songId)
  return (
    <Button
      variant="ghost"
      size={showLabel ? 'sm' : 'icon-sm'}
      className="playback-like"
      aria-label={
        showLabel
          ? liked
            ? '取消收藏歌曲'
            : '收藏歌曲'
          : liked
            ? '取消收藏当前歌曲'
            : '收藏当前歌曲'
      }
      aria-pressed={liked}
      aria-busy={likePending}
      title={
        !track
          ? '选择一首音乐后收藏'
          : songId === undefined
            ? '当前来源账号不支持收藏这首歌曲'
            : !profile
              ? '登录后收藏歌曲'
              : !likesReady
                ? '正在读取收藏状态'
                : canToggleLike === false
                  ? '收藏状态（当前来源仅支持读取）'
                  : liked
                    ? '取消收藏'
                    : '收藏'
      }
      disabled={
        !isTauri() ||
        songId === undefined ||
        !profile ||
        !likesReady ||
        likePending ||
        canToggleLike === false
      }
      onClick={() => {
        if (songId !== undefined) void toggleLike(songId).catch(onError)
      }}
    >
      <Heart aria-hidden="true" />
      {showLabel && (likePending ? '处理中…' : liked ? '已收藏' : '收藏')}
    </Button>
  )
}
