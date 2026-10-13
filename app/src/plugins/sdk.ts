import { useCallback, useContext, useMemo, useSyncExternalStore } from 'react'
import { listen } from '@tauri-apps/api/event'
import { nativeCall, usePlayer as useHostPlayer, useProgress } from '@/lib/player'
import { useTheme as useHostTheme } from '@/features/settings/use-theme'
import { useCoverSource as useHostCoverSource } from '@/components/music/use-cover-source'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { checkScope, useScope, PluginNavigationScope, type Scope } from './scope'
import type { ConfigSnapshot } from './configuration-types'
export type { ConfigSnapshot } from './configuration-types'
import { pluginPath, resolvePluginPath } from './types'
import { registerCollectionTab, type CollectionTabRegistration } from './collection-tabs'
export type { PluginCollection, CollectionTabRegistration } from './collection-tabs'
export { MusicPageHeader } from '@/components/music/music-page'
export { Button } from '@/components/ui/button'
export { Progress } from '@/components/ui/progress'
export { Input } from '@/components/ui/input'
export { Icon } from './icon'
export {
  Tabs,
  TabsList,
  TabsTab,
  TabsPanel,
  TabsPanels
} from '@/components/animate-ui/components/base/tabs'
export { DownloadedSongList } from './downloaded-song-list'
export type { PluginSong } from './types'
export type PluginMenuSong = ReturnType<typeof import('./song-menu-context').publicMenuSong>
export { SongArtists, SongLikeButton, SongIdentity } from './song-components'

export function useSongPlayback() {
  const scope = useScope('player:control')
  return useCallback(
    (id: number, mode: 'now' | 'next') => hostCall<void>(scope, 'netease.play-song', { id, mode }),
    [scope]
  )
}

export type * from '@/features/music/types'
export function useMusicSource(): import('@/features/music/types').MusicSourceClient {
  const scope = useScope()
  return useMemo(
    () => ({
      sources: () => {
        checkScope(scope, 'music:metadata')
        return hostCall<import('@/features/music/types').SourceDescriptor[]>(
          scope,
          'music.sources',
          {}
        )
      },
      getTrack: (reference: import('@/features/music/types').EntityRef) => {
        checkScope(scope, 'music:metadata')
        return hostCall<import('@/features/music/types').MusicTrack>(scope, 'music.read-track', {
          reference
        })
      },
      query: (source: string, request: import('@/features/music/types').MusicReadRequest) => {
        const account = [
          'librarySummary',
          'libraryCollections',
          'history',
          'favorites',
          'recommendedTracks'
        ].includes(request.operation)
        checkScope(scope, account ? 'music:library' : 'music:metadata')
        return hostCall<Exclude<import('@/features/music/types').MusicResponse, { type: 'write' }>>(
          scope,
          'music.query',
          { source, request }
        )
      },
      write: (source: string, request: import('@/features/music/types').MusicWriteRequest) => {
        checkScope(scope, 'music:write')
        return hostCall<Extract<import('@/features/music/types').MusicResponse, { type: 'write' }>>(
          scope,
          'music.write',
          { source, request }
        )
      }
    }),
    [scope]
  )
}
/** @deprecated Use useMusicSource for metadata and business queries. */
export function useNetease() {
  const scope = useScope('music:metadata')
  return useMemo(
    () => ({
      getSong: (id: number) =>
        hostCall<import('./types').PluginSong>(scope, 'netease.get-song', { id }),
      playSong: (id: number, mode: 'now' | 'next') =>
        hostCall<void>(scope, 'netease.play-song', { id, mode })
    }),
    [scope]
  )
}

export interface PluginHttpRequest {
  url: string
  method?: 'GET' | 'HEAD' | 'POST'
  headers?: Record<string, string>
  body?: string
  timeoutMs?: number
  responseType?: 'text' | 'none'
}
export interface PluginHttpResponse {
  status: number
  headers: Record<string, string>
  body: string
  cookies?: string[]
}
export function usePluginHttp() {
  const scope = useScope('http:request')
  return useMemo(
    () => ({
      request: (request: PluginHttpRequest) =>
        hostCall<PluginHttpResponse>(scope, 'http.request', request)
    }),
    [scope]
  )
}

export function usePluginClipboard() {
  const scope = useScope('clipboard:read')
  return useMemo(
    () => ({
      readText: () => hostCall<string>(scope, 'clipboard.read-text', {}),
      async subscribe(callback: (text: string) => void): Promise<() => void> {
        checkScope(scope, 'clipboard:read')
        let disposed = false
        const unlisten = await listen<{ pluginId: string; generation: number; text: string }>(
          'plugin-clipboard-changed',
          ({ payload }) => {
            if (
              disposed ||
              !scope.active ||
              payload.pluginId !== scope.descriptor.manifest.id ||
              payload.generation !== scope.descriptor.generation
            )
              return
            callback(payload.text)
          }
        )
        if (!scope.active) {
          unlisten()
          checkScope(scope)
        }
        const cleanup = () => {
          if (disposed) return
          disposed = true
          unlisten()
          scope.cleanups?.delete(cleanup)
        }
        scope.cleanups ??= new Set()
        scope.cleanups.add(cleanup)
        return cleanup
      }
    }),
    [scope]
  )
}

async function hostCall<T>(scope: Scope, operation: string, args: unknown): Promise<T> {
  checkScope(scope)
  const result = await nativeCall<string>('plugin_host_call', {
    id: scope.descriptor.manifest.id,
    generation: scope.descriptor.generation,
    operation,
    args: JSON.stringify(args)
  })
  checkScope(scope)
  return JSON.parse(result) as T
}
export function usePluginBackend() {
  const scope = useScope()
  return useMemo(
    () => ({
      async call<T = unknown>(method: string, args: unknown = null): Promise<T> {
        checkScope(scope)
        const result = await nativeCall<string>('plugin_call', {
          id: scope.descriptor.manifest.id,
          generation: scope.descriptor.generation,
          method,
          args: JSON.stringify(args)
        })
        checkScope(scope)
        return JSON.parse(result) as T
      }
    }),
    [scope]
  )
}
export function usePluginEvent<T = unknown>(event: string): T | undefined {
  const scope = useScope()
  if (!/^[a-zA-Z0-9_-]{1,64}$/.test(event)) throw new Error('事件名称无效')
  const subscribe = useCallback(
    (listener: () => void) => {
      scope.listeners.add(listener)
      return () => {
        scope.listeners.delete(listener)
      }
    },
    [scope]
  )
  return useSyncExternalStore(subscribe, () => scope.events.get(event) as T | undefined)
}
export function usePluginStorage() {
  const scope = useScope('storage')
  return useMemo(
    () => ({
      get: <T = unknown>(key: string) => hostCall<T | null>(scope, 'storage.get', { key }),
      set: (key: string, value: unknown) => hostCall<void>(scope, 'storage.set', { key, value }),
      delete: (key: string) => hostCall<void>(scope, 'storage.delete', { key })
    }),
    [scope]
  )
}
export function usePluginNavigate() {
  const scope = useScope('ui')
  const local = useContext(PluginNavigationScope)
  const { navigate } = useMusicNavigation()
  return useCallback(
    (path = '/') => {
      checkScope(scope, 'ui')
      pluginPath(scope.descriptor.manifest.id, path)
      if (local) {
        local.navigate(path)
        return
      }
      navigate('plugin', pluginPath(scope.descriptor.manifest.id, path))
    },
    [scope, navigate, local]
  )
}
export function usePluginRoute() {
  const scope = useScope('ui')
  const local = useContext(PluginNavigationScope)
  const { page } = useMusicNavigation()
  if (local) return { pathname: local.pathname, search: local.search }
  const route = page.view === 'plugin' ? resolvePluginPath(page.query) : undefined
  return route?.pluginId === scope.descriptor.manifest.id
    ? { pathname: route.pathname, search: route.search }
    : { pathname: '/', search: '' }
}
export function usePluginConfig() {
  const scope = useScope('config')
  return useMemo(
    () => ({
      getSnapshot: () => hostCall<ConfigSnapshot>(scope, 'config.get', {}),
      update: (patch: Record<string, unknown>, revision: number) =>
        hostCall<ConfigSnapshot>(scope, 'config.update', { patch, revision }),
      reset: (revision: number, keys?: string[]) =>
        hostCall<ConfigSnapshot>(scope, 'config.reset', { revision, keys }),
      async subscribe(callback: (snapshot: ConfigSnapshot) => void) {
        checkScope(scope, 'config')
        let disposed = false
        let serial = 0
        const unlisten = await listen<{ pluginId: string }>(
          'plugin-config-changed',
          ({ payload }) => {
            if (payload.pluginId !== scope.descriptor.manifest.id || disposed || !scope.active)
              return
            const request = ++serial
            void hostCall<ConfigSnapshot>(scope, 'config.get', {})
              .then((value) => {
                if (!disposed && request === serial && scope.active) callback(value)
              })
              .catch(console.error)
          }
        )
        if (!scope.active) {
          unlisten()
          checkScope(scope)
        }
        const cleanup = () => {
          if (disposed) return
          disposed = true
          unlisten()
          scope.cleanups?.delete(cleanup)
        }
        scope.cleanups ??= new Set()
        scope.cleanups.add(cleanup)
        return cleanup
      }
    }),
    [scope]
  )
}
export function usePluginFiles() {
  const scope = useScope()
  return useMemo(
    () => ({
      pickAudio: (extensions?: string[]) =>
        hostCall<{ root: string; path: string; name: string }[]>(scope, 'files.pick-audio', {
          extensions
        }),
      decodeAudio: (root: string, path: string) =>
        hostCall<{ path: string; format: 'mp3' | 'flac'; bytes: number }>(
          scope,
          'files.decode-audio',
          { root, path }
        ),
      roots: () => hostCall<{ id: string; writable: boolean }[]>(scope, 'files.roots', {}),
      stat: (root: string, path = '') =>
        hostCall<{ isDirectory: boolean; size: number }>(scope, 'files.stat', { root, path }),
      list: (root: string, path = '', offset = 0) =>
        hostCall<{
          entries: { name: string; isDirectory: boolean; isLink: boolean }[]
          nextOffset: number | null
        }>(scope, 'files.list', { root, path, offset }),
      mkdir: (root: string, path: string) => hostCall<void>(scope, 'files.mkdir', { root, path }),
      rename: (root: string, path: string, to: string) =>
        hostCall<void>(scope, 'files.rename', { root, path, to }),
      publish: (root: string, path: string, to: string) =>
        hostCall<void>(scope, 'files.publish', { root, path, to }),
      remove: (root: string, path: string) => hostCall<void>(scope, 'files.remove', { root, path }),
      open: async (root: string, path: string, mode: 'read' | 'readWrite' | 'create' = 'read') => {
        const { handle } = await hostCall<{ handle: number }>(scope, 'files.open', {
          root,
          path,
          mode
        })
        return {
          async read(offset: number, length = 32768) {
            const { data } = await hostCall<{ data: string }>(scope, 'files.read', {
              handle,
              offset,
              length
            })
            return Uint8Array.from(atob(data), (c) => c.charCodeAt(0))
          },
          write(offset: number, bytes: Uint8Array) {
            if (bytes.length > 32768) throw new Error('写入块超过 32KiB')
            const data = btoa(Array.from(bytes, (byte) => String.fromCharCode(byte)).join(''))
            return hostCall<{ bytes: number }>(scope, 'files.write', { handle, offset, data })
          },
          truncate: (length: number) => hostCall<void>(scope, 'files.truncate', { handle, length }),
          close: () => hostCall<void>(scope, 'files.close', { handle })
        }
      }
    }),
    [scope]
  )
}
export function usePlayer() {
  const scope = useScope('player:read')
  const state = useHostPlayer('index', 'queue', 'status', 'volume')
  const progress = useProgress()
  return useMemo(() => {
    const track = state.index === null ? undefined : state.queue[state.index]
    return {
      status: state.status,
      positionMs: progress.positionMs,
      durationMs: progress.durationMs,
      volume: state.volume,
      track: track
        ? {
            title: track.title,
            artist: track.artist,
            album: track.album,
            durationMs: track.durationMs,
            cover: /^https?:/.test(track.cover) ? track.cover : ''
          }
        : null,
      control: (action: 'pause' | 'resume' | 'next' | 'previous' | 'stop') => {
        checkScope(scope, 'player:control')
        return hostCall<void>(scope, 'player.control', { action })
      }
    }
  }, [state, progress, scope])
}
export function useTheme() {
  useScope('ui')
  const [theme] = useHostTheme()
  return theme
}
export function useCoverSource(cover?: string, enabled = true) {
  useScope('ui')
  return useHostCoverSource(cover, enabled)
}

export interface PluginClient {
  registerCollectionTab(tab: CollectionTabRegistration): () => void
  call<T = unknown>(operation: string, args?: unknown): Promise<T>
  openPage(path?: string): void
  openConfiguration(): void
}
export function createPluginClient(scope: Scope, navigate: (path: string) => void): PluginClient {
  return {
    registerCollectionTab: (tab) => registerCollectionTab(scope, tab),
    call: (operation, args = {}) => hostCall(scope, operation, args),
    openPage: (path = '/') => {
      checkScope(scope, 'ui')
      navigate(pluginPath(scope.descriptor.manifest.id, path))
    },
    openConfiguration: () => {
      checkScope(scope, 'ui')
      window.dispatchEvent(
        new CustomEvent('plugin-open-configuration', { detail: scope.descriptor.manifest.id })
      )
    }
  }
}
