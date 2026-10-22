import { usePlugins } from '@/plugins/host'
import { pluginPath } from '@/plugins/types'

export function useMusicTools() {
  const { plugins, loaded } = usePlugins()
  return plugins
    .filter((plugin) => loaded.has(plugin.manifest.id))
    .flatMap((plugin) =>
      plugin.manifest.contributes.navigation
        .filter((item) => item.category === 'tool')
        .flatMap((item) => {
          const target = plugin.manifest.contributes.pages.find((page) => page.id === item.page)
          return target
            ? [
                {
                  id: `${plugin.manifest.id}:${item.id}`,
                  plugin,
                  item,
                  path: pluginPath(plugin.manifest.id, target.path)
                }
              ]
            : []
        })
    )
}
