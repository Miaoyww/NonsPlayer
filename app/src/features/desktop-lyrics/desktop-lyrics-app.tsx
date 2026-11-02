import { listen } from '@tauri-apps/api/event'
import {
  AArrowDown,
  AArrowUp,
  AudioLines,
  Languages,
  Lock,
  LockOpen,
  Pause,
  Play,
  SkipBack,
  SkipForward,
  X
} from 'lucide-react'
import { useCallback, useEffect, useRef, useState, type PointerEvent } from 'react'
import { reportCaughtError } from '@/lib/diagnostics'
import {
  adjacentIndex,
  connectPlayer,
  nativeCall,
  usePlayer,
  useProgress,
  type Track
} from '@/lib/player'
import { offsetLyrics, type LyricCorrection } from '@/features/lyrics/correction'
import { loadLyrics, lyricTrackVersion } from '@/features/lyrics/load-lyrics'
import { parseLyrics } from '@/features/lyrics/parse-lyrics'
import { useLyricSources } from '@/features/lyrics/use-lyric-sources'
import {
  maxLyricsFontScale,
  minLyricsFontScale,
  useLyricsSettings
} from '@/features/lyrics/use-lyrics-settings'
import { DesktopLyricStage } from './desktop-lyric-stage'
import { lyricPronunciation } from './lyric-display'
import {
  dragDesktopLyrics,
  setDesktopLyricsEnabled,
  setDesktopLyricsHeight,
  setDesktopLyricsLocked,
  useDesktopLyricsHover,
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
  const windowHovered = useDesktopLyricsHover()
  const snapshot = usePlayer('index', 'queue', 'repeatMode', 'shuffle', 'shuffleOrder', 'status')
  const track = snapshot.index === null ? undefined : snapshot.queue[snapshot.index]
  const playing = snapshot.status === 'playing'
  const transportPlaying = ['playing', 'buffering', 'loading'].includes(snapshot.status)
  const previousIndex = adjacentIndex(snapshot, 'previous')
  const nextIndex = adjacentIndex(snapshot, 'next')
  const {
    desktopLyricsFontWeight,
    desktopLyricsFontScale,
    setDesktopLyricsFontScale,
    showDesktopLyricsDoubleLine,
    showDesktopLyricsTranslation,
    setShowDesktopLyricsTranslation,
    showDesktopLyricsPronunciation,
    setShowDesktopLyricsPronunciation,
    desktopLyricsSungColor
  } = useLyricsSettings()
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
    const target = Math.ceil(height + 76)
    if (Number.isNaN(target) || Math.abs(target - lastHeight.current) < 1) return
    lastHeight.current = target
    void setDesktopLyricsHeight(target).catch((cause) =>
      reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-app.tsx', cause)
    )
  }, [])
  const beforeFirstLine = lines.length > 0 && progress.positionMs < lines[0].startTime
  const showTrackInfo = !!track && !loading && (!lines.length || beforeFirstLine)
  const hasTranslation = lines.some((line) => !!line.translatedLyric.trim())
  const hasPronunciation = lines.some((line) => !!lyricPronunciation(line).trim())

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
      {locked && (
        <div className={`desktop-lyrics-unlock-zone${windowHovered ? ' visible' : ''}`}>
          <button
            type="button"
            aria-label="解锁桌面歌词窗口"
            title="解锁"
            onClick={() => run(() => setDesktopLyricsLocked(false))}
          >
            <LockOpen aria-hidden="true" />
          </button>
        </div>
      )}
      <div className="desktop-lyrics-header" aria-hidden={locked ? true : undefined}>
        <div className="desktop-lyrics-track-info">
          <span className="desktop-lyrics-track-title">{track?.title ?? ''}</span>
          <span className="desktop-lyrics-track-artist">{track?.artist ?? ''}</span>
        </div>
        <div className="desktop-lyrics-playback" role="group" aria-label="播放控制">
          <button
            type="button"
            aria-label="上一首"
            title="上一首"
            disabled={!track || previousIndex === null}
            onClick={() => run(() => nativeCall('player_action', { action: 'previous' }))}
          >
            <SkipBack aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label={transportPlaying ? '暂停' : '播放'}
            title={transportPlaying ? '暂停' : '播放'}
            disabled={!track}
            onClick={() => run(() => nativeCall('player_action', { action: 'toggle' }))}
          >
            {transportPlaying ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
          </button>
          <button
            type="button"
            aria-label="下一首"
            title="下一首"
            disabled={!track || nextIndex === null}
            onClick={() => run(() => nativeCall('player_action', { action: 'next' }))}
          >
            <SkipForward aria-hidden="true" />
          </button>
        </div>
        <div className="desktop-lyrics-controls">
          <button
            type="button"
            aria-label="减小桌面歌词字号"
            title="减小字号"
            disabled={desktopLyricsFontScale <= minLyricsFontScale}
            onClick={() => setDesktopLyricsFontScale(desktopLyricsFontScale - 0.05)}
          >
            <AArrowDown aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label="增大桌面歌词字号"
            title="增大字号"
            disabled={desktopLyricsFontScale >= maxLyricsFontScale}
            onClick={() => setDesktopLyricsFontScale(desktopLyricsFontScale + 0.05)}
          >
            <AArrowUp aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label="显示桌面歌词翻译"
            aria-pressed={hasTranslation && showDesktopLyricsTranslation}
            title={
              hasTranslation ? (showDesktopLyricsTranslation ? '隐藏翻译' : '显示翻译') : '暂无翻译'
            }
            disabled={!hasTranslation}
            onClick={() => setShowDesktopLyricsTranslation(!showDesktopLyricsTranslation)}
          >
            <Languages aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label="显示桌面歌词发音"
            aria-pressed={hasPronunciation && showDesktopLyricsPronunciation}
            title={
              hasPronunciation
                ? showDesktopLyricsPronunciation
                  ? '隐藏发音'
                  : '显示发音'
                : '暂无发音'
            }
            disabled={!hasPronunciation}
            onClick={() => setShowDesktopLyricsPronunciation(!showDesktopLyricsPronunciation)}
          >
            <AudioLines aria-hidden="true" />
          </button>
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
      </div>
      {lines.length > 0 && !showTrackInfo && (
        <DesktopLyricStage
          lines={lines}
          trackId={track?.key ?? ''}
          playing={playing}
          fontScale={desktopLyricsFontScale}
          fontWeight={desktopLyricsFontWeight}
          doubleLine={showDesktopLyricsDoubleLine}
          showTranslation={showDesktopLyricsTranslation}
          showPronunciation={showDesktopLyricsPronunciation}
          sungColor={desktopLyricsSungColor}
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
