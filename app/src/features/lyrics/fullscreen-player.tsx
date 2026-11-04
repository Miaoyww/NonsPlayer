import { lazy, Suspense, useEffect, useState } from 'react'
import type { PlaybackNoticeMessage } from '@/features/playback/bar/playback-notice'

const LyricsView = lazy(() => import('./lyrics-view'))

function PlayerLoading({ open, onExitComplete }: { open: boolean; onExitComplete: () => void }) {
  useEffect(() => {
    if (!open) onExitComplete()
  }, [open, onExitComplete])
  return open ? (
    <div
      className="now-playing absolute inset-0 z-[5] flex items-center justify-center"
      role="status"
      aria-label="正在加载播放器"
    >
      <span>正在加载播放器…</span>
    </div>
  ) : null
}

export function FullscreenPlayer({
  open,
  notice,
  onClose,
  onExitComplete
}: {
  open: boolean
  notice?: PlaybackNoticeMessage
  onClose: () => void
  onExitComplete: () => void
}) {
  const [visited, setVisited] = useState(open)
  useEffect(() => {
    if (open) setVisited(true)
  }, [open])
  if (!open && !visited) return null
  return (
    <Suspense fallback={<PlayerLoading open={open} onExitComplete={onExitComplete} />}>
      <LyricsView open={open} notice={notice} onClose={onClose} onExitComplete={onExitComplete} />
    </Suspense>
  )
}
