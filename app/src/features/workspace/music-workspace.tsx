import { MusicPageSkeleton, LibraryPageSkeleton } from '@/components/music/loading'
import { usePlaybackShortcuts } from '@/features/playback/shortcuts/use-playback-shortcuts'
import { CollectionActionsProvider } from '@/features/library/collection-actions'
import { lazy, Suspense, useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { X } from 'lucide-react'
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
import LyricsView from '@/features/lyrics/lyrics-view'
import { useMusicSources } from '@/features/music/use-music-sources'
import { sourceLabel } from '@/features/music/source-label'

const LocalPage = lazy(() => import('@/features/local/local-page'))
const SearchPage = lazy(() => import('@/features/search/search-page'))
const ArtistPage = lazy(() => import('@/features/library/artist-page'))
const AlbumPage = lazy(() => import('@/features/library/album-page'))
const Discovery = lazy(() => import('@/features/discovery/discovery'))
const AccountLoginPage = lazy(() => import('@/features/account/account-login-page'))
const MusicLibrary = lazy(() => import('@/features/library/music-library'))
const EntityPage = lazy(() => import('@/features/music/entity-page'))
const SourceWorkspace = lazy(() => import('@/features/music/source-workspace'))
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
  const { page, navigate } = useMusicNavigation()
  const view = page.view
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
  const [error, setError] = useState<string>()
  const [notice, setNotice] = useState<PlaybackNoticeMessage>()
  const noticeSerial = useRef(0)
  const showNotice = useCallback(
    (message: string) => setNotice({ id: ++noticeSerial.current, message }),
    []
  )
  useEffect(() => {
    if (!notice) return
    const timer = window.setTimeout(() => setNotice(undefined), 4000)
    return () => window.clearTimeout(timer)
  }, [notice])
  const [refresh, setRefresh] = useState(0)
  const onError = useCallback((cause: unknown) => setError(errorText(cause)), [])
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
    setError(undefined)
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
      if (report.errors.length) setError(report.errors.join('；'))
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
      <SongActionsProvider onError={onError} onNotice={showNotice}>
        <div
          className="music-workspace flex min-h-0 flex-1 flex-col"
          data-playback-bar={barMode}
          data-density={density}
        >
          <AnimatePresence onExitComplete={onPlayerExitComplete}>
            {nowPlaying && <LyricsView key="now-playing" onQueue={openQueue} />}
          </AnimatePresence>
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
              {(error || state.error || state.mediaError) && (
                <div
                  role="alert"
                  className="flex items-start gap-3 border-b border-border bg-destructive/5 px-8 py-3 text-sm text-destructive"
                >
                  <p className="min-w-0 flex-1">{error ?? state.error ?? state.mediaError}</p>
                  {error && (
                    <ActionButton
                      variant="ghost"
                      size="icon-sm"
                      aria-label="关闭提示"
                      onClick={() => setError(undefined)}
                    >
                      <X aria-hidden="true" />
                    </ActionButton>
                  )}
                </div>
              )}
              {page.collection?.reference && ['artist', 'album', 'collection'].includes(view) ? (
                <Suspense
                  fallback={<MusicPageSkeleton view="collection" collection={page.collection} />}
                >
                  <EntityPage
                    key={JSON.stringify(page.collection.reference)}
                    collection={page.collection}
                    onError={onError}
                  />
                </Suspense>
              ) : (view === 'artist' || view === 'album') && page.collection ? (
                <Suspense
                  fallback={<MusicPageSkeleton view="collection" collection={page.collection} />}
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
              ) : view === 'search' ? (
                <Suspense fallback={<MusicPageSkeleton view="search" title={page.query} />}>
                  <SearchPage key={page.query} onError={onError} onNotice={showNotice} />
                </Suspense>
              ) : (view === 'discover' || view === 'library') &&
                isTauri() &&
                (!currentSource || !currentStatus?.enabled) ? (
                <MusicPage>
                  <p role="status" className="py-6 text-sm text-muted-foreground">
                    {sources.error ||
                      (currentSource
                        ? `${sourceLabel(currentSource)}当前不可用，请选择其他来源或在设置中启用。`
                        : '正在读取音乐来源…')}
                  </p>
                  <ActionButton variant="outline" onClick={() => navigate('accounts')}>
                    选择音乐来源
                  </ActionButton>
                </MusicPage>
              ) : (view === 'discover' || view === 'library') &&
                currentSource &&
                currentSource !== 'netease' ? (
                <Suspense
                  fallback={
                    view === 'library' ? (
                      <LibraryPageSkeleton />
                    ) : (
                      <MusicPageSkeleton view="discover" />
                    )
                  }
                >
                  <SourceWorkspace
                    key={`${currentSource}:${view}:${sources.revision}`}
                    source={currentSource}
                    name={currentStatus?.descriptor.displayName ?? currentSource}
                    discovery={view === 'discover'}
                    onError={onError}
                    onNotice={showNotice}
                  />
                </Suspense>
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
                    key={view === 'library' ? `${currentSource}:${sources.revision}` : undefined}
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
          {!playerVisible &&
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
      </SongActionsProvider>
    </CollectionActionsProvider>
  )
}
