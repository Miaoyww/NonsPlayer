import { ArrowUpRight, Wrench } from 'lucide-react'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'
import { Button } from '@/components/ui/button'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { usePlugins } from '@/plugins/host'
import { pluginPath } from '@/plugins/types'

export function ToolsPage() {
  const { navigate } = useMusicNavigation()
  return (
    <MusicPage aria-label="音乐工具">
      <MusicPageHeader title="工具" />
      <p className="mb-6 text-sm text-muted-foreground">处理音乐文件，让收藏更易整理。</p>
      <ToolsContent onOpenTool={(path) => navigate('plugin', path)} />
    </MusicPage>
  )
}

export function ToolsContent({ onOpenTool }: { onOpenTool: (path: string) => void }) {
  const { plugins, loaded } = usePlugins()
  const tools = plugins
    .filter((plugin) => loaded.has(plugin.manifest.id))
    .flatMap((plugin) =>
      plugin.manifest.contributes.navigation
        .filter((item) => item.category === 'tool')
        .map((item) => ({ plugin, item }))
    )
  return (
    <>
      {tools.length ? (
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {tools.map(({ plugin, item }) => {
            const target = plugin.manifest.contributes.pages.find((page) => page.id === item.page)
            if (!target) return null
            return (
              <Button
                key={`${plugin.manifest.id}:${item.id}`}
                variant="outline"
                className="h-auto min-h-40 items-start justify-start whitespace-normal rounded-2xl p-5 text-left"
                onClick={() => onOpenTool(pluginPath(plugin.manifest.id, target.path))}
              >
                <span className="flex w-full flex-col gap-4">
                  <span className="flex items-center justify-between">
                    <Wrench aria-hidden="true" className="size-5 text-primary" />
                    <ArrowUpRight aria-hidden="true" className="size-4 text-muted-foreground" />
                  </span>
                  <span className="text-base font-semibold">{item.label}</span>
                  <span className="text-sm font-normal leading-6 text-muted-foreground">
                    {plugin.manifest.description ?? plugin.manifest.name}
                  </span>
                </span>
              </Button>
            )
          })}
        </div>
      ) : (
        <div className="rounded-2xl border border-dashed border-border px-6 py-12 text-center">
          <Wrench aria-hidden="true" className="mx-auto mb-4 size-8 text-muted-foreground" />
          <p>还没有启用音乐工具</p>
          <p className="mt-2 text-sm text-muted-foreground">
            在设置 → 插件中安装并启用工具插件，入口会显示在这里。
          </p>
        </div>
      )}
    </>
  )
}
