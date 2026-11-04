import { createContext, useContext } from 'react'
import type { PluginDescriptor } from './types'

export interface Scope {
  descriptor: PluginDescriptor
  active: boolean
  events: Map<string, unknown>
  listeners: Set<() => void>
  cleanups?: Set<() => void>
}
export const PluginScope = createContext<Scope | null>(null)
export const PluginNavigationScope = createContext<{
  pathname: string
  search: string
  navigate: (path: string) => void
} | null>(null)
export const PluginFullscreenPlayerScope = createContext<{
  pageId: string | null
  showPage: (pageId: string) => void
} | null>(null)
export function checkScope(scope: Scope, permission?: string) {
  if (!scope.active) throw new Error('插件已禁用或卸载')
  if (permission && !scope.descriptor.manifest.permissions.includes(permission))
    throw new Error(`插件缺少 ${permission} 权限`)
}
export function useScope(permission?: string) {
  const scope = useContext(PluginScope)
  if (!scope) throw new Error('Plugin SDK 必须在插件组件中使用')
  checkScope(scope, permission)
  return scope
}
