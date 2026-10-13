import { usePlugins } from '@/plugins/host'
import { pluginPath } from '@/plugins/types'
import { useMusicNavigation } from './music-navigation'

export function usePrimaryMusicPage() {
  const { page } = useMusicNavigation()
  const { plugins } = usePlugins()
  if (['library', 'discover', 'local', 'tools'].includes(page.view))
    return !page.query && !page.collection
  if (page.view !== 'plugin') return false
  return plugins.some(
    (plugin) =>
      plugin.loaded &&
      plugin.manifest.contributes.navigation.some((item) => {
        const target = plugin.manifest.contributes.pages.find(
          (candidate) => candidate.id === item.page
        )
        return target && page.query === pluginPath(plugin.manifest.id, target.path)
      })
  )
}
