import { usePlayer } from '@/lib/player'
import { useEffect, useState } from 'react'
import { ErrorNoticeViewport } from '@/components/error-notice'
import { NOTICE_DURATION_MS } from './error-notices'

export interface PlaybackNoticeMessage {
  id: number
  message: string
}

export function PlaybackNotice({ notice }: { notice?: PlaybackNoticeMessage }) {
  const { playbackSource, status, revision } = usePlayer('playbackSource', 'status', 'revision')
  const changed = playbackSource?.changed
    ? `${status === 'error' ? '备用音源获取失败' : status === 'loading' ? '正在尝试其它音源' : '播放音源已变更为'} ${playbackSource.displayName} · ${playbackSource.title} — ${playbackSource.artist}`
    : undefined
  const [sourceNotice, setSourceNotice] = useState(changed)
  useEffect(() => {
    setSourceNotice(changed)
    if (!changed) return
    const timer = window.setTimeout(() => setSourceNotice(undefined), NOTICE_DURATION_MS)
    return () => window.clearTimeout(timer)
  }, [changed, revision, status])
  return (
    <div
      data-playback-notices
      className="playback-notice-slot"
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      {notice && (
        <p key={notice.id} className="playback-notice glass-surface">
          {notice.message}
        </p>
      )}
      <ErrorNoticeViewport />
      {sourceNotice && (
        <p
          key={`${revision}:${sourceNotice}`}
          className="playback-notice playback-source-notice glass-surface line-clamp-2"
          title={sourceNotice}
        >
          {sourceNotice}
        </p>
      )}
    </div>
  )
}
