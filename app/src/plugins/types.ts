import { reportCaughtError } from '@/lib/diagnostics'
export interface PluginManifest {
  id: string
  name: string
  description?: string | null
  repository?: string | null
  version: string
  backend: string | null
  frontend: string | null
  configuration?: string | null
  permissions: string[]
  httpHosts?: string[]
  fileRoots?: string[]
  engines: { app: string; pluginApi: string; uiApi: string }
  contributes: {
    views: { id: string; slot: string; export: string }[]
    pages: { id: string; path: string; export: string }[]
    navigation: { id: string; label: string; page: string; category?: 'tool' | null }[]
    contextMenus?: {
      id: string
      target: 'song'
      source: string
      label: string
      icon?: string | null
      export: string
    }[]
  }
}
export interface PluginDescriptor {
  manifest: PluginManifest
  enabled: boolean
  loaded: boolean
  generation: number
  error: string | null
}
export type PluginSong = import('@/features/music/types').MusicTrack

export function pluginPath(pluginId: string, path = '/') {
  if (!/^[a-z][a-z0-9-]{0,63}$/.test(pluginId)) throw new Error('插件 ID 无效')
  if (
    !path.startsWith('/') ||
    path.startsWith('//') ||
    /[\\%:#]/.test(path) ||
    path.split(/[/?]/).some((part) => part === '.' || part === '..')
  )
    throw new Error('插件页面路径无效')
  return `/plugins/${pluginId}${path}`
}
export function resolvePluginPath(path: string) {
  const match = /^\/plugins\/([a-z][a-z0-9-]{0,63})(\/.*)?$/.exec(path)
  if (!match) return undefined
  const relative = match[2] || '/'
  try {
    pluginPath(match[1], relative)
  } catch (cause) {
    reportCaughtError('app/src/plugins/types.ts', cause)

    return undefined
  }
  const [pathname, ...query] = relative.split('?')
  return { pluginId: match[1], pathname, search: query.length ? `?${query.join('?')}` : '' }
}
export function resolvePluginPage(manifest: PluginManifest, pathname: string) {
  return manifest.contributes.pages
    .filter(
      (page) =>
        page.path === '/' ||
        pathname === page.path ||
        pathname.startsWith(`${page.path.replace(/\/$/, '')}/`)
    )
    .sort((a, b) => b.path.length - a.path.length)[0]
}
