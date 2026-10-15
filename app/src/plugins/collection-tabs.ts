import type { ComponentType } from 'react'
import { checkScope, type Scope } from './scope'

export interface PluginCollection {
  source: string
  kind: 'playlist' | 'album' | 'artist'
  id: number | string
  name: string
  subtitle: string
  cover: string
  trackCount: number
}
export interface CollectionTabRegistration {
  id: string
  label: string
  kinds?: PluginCollection['kind'][]
  sources?: PluginCollection['source'][]
  component: ComponentType<{ collection: PluginCollection }>
}
export interface CollectionTabEntry extends CollectionTabRegistration {
  key: string
  scope: Scope
}
let entries: CollectionTabEntry[] = []
const listeners = new Set<() => void>()
export const collectionTabSnapshot = () => entries
export function subscribeCollectionTabs(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
const notify = () => listeners.forEach((listener) => listener())

export function registerCollectionTab(scope: Scope, tab: CollectionTabRegistration) {
  checkScope(scope, 'ui')
  if (!/^[a-z][a-z0-9-]{0,63}$/.test(tab.id) || !tab.label.trim() || tab.label.length > 40)
    throw new Error('详情 tab ID 或标题无效')
  if (
    typeof tab.component !== 'function' &&
    !(typeof tab.component === 'object' && tab.component !== null && '$$typeof' in tab.component)
  )
    throw new Error('详情 tab 需要 React 组件')
  if (
    tab.kinds?.some((kind) => !['playlist', 'album', 'artist'].includes(kind)) ||
    tab.sources?.some((source) => !/^[a-z][a-z0-9-]{0,63}$/.test(source))
  )
    throw new Error('详情 tab 类型或来源无效')
  const key = `${scope.descriptor.manifest.id}:${scope.descriptor.generation}:${tab.id}`
  if (entries.some((entry) => entry.key === key)) throw new Error('详情 tab ID 重复')
  if (entries.filter((entry) => entry.scope === scope).length >= 8)
    throw new Error('每个插件最多注册 8 个详情 tab')
  const entry = {
    ...tab,
    label: tab.label.trim(),
    kinds: tab.kinds?.slice(),
    sources: tab.sources?.slice(),
    key,
    scope
  }
  entries = [...entries, entry]
  let disposed = false
  const cleanup = () => {
    if (disposed) return
    disposed = true
    entries = entries.filter((value) => value !== entry)
    scope.cleanups?.delete(cleanup)
    notify()
  }
  scope.cleanups ??= new Set()
  scope.cleanups.add(cleanup)
  notify()
  return cleanup
}
