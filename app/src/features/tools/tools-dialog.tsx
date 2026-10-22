import { useState } from 'react'
import { ArrowLeft, Wrench, X } from 'lucide-react'
import { useReducedMotion } from 'motion/react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogPopup,
  DialogTitle,
  DialogTrigger
} from '@/components/animate-ui/components/base/dialog'
import { PluginPageHost } from '@/plugins/host'
import { pluginPath, resolvePluginPath } from '@/plugins/types'
import { ToolsContent } from './tools-page'

export function ToolsDialog() {
  const [open, setOpen] = useState(false)
  const [path, setPath] = useState<string>()
  const reducedMotion = useReducedMotion()
  const route = path ? resolvePluginPath(path) : undefined

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) setPath(undefined)
      }}
    >
      <DialogTrigger
        render={<Button variant="ghost" size="icon-sm" aria-label="工具" title="工具" />}
      >
        <Wrench aria-hidden="true" />
      </DialogTrigger>
      <DialogPopup
        showCloseButton={false}
        className="flex flex-col overflow-hidden rounded-lg"
        style={{ width: 1024, maxWidth: 'calc(100vw - 40px)', height: '85dvh', padding: 0, gap: 0 }}
        initial={{ opacity: 0, scale: reducedMotion ? 1 : 0.97 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: reducedMotion ? 1 : 0.98 }}
        transition={{ duration: reducedMotion ? 0 : 0.18 }}
      >
        <div className="flex shrink-0 items-center gap-3 border-b px-5 py-4">
          {path && (
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="返回工具列表"
              title="返回工具列表"
              onClick={() => setPath(undefined)}
            >
              <ArrowLeft aria-hidden="true" />
            </Button>
          )}
          <div className="min-w-0 flex-1">
            <DialogTitle className="text-[26px] font-bold leading-none tracking-tight">
              工具
            </DialogTitle>
            <DialogDescription className="mt-1.5 text-sm text-muted-foreground">
              处理音乐文件，让收藏更易整理。
            </DialogDescription>
          </div>
          <DialogClose
            render={
              <Button variant="ghost" size="icon-sm" aria-label="关闭工具" title="关闭工具" />
            }
          >
            <X aria-hidden="true" />
          </DialogClose>
        </div>
        <section
          key={path ? route?.pluginId : 'tools'}
          aria-label={path ? '工具内容' : '工具列表'}
          className="music-library min-w-0 flex-1 overflow-auto p-5 sm:p-10"
        >
          {open &&
            (path && route ? (
              <PluginPageHost
                path={path}
                navigate={(relative) => setPath(pluginPath(route.pluginId, relative))}
              />
            ) : (
              <ToolsContent onOpenTool={setPath} />
            ))}
        </section>
      </DialogPopup>
    </Dialog>
  )
}
