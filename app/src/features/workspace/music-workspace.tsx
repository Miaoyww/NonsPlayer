import { supportsMusicPage } from '@/features/music/source-capabilities'
import { ListFindProvider } from '@/components/music/list-find'
import { MusicPageSkeleton, LibraryPageSkeleton } from '@/components/music/loading'
import { usePlaybackShortcuts } from '@/features/playback/shortcuts/use-playback-shortcuts'
import { CollectionActionsProvider } from '@/features/library/collection-actions'
import { lazy, Suspense, useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { ErrorNotice } from '@/components/error-notice'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { AnimatePresence } from 'motion/react'
import { connectPlayer, errorText, nativeCall, usePlayer } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'
import { MusicPage } from '@/components/music/music-page'
import { PlaybackBar } from '@/features/playback/playback-bar'
import { PersistentPlaybackBar } from '@/features/playback/persistent-playback-bar'
import { usePlaybackBarMode } from '@/features/playback/use-playback-bar-mode'
import type { PlaybackNoticeMessage } from '@/features/playback/playback-notice'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { QualitySelect } from '@/features/playback/music-options'
import { SongActionsProvider } from '@/components/music/song-actions'
import { QueuePage } from '@/features/queue/queue-page'
import { ToolsPage } from '@/features/tools/tools-page'
import { PluginPageHost } from '@/plugins/host'
import { useInterfaceDensity } from '@/features/settings/use-interface-density'
import { useMusicSources } from '@/features/music/use-music-sources'
import { sourceLabel } from '@/features/music/source-label'
import { useAndroidBack } from './use-android-back'
import { isAndroid } from '@/lib/platform'
import { usePrimaryMusicPage } from './use-primary-music-page'
import { MobilePageHeader } from './mobile-page-header'
import { settingsSections } from '@/features/settings/settings-sections'
import { TrackSearchContext } from '@/components/music/track-search-context'

const LyricsView = lazy(() => import('@/features/lyrics/lyrics-view'))
const LocalPage = lazy(() => import('@/features/local/local-page'))
const SearchPage = lazy(() => import('@/features/search/search-page'))
const SettingsPage = lazy(() => import('@/features/settings/settings-page'))
const ArtistPage = lazy(() => import('@/features/library/artist-page'))
const AlbumPage = lazy(() => import('@/features/library/album-page'))
const Discovery = lazy(() => import('@/features/discovery/discovery'))
const AccountLoginPage = lazy(() => import('@/features/account/account-login-page'))
const MusicLibrary = lazy(() => import('@/features/library/music-library'))
const EntityPage = lazy(() => import('@/features/music/entity-page'))
interface ImportReport {
  imported: number
  skipped: number
  errors: string[]
}

export function MusicWorkspace({
  nowPlaying,
  playerVisible,
  onNowPlayingChange,
  onPlayerExitComplete
}: {
  nowPlaying: boolean
  playerVisible: boolean
  onNowPlayingChange: (value: boolean) => void
  onPlayerExitComplete: () => void
}) {
  const state = usePlayer('error', 'mediaError')
  const [barMode] = usePlaybackBarMode()
  const [density] = useInterfaceDensity()
  const { page, navigate, back } = useMusicNavigation()
  const primaryPage = usePrimaryMusicPage()
  const settingsSection = settingsSections.find((item) => item.id === page.query.split('/')[0])
  useAndroidBack({ nowPlaying, closePlayer: () => onNowPlayingChange(false), back })
  const view = page.view
  const canSearchList = [
    'collection',
    'album',
    'artist',
    'local-playlist',
    'local-album',
    'local-artist'
  ].includes(view)
  const [listQuery, setListQuery] = useState('')
  const [listSearchOpen, setListSearchOpen] = useState(false)
  const closeListSearch = () => {
    setListSearchOpen(false)
    setListQuery('')
  }
  useEffect(() => {
    closeListSearch()
  }, [page])
  useEffect(() => {
    if (!isAndroid() || !listSearchOpen) return
    const close = (event: Event) => {
      event.preventDefault()
      closeListSearch()
    }
    window.addEventListener('nons-android-back', close)
    return () => window.removeEventListener('nons-android-back', close)
  }, [listSearchOpen])
  const sources = useMusicSources()
  const currentSource = sources.preferences?.current
  const currentStatus = sources.statuses.find((item) => item.descriptor.source === currentSource)
  const previousPage = useRef(page)
  useEffect(() => {
    if (previousPage.current !== page) {
      previousPage.current = page
      onNowPlayingChange(false)
    }
  }, [page, onNowPlayingChange])
  const [queueVisit, setQueueVisit] = useState(0)
  const openQueue = () => {
    setQueueVisit((value) => value + 1)
    navigate('queue')
    onNowPlayingChange(false)
  }
  const [importing, setImporting] = useState(false)
  const [notice, setNotice] = useState<PlaybackNoticeMessage>()
  const noticeSerial = useRef(0)
  const showNotice = useCallback(
    (message: string) => setNotice({ id: ++noticeSerial.current, message }),
    []
  )
  useEffect(() => {
    if (!notice) return
    const timer = window.setTimeout(() => setNotice(undefined), 3000)
    return () => window.clearTimeout(timer)
  }, [notice])
  const [refresh, setRefresh] = useState(0)
  const [operationError, setOperationError] = useState<{ id: number; message: string }>()
  const onError = useCallback((cause: unknown) => {
    setOperationError((previous) => ({ id: (previous?.id ?? 0) + 1, message: errorText(cause) }))
  }, [])
  usePlaybackShortcuts(onError)
  const wasPlayerVisible = useRef(playerVisible)

  useEffect(() => {
    if (wasPlayerVisible.current && !playerVisible)
      document.querySelector<HTMLButtonElement>('[aria-label="打开正在播放"]')?.focus()
    wasPlayerVisible.current = playerVisible
  }, [playerVisible])

  useEffect(() => {
    if (!nowPlaying) return
    const exit = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !event.defaultPrevented) onNowPlayingChange(false)
    }
    document.addEventListener('keydown', exit)
    return () => {
      document.removeEventListener('keydown', exit)
    }
  }, [nowPlaying, onNowPlayingChange])

  useEffect(() => {
    let disposed = false
    let stop: (() => void) | undefined
    void connectPlayer()
      .then((cleanup) => {
        if (disposed) cleanup()
        else stop = cleanup
      })
      .catch(onError)
    return () => {
      disposed = true
      stop?.()
    }
  }, [onError])

  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    let stop: (() => void) | undefined
    void listen<{ scanning?: boolean; error?: string }>('local-library-updated', ({ payload }) => {
      if (disposed) return
      if (payload?.error) onError(payload.error)
      if (!payload?.scanning) {
        invalidateNativeCache([
          'local_music',
          'local_entities',
          'local_entity_detail',
          'local_entity_tracks',
          'local_track_information',
          'track_lyrics'
        ])
        setRefresh((value) => value + 1)
      }
    })
      .then((fn) => {
        if (disposed) fn()
        else stop = fn
      })
      .catch(onError)
    return () => {
      disposed = true
      stop?.()
    }
  }, [onError])

  async function importMusic(directory: boolean) {
    setNotice(undefined)
    try {
      const selected = await open({
        directory,
        multiple: !directory,
        title: directory ? '导入音乐目录' : '打开音乐文件',
        filters: directory
          ? undefined
          : [
              {
                name: '音频',
                extensions: [
                  'mp3',
                  'flac',
                  'wav',
                  'm4a',
                  'aac',
                  'ogg',
                  'opus',
                  'aiff',
                  'ape',
                  'wv',
                  'ncm'
                ]
              }
            ]
      })
      if (!selected) return
      setImporting(true)
      const report = await nativeCall<ImportReport>('import_music', {
        paths: Array.isArray(selected) ? selected : [selected]
      })
      showNotice(
        `已导入 ${report.imported} 首音乐${report.skipped ? `，跳过 ${report.skipped} 个无法读取的文件` : ''}。`
      )
      if (report.errors.length) onError(report.errors.join('；'))
      if (view === 'local') setRefresh((value) => value + 1)
      else navigate('local')
    } catch (cause) {
      onError(cause)
    } finally {
      setImporting(false)
    }
  }

  return (
    <CollectionActionsProvider onError={onError} onNotice={showNotice}>
      <ErrorNotice key={operationError?.id} error={operationError?.message} />
      <ErrorNotice error={state.error} />
      <ErrorNotice error={state.mediaError} />
      <ErrorNotice error={sources.error} />
      <SongActionsProvider onError={onError} onNotice={showNotice}>
        <ListFindProvider
          playerVisible={playerVisible}
          onGlobalSearch={() => {
            onNowPlayingChange(false)
            requestAnimationFrame(() =>
              document.querySelector<HTMLInputElement>('[data-global-music-search]')?.focus()
            )
          }}
        >
          <div
            className="music-workspace relative flex min-h-0 flex-1 flex-col"
            data-playback-bar={barMode}
            data-density={density}
          >
            {isAndroid() && !primaryPage && !playerVisible && (
              <MobilePageHeader
                title={
                  page.view === 'settings'
                    ? (settingsSection?.label ?? '设置')
                    : page.view === 'search'
                      ? '搜索'
                      : (page.collection?.name ??
                        (
                          {
                            accounts: '音乐账号',
                            queue: '播放队列',
                            discover: '发现',
                            local: '本地音乐',
                            'local-artist': '本地艺人',
                            'local-album': '本地专辑',
                            'local-playlist': '本地歌单'
                          } as Record<string, string>
                        )[page.view] ??
                        '音乐')
                }
                description={
                  page.view === 'settings'
                    ? (settingsSection?.description ?? '个性化与全局设置')
                    : undefined
                }
                onBack={back}
                search={
                  canSearchList
                    ? {
                        open: listSearchOpen,
                        query: listQuery,
                        onOpen: () => setListSearchOpen(true),
                        onClose: closeListSearch,
                        onChange: setListQuery
                      }
                    : undefined
                }
              />
            )}
            <AnimatePresence onExitComplete={onPlayerExitComplete}>
              {nowPlaying && (
                <Suspense
                  key="now-playing"
                  fallback={
                    <div
                      className="now-playing absolute inset-0 z-[5] flex items-center justify-center"
                      role="status"
                      aria-label="正在加载播放器"
                    >
                      <span>正在加载播放器…</span>
                    </div>
                  }
                >
                  <LyricsView
                    onQueue={openQueue}
                    notice={notice}
                    onClose={() => onNowPlayingChange(false)}
                  />
                </Suspense>
              )}
            </AnimatePresence>
            <TrackSearchContext.Provider
              value={isAndroid() && canSearchList ? listQuery : undefined}
            >
              <div className={nowPlaying ? 'hidden' : 'flex min-h-0 flex-1'}>
                <main
                  id="music-content"
                  className="flex min-h-0 min-w-0 flex-1 flex-col"
                  aria-label="音乐工作区"
                >
                  {!isTauri() && (
                    <p
                      role="status"
                      className="border-b border-border bg-muted/50 px-8 py-3 text-sm text-muted-foreground"
                    >
                      这是界面预览。播放、搜索和导入功能需要在桌面应用中使用。
                    </p>
                  )}
                  {page.collection?.reference &&
                  ['artist', 'album', 'collection'].includes(view) ? (
                    <Suspense
                      fallback={
                        <MusicPageSkeleton view="collection" collection={page.collection} />
                      }
                    >
                      <EntityPage
                        key={JSON.stringify(page.collection.reference)}
                        collection={page.collection}
                        onError={onError}
                        onNotice={showNotice}
                      />
                    </Suspense>
                  ) : (view === 'artist' || view === 'album') && page.collection ? (
                    <Suspense
                      fallback={
                        <MusicPageSkeleton view="collection" collection={page.collection} />
                      }
                    >
                      {view === 'artist' ? (
                        <ArtistPage
                          key={page.collection.id}
                          collection={page.collection}
                          onError={onError}
                          onNotice={showNotice}
                        />
                      ) : (
                        <AlbumPage
                          key={page.collection.id}
                          collection={page.collection}
                          onError={onError}
                          onNotice={showNotice}
                        />
                      )}
                    </Suspense>
                  ) : view === 'accounts' ? (
                    <Suspense fallback={<MusicPageSkeleton view="accounts" />}>
                      <AccountLoginPage />
                    </Suspense>
                  ) : view === 'tools' ? (
                    <ToolsPage />
                  ) : view === 'plugin' ? (
                    <MusicPage>
                      <PluginPageHost path={page.query} />
                    </MusicPage>
                  ) : view === 'queue' ? (
                    <QueuePage key={queueVisit} onError={onError} />
                  ) : view === 'settings' ? (
                    <Suspense
                      fallback={
                        <MusicPage>
                          <p role="status">正在加载设置…</p>
                        </MusicPage>
                      }
                    >
                      <SettingsPage />
                    </Suspense>
                  ) : view === 'search' ? (
                    <Suspense fallback={<MusicPageSkeleton view="search" title={page.query} />}>
                      <SearchPage
                        key={isAndroid() ? 'mobile-search' : page.query}
                        onError={onError}
                        onNotice={showNotice}
                      />
                    </Suspense>
                  ) : (view === 'discover' || view === 'library') &&
                    isTauri() &&
                    (!currentSource || !currentStatus?.enabled) ? (
                    sources.loading && !sources.error ? (
                      view === 'library' ? (
                        <LibraryPageSkeleton />
                      ) : (
                        <MusicPageSkeleton view="discover" />
                      )
                    ) : (
                      <MusicPage>
                        <p role="status" className="py-6 text-sm text-muted-foreground">
                          {currentSource
                            ? `${sourceLabel(
                                currentSource,
                                sources.statuses.map((value) => value.descriptor)
                              )}当前不可用，请选择其他来源或在设置中启用。`
                            : '正在读取音乐来源…'}
                        </p>
                        <ActionButton variant="outline" onClick={() => navigate('accounts')}>
                          选择音乐来源
                        </ActionButton>
                      </MusicPage>
                    )
                  ) : (view === 'discover' || view === 'library') &&
                    !supportsMusicPage(currentStatus?.descriptor, view) ? (
                    <MusicPage>
                      <p role="status" className="py-6 text-sm text-muted-foreground">
                        此适配器未提供{view === 'library' ? '音乐库' : '发现'}页面，请选择其他账号。
                      </p>
                    </MusicPage>
                  ) : view === 'discover' ? (
                    <Suspense
                      fallback={
                        <MusicPageSkeleton
                          view="discover"
                          tracks={page.query === 'daily'}
                          title="每日推荐"
                        />
                      }
                    >
                      <Discovery
                        key={`${currentSource}:${sources.revision}`}
                        onError={onError}
                        onNotice={showNotice}
                      />
                    </Suspense>
                  ) : view === 'library' || view === 'collection' ? (
                    <Suspense
                      fallback={
                        view === 'library' ? (
                          <LibraryPageSkeleton />
                        ) : (
                          <MusicPageSkeleton view="collection" collection={page.collection} />
                        )
                      }
                    >
                      <MusicLibrary
                        readOnly={!currentStatus?.descriptor.capabilities.includes('playlistWrite')}
                        key={
                          view === 'library' ? `${currentSource}:${sources.revision}` : undefined
                        }
                        onError={onError}
                        onNotice={showNotice}
                      />
                    </Suspense>
                  ) : (
                    <Suspense fallback={<MusicPageSkeleton view="local" />}>
                      <LocalPage
                        refresh={refresh}
                        importing={importing}
                        onImport={() => void importMusic(false)}
                        onError={onError}
                        onNotice={showNotice}
                      />
                    </Suspense>
                  )}
                </main>
              </div>
            </TrackSearchContext.Provider>
            {!playerVisible &&
              !(isAndroid() && view === 'settings') &&
              (barMode === 'persistent' ? (
                <PersistentPlaybackBar
                  notice={notice}
                  onLyrics={() => onNowPlayingChange(true)}
                  onQueue={openQueue}
                  onError={onError}
                />
              ) : barMode === 'collapsible' ? (
                <PlaybackBar
                  notice={notice}
                  qualityControl={<QualitySelect />}
                  onLyrics={() => onNowPlayingChange(true)}
                  onQueue={openQueue}
                  onError={onError}
                />
              ) : null)}
          </div>
        </ListFindProvider>
      </SongActionsProvider>
    </CollectionActionsProvider>
  )
}
