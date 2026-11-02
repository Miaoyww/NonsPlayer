import { LyricPlayer, type LyricPlayerRef } from '@applemusic-like-lyrics/react'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import '@applemusic-like-lyrics/core/style.css'
import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import { reportCaughtError } from '@/lib/diagnostics'
import {
  connectPlayer,
  currentPosition,
  getProgress,
  nativeCall,
  usePlayer,
  useProgress,
  type Track
} from '@/lib/player'
import { offsetLyrics, type LyricCorrection } from '@/features/lyrics/correction'
import { loadLyrics, lyricTrackVersion } from '@/features/lyrics/load-lyrics'
import { parseLyrics } from '@/features/lyrics/parse-lyrics'
import { useLyricSources } from '@/features/lyrics/use-lyric-sources'
import { useLyricsSettings } from '@/features/lyrics/use-lyrics-settings'
import {
  dragDesktopLyrics,
  setDesktopLyricsEnabled,
  setDesktopLyricsLocked,
  useDesktopLyricsState
} from './state'

const INITIAL_SURFACE_HEIGHT = 140

interface RenderedWord {
  mainElement: HTMLElement
  endTime: number
}

interface RenderedGroup {
  mainLine?: {
    splittedWords?: RenderedWord[]
  }
}

function paintSungWords(groups: readonly unknown[], position: number) {
  for (const group of groups) {
    for (const word of (group as RenderedGroup).mainLine?.splittedWords ?? [])
      word.mainElement.classList.toggle('desktop-lyrics-sung', word.endTime <= position)
  }
}

function lineIndexAt(lines: LyricLine[], time: number) {
  let low = 0
  let high = lines.length
  while (low < high) {
    const middle = (low + high) >> 1
    if (lines[middle].startTime <= time) low = middle + 1
    else high = middle
  }
  return low - 1
}

function useDesktopLyrics(track: Track | undefined) {
  const { sources, ready } = useLyricSources()
  const { lyricsOffsetMs } = useLyricsSettings()
  const [lines, setLines] = useState<LyricLine[]>([])
  const [loading, setLoading] = useState(false)
  const [revision, setRevision] = useState(0)
  const version = track ? lyricTrackVersion(track) : ''

  useEffect(() => {
    if (!track) return
    let disposed = false
    const cleanups: (() => void)[] = []
    const bind = async () => {
      for (const event of ['lyrics-updated', 'lyric-correction-changed']) {
        const cleanup = await listen<{ key: string } | string>(event, ({ payload }) => {
          const key = typeof payload === 'string' ? payload : payload.key
          if (!disposed && key === track.key) setRevision((value) => value + 1)
        })
        if (disposed) cleanup()
        else cleanups.push(cleanup)
      }
    }
    void bind().catch((cause) =>
      reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
    )
    return () => {
      disposed = true
      cleanups.forEach((cleanup) => cleanup())
    }
  }, [track?.key])

  useEffect(() => {
    setLines([])
    if (!track) {
      setLoading(false)
      return
    }
    if (!ready) {
      setLoading(true)
      return
    }
    let disposed = false
    setLoading(true)
    void nativeCall<LyricCorrection>('lyric_correction', { key: track.key })
      .then((correction) =>
        disposed
          ? null
          : loadLyrics(
              track,
              false,
              sources,
              (value) =>
                value
                  ? offsetLyrics(
                      parseLyrics(
                        value,
                        track.durationMs,
                        track.source.kind === 'local' ? track.source.cue : undefined
                      ),
                      correction.offsetMs ?? lyricsOffsetMs
                    )
                  : [],
              () => !disposed,
              !!correction.candidate
            )
      )
      .then((result) => {
        if (!disposed && result) setLines(result.parsed)
      })
      .catch((cause) => {
        if (disposed) return
        reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
        setLines([])
      })
      .finally(() => {
        if (!disposed) setLoading(false)
      })
    return () => {
      disposed = true
    }
  }, [track, version, ready, sources, lyricsOffsetMs, revision])

  return { lines, loading }
}

export function DesktopLyricsApp() {
  const { locked } = useDesktopLyricsState()
  const snapshot = usePlayer('index', 'queue', 'status')
  const track = snapshot.index === null ? undefined : snapshot.queue[snapshot.index]
  const playing = snapshot.status === 'playing'
  const progress = useProgress(true)
  const { fontWeight, fontScale } = useLyricsSettings()
  const { lines, loading } = useDesktopLyrics(track)
  const displayedLines = useMemo(
    () =>
      lines.map((line) => ({
        ...line,
        romanLyric: '',
        words: line.words.map((word) => ({ ...word, romanWord: undefined, ruby: undefined }))
      })),
    [lines]
  )
  const [renderer, setRenderer] = useState<LyricPlayerRef | null>(null)
  const retainRenderer = useCallback((value: LyricPlayerRef | null) => setRenderer(value), [])
  const [surfaceHeight, setSurfaceHeight] = useState(INITIAL_SURFACE_HEIGHT)
  const heightRef = useRef(0)
  const seekRef = useRef(true)

  useEffect(() => {
    let disposed = false
    let stop: (() => void) | undefined
    void connectPlayer({ observerOnly: true })
      .then((cleanup) => {
        if (disposed) cleanup()
        else stop = cleanup
      })
      .catch((cause) =>
        reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
      )
    return () => {
      disposed = true
      stop?.()
    }
  }, [])

  useEffect(() => {
    seekRef.current = true
    heightRef.current = 0
    setSurfaceHeight(INITIAL_SURFACE_HEIGHT)
  }, [track?.key])

  useEffect(() => {
    const core = renderer?.lyricPlayer
    if (!core) return
    let disposed = false
    let frame = 0
    void document.fonts.ready.then(() => {
      if (disposed) return
      frame = requestAnimationFrame(() => {
        core.rebuildLyricView()
        paintSungWords(core.currentLyricGroups, getProgress().positionMs)
      })
    })
    return () => {
      disposed = true
      cancelAnimationFrame(frame)
    }
  }, [renderer, fontWeight, fontScale])

  useEffect(() => {
    const core = renderer?.lyricPlayer
    if (!core) return
    core.setHidePassedLines(false)
  }, [renderer])

  useEffect(() => {
    const core = renderer?.lyricPlayer
    if (!core) return
    let frame = 0
    let previous = performance.now()
    const started = previous
    const tick = (now: number) => {
      const delta = Math.min(50, now - previous)
      previous = now
      const position = playing ? currentPosition(now) : getProgress().positionMs
      core.setCurrentTime(position, seekRef.current)
      seekRef.current = false
      core.update(delta)
      paintSungWords(core.currentLyricGroups, position)
      if (displayedLines.length) {
        const index = Math.max(
          0,
          Math.min(displayedLines.length - 1, lineIndexAt(displayedLines, position))
        )
        const element = core.currentLyricGroups[index]?.getElement()
        const height = element?.getBoundingClientRect().height ?? 0
        if (height > 1 && Math.abs(height - heightRef.current) > 1) {
          heightRef.current = height
          setSurfaceHeight(Math.ceil(height))
        }
      }
      if (playing || now - started < 1000) frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [renderer, playing, displayedLines])

  useEffect(() => {
    if (playing) return
    const core = renderer?.lyricPlayer
    if (!core) return
    core.setCurrentTime(progress.positionMs, true)
    core.update(0)
    paintSungWords(core.currentLyricGroups, progress.positionMs)
  }, [renderer, playing, progress.positionMs])

  useEffect(() => {
    if (locked) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      void setDesktopLyricsLocked(true).catch((cause) =>
        reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
      )
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [locked])

  const run = (action: () => Promise<unknown>) =>
    void action().catch((cause) =>
      reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
    )
  const placeholder = !!track && !displayedLines.length && !loading

  return (
    <div
      className={`desktop-lyrics${locked ? '' : ' unlocked'}`}
      style={
        {
          fontWeight,
          '--desktop-lyrics-font-size': `calc(clamp(32px, 3.6vw, 56px) * ${Math.min(fontScale, 1.4)})`
        } as CSSProperties
      }
    >
      {!locked && (
        <div className="desktop-lyrics-toolbar">
          <button
            type="button"
            className="desktop-lyrics-drag"
            onPointerDown={(event) => {
              event.preventDefault()
              run(dragDesktopLyrics)
            }}
          >
            拖动
          </button>
          <button type="button" onClick={() => run(() => setDesktopLyricsLocked(true))}>
            锁定
          </button>
          <button
            type="button"
            onClick={() => {
              // The window is destroyed while this call is in flight.
              void setDesktopLyricsEnabled(false).catch(() => {})
            }}
          >
            关闭
          </button>
        </div>
      )}
      {displayedLines.length > 0 && (
        <div className="desktop-lyrics-surface" style={{ height: surfaceHeight }}>
          <LyricPlayer
            ref={retainRenderer}
            className="desktop-lyrics-player"
            lyricLines={displayedLines}
            disabled
            playing={playing}
            alignAnchor="center"
            alignPosition={0.5}
            enableBlur={false}
            enableScale={false}
            wordFadeWidth={0.5}
          />
        </div>
      )}
      {placeholder && <p className="desktop-lyrics-empty">暂无歌词</p>}
    </div>
  )
}
