import { Button } from '@/components/ui/button'
import { cn } from 'cn'
import { usePlugins } from '@/plugins/host'
import { pluginPath } from '@/plugins/types'
import { MusicNavButtons } from './music-nav-buttons'
import { useMusicNavigation } from './music-navigation'

export function MusicNavigationItems({ mobile = false }: { mobile?: boolean }) {
  const { plugins, loaded } = usePlugins()
  const { page, navigate } = useMusicNavigation()
  const hasTools = plugins.some(
    (plugin) =>
      loaded.has(plugin.manifest.id) &&
      plugin.manifest.contributes.navigation.some((item) => item.category === 'tool')
  )
  const toolsSelected =
    page.view === 'tools' ||
    (page.view === 'plugin' &&
      plugins.some(
        (plugin) =>
          plugin.manifest.contributes.navigation.some((item) => item.category === 'tool') &&
          page.query.startsWith(`/plugins/${plugin.manifest.id}/`)
      ))
  return (
    <>
      {mobile && hasTools && (
        <Button
          variant="ghost"
          className={cn('mobile-navigation-item', toolsSelected && 'is-selected')}
          aria-current={toolsSelected ? 'page' : undefined}
          onClick={() => navigate('tools')}
        >
          工具
        </Button>
      )}
      <MusicNavButtons mobile={mobile} />
      {plugins
        .filter((plugin) => plugin.loaded)
        .flatMap((plugin) =>
          plugin.manifest.contributes.navigation
            .filter((item) => item.category !== 'tool')
            .map((item) => {
              const target = plugin.manifest.contributes.pages.find(
                (page) => page.id === item.page
              )!
              const path = pluginPath(plugin.manifest.id, target.path)
              const selected =
                page.view === 'plugin' &&
                (page.query === path || page.query.startsWith(`${path.replace(/\/$/, '')}/`))
              return (
                <Button
                  key={`${plugin.manifest.id}:${item.id}`}
                  variant="ghost"
                  className={cn(mobile && 'mobile-navigation-item', selected && 'is-selected')}
                  aria-current={selected ? 'page' : undefined}
                  onClick={() => navigate('plugin', path)}
                >
                  <span className="mobile-navigation-label">{item.label}</span>
                </Button>
              )
            })
        )}
    </>
  )
}
