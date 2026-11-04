import { FolderPlus } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { ActionButton } from '@/components/music/action-button'
import { useSongActions } from '@/components/music/song-actions'
import { useAccount } from '@/features/account/account'
import { onlineReference } from '@/features/music/references'
import { useMusicSources } from '@/features/music/use-music-sources'
import type { Track } from '@/lib/player'

export function CurrentTrackCollect({ track }: { track?: Track }) {
  const actions = useSongActions()
  const { profile } = useAccount(track ? onlineReference(track)?.source : undefined)
  const { statuses } = useMusicSources()
  const reference = track ? onlineReference(track) : null
  const canCollect =
    !!track &&
    (track.source.kind === 'local' ||
      (reference?.source === profile?.reference.source &&
        statuses.some(
          (value) =>
            value.enabled &&
            value.descriptor.source === reference?.source &&
            value.descriptor.capabilities.includes('playlistWrite')
        )))
  const label = track?.source.kind === 'local' ? '添加到本地歌单' : '收藏到歌单'
  const disabled = !isTauri() || !canCollect
  const title = !track
    ? '选择一首音乐后收藏到歌单'
    : !isTauri()
      ? '仅桌面端支持收藏到歌单'
      : !profile
        ? '登录后收藏到歌单'
        : !canCollect
          ? '当前来源不支持收藏到歌单'
          : label

  return (
    <ActionButton
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      title={title}
      disabled={disabled}
      onClick={() => {
        if (track && canCollect) actions.collect(track)
      }}
    >
      <FolderPlus aria-hidden="true" />
    </ActionButton>
  )
}
