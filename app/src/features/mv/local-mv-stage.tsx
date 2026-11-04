import { currentPosition, getProgress, useProgress } from '@/lib/player'
import { useEffect, useRef, useState } from 'react'
import { localMvNeedsSync, localMvTargetSeconds, localMvWaiting } from './sync'

function align(element: HTMLVideoElement, positionMs: number, offsetMs: number, force: boolean) {
  if (element.readyState < 1 || (!force && element.seeking)) return
  if (localMvWaiting(positionMs, offsetMs)) {
    if (element.currentTime > 0.05) element.currentTime = 0
    return
  }
  const target = localMvTargetSeconds(positionMs, element.duration, offsetMs)
  if (!force && !localMvNeedsSync(element.currentTime, target)) return
  if (element.ended && target >= element.duration) return
  try {
    element.currentTime = target
  } catch {
    /* A source switch can invalidate the seek before metadata is ready. */
  }
}

export function LocalMvStage({
  source,
  active,
  offsetMs,
  onError
}: {
  source: string
  active: boolean
  offsetMs: number
  onError: (cause: unknown) => void
}) {
  const video = useRef<HTMLVideoElement>(null)
  const errorHandler = useRef(onError)
  errorHandler.current = onError
  const [visible, setVisible] = useState(() => document.visibilityState !== 'hidden')
  const progress = useProgress(active && visible)
  const playing = active && visible && progress.status === 'playing'

  useEffect(() => {
    const changed = () => setVisible(document.visibilityState !== 'hidden')
    document.addEventListener('visibilitychange', changed)
    return () => document.removeEventListener('visibilitychange', changed)
  }, [])

  useEffect(() => {
    const element = video.current
    if (!element) return
    element.muted = true
    element.defaultMuted = true
    element.volume = 0
    if (!playing) {
      element.pause()
      align(element, progress.positionMs, offsetMs, true)
      return
    }
    const position = currentPosition()
    align(element, position, offsetMs, true)
    if (!localMvWaiting(position, offsetMs)) {
      void element.play().catch((cause) => {
        if (cause instanceof DOMException && cause.name === 'AbortError') return
        errorHandler.current(new Error('本地 MV 无法开始播放。'))
      })
    }
  }, [source, offsetMs, playing])

  useEffect(() => {
    if (playing) return
    const element = video.current
    if (element) align(element, progress.positionMs, offsetMs, true)
  }, [offsetMs, playing, progress.positionMs, progress.revision])

  useEffect(() => {
    if (!playing) return
    let frame = 0
    const tick = () => {
      const element = video.current
      const live = getProgress()
      if (element && live.status === 'playing' && performance.now() - live.receivedAt < 500) {
        const position = currentPosition()
        const waiting = localMvWaiting(position, offsetMs)
        align(element, position, offsetMs, false)
        if (waiting) {
          if (!element.paused) element.pause()
        } else if (element.paused && !element.ended) {
          void element.play().catch((cause) => {
            if (cause instanceof DOMException && cause.name === 'AbortError') return
            errorHandler.current(new Error('本地 MV 无法开始播放。'))
          })
        }
      }
      frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [offsetMs, playing])

  return (
    <div className="local-mv-stage" aria-hidden="true">
      <video
        ref={video}
        className="local-mv-video"
        src={source}
        muted
        playsInline
        preload="auto"
        disablePictureInPicture
        controls={false}
        tabIndex={-1}
        onLoadedMetadata={(event) =>
          align(event.currentTarget, progress.positionMs, offsetMs, true)
        }
        onError={() => errorHandler.current(new Error('本地 MV 无法播放，请确认视频格式受支持。'))}
      />
    </div>
  )
}
