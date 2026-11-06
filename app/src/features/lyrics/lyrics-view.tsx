import { reportCaughtError } from '@/lib/diagnostics'
import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { ErrorNotice } from '@/components/error-notice'
import { TrackAlbum, TrackArtists } from '@/components/music/music-links'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { isTauri } from '@tauri-apps/api/core'
import { motion, useIsPresent, useReducedMotion } from 'motion/react'
import {
  AudioLines,
  Captions,
  Clapperboard,
  DiscAlbum,
  Languages,
  MessageSquareText,
  Music2
} from 'lucide-react'
import { errorText, getPlayer, nativeCall, usePlayer, type Lyrics, type Track } from '@/lib/player'
import { offsetLyrics, automaticLyrics, type LyricCorrection } from './correction'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { CoverContextMenu } from '@/features/playback/now-playing/cover-context-menu'
import { AlbumBackground } from '@/features/lyrics/album-background'
import { FullscreenPlaybackBar } from '@/features/playback/bar/fullscreen-playback-bar'
import { NowPlayingMenu } from '@/features/playback/now-playing/now-playing-menu'
import { QueuePanel } from '@/features/queue/queue-panel'
import { NowPlayingTags } from '@/features/playback/now-playing/now-playing-tags'
import { NowPlayingSource } from '@/features/playback/now-playing/now-playing-source'
import { NowPlayingLyricSource } from '@/features/lyrics/now-playing-lyric-source'
import {
  getLyricSources,
  useLyricSources,
  type LyricSources
} from '@/features/lyrics/use-lyric-sources'
import { loadLyrics, lyricTrackVersion } from '@/features/lyrics/load-lyrics'
import { EmptyLyricsError, parseLyrics } from '@/features/lyrics/parse-lyrics'
import { LyricLoading } from './lyric-loading'
import { LyricOffsetControl } from './lyric-offset-control'
import { isAndroid } from '@/lib/platform'
import { MobileFullscreenPlayer } from '@/features/playback/mobile/mobile-fullscreen-player'
import { PlaybackNotice, type PlaybackNoticeMessage } from '@/features/playback/bar/playback-notice'
import { NowPlayingInformation } from '@/features/playback/now-playing/now-playing-information'
import { requestCache, RUNTIME_TTL } from '@/lib/runtime-cache'
import { useLyricsSettings } from './use-lyrics-settings'
import { Panel } from '@/components/ui/panel'
import { Spinner } from '@/components/ui/spinner'
import { LocalMvStage } from '@/features/mv/local-mv-stage'
import { mvButtonTarget, useLocalMv } from '@/features/mv/use-local-mv'
import { MvAlignmentPopover } from '@/features/mv/mv-alignment-popover'
import { MvPanel } from '@/features/mv/mv-panel'
import { MvLyricCard } from '@/features/mv/mv-lyric-card'
import { usePanelHidden } from './panel-visibility'
import {
  FULLSCREEN_PLAYER_ACTION_SLOT,
  FULLSCREEN_PLAYER_PANEL_SLOT,
  PluginSlot,
  PluginView,
  usePluginViews,
  type PluginFullscreenPlayerBinding
} from '@/plugins/host'

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
  const state = usePlayer(
    'actualQuality',
    'playbackMode',
    'error',
    'index',
    'mediaError',
    'queue',
    'revision',
    'status'
  )
  const track = state.index !== null ? state.queue[state.index] : undefined
  const trackVersion = useMemo(() => lyricTrackVersion(track), [track])
  const aliasText = track
    ? [...new Set(track.aliases?.map((alias) => alias.trim()) ?? [])]
        .filter((alias) => alias && alias !== track.title)
        .join(' / ')
    : ''
  const { sources } = useLyricSources()
  const { lyricsOffsetMs } = useLyricsSettings()
  const { options: localPreferences } = useLocalPreferences()
  const [display, setDisplay] = useState<{
    key: string
    revision: number
    lines: LyricLine[]
    offsetMs: number
    format?: Lyrics['format']
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
  const lines = useMemo(
    () =>
      currentDisplay?.lines.length
        ? offsetLyrics(currentDisplay.lines, lyricsOffsetMs + currentDisplay.offsetMs)
        : [],
    [currentDisplay, lyricsOffsetMs]
  )
  const [offsetSaving, setOffsetSaving] = useState(false)
  const offsetBusy = useRef(false)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string>()
  const playbackError = error
  const mv = useLocalMv(track?.key, setError)
  const mvActive = !mobile && mv.mode === 'mv' && !!mv.selection?.url
  const showMv = mvActive && (open || !hidden)
  const [showTranslation, setShowTranslation] = useState(true)
  const [showPronunciation, setShowPronunciation] = useState(true)
  const [showMvLyrics, setShowMvLyrics] = useState(true)
  const [panelPage, setPanelPage] = useState('lyrics')
  const [panelHidden, setPanelHidden] = usePanelHidden()
  const pluginPanelPages = usePluginViews(FULLSCREEN_PLAYER_PANEL_SLOT)
  const activePluginPanel = pluginPanelPages.find(
    (page) => `plugin:${page.pluginId}:${page.viewId}` === panelPage
  )
  const pluginPanelBinding = useMemo<PluginFullscreenPlayerBinding>(
    () => ({
      active: activePluginPanel
        ? { pluginId: activePluginPanel.pluginId, viewId: activePluginPanel.viewId }
        : null,
      showPage: (pluginId, viewId) => {
        setPanelPage(`plugin:${pluginId}:${viewId}`)
        setPanelHidden(false)
      }
    }),
    [activePluginPanel?.pluginId, activePluginPanel?.viewId]
  )
  useEffect(() => {
    if (
      panelPage.startsWith('plugin:') &&
      !pluginPanelPages.some((page) => `plugin:${page.pluginId}:${page.viewId}` === panelPage)
    )
      setPanelPage('lyrics')
  }, [panelPage, pluginPanelPages])
  const hasTranslation = lines.some((line) => !!line.translatedLyric.trim())
  const hasPronunciation = lines.some(
    (line) =>
      !!line.romanLyric.trim() ||
      line.words.some((word) => !!word.romanWord?.trim() || !!word.ruby?.length)
  )
  const generation = useRef(0)
  const appliedSource = useRef<string | undefined>(undefined)
  const correction = useRef<LyricCorrection>(automaticLyrics)
  const offsetRevision = useRef(0)
  const reusable = useRef<
    | {
        version: string
        revision: number
        sources: LyricSources
        preferences: typeof localPreferences
        local: boolean
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
      const parsed = value ? parseLyrics(value, duration, cue) : []
      appliedSource.current = value?.source
      reusable.current = value
        ? {
            version: trackVersion,
            revision,
            sources,
            preferences: localPreferences,
            local: value.source === 'local',
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
        offsetMs: correction.current.offsetMs ?? 0,
        format: value?.format
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
        !previous.local &&
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
        const readingOffsetRevision = offsetRevision.current
        const selected = await nativeCall<LyricCorrection>('lyric_correction', { key: track.key })
        if (!isCurrent()) return
        correction.current = {
          ...selected,
          offsetMs:
            readingOffsetRevision === offsetRevision.current
              ? selected.offsetMs
              : correction.current.offsetMs
        }
        await loadLyrics(
          track,
          refresh || !!selected.localTtml,
          sources,
          (value) => {
            if (!value && (selected.candidate || selected.localTtml || selected.source))
              throw new Error('已选歌词版本暂不可用，请刷新重试或在歌词来源中恢复自动匹配。')
            return apply(
              value,
              track.durationMs,
              track.key,
              state.revision,
              track.source.kind === 'local' ? track.source.cue : undefined
            )
          },
          isCurrent,
          !!(selected.candidate || selected.localTtml || selected.source),
          selected.source
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
        (!correction.current.source || payload.lyrics.source === correction.current.source) &&
        getLyricSources() === sources &&
        !(
          track.source.kind === 'local' &&
          sources.localSource === 'local' &&
          !correction.current.source
        ) &&
        payload.lyrics.source !== 'local' &&
        isCurrentPlayback(track.key, state.revision, trackVersion) &&
        appliedSource.current === payload.lyrics.source &&
        payload.key === track.key &&
        (!payload.track || lyricTrackVersion(payload.track) === trackVersion) &&
        (payload.lyrics.source !== 'amll' || sources.amll) &&
        (payload.lyrics.source !== 'qq' || sources.qq || correction.current.source === 'qq')
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

  useEffect(() => {
    if (!isTauri() || !track) return
    let disposed = false
    let stop: (() => void) | undefined
    void listen<{ key: string; offsetMs: number }>('lyric-offset-changed', ({ payload }) => {
      if (
        disposed ||
        payload.key !== track.key ||
        !isCurrentPlayback(track.key, state.revision, trackVersion)
      )
        return
      offsetRevision.current++
      correction.current = { ...correction.current, offsetMs: payload.offsetMs }
      setDisplay((previous) =>
        previous?.key === track.key && previous.revision === state.revision
          ? { ...previous, offsetMs: payload.offsetMs }
          : previous
      )
    })
      .then((cleanup) => {
        if (disposed) cleanup()
        else stop = cleanup
      })
      .catch((cause) => {
        reportCaughtError('lyrics.offset.listen', cause)
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
      stop?.()
    }
  }, [trackVersion, state.revision])

  async function saveOffset(offsetMs: number) {
    if (
      !track ||
      !currentDisplay ||
      offsetBusy.current ||
      !isCurrentPlayback(track.key, state.revision, trackVersion)
    )
      return
    offsetBusy.current = true
    setOffsetSaving(true)
    try {
      await nativeCall('set_lyric_offset', { key: track.key, offsetMs })
      if (!isCurrentPlayback(track.key, state.revision, trackVersion)) return
      offsetRevision.current++
      correction.current = { ...correction.current, offsetMs }
      setDisplay((previous) =>
        previous?.key === track.key && previous.revision === state.revision
          ? { ...previous, offsetMs }
          : previous
      )
      setError(undefined)
    } catch (cause) {
      reportCaughtError('lyrics.offset.save', cause)
      if (isCurrentPlayback(track.key, state.revision, trackVersion)) setError(errorText(cause))
    } finally {
      offsetBusy.current = false
      setOffsetSaving(false)
    }
  }

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
        active={active && rendering && !mvActive}
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
  const togglePanelPage = (page: string) => {
    setPanelHidden(false)
    setPanelPage((value) => (value === page ? 'lyrics' : page))
  }
  const mvPage = panelPage === 'mv'
  const mvMode = mvActive || mvPage
  const activateMv = async () => {
    if (mvButtonTarget(mv.mode, panelPage) === 'lyrics') {
      setPanelPage('lyrics')
      return
    }
    const result = await mv.toggle()
    if (result === 'select') setPanelPage('mv')
    else if (result) setPanelPage('lyrics')
  }
  const albumBackground = (
    <AlbumBackground
      cover={track?.cover}
      playing={state.status === 'playing'}
      hasLyrics={lines.length > 0}
      active={rendering && !mvActive && (entered || !!reduced)}
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
      data-display-mode={mvActive ? 'mv' : 'lyrics'}
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
          loading={loading}
          onRefresh={() => void requestLyrics(true)}
          onClose={onClose}
          onError={(cause) => setError(errorText(cause))}
          renderLyrics={renderLyrics}
          lyricControls={lyricControls}
        />
      ) : (
        <>
          {showMv && mv.selection && (
            <LocalMvStage
              key={mv.selection.url}
              source={mv.selection.url}
              active={isPresent}
              offsetMs={mv.selection?.offsetMs ?? 0}
              onError={(cause) => setError(errorText(cause))}
            />
          )}
          <div
            className="now-playing-content"
            data-mv-active={mvActive}
            aria-hidden={mvActive}
            inert={mvActive}
          >
            {albumBackground}
            <div
              className={`now-playing-layout relative min-h-0 flex-1 ${showLyrics ? 'has-lyrics' : ''}`}
              data-panel-hidden={showLyrics && panelHidden ? 'true' : undefined}
            >
              <div className="now-playing-details flex min-h-0 min-w-0 flex-col overflow-y-auto">
                <CoverContextMenu cover={track?.cover} title={track?.title ?? '封面'}>
                  <div className="now-playing-cover-slot">
                    <Cover
                      size="original"
                      cover={track?.cover}
                      loading="eager"
                      className="now-playing-cover aspect-square"
                    />
                  </div>
                </CoverContextMenu>
                <div className="now-playing-identity shrink-0">
                  <div className="now-playing-title-row">
                    <div className="now-playing-title-block">
                      <h1 className="now-playing-title">{track?.title ?? '让音乐开始'}</h1>
                      {aliasText && (
                        <p className="now-playing-aliases text-white/50" title={aliasText}>
                          {aliasText}
                        </p>
                      )}
                    </div>
                    <NowPlayingMenu
                      track={track}
                      loading={loading}
                      onRefresh={() => void requestLyrics(true)}
                      mvLinked={mv.linked}
                      mvBusy={mv.busy}
                      onMvMenuOpen={() => void mv.refreshBinding()}
                      onUnlinkMv={() => void mv.unlink()}
                      showCollect={false}
                    />
                  </div>
                  <NowPlayingTags
                    track={track}
                    quality={state.actualQuality}
                    playbackMode={state.playbackMode}
                    lyricFormat={currentDisplay?.format}
                    onSource={() => togglePanelPage('source')}
                    sourceActive={panelPage === 'source'}
                    onLyrics={() => togglePanelPage('lyric-source')}
                    lyricsActive={panelPage === 'lyric-source'}
                    informationActive={panelPage === 'information'}
                    onInformation={() => togglePanelPage('information')}
                  />
                  <div className="now-playing-meta">
                    <div className="now-playing-meta-row text-white/75">
                      <Music2 aria-hidden="true" />
                      <span className="now-playing-meta-text">
                        {track ? <TrackArtists track={track} /> : '选择一首喜欢的音乐'}
                      </span>
                    </div>
                    <div className="now-playing-meta-row text-white/75">
                      <DiscAlbum aria-hidden="true" />
                      <span className="now-playing-meta-text" title={track?.album || '未知专辑'}>
                        {track ? (
                          track.album ? (
                            <TrackAlbum track={track} />
                          ) : (
                            '未知专辑'
                          )
                        ) : (
                          '选择一首喜欢的音乐'
                        )}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
              {showLyrics && (
                <div className="now-playing-panel-region" data-panel-page={panelPage}>
                  <div
                    className="now-playing-lyrics min-h-0 min-w-0 overflow-hidden"
                    data-panel-page={panelPage}
                    data-plugin-panel={activePluginPanel ? 'true' : undefined}
                    data-hidden={panelHidden ? 'true' : undefined}
                    aria-hidden={panelHidden}
                    inert={panelHidden}
                  >
                    <Panel
                      className="h-full"
                      value={panelHidden ? 'lyrics' : panelPage}
                      pages={[
                        {
                          value: 'lyrics',
                          content: renderLyrics(panelPage === 'lyrics' && !panelHidden),
                          keepMounted: true
                        },
                        {
                          value: 'information',
                          content: <NowPlayingInformation track={track} />
                        },
                        {
                          value: 'source',
                          content: (
                            <NowPlayingSource
                              track={track}
                              onClose={() => setPanelPage('lyrics')}
                            />
                          )
                        },
                        {
                          value: 'lyric-source',
                          content: (
                            <NowPlayingLyricSource
                              track={track}
                              onClose={() => setPanelPage('lyrics')}
                            />
                          )
                        },
                        {
                          value: 'queue',
                          content: (
                            <QueuePanel
                              onPage={onQueue}
                              onError={(cause) => setError(errorText(cause))}
                            />
                          )
                        },
                        {
                          value: 'mv',
                          content: (
                            <MvPanel
                              trackKey={track.key}
                              defaultQuery={[track.title, track.artist].filter(Boolean).join(' ')}
                              selection={mv.selection}
                              linked={mv.linked}
                              busy={mv.busy}
                              onError={setError}
                              onChoose={() => void mv.choose()}
                              onApply={() => {
                                void mv.apply().then((applied) => {
                                  if (applied) setPanelPage('lyrics')
                                })
                              }}
                              onBind={(source, id, title) => {
                                return mv.bindRemote(source, id, title).then((applied) => {
                                  if (applied) setPanelPage('lyrics')
                                  return applied
                                })
                              }}
                              onCancel={() => setPanelPage('lyrics')}
                            />
                          )
                        },
                        ...pluginPanelPages.map((page) => ({
                          value: `plugin:${page.pluginId}:${page.viewId}`,
                          content: (
                            <div className="fullscreen-plugin-panel">
                              <PluginView
                                pluginId={page.pluginId}
                                viewId={page.viewId}
                                fullscreenPlayer={pluginPanelBinding}
                              />
                            </div>
                          )
                        }))
                      ]}
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
          <div className="fullscreen-display-controls" data-mode={mvActive ? 'mv' : 'lyrics'}>
            {!mvActive && panelPage === 'lyrics' && lyricControls}
            {mvActive && (
              <MvAlignmentPopover
                offsetMs={mv.selection?.offsetMs ?? 0}
                onChange={(value) => void mv.setOffset(value)}
              />
            )}
            {mvActive && (
              <ActionButton
                variant="ghost"
                size="icon"
                className="fullscreen-mode-button"
                aria-label={showMvLyrics ? '隐藏 MV 歌词卡片' : '显示 MV 歌词卡片'}
                aria-pressed={showMvLyrics}
                title={showMvLyrics ? '隐藏 MV 歌词卡片' : '显示 MV 歌词卡片'}
                onClick={() => setShowMvLyrics((value) => !value)}
              >
                <MessageSquareText aria-hidden="true" />
              </ActionButton>
            )}
            <ActionButton
              variant="ghost"
              size="icon"
              className="fullscreen-mode-button"
              aria-label={mv.busy ? '正在读取本地 MV' : mvMode ? '返回歌词' : '打开本地 MV'}
              aria-pressed={mvMode}
              title={
                mv.busy
                  ? '正在读取本地 MV…'
                  : mvMode
                    ? '返回歌词'
                    : '打开本地 MV；未关联时进入选择页'
              }
              disabled={!track || mv.busy}
              onClick={() => void activateMv()}
            >
              {mv.busy ? (
                <Spinner />
              ) : mvMode ? (
                <Captions aria-hidden="true" />
              ) : (
                <Clapperboard aria-hidden="true" />
              )}
            </ActionButton>
          </div>
          {!mvActive && !panelHidden && panelPage === 'lyrics' && track && (
            <LyricOffsetControl
              offsetMs={currentDisplay?.offsetMs ?? 0}
              disabled={!currentDisplay || offsetSaving}
              onChange={(value) => void saveOffset(value)}
            />
          )}
          {mvActive && showMvLyrics && (
            <MvLyricCard
              lines={lines}
              showTranslation={showTranslation && hasTranslation}
              showPronunciation={showPronunciation && hasPronunciation}
            />
          )}
          <FullscreenPlaybackBar
            onQueuePanel={() => {
              setPanelHidden(false)
              setPanelPage((value) => (value === 'queue' ? 'lyrics' : 'queue'))
            }}
            queueActive={panelPage === 'queue'}
            onError={(cause) => setError(errorText(cause))}
            pluginActions={
              <div className="fullscreen-playback-plugin-actions">
                <PluginSlot
                  name={FULLSCREEN_PLAYER_ACTION_SLOT}
                  fullscreenPlayer={pluginPanelBinding}
                />
              </div>
            }
          />
        </>
      )}
      <div className="fullscreen-playback-notice">
        <ErrorNotice error={playbackError} />
        <PlaybackNotice notice={notice} />
      </div>
    </motion.section>
  )
}
