import { useState } from 'react'
import { Wrench } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogTrigger } from '@/components/animate-ui/components/base/dialog'
import { WorkspaceDialogPopup } from '@/components/workspace-dialog'
import { PluginPageHost } from '@/plugins/host'
import { pluginPath, resolvePluginPath } from '@/plugins/types'
import { useMusicTools } from './use-music-tools'

export function ToolsDialog() {
  const [open, setOpen] = useState(false)
  const [selection, setSelection] = useState<{ id: string; path: string }>()
  const tools = useMusicTools()
  const selected = tools.find((tool) => tool.id === selection?.id) ?? tools[0]
  const path = selected && (selection?.id === selected.id ? selection.path : selected.path)
  const route = path ? resolvePluginPath(path) : undefined

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) setSelection(undefined)
      }}
    >
      <DialogTrigger
        render={<Button variant="ghost" size="icon-sm" aria-label="工具" title="工具" />}
      >
        <Wrench aria-hidden="true" />
      </DialogTrigger>
      <WorkspaceDialogPopup
        title="工具"
        description="音乐文件与收藏整理"
        navigation={tools.map((tool) => (
          <Button
            key={tool.id}
            variant={selected?.id === tool.id ? 'secondary' : 'ghost'}
            className="justify-start gap-2.5 rounded-lg px-3"
            aria-current={selected?.id === tool.id ? 'page' : undefined}
            title={tool.item.label}
            onClick={() => setSelection({ id: tool.id, path: tool.path })}
          >
            <Wrench aria-hidden="true" />
            <span className="truncate">{tool.item.label}</span>
          </Button>
        ))}
        contentLabel={selected?.item.label ?? '音乐工具'}
        contentKey={selected?.id}
        contentClassName="music-library [&_.library-heading]:text-xl [&_.library-heading]:leading-7 [&_.music-page-heading]:mb-0"
      >
        {open &&
          (selected && path && route ? (
            <PluginPageHost
              path={path}
              navigate={(relative) =>
                setSelection({ id: selected.id, path: pluginPath(route.pluginId, relative) })
              }
            />
          ) : (
            <div className="space-y-2">
              <h2 className="text-xl font-bold">还没有启用音乐工具</h2>
              <p className="text-sm text-muted-foreground">
                在设置 → 插件中安装并启用工具插件，入口会显示在这里。
              </p>
            </div>
          ))}
      </WorkspaceDialogPopup>
    </Dialog>
  )
}
