import type { LyricLine } from '@applemusic-like-lyrics/core'
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react'
import { currentPosition, getProgress, useProgress } from '@/lib/player'
import { DesktopLyricStage as DesktopLyricStageController } from './lyric-stage'
import './lyric-stage.css'

export function DesktopLyricStage({
  lines,
  trackId,
  playing,
  fontScale,
  fontWeight,
  onContentHeight
}: {
  lines: LyricLine[]
  trackId: string
  playing: boolean
  fontScale: number
  fontWeight: number
  onContentHeight?: (height: number) => void
}) {
  const rootRef = useRef<HTMLDivElement>(null)
  const stageRef = useRef<DesktopLyricStageController | null>(null)
  const sourceRef = useRef({ lines, trackId })
  sourceRef.current = { lines, trackId }
  const playingRef = useRef(playing)
  playingRef.current = playing
  const [visible, setVisible] = useState(
    () => typeof document === 'undefined' || document.visibilityState !== 'hidden'
  )
  const pausedProgress = useProgress(!playing)

  useLayoutEffect(() => {
    const root = rootRef.current
    if (!root) return
    const stage = new DesktopLyricStageController(root)
    stageRef.current = stage
    stage.setSource(sourceRef.current.lines, sourceRef.current.trackId)
    stage.update(currentPosition(), playingRef.current, true)
    return () => {
      stage.destroy()
      stageRef.current = null
    }
  }, [])

  useEffect(() => {
    const changed = () => setVisible(document.visibilityState !== 'hidden')
    document.addEventListener('visibilitychange', changed)
    return () => document.removeEventListener('visibilitychange', changed)
  }, [])

  useEffect(() => {
    const stage = stageRef.current
    if (!stage) return
    stage.setSource(lines, trackId)
    stage.update(playing ? currentPosition() : getProgress().positionMs, playing, true)
  }, [lines, trackId, playing])

  useEffect(() => {
    if (playing) return
    stageRef.current?.update(pausedProgress.positionMs, false, false)
  }, [pausedProgress.positionMs, playing])

  useEffect(() => {
    if (!playing || !visible) return
    let frame = 0
    const tick = (now: number) => {
      stageRef.current?.update(currentPosition(now), true, true)
      frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [playing, visible])

  useLayoutEffect(() => {
    stageRef.current?.refresh()
  }, [fontScale, fontWeight])

  useEffect(() => {
    const viewport = rootRef.current?.querySelector<HTMLElement>('[data-lyric="viewport"]')
    if (!viewport || !onContentHeight) return
    const report = () => onContentHeight(viewport.clientHeight)
    report()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(report)
    observer.observe(viewport)
    return () => observer.disconnect()
  }, [onContentHeight])

  return (
    <div
      ref={rootRef}
      className="desktop-lyrics-stage"
      style={
        {
          '--desktop-lyrics-current-size': `calc(clamp(32px, 3.6vw, 56px) * ${Math.min(fontScale, 1.4)})`,
          '--desktop-lyrics-font-weight': String(fontWeight)
        } as CSSProperties
      }
      aria-live="polite"
    >
      <div className="desktop-lyrics-viewport" data-lyric="viewport">
        <div className="desktop-lyrics-track" data-lyric="track">
          <div className="desktop-lyrics-line current-line" data-lyric="current-line">
            <span className="line-text-viewport">
              <span className="line-text-stack">
                <span className="line-text line-text-base" data-lyric="current-text" />
                <span
                  className="line-text line-text-scan"
                  data-lyric="current-scan"
                  aria-hidden="true"
                />
              </span>
            </span>
          </div>
          <div className="desktop-lyrics-line next-line" data-lyric="secondary-line">
            <span className="line-text-viewport">
              <span className="line-text-stack">
                <span className="line-text line-text-base" data-lyric="secondary-text" />
                <span
                  className="line-text line-text-scan"
                  data-lyric="secondary-scan"
                  aria-hidden="true"
                />
              </span>
            </span>
          </div>
          <div
            className="incoming-translation-pair"
            data-lyric="incoming-translation-pair"
            aria-hidden="true"
          >
            <div className="desktop-lyrics-line current-line" data-lyric="incoming-original-line">
              <span className="line-text-viewport">
                <span className="line-text-stack">
                  <span className="line-text line-text-base" data-lyric="incoming-original-text" />
                  <span className="line-text line-text-scan" data-lyric="incoming-original-scan" />
                </span>
              </span>
            </div>
            <div
              className="desktop-lyrics-line translation-line"
              data-lyric="incoming-translation-line"
            >
              <span className="line-text-viewport">
                <span className="line-text-stack">
                  <span
                    className="line-text line-text-base"
                    data-lyric="incoming-translation-text"
                  />
                </span>
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
