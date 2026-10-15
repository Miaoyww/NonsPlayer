import { usePlayer } from '@/lib/player'

export interface PlaybackNoticeMessage {
  id: number
  message: string
}

export function PlaybackNotice({ notice }: { notice?: PlaybackNoticeMessage }) {
  const { playbackSource, status } = usePlayer('playbackSource', 'status')
  const changed = playbackSource?.changed
    ? `${status === 'error' ? '备用音源获取失败' : status === 'loading' ? '正在尝试其它音源' : '播放音源已变更为'} ${playbackSource.displayName} · ${playbackSource.title} — ${playbackSource.artist}`
    : undefined
  return (
    <div className="playback-notice-slot" role="status" aria-live="polite" aria-atomic="true">
      {notice && (
        <p key={notice.id} className="playback-notice glass-surface">
          {notice.message}
        </p>
      )}
      {changed && (
        <p
          className="playback-notice playback-source-notice glass-surface line-clamp-2"
          title={changed}
        >
          {changed}
        </p>
      )}
    </div>
  )
}
