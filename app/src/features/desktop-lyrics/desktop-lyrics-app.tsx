import { listen } from '@tauri-apps/api/event'
import { Lock, X } from 'lucide-react'
import { useCallback, useEffect, useRef, useState, type PointerEvent } from 'react'
import { reportCaughtError } from '@/lib/diagnostics'
import { connectPlayer, nativeCall, usePlayer, useProgress, type Track } from '@/lib/player'
import { offsetLyrics, type LyricCorrection } from '@/features/lyrics/correction'
import { loadLyrics, lyricTrackVersion } from '@/features/lyrics/load-lyrics'
import { parseLyrics } from '@/features/lyrics/parse-lyrics'
import { useLyricSources } from '@/features/lyrics/use-lyric-sources'
import { useLyricsSettings } from '@/features/lyrics/use-lyrics-settings'
import { DesktopLyricStage } from './desktop-lyric-stage'
import {
  dragDesktopLyrics,
  setDesktopLyricsEnabled,
  setDesktopLyricsHeight,
  setDesktopLyricsLocked,
  useDesktopLyricsState
} from './state'

function useDesktopLyrics(track: Track | undefined) {
  const { sources, ready } = useLyricSources()
  const { lyricsOffsetMs } = useLyricsSettings()
  const [lines, setLines] = useState<ReturnType<typeof parseLyrics>>([])
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
  const { fontWeight, fontScale } = useLyricsSettings()
  const { lines, loading } = useDesktopLyrics(track)
  const progress = useProgress(true)
  const lastHeight = useRef(0)
  const placeholderRef = useRef<HTMLDivElement>(null)

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
  const startDragging = (event: PointerEvent<HTMLDivElement>) => {
    if (locked || event.button !== 0) return
    if ((event.target as Element).closest('button')) return
    event.preventDefault()
    run(dragDesktopLyrics)
  }
  const handleContentHeight = useCallback((height: number) => {
    const target = Math.ceil(height + 32)
    if (Number.isNaN(target) || Math.abs(target - lastHeight.current) < 1) return
    lastHeight.current = target
    void setDesktopLyricsHeight(target).catch((cause) =>
      reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
    )
  }, [])
  const beforeFirstLine = lines.length > 0 && progress.positionMs < lines[0].startTime
  const showTrackInfo = !!track && !loading && (!lines.length || beforeFirstLine)

  useEffect(() => {
    const element = placeholderRef.current
    if (!showTrackInfo || !element) return
    const report = () => handleContentHeight(element.getBoundingClientRect().height)
    report()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(report)
    observer.observe(element)
    return () => observer.disconnect()
  }, [handleContentHeight, showTrackInfo, track?.title, track?.artist])

  return (
    <div className={`desktop-lyrics${locked ? '' : ' unlocked'}`} onPointerDown={startDragging}>
      {!locked && (
        <div className="desktop-lyrics-controls">
          <button
            type="button"
            aria-label="锁定桌面歌词窗口"
            title="锁定"
            onClick={() => run(() => setDesktopLyricsLocked(true))}
          >
            <Lock aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label="关闭桌面歌词"
            title="关闭"
            onClick={() => {
              // The window is destroyed while this call is in flight.
              void setDesktopLyricsEnabled(false).catch(() => {})
            }}
          >
            <X aria-hidden="true" />
          </button>
        </div>
      )}
      {lines.length > 0 && !showTrackInfo && (
        <DesktopLyricStage
          lines={lines}
          trackId={track?.key ?? ''}
          playing={playing}
          fontScale={fontScale}
          fontWeight={fontWeight}
          onContentHeight={handleContentHeight}
        />
      )}
      {showTrackInfo && track && (
        <div ref={placeholderRef} className="desktop-lyrics-empty" role="status">
          <p className="desktop-lyrics-empty-title">{track.title}</p>
          {track.artist && <p className="desktop-lyrics-empty-artist">{track.artist}</p>}
        </div>
      )}
    </div>
  )
}
