import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useRef, useState } from 'react'
import { reportCaughtError } from '@/lib/diagnostics'
import { connectPlayer, nativeCall, usePlayer, type Track } from '@/lib/player'
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
  const lastHeight = useRef(0)

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
  const handleContentHeight = useCallback(
    (height: number) => {
      const target = Math.ceil(height + (locked ? 16 : 84))
      if (Number.isNaN(target) || Math.abs(target - lastHeight.current) < 1) return
      lastHeight.current = target
      void setDesktopLyricsHeight(target).catch((cause) =>
        reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
      )
    },
    [locked]
  )
  const placeholder = !!track && !lines.length && !loading

  return (
    <div className={`desktop-lyrics${locked ? '' : ' unlocked'}`}>
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
      {lines.length > 0 && (
        <DesktopLyricStage
          lines={lines}
          trackId={track?.key ?? ''}
          playing={playing}
          fontScale={fontScale}
          fontWeight={fontWeight}
          onContentHeight={handleContentHeight}
        />
      )}
      {placeholder && <p className="desktop-lyrics-empty">暂无歌词</p>}
    </div>
  )
}
