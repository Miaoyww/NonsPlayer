import { useEffect, useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { currentPosition, formatTime, getProgress, nativeCall, useProgress } from '@/lib/player'
import { PlayerSlider } from '@/features/playback/player-slider'

export function PlaybackTimeline({
  onError,
  layout = 'below',
  showTimeOnHover = false,
  timeRowContent
}: {
  onError: (error: unknown) => void
  layout?: 'below' | 'inline' | 'edge'
  showTimeOnHover?: boolean
  timeRowContent?: ReactNode
}) {
  const progress = useProgress()
  const [drag, setDrag] = useState<number | null>(null)
  const [hover, setHover] = useState<number | null>(null)
  const [focused, setFocused] = useState(false)
  const [clock, setClock] = useState(currentPosition())
  const position = Math.min(drag ?? clock, Math.max(1, progress.durationMs))
  const labelsVisible = layout !== 'edge' && !showTimeOnHover
  const disabled =
    !isTauri() || !progress.durationMs || ['stopped', 'error', 'loading'].includes(progress.status)

  useEffect(() => {
    if (drag !== null || disabled) return
    let frame = 0
    function paint(now = performance.now()) {
      const live = getProgress()
      if (document.visibilityState === 'hidden' || live.revision !== progress.revision) return
      const position = currentPosition(now)
      setClock(position)
      // Extrapolate only within the existing bounded playback clock. Late native
      // updates freeze the display instead of inventing continued playback.
      if (live.status === 'playing' && now - live.receivedAt < 500)
        frame = requestAnimationFrame(paint)
    }
    function resume() {
      cancelAnimationFrame(frame)
      paint()
    }
    paint()
    document.addEventListener('visibilitychange', resume)
    return () => {
      cancelAnimationFrame(frame)
      document.removeEventListener('visibilitychange', resume)
    }
  }, [progress, drag, disabled, layout])

  function commit(value: number) {
    void nativeCall('player_seek', { positionMs: value }).catch(onError)
    setClock(value)
    setDrag(null)
  }

  return (
    <div
      className={`playback-timeline playback-timeline-${layout} text-xs tabular-nums text-muted-foreground`}
    >
      {labelsVisible && <span className="timeline-elapsed">{formatTime(position)}</span>}
      <PlayerSlider
        aria-label="播放进度"
        min={0}
        max={Math.max(1, progress.durationMs)}
        step={1}
        value={position}
        aria-valuetext={`${formatTime(position)} / ${formatTime(progress.durationMs)}`}
        disabled={disabled}
        onPointerMove={
          showTimeOnHover
            ? (event) => {
                const rect = event.currentTarget.getBoundingClientRect()
                setHover(
                  Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)) *
                    progress.durationMs
                )
              }
            : undefined
        }
        onPointerLeave={() => setHover(null)}
        onFocus={() => setFocused(true)}
        onValueChange={(value) => setDrag(Number(value))}
        onValueCommitted={(value) => commit(Number(value))}
        onPointerCancel={() => setDrag(null)}
        onBlur={() => {
          setDrag(null)
          setFocused(false)
        }}
        thumbProps={{
          onKeyDown: (event) => {
            const delta =
              event.key === 'ArrowLeft' || event.key === 'ArrowDown'
                ? -1000
                : event.key === 'ArrowRight' || event.key === 'ArrowUp'
                  ? 1000
                  : event.key === 'PageDown'
                    ? -10_000
                    : event.key === 'PageUp'
                      ? 10_000
                      : null
            if (delta === null && event.key !== 'Home' && event.key !== 'End') return
            event.preventDefault()
            const next =
              event.key === 'Home'
                ? 0
                : event.key === 'End'
                  ? progress.durationMs
                  : Math.max(
                      0,
                      Math.min(
                        progress.durationMs,
                        Number(event.currentTarget.value) + (delta ?? 0)
                      )
                    )
            commit(next)
          }
        }}
        className="timeline-slider"
      />
      {labelsVisible && (
        <span className="timeline-duration">
          {layout === 'below'
            ? `-${formatTime(Math.max(0, progress.durationMs - position))}`
            : formatTime(progress.durationMs)}
        </span>
      )}
      {layout === 'below' && timeRowContent && (
        <div className="timeline-center">{timeRowContent}</div>
      )}
      {showTimeOnHover && (hover !== null || focused || drag !== null) && (
        <span
          className="timeline-tooltip glass-surface"
          aria-hidden="true"
          style={{
            left: `${Math.max(0, Math.min(100, ((drag ?? hover ?? position) / Math.max(1, progress.durationMs)) * 100))}%`
          }}
        >
          {formatTime(drag ?? hover ?? position)} / {formatTime(progress.durationMs)}
        </span>
      )}
    </div>
  )
}
