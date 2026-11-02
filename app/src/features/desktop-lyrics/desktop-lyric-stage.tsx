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
  doubleLine,
  showTranslation,
  showPronunciation,
  sungColor,
  onContentHeight
}: {
  lines: LyricLine[]
  trackId: string
  playing: boolean
  fontScale: number
  fontWeight: number
  doubleLine: boolean
  showTranslation: boolean
  showPronunciation: boolean
  sungColor: string
  onContentHeight?: (height: number) => void
}) {
  const rootRef = useRef<HTMLDivElement>(null)
  const stageRef = useRef<DesktopLyricStageController | null>(null)
  const auxiliaryVisibilityRef = useRef({ showTranslation, showPronunciation })
  auxiliaryVisibilityRef.current = { showTranslation, showPronunciation }
  const doubleLineRef = useRef(doubleLine)
  doubleLineRef.current = doubleLine
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
    stage.setAuxiliaryVisibility(
      auxiliaryVisibilityRef.current.showTranslation,
      auxiliaryVisibilityRef.current.showPronunciation
    )
    stage.setDoubleLine(doubleLineRef.current)
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
    stage.setAuxiliaryVisibility(showTranslation, showPronunciation)
    stage.update(playing ? currentPosition() : getProgress().positionMs, playing, true)
  }, [showTranslation, showPronunciation])

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
    stageRef.current?.setDoubleLine(doubleLine)
    stageRef.current?.refresh()
  }, [doubleLine, fontScale, fontWeight])

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
          '--desktop-lyrics-current-size': `calc(clamp(32px, 3.6vw, 56px) * ${fontScale})`,
          '--desktop-lyrics-font-weight': String(fontWeight),
          '--desktop-lyrics-sung-color': sungColor
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
          <div className="desktop-lyrics-line next-line secondary-line" data-lyric="secondary-line">
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
          <div className="desktop-lyrics-line next-line tertiary-line" data-lyric="tertiary-line">
            <span className="line-text-viewport">
              <span className="line-text-stack">
                <span className="line-text line-text-base" data-lyric="tertiary-text" />
                <span
                  className="line-text line-text-scan"
                  data-lyric="tertiary-scan"
                  aria-hidden="true"
                />
              </span>
            </span>
          </div>
          <div
            className="incoming-auxiliary-pair"
            data-lyric="incoming-auxiliary-pair"
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
              className="desktop-lyrics-line auxiliary-line"
              data-lyric="incoming-auxiliary-1-line"
            >
              <span className="line-text-viewport">
                <span className="line-text-stack">
                  <span
                    className="line-text line-text-base"
                    data-lyric="incoming-auxiliary-1-text"
                  />
                </span>
              </span>
            </div>
            <div
              className="desktop-lyrics-line auxiliary-line tertiary-line"
              data-lyric="incoming-auxiliary-2-line"
            >
              <span className="line-text-viewport">
                <span className="line-text-stack">
                  <span
                    className="line-text line-text-base"
                    data-lyric="incoming-auxiliary-2-text"
                  />
                </span>
              </span>
            </div>
          </div>
          <div className="desktop-lyrics-line next-line preview-line" data-lyric="preview-line">
            <span className="line-text-viewport">
              <span className="line-text-stack">
                <span className="line-text line-text-base" data-lyric="preview-text" />
              </span>
            </span>
          </div>
        </div>
      </div>
    </div>
  )
}
