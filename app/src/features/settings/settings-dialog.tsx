import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  Blocks,
  FolderCog,
  Info,
  Keyboard,
  LayoutGrid,
  Mic2,
  SlidersHorizontal,
  Settings,
  Volume2,
  X,
  Menu
} from 'lucide-react'
import { motion, useReducedMotion } from 'motion/react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogPopup,
  DialogTitle,
  DialogTrigger
} from '@/components/animate-ui/components/base/dialog'
import { useTheme } from '@/features/settings/use-theme'
import { SettingsContent } from './settings-content'
import { version } from '../../../package.json'
import { nativeCall } from '@/lib/player'
import {
  Sheet,
  SheetTrigger,
  SheetContent,
  SheetTitle,
  SheetDescription,
  SheetClose
} from '@/components/ui/sheet'
import { SettingsHome } from './settings-home'

export function SettingsDialog({
  onOpenPage,
  onOpenAccounts
}: { onOpenPage?: (section?: string) => void; onOpenAccounts?: () => void } = {}) {
  const openPage = useRef(onOpenPage)
  openPage.current = onOpenPage
  const [open, setOpen] = useState(false)
  const [pluginId, setPluginId] = useState<string>()
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    let stop: (() => void) | undefined
    const openSettings = () => {
      setPluginId(undefined)
      setSection('general')
      setOpen(true)
    }
    void listen('open-settings', openSettings)
      .then(async (unlisten) => {
        if (disposed) unlisten()
        else {
          stop = unlisten
          const pending = await nativeCall<{ settings: boolean; shortcut: string | null }>(
            'main_webview_ready'
          )
          if (disposed) return
          if (pending.settings) openSettings()
          if (pending.shortcut)
            window.dispatchEvent(
              new CustomEvent('resume-playback-shortcut', { detail: pending.shortcut })
            )
        }
      })
      .catch(console.error)
    return () => {
      disposed = true
      stop?.()
    }
  }, [])
  useEffect(() => {
    const handler = (event: Event) => {
      const id = (event as CustomEvent<string>).detail
      if (typeof id !== 'string') return
      setPluginId(id)
      setSection('plugins')
      if (openPage.current) openPage.current(`plugins/${id}`)
      else setOpen(true)
    }
    window.addEventListener('plugin-open-configuration', handler)
    return () => window.removeEventListener('plugin-open-configuration', handler)
  }, [])
  const [section, setSection] = useState<
    | 'general'
    | 'personalization'
    | 'playback'
    | 'lyrics'
    | 'local-cache'
    | 'adapters'
    | 'plugins'
    | 'shortcuts'
    | 'about'
  >('general')
  const [theme, setTheme] = useTheme()
  const reducedMotion = useReducedMotion()
  useEffect(() => {
    if (!open || !onOpenPage) return
    const close = (event: Event) => {
      event.preventDefault()
      setOpen(false)
    }
    window.addEventListener('nons-android-back', close)
    return () => window.removeEventListener('nons-android-back', close)
  }, [open, onOpenPage])

  if (onOpenPage)
    return (
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetTrigger
          render={
            <Button
              variant="ghost"
              size="icon-lg"
              aria-label="打开菜单"
              className="titlebar-settings"
            />
          }
        >
          <Menu aria-hidden="true" />
        </SheetTrigger>
        <SheetContent>
          <div className="flex shrink-0 items-center justify-between gap-3 px-4 pb-4 pt-6">
            <div>
              <SheetTitle className="text-xl font-bold">设置</SheetTitle>
              <SheetDescription className="mt-1 text-sm text-muted-foreground">
                个性化与全局设置
              </SheetDescription>
            </div>
            <SheetClose render={<Button variant="ghost" size="icon-lg" aria-label="关闭菜单" />}>
              <X aria-hidden="true" />
            </SheetClose>
          </div>
          <SettingsHome
            onSelect={(view, query) => {
              setOpen(false)
              if (view === 'accounts') onOpenAccounts?.()
              else onOpenPage(query)
            }}
          />
        </SheetContent>
      </Sheet>
    )

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) {
          setPluginId(undefined)
          setSection('general')
        }
      }}
    >
      <DialogTrigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            className="titlebar-settings rounded-lg"
            aria-label="设置"
            title="设置"
          />
        }
      >
        <motion.span
          className="flex items-center justify-center"
          whileHover={reducedMotion ? undefined : { rotate: 45 }}
          transition={{ duration: 0.2 }}
        >
          <Settings aria-hidden="true" />
        </motion.span>
      </DialogTrigger>
      <DialogPopup
        showCloseButton={false}
        className="settings-dialog overflow-hidden rounded-lg"
        style={{ width: 1024, maxWidth: 'calc(100vw - 40px)', height: '85dvh', padding: 0, gap: 0 }}
        initial={{ opacity: 0, scale: reducedMotion ? 1 : 0.97 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: reducedMotion ? 1 : 0.98 }}
        transition={{ duration: reducedMotion ? 0 : 0.18 }}
      >
        <DialogClose
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              className="absolute right-4 top-4 rounded-lg"
              aria-label="关闭设置"
              title="关闭设置"
            />
          }
        >
          <X aria-hidden="true" />
        </DialogClose>
        <div className="flex h-full min-h-0 w-full overflow-hidden">
          <aside className="flex w-44 shrink-0 flex-col bg-muted/50 sm:w-60">
            <div className="px-5 pb-4 pt-5">
              <DialogTitle className="text-[26px] font-bold leading-none tracking-tight">
                设置
              </DialogTitle>
              <DialogDescription className="mt-1.5 text-sm text-muted-foreground">
                个性化与全局设置
              </DialogDescription>
            </div>
            <div className="mx-3 border-t border-border" role="separator" />
            <nav
              aria-label="设置导航"
              className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-auto px-3 py-3"
            >
              <Button
                variant={section === 'general' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'general' ? 'page' : undefined}
                onClick={() => setSection('general')}
              >
                <Settings aria-hidden="true" />
                <span>常规设置</span>
              </Button>
              <Button
                variant={section === 'personalization' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'personalization' ? 'page' : undefined}
                onClick={() => setSection('personalization')}
              >
                <SlidersHorizontal aria-hidden="true" />
                <span>个性化设置</span>
              </Button>
              <Button
                variant={section === 'playback' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'playback' ? 'page' : undefined}
                onClick={() => setSection('playback')}
              >
                <Volume2 aria-hidden="true" />
                <span>播放设置</span>
              </Button>
              <Button
                variant={section === 'shortcuts' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'shortcuts' ? 'page' : undefined}
                onClick={() => setSection('shortcuts')}
              >
                <Keyboard aria-hidden="true" />
                <span>快捷键设置</span>
              </Button>
              <Button
                variant={section === 'lyrics' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'lyrics' ? 'page' : undefined}
                onClick={() => setSection('lyrics')}
              >
                <Mic2 aria-hidden="true" />
                <span>歌词设置</span>
              </Button>
              <Button
                variant={section === 'local-cache' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'local-cache' ? 'page' : undefined}
                onClick={() => setSection('local-cache')}
              >
                <FolderCog aria-hidden="true" />
                <span>本地与缓存</span>
              </Button>
              <Button
                variant={section === 'adapters' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'adapters' ? 'page' : undefined}
                onClick={() => setSection('adapters')}
              >
                <Blocks aria-hidden="true" />
                <span>适配器设置</span>
              </Button>
              <Button
                variant={section === 'plugins' ? 'secondary' : 'ghost'}
                className="justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'plugins' ? 'page' : undefined}
                onClick={() => setSection('plugins')}
              >
                <LayoutGrid aria-hidden="true" />
                <span>插件设置</span>
              </Button>
              <Button
                variant={section === 'about' ? 'secondary' : 'ghost'}
                className="mt-auto justify-start gap-2.5 rounded-lg px-3"
                aria-current={section === 'about' ? 'page' : undefined}
                onClick={() => setSection('about')}
              >
                <Info aria-hidden="true" />
                <span>关于</span>
              </Button>
            </nav>
            <div className="flex flex-col gap-1 px-5 pb-5 pt-2">
              <span className="text-sm font-semibold">Nons</span>
              <span className="text-xs text-muted-foreground">Version {version}</span>
            </div>
          </aside>
          <section
            aria-label={
              section === 'general'
                ? '常规设置'
                : section === 'personalization'
                  ? '个性化设置'
                  : section === 'playback'
                    ? '播放设置'
                    : section === 'lyrics'
                      ? '歌词设置'
                      : section === 'local-cache'
                        ? '本地与缓存'
                        : section === 'adapters'
                          ? '适配器设置'
                          : section === 'plugins'
                            ? '插件设置'
                            : section === 'shortcuts'
                              ? '快捷键设置'
                              : '关于 Nons'
            }
            className="min-w-0 flex-1 overflow-auto bg-background p-5 pt-14 sm:p-10"
          >
            <motion.div
              key={section}
              initial={{ opacity: reducedMotion ? 1 : 0, y: reducedMotion ? 0 : 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.2 }}
            >
              <SettingsContent
                section={section}
                theme={theme}
                onThemeChange={setTheme}
                pluginId={pluginId}
              />
            </motion.div>
          </section>
        </div>
      </DialogPopup>
    </Dialog>
  )
}
