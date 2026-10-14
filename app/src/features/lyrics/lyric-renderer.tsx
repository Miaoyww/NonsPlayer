import { LyricPlayer, type LyricPlayerRef } from '@applemusic-like-lyrics/react'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import '@applemusic-like-lyrics/core/style.css'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { useReducedMotion } from 'motion/react'
import { currentPosition, nativeCall, useProgress } from '@/lib/player'
import { useFontSettings } from '@/features/settings/use-font-settings'
import { isAndroid } from '@/lib/platform'

export default function LyricRenderer({
  lines,
  showTranslation = true,
  showPronunciation = true,
  onError,
  active = true,
  retainWhenHidden = false
}: {
  lines: LyricLine[]
  showTranslation?: boolean
  showPronunciation?: boolean
  onError: (cause: unknown) => void
  active?: boolean
  retainWhenHidden?: boolean
}) {
  const { fonts } = useFontSettings()
  const lyricFont = fonts.lyrics || fonts.app
  // AMLL consumes immutable lyric lines; retain the original auxiliary lyrics for restoring them.
  const displayedLines = useMemo(
    () =>
      showTranslation && showPronunciation
        ? lines
        : lines.map((line) => ({
            ...line,
            translatedLyric: showTranslation ? line.translatedLyric : '',
            romanLyric: showPronunciation ? line.romanLyric : '',
            words: showPronunciation
              ? line.words
              : line.words.map((word) => ({ ...word, romanWord: undefined, ruby: undefined }))
          })),
    [lines, showTranslation, showPronunciation]
  )
  const [visible, setVisible] = useState(document.visibilityState !== 'hidden')
  const enabled = visible && active
  const progress = useProgress(enabled)
  const [renderer, setRenderer] = useState<LyricPlayerRef | null>(null)
  const retainRenderer = useCallback((value: LyricPlayerRef | null) => setRenderer(value), [])
  const [layoutVersion, setLayoutVersion] = useState(0)
  const reduced = useReducedMotion()
  const playing = progress.status === 'playing'
  const pausedPosition = playing ? 0 : progress.positionMs
  const lastFont = useRef<string | undefined>(undefined)
  useEffect(() => {
    const player = renderer?.lyricPlayer
    if (!enabled || !player || lastFont.current === lyricFont) return
    let disposed = false
    let frame = 0
    // Font metrics affect word masks as well as line positions, including while paused.
    void document.fonts.ready.then(() => {
      if (disposed) return
      frame = requestAnimationFrame(() => {
        player.rebuildLyricView()
        lastFont.current = lyricFont
        setLayoutVersion((version) => version + 1)
      })
    })
    return () => {
      disposed = true
      cancelAnimationFrame(frame)
    }
  }, [lyricFont, renderer, enabled])
  useEffect(() => {
    const changed = () => setVisible(document.visibilityState !== 'hidden')
    document.addEventListener('visibilitychange', changed)
    return () => document.removeEventListener('visibilitychange', changed)
  }, [])
  useEffect(() => {
    if (!renderer?.wrapperEl || !enabled) return
    const observer = new ResizeObserver(() => setLayoutVersion((version) => version + 1))
    observer.observe(renderer.wrapperEl)
    return () => observer.disconnect()
  }, [renderer, enabled])
  useEffect(() => {
    const player = renderer?.lyricPlayer
    if (!enabled || !player) return
    let frame = 0
    let previous = performance.now()
    const started = previous
    const tick = (now: number) => {
      player.setCurrentTime(playing ? currentPosition(now) : pausedPosition)
      player.update(Math.min(50, now - previous))
      previous = now
      // Let initial/seek/resize layout settle even when entering a paused song.
      if (playing || now - started < 1000) frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [enabled, playing, pausedPosition, renderer, displayedLines, layoutVersion])
  // AMLL's automatic frame loop is disabled; only the visible lyric subtree
  // receives an interpolated clock. React does not render on every frame.
  return retainWhenHidden || visible ? (
    <LyricPlayer
      ref={retainRenderer}
      className="lyric-renderer h-full w-full"
      lyricLines={displayedLines}
      disabled
      playing={enabled && playing}
      currentTime={progress.positionMs}
      onLyricLineClick={(event) => {
        if (!isTauri() || isAndroid()) return
        void nativeCall('player_seek', { positionMs: event.line.getLine().startTime }).catch(
          onError
        )
      }}
      enableBlur={!reduced}
      enableScale={!reduced}
      enableSpring={!reduced}
      alignPosition={0.4}
    />
  ) : null
}
