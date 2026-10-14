import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { TrackArtists } from '@/components/music/music-links'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import { lazy, Suspense, useCallback, useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { isTauri } from '@tauri-apps/api/core'
import { motion, useIsPresent, useReducedMotion } from 'motion/react'
import { AudioLines, Languages } from 'lucide-react'
import { errorText, getPlayer, usePlayer, type Lyrics } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { AlbumBackground } from '@/features/lyrics/album-background'
import { NowPlayingControls } from '@/features/playback/now-playing-controls'
import { NowPlayingMenu } from '@/features/playback/now-playing-menu'
import {
  getLyricSources,
  useLyricSources,
  type LyricSources
} from '@/features/lyrics/use-lyric-sources'
import { loadLyrics } from '@/features/lyrics/load-lyrics'
import { EmptyLyricsError, parseLyrics } from '@/features/lyrics/parse-lyrics'
import { LyricLoading } from './lyric-loading'
import { isAndroid } from '@/lib/platform'
import { MobileFullscreenPlayer } from '@/features/playback/mobile-fullscreen-player'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/playback-notice'

function isCurrentPlayback(key: string, revision: number) {
  const active = getPlayer()
  return (
    active.revision === revision && active.index !== null && active.queue[active.index]?.key === key
  )
}

const DeferredLyricRenderer = lazy(() => import('./lyric-renderer'))

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
  onQueue,
  notice,
  onNotice,
  onClose
}: {
  onQueue: () => void
  notice?: PlaybackNoticeMessage
  onNotice: (message: string) => void
  onClose: () => void
}) {
  const reduced = useReducedMotion()
  const isPresent = useIsPresent()
  const mobile = isAndroid()
  const [entered, setEntered] = useState(!!reduced)
  const rendering = isPresent && (entered || !!reduced)
  const state = usePlayer('error', 'index', 'mediaError', 'queue', 'revision', 'status')
  const track = state.index !== null ? state.queue[state.index] : undefined
  const { sources } = useLyricSources()
  const [display, setDisplay] = useState<{
    key: string
    revision: number
    lines: LyricLine[]
    source?: string
    selection: LyricSources
  }>()
  const currentDisplay =
    display?.key === track?.key &&
    display?.revision === state.revision &&
    display?.selection === sources
      ? display
      : undefined
  const lines = currentDisplay?.lines ?? []
  const source = currentDisplay?.source
  const { options: localPreferences } = useLocalPreferences()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string>()
  const playbackError = error ?? state.error ?? state.mediaError
  useEffect(() => {
    if (playbackError) onNotice(playbackError)
  }, [playbackError, onNotice])
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
  const apply = useCallback(
    (value: Lyrics | null, duration: number, key: string, revision: number) => {
      const parsed = value ? parseLyrics(value, duration) : []
      appliedSource.current = value?.source
      setDisplay({
        key,
        revision,
        selection: sources,
        lines: parsed,
        source: value
          ? { amll: 'AMLL DB', qq: 'QQ 音乐', netease: '网易云音乐', local: '本地' }[value.source]
          : undefined
      })
    },
    [sources]
  )
  const requestLyrics = useCallback(
    async (refresh = false) => {
      if (getLyricSources() !== sources) return
      if (track && !isCurrentPlayback(track.key, state.revision)) return
      const serial = ++generation.current
      appliedSource.current = undefined
      if (!refresh) setDisplay(undefined)
      setError(undefined)
      setLoading(false)
      if (!track || !isTauri()) return
      setLoading(true)
      // Native playback can switch before React's passive-effect cleanup runs.
      const isCurrent = () =>
        generation.current === serial &&
        getLyricSources() === sources &&
        isCurrentPlayback(track.key, state.revision)
      try {
        await loadLyrics(
          track,
          refresh,
          sources,
          (value) => apply(value, track.durationMs, track.key, state.revision),
          isCurrent
        )
      } catch (cause) {
        if (isCurrent() && !(cause instanceof EmptyLyricsError)) setError(errorText(cause))
      } finally {
        if (isCurrent()) setLoading(false)
      }
    },
    [track?.key, state.revision, apply, sources, localPreferences]
  )
  useEffect(() => {
    void requestLyrics()
    return () => {
      generation.current++
    }
  }, [requestLyrics])
  useEffect(() => {
    if (!isTauri() || !track) return
    let disposed = false
    let stop: (() => void) | undefined
    void listen<{ key: string; lyrics: Lyrics }>('lyrics-updated', ({ payload }) => {
      if (
        !disposed &&
        getLyricSources() === sources &&
        !(track.source.kind === 'local' && sources.localSource === 'local') &&
        payload.lyrics.source !== 'local' &&
        isCurrentPlayback(track.key, state.revision) &&
        appliedSource.current === payload.lyrics.source &&
        payload.key === track.key &&
        (payload.lyrics.source !== 'amll' || sources.amll) &&
        (payload.lyrics.source !== 'qq' || sources.qq)
      ) {
        try {
          apply(payload.lyrics, track.durationMs, track.key, state.revision)
        } catch {
          /* Retain the last usable lyrics. */
        }
      }
    })
      .then((unlisten) => {
        if (disposed) unlisten()
        else stop = unlisten
      })
      .catch((cause) => {
        if (!disposed && isCurrentPlayback(track.key, state.revision)) setError(errorText(cause))
      })
    return () => {
      disposed = true
      stop?.()
    }
  }, [track?.key, state.revision, apply, sources, localPreferences])

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
    lines.length && rendering ? (
      <LyricRenderer
        key={track?.key}
        lines={lines}
        showTranslation={showTranslation}
        showPronunciation={showPronunciation}
        active={active}
        retainWhenHidden={mobile}
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
      active={rendering}
      enabled={entered || !!reduced}
    />
  )
  return (
    <motion.section
      className="nons-lyrics now-playing absolute inset-0 z-[5] flex flex-col pt-12"
      aria-label="正在播放"
      aria-hidden={!isPresent}
      inert={!isPresent}
      initial={{ y: reduced ? 0 : '100%' }}
      animate={{ y: 0 }}
      onAnimationComplete={() => {
        if (isPresent) setEntered(true)
      }}
      style={{ willChange: entered && isPresent ? undefined : 'transform' }}
      transition={{ type: 'tween', duration: reduced ? 0 : 0.36, ease: [0.22, 1, 0.36, 1] }}
      exit={{
        y: reduced ? 0 : '100%',
        transition: { type: 'tween', duration: reduced ? 0 : 0.3, ease: [0.4, 0, 1, 1] }
      }}
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
              <div className="now-playing-cover-slot">
                <Cover
                  size="original"
                  cover={track?.cover}
                  loading="eager"
                  className="now-playing-cover aspect-square rounded-xl shadow-2xl"
                />
              </div>
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
        <PlaybackNotice notice={notice} />
      </div>
    </motion.section>
  )
}
