import { reportCaughtError } from '@/lib/diagnostics'
import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { ErrorNotice } from '@/components/error-notice'
import { TrackArtists } from '@/components/music/music-links'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { isTauri } from '@tauri-apps/api/core'
import { motion, useIsPresent, useReducedMotion } from 'motion/react'
import { AudioLines, Languages } from 'lucide-react'
import { errorText, getPlayer, nativeCall, usePlayer, type Lyrics, type Track } from '@/lib/player'
import { offsetLyrics, automaticLyrics, type LyricCorrection } from './correction'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { CoverContextMenu } from '@/features/playback/cover-context-menu'
import { AlbumBackground } from '@/features/lyrics/album-background'
import { NowPlayingControls } from '@/features/playback/now-playing-controls'
import { NowPlayingMenu } from '@/features/playback/now-playing-menu'
import {
  getLyricSources,
  useLyricSources,
  type LyricSources
} from '@/features/lyrics/use-lyric-sources'
import { loadLyrics, lyricTrackVersion } from '@/features/lyrics/load-lyrics'
import { EmptyLyricsError, parseLyrics } from '@/features/lyrics/parse-lyrics'
import { LyricLoading } from './lyric-loading'
import { isAndroid } from '@/lib/platform'
import { MobileFullscreenPlayer } from '@/features/playback/mobile-fullscreen-player'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/playback-notice'
import { requestCache, RUNTIME_TTL } from '@/lib/runtime-cache'
import { useLyricsSettings } from './use-lyrics-settings'

function isCurrentPlayback(key: string, revision: number, version: string) {
  const active = getPlayer()
  return (
    active.revision === revision &&
    active.index !== null &&
    active.queue[active.index]?.key === key &&
    lyricTrackVersion(active.queue[active.index]) === version
  )
}

const loadLyricRenderer = () => import('./lyric-renderer')
const DeferredLyricRenderer = lazy(loadLyricRenderer)

function LyricRenderer(props: {
  lines: LyricLine[]
  showTranslation: boolean
  showPronunciation: boolean
  active?: boolean
  retainWhenHidden?: boolean
  onError: (cause: unknown) => void
}) {
  return (
    <Suspense fallback={<LyricLoading />}>
      <DeferredLyricRenderer {...props} />
    </Suspense>
  )
}

export default function LyricsView({
  open = true,
  onQueue,
  notice,
  onClose,
  onExitComplete
}: {
  open?: boolean
  onQueue: () => void
  notice?: PlaybackNoticeMessage
  onClose: () => void
  onExitComplete?: () => void
}) {
  const reduced = useReducedMotion()
  const present = useIsPresent()
  const isPresent = open && present
  const openRef = useRef(isPresent)
  openRef.current = isPresent
  const [hidden, setHidden] = useState(false)
  useEffect(() => {
    if (open) setHidden(false)
  }, [open])
  const mobile = isAndroid()
  const [entered, setEntered] = useState(!!reduced)
  const rendering = isPresent
  useEffect(() => {
    // Load alongside the lyrics request, without waiting for data or entry animation.
    void loadLyricRenderer().catch((cause) => {
      reportCaughtError('app/src/features/lyrics/lyrics-view.tsx', cause)
    })
  }, [])
  const state = usePlayer('error', 'index', 'mediaError', 'queue', 'revision', 'status')
  const track = state.index !== null ? state.queue[state.index] : undefined
  const trackVersion = useMemo(() => lyricTrackVersion(track), [track])
  const { sources } = useLyricSources()
  const { lyricsOffsetMs } = useLyricsSettings()
  const { options: localPreferences } = useLocalPreferences()
  const [display, setDisplay] = useState<{
    key: string
    revision: number
    lines: LyricLine[]
    source?: string
    selection: LyricSources
    version: string
    preferences: typeof localPreferences
  }>()
  const currentDisplay =
    display?.key === track?.key &&
    display?.revision === state.revision &&
    display?.selection === sources &&
    display?.version === trackVersion &&
    display?.preferences === localPreferences
      ? display
      : undefined
  const lines = currentDisplay?.lines ?? []
  const source = currentDisplay?.source
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string>()
  const playbackError = error
  const [showTranslation, setShowTranslation] = useState(true)
  const [showPronunciation, setShowPronunciation] = useState(true)
  const hasTranslation = lines.some((line) => !!line.translatedLyric.trim())
  const hasPronunciation = lines.some(
    (line) =>
      !!line.romanLyric.trim() ||
      line.words.some((word) => !!word.romanWord?.trim() || !!word.ruby?.length)
  )
  const generation = useRef(0)
  const appliedSource = useRef<string | undefined>(undefined)
  const correction = useRef<LyricCorrection>(automaticLyrics)
  const reusable = useRef<
    | {
        version: string
        revision: number
        sources: LyricSources
        preferences: typeof localPreferences
        lyricsOffsetMs: number
        cacheRevision: number
        updatedAt: number
      }
    | undefined
  >(undefined)
  const apply = useCallback(
    (
      value: Lyrics | null,
      duration: number,
      key: string,
      revision: number,
      cue?: { startMs: number; endMs: number }
    ) => {
      const parsed = value
        ? offsetLyrics(
            parseLyrics(value, duration, cue),
            correction.current.offsetMs ?? lyricsOffsetMs
          )
        : []
      appliedSource.current = value?.source
      reusable.current = value
        ? {
            version: trackVersion,
            revision,
            sources,
            preferences: localPreferences,
            lyricsOffsetMs,
            cacheRevision: requestCache.revision,
            updatedAt: Date.now()
          }
        : undefined
      setDisplay({
        key,
        revision,
        selection: sources,
        version: trackVersion,
        preferences: localPreferences,
        lines: parsed,
        source: value
          ? { amll: 'AMLL DB', qq: 'QQ 音乐', netease: '网易云音乐', local: '本地' }[value.source]
          : undefined
      })
    },
    [sources, trackVersion, localPreferences, lyricsOffsetMs]
  )
  const requestLyrics = useCallback(
    async (refresh = false, reuse = false) => {
      if (!openRef.current) return
      if (getLyricSources() !== sources) return
      if (track && !isCurrentPlayback(track.key, state.revision, trackVersion)) return
      const previous = reusable.current
      if (
        reuse &&
        !refresh &&
        previous &&
        !(track?.source.kind === 'local' && sources.localSource === 'local') &&
        previous.version === trackVersion &&
        previous.revision === state.revision &&
        previous.sources === sources &&
        previous.preferences === localPreferences &&
        previous.lyricsOffsetMs === lyricsOffsetMs &&
        previous.cacheRevision === requestCache.revision &&
        Date.now() - previous.updatedAt < RUNTIME_TTL
      ) {
        setLoading(false)
        return
      }
      const serial = ++generation.current
      reusable.current = undefined
      appliedSource.current = undefined
      setError(undefined)
      setLoading(false)
      if (!track || !isTauri()) return
      setLoading(true)
      // Native playback can switch before React's passive-effect cleanup runs.
      const isCurrent = () =>
        generation.current === serial &&
        openRef.current &&
        getLyricSources() === sources &&
        isCurrentPlayback(track.key, state.revision, trackVersion)
      try {
        const selected = await nativeCall<LyricCorrection>('lyric_correction', { key: track.key })
        if (!isCurrent()) return
        correction.current = selected
        await loadLyrics(
          track,
          refresh,
          sources,
          (value) => {
            if (!value && selected.candidate)
              throw new Error('已选歌词版本暂不可用，请刷新重试或在歌词来源中重新选择。')
            return apply(
              value,
              track.durationMs,
              track.key,
              state.revision,
              track.source.kind === 'local' ? track.source.cue : undefined
            )
          },
          isCurrent,
          !!selected.candidate
        )
      } catch (cause) {
        reportCaughtError('app/src/features/lyrics/lyrics-view.tsx', cause)

        if (isCurrent() && !(cause instanceof EmptyLyricsError)) setError(errorText(cause))
      } finally {
        if (isCurrent()) setLoading(false)
      }
    },
    [trackVersion, state.revision, apply, sources, localPreferences]
  )
  useEffect(() => {
    if (isPresent) void requestLyrics(false, true)
    return () => {
      generation.current++
    }
  }, [requestLyrics, isPresent])
  useEffect(() => {
    if (!isTauri() || !track) return
    let disposed = false
    let stop: (() => void) | undefined
    void listen<{ key: string; lyrics: Lyrics; track?: Track }>('lyrics-updated', ({ payload }) => {
      if (
        !disposed &&
        openRef.current &&
        !correction.current.candidate &&
        getLyricSources() === sources &&
        !(track.source.kind === 'local' && sources.localSource === 'local') &&
        payload.lyrics.source !== 'local' &&
        isCurrentPlayback(track.key, state.revision, trackVersion) &&
        appliedSource.current === payload.lyrics.source &&
        payload.key === track.key &&
        (!payload.track || lyricTrackVersion(payload.track) === trackVersion) &&
        (payload.lyrics.source !== 'amll' || sources.amll) &&
        (payload.lyrics.source !== 'qq' || sources.qq)
      ) {
        try {
          apply(payload.lyrics, track.durationMs, track.key, state.revision)
        } catch (cause) {
          reportCaughtError('app/src/features/lyrics/lyrics-view.tsx', cause)

          /* Retain the last usable lyrics. */
        }
      }
    })
      .then((unlisten) => {
        if (disposed) unlisten()
        else stop = unlisten
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/lyrics/lyrics-view.tsx', cause)

        if (!disposed && isCurrentPlayback(track.key, state.revision, trackVersion))
          setError(errorText(cause))
      })
    return () => {
      disposed = true
      stop?.()
    }
  }, [trackVersion, state.revision, apply, sources, localPreferences])

  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    let stop: (() => void) | undefined
    void listen<string>('lyric-correction-changed', ({ payload }) => {
      if (!disposed && payload === track?.key) {
        reusable.current = undefined
        void requestLyrics()
      }
    })
      .then((cleanup) => {
        if (disposed) cleanup()
        else stop = cleanup
      })
      .catch((cause) => {
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
      stop?.()
    }
  }, [requestLyrics, track?.key])

  const lyricControls = (
    <div className="lyrics-display-controls" role="group" aria-label="歌词显示">
      <ActionButton
        variant="ghost"
        size="icon"
        disabled={!hasTranslation}
        aria-label="显示歌词翻译"
        aria-pressed={hasTranslation && showTranslation}
        title={hasTranslation ? (showTranslation ? '隐藏翻译' : '显示翻译') : '暂无翻译'}
        onClick={() => setShowTranslation((value) => !value)}
      >
        <Languages aria-hidden="true" />
      </ActionButton>
      <ActionButton
        variant="ghost"
        size="icon"
        disabled={!hasPronunciation}
        aria-label="显示歌词发音"
        aria-pressed={hasPronunciation && showPronunciation}
        title={hasPronunciation ? (showPronunciation ? '隐藏发音' : '显示发音') : '暂无发音'}
        onClick={() => setShowPronunciation((value) => !value)}
      >
        <AudioLines aria-hidden="true" />
      </ActionButton>
    </div>
  )
  const renderLyrics = (active = true) =>
    lines.length ? (
      <LyricRenderer
        key={track?.key}
        lines={lines}
        showTranslation={showTranslation}
        showPronunciation={showPronunciation}
        active={active && rendering}
        retainWhenHidden
        onError={(cause) => setError(errorText(cause))}
      />
    ) : loading || lines.length ? (
      <LyricLoading />
    ) : (
      <div
        role="status"
        className="flex h-full items-center justify-center text-sm text-muted-foreground"
      >
        暂无歌词
      </div>
    )
  const showLyrics = !!track
  const albumBackground = (
    <AlbumBackground
      cover={track?.cover}
      playing={state.status === 'playing'}
      hasLyrics={lines.length > 0}
      active={rendering && (entered || !!reduced)}
      enabled={entered || !!reduced}
      retainWhenInactive
    />
  )
  return (
    <motion.section
      className="nons-lyrics now-playing absolute inset-0 z-[5] flex flex-col pt-12"
      aria-label="正在播放"
      aria-hidden={!isPresent}
      inert={!isPresent}
      initial={{ y: reduced ? 0 : '100%' }}
      animate={open ? 'open' : 'closed'}
      variants={{
        open: { y: 0 },
        closed: {
          y: reduced ? 0 : '100%',
          transition: { type: 'tween', duration: reduced ? 0 : 0.3, ease: [0.4, 0, 1, 1] }
        }
      }}
      onAnimationComplete={(definition) => {
        if (definition === 'open' && openRef.current) setEntered(true)
        if (definition === 'closed' && !openRef.current) {
          setHidden(true)
          onExitComplete?.()
        }
      }}
      style={{
        visibility: !open && hidden ? 'hidden' : undefined,
        willChange: !open && hidden ? undefined : 'transform'
      }}
      transition={{ type: 'tween', duration: reduced ? 0 : 0.36, ease: [0.22, 1, 0.36, 1] }}
    >
      {mobile ? (
        <MobileFullscreenPlayer
          background={albumBackground}
          track={track}
          present={isPresent}
          source={source}
          loading={loading}
          onRefresh={() => void requestLyrics(true)}
          onClose={onClose}
          onError={(cause) => setError(errorText(cause))}
          renderLyrics={renderLyrics}
          lyricControls={lyricControls}
        />
      ) : (
        <>
          {albumBackground}
          {lyricControls}
          <div
            className={`now-playing-layout relative min-h-0 flex-1 ${showLyrics ? 'has-lyrics' : ''}`}
          >
            <div className="now-playing-details flex min-h-0 min-w-0 flex-col justify-center gap-5 overflow-y-auto">
              <CoverContextMenu cover={track?.cover} title={track?.title ?? '封面'}>
                <div className="now-playing-cover-slot">
                  <Cover
                    size="original"
                    cover={track?.cover}
                    loading="eager"
                    className="now-playing-cover aspect-square rounded-xl shadow-2xl"
                  />
                </div>
              </CoverContextMenu>
              <div className="flex shrink-0 items-start justify-between gap-4">
                <div className="min-w-0">
                  <h1 className="truncate text-xl font-semibold tracking-tight">
                    {track?.title ?? '让音乐开始'}
                  </h1>
                  <p className="mt-1 truncate text-muted-foreground">
                    {track ? <TrackArtists track={track} /> : '选择一首喜欢的音乐'}
                  </p>
                </div>
                <NowPlayingMenu
                  track={track}
                  source={source}
                  loading={loading}
                  onRefresh={() => void requestLyrics(true)}
                />
              </div>
              <NowPlayingControls
                onQueue={onQueue}
                active={isPresent}
                onError={(cause) => setError(errorText(cause))}
              />
            </div>
            {showLyrics && (
              <div className="now-playing-lyrics min-h-0 min-w-0 overflow-hidden">
                {renderLyrics()}
              </div>
            )}
          </div>
        </>
      )}
      <div className="fullscreen-playback-notice">
        <ErrorNotice error={playbackError} />
        <PlaybackNotice notice={notice} />
      </div>
    </motion.section>
  )
}
