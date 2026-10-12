import { useEffect, useState, type MouseEvent } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  Copy,
  Minus,
  Square,
  Wrench,
  X
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { SettingsDialog } from '@/features/settings/settings-dialog'
import { AccountMenu } from '@/features/account/account-menu'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { MusicSearch } from '@/features/search/music-search'
import { PlaylistSidebar } from '@/features/library/playlist-sidebar'
import { usePlugins } from '@/plugins/host'
import { pluginPath } from '@/plugins/types'

export function Titlebar({
  playerMode = false,
  onBack
}: {
  playerMode?: boolean
  onBack?: () => void
}) {
  const { plugins, loaded } = usePlugins()
  const hasTools = plugins.some(
    (plugin) =>
      loaded.has(plugin.manifest.id) &&
      plugin.manifest.contributes.navigation.some((item) => item.category === 'tool')
  )
  const native = isTauri()
  const { page, navigate, back, forward, canBack, canForward } = useMusicNavigation()
  const toolsSelected =
    page.view === 'tools' ||
    (page.view === 'plugin' &&
      plugins.some(
        (plugin) =>
          plugin.manifest.contributes.navigation.some((item) => item.category === 'tool') &&
          page.query.startsWith(`/plugins/${plugin.manifest.id}/`)
      ))
  const [maximized, setMaximized] = useState(false)
  const [focused, setFocused] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!native) return
    const appWindow = getCurrentWindow()
    let disposed = false
    const listeners: UnlistenFn[] = []
    const retain = (unlisten: UnlistenFn) => {
      if (disposed) unlisten()
      else listeners.push(unlisten)
    }
    const updateMaximized = async () => {
      const value = await appWindow.isMaximized()
      if (!disposed) setMaximized(value)
    }
    void updateMaximized().catch(console.error)
    void appWindow
      .isFocused()
      .then((value) => {
        if (!disposed) setFocused(value)
      })
      .catch(console.error)
    void appWindow
      .onResized(() => {
        void updateMaximized().catch(console.error)
      })
      .then(retain)
      .catch(console.error)
    void appWindow
      .onFocusChanged(({ payload }) => {
        if (!disposed) setFocused(payload)
      })
      .then(retain)
      .catch(console.error)
    return () => {
      disposed = true
      listeners.forEach((unlisten) => unlisten())
    }
  }, [native])

  async function windowAction(action: 'minimize' | 'toggleMaximize' | 'close' | 'startDragging') {
    if (!native) return
    try {
      setError(null)
      const appWindow = getCurrentWindow()
      await appWindow[action]()
      if (action === 'toggleMaximize') setMaximized(await appWindow.isMaximized())
    } catch (cause) {
      console.error('Window action failed', cause)
      setError('窗口操作失败，请重试。')
    }
  }

  function drag(event: MouseEvent<HTMLDivElement>) {
    if (event.button !== 0) return
    const target = event.target as Element | null
    // Portaled popups (e.g. the playlist sidebar) bubble through React but live outside this DOM node.
    if (target && event.currentTarget && !event.currentTarget.contains(target)) return
    if (target?.closest?.('button, a, input, select, textarea, [role="button"]')) return
    void windowAction(event.detail === 2 ? 'toggleMaximize' : 'startDragging')
  }

  return (
    <>
      <header
        className={`titlebar relative z-10 shrink-0 select-none items-center ${playerMode ? 'flex h-[68px] bg-transparent' : 'glass-surface grid h-12 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]'}`}
        data-focused={focused}
      >
        <div className={`flex h-full min-w-0 ${playerMode ? 'flex-1' : 'items-center'}`}>
          <div
            className={`flex h-full min-w-0 items-center gap-2.5 ${playerMode ? 'flex-1' : 'pl-4 pr-1'}`}
            onMouseDown={drag}
          >
            {playerMode ? (
              <Button
                autoFocus
                variant="ghost"
                size="icon-lg"
                className="now-playing-back"
                aria-label="返回音乐"
                title="返回音乐"
                onMouseDown={(event) => event.stopPropagation()}
                onClick={onBack}
              >
                <ChevronDown aria-hidden="true" />
              </Button>
            ) : null}
            <PlaylistSidebar hidden={playerMode} />
            {!playerMode && hasTools && (
              <Button
                variant={toolsSelected ? 'secondary' : 'ghost'}
                size="icon-sm"
                aria-label="工具"
                title="工具"
                aria-current={toolsSelected ? 'page' : undefined}
                onClick={() => navigate('tools')}
              >
                <Wrench aria-hidden="true" />
              </Button>
            )}
          </div>

          {!playerMode && (
            <>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="后退"
                  disabled={!canBack}
                  onClick={back}
                >
                  <ChevronLeft aria-hidden="true" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="前进"
                  disabled={!canForward}
                  onClick={forward}
                >
                  <ChevronRight aria-hidden="true" />
                </Button>
              </div>
              {!page.view.startsWith('local') && <MusicSearch />}
              <div className="h-full min-w-2 flex-1" onMouseDown={drag} />
            </>
          )}
        </div>
        {!playerMode && (
          <nav aria-label="音乐导航" className="flex shrink-0 items-center gap-1">
            {(
              [
                ['library', '音乐库'],
                ['discover', '发现'],
                ['local', '本地']
              ] as const
            ).map(([view, label]) => (
              <Button
                key={view}
                variant={
                  page.view === view ||
                  (view === 'library' && ['collection', 'accounts'].includes(page.view)) ||
                  (view === 'discover' && page.view === 'search') ||
                  (view === 'local' && page.view.startsWith('local-'))
                    ? 'secondary'
                    : 'ghost'
                }
                aria-current={
                  page.view === view ||
                  (view === 'library' && ['collection', 'accounts'].includes(page.view)) ||
                  (view === 'discover' && page.view === 'search') ||
                  (view === 'local' && page.view.startsWith('local-'))
                    ? 'page'
                    : undefined
                }
                onClick={() => navigate(view)}
              >
                {label}
              </Button>
            ))}
            {plugins
              .filter((p) => p.loaded)
              .flatMap((plugin) =>
                plugin.manifest.contributes.navigation
                  .filter((item) => item.category !== 'tool')
                  .map((item) => {
                    const target = plugin.manifest.contributes.pages.find(
                      (p) => p.id === item.page
                    )!
                    const path = pluginPath(plugin.manifest.id, target.path)
                    const selected =
                      page.view === 'plugin' &&
                      (page.query === path || page.query.startsWith(`${path.replace(/\/$/, '')}/`))
                    return (
                      <Button
                        key={`${plugin.manifest.id}:${item.id}`}
                        variant={selected ? 'secondary' : 'ghost'}
                        aria-current={selected ? 'page' : undefined}
                        onClick={() => navigate('plugin', path)}
                      >
                        {item.label}
                      </Button>
                    )
                  })
              )}
          </nav>
        )}
        <div className="flex h-full min-w-0 items-center justify-end pl-3">
          {!playerMode && <div className="h-full min-w-0 flex-1" onMouseDown={drag} />}
          <div className="flex h-full shrink-0 items-center gap-1 pr-1.5">
            <div className={playerMode ? 'hidden' : 'flex items-center gap-1'}>
              <AccountMenu />
              <SettingsDialog />
            </div>

            {!playerMode && <span className="mx-1.5 h-4 w-px bg-border" aria-hidden="true" />}
            <Button
              variant="ghost"
              size="icon-sm"
              className="titlebar-control"
              aria-label="最小化"
              title="最小化"
              disabled={!native}
              onClick={() => void windowAction('minimize')}
            >
              <Minus aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              className="titlebar-control"
              aria-label={maximized ? '还原窗口' : '最大化'}
              title={maximized ? '还原窗口' : '最大化'}
              disabled={!native}
              onClick={() => void windowAction('toggleMaximize')}
            >
              {maximized ? <Copy aria-hidden="true" /> : <Square aria-hidden="true" />}
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              className="titlebar-control titlebar-close"
              aria-label="关闭窗口"
              title="关闭窗口"
              disabled={!native}
              onClick={() => void windowAction('close')}
            >
              <X aria-hidden="true" />
            </Button>
          </div>
        </div>
      </header>
      {error && (
        <p
          role="alert"
          className="border-b border-border bg-destructive/10 px-4 py-2 text-xs text-destructive"
        >
          {error}
        </p>
      )}
    </>
  )
}
