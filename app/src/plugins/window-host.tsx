import { Component, useEffect, useState, type ComponentType, type ReactNode } from 'react'
import { listen } from '@tauri-apps/api/event'
import { connectPlayer, errorText, nativeCall } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import { useTheme } from '@/features/settings/use-theme'
import { MusicNavigationStateProvider } from '@/features/workspace/music-navigation'
import { pluginResourceUrl } from './host'
import { PluginScope, type Scope } from './scope'
import type { PluginDescriptor } from './types'

export function isPluginWindow() {
  return new URLSearchParams(window.location.search).has('pluginWindow')
}

class WindowBoundary extends Component<{ children: ReactNode; scope: Scope }, { error?: string }> {
  state: { error?: string } = {}
  static getDerivedStateFromError(error: unknown) {
    return { error: errorText(error) }
  }
  componentDidCatch(error: Error) {
    reportCaughtError('plugins.window.render', error)
    const scope = this.props.scope
    scope.active = false
    scope.cleanups?.forEach((cleanup) => cleanup())
    void nativeCall('plugin_fault', {
      id: scope.descriptor.manifest.id,
      generation: scope.descriptor.generation,
      error: error.message
    }).catch((cause) => reportCaughtError('plugins.window.fault', cause))
  }
  render() {
    return this.state.error ? <p role="alert">{this.state.error}</p> : this.props.children
  }
}

function WindowContent() {
  useTheme()
  const [loaded, setLoaded] = useState<{ scope: Scope; View: ComponentType }>()
  const [error, setError] = useState<string>()
  useEffect(() => {
    let disposed = false
    let scope: Scope | undefined
    const cleanups: (() => void)[] = []
    const retain = (cleanup: () => void) => {
      if (disposed) cleanup()
      else cleanups.push(cleanup)
    }
    const release = () => {
      if (!scope) return
      scope.active = false
      scope.cleanups?.forEach((cleanup) => cleanup())
      scope.events.clear()
      scope.listeners.forEach((listener) => listener())
      scope.listeners.clear()
    }
    void (async () => {
      const query = new URLSearchParams(window.location.search)
      const id = query.get('pluginWindow')
      const generation = Number(query.get('generation'))
      const windowId = query.get('window')
      if (!id || !windowId || !Number.isSafeInteger(generation) || generation <= 0)
        throw new Error('插件窗口参数无效')
      let descriptor: PluginDescriptor | undefined
      const deadline = performance.now() + 6000
      // Backend initialize may open a window before its new generation is published.
      while (!disposed && performance.now() < deadline) {
        const plugins = await nativeCall<PluginDescriptor[]>('plugin_list')
        const candidate = plugins.find((plugin) => plugin.manifest.id === id)
        if (!candidate || !candidate.enabled || candidate.generation > generation)
          throw new Error('插件已停用或重新加载')
        if (candidate.generation === generation && candidate.loaded) {
          descriptor = candidate
          break
        }
        await new Promise((resolve) => setTimeout(resolve, 100))
      }
      if (disposed) return
      const contribution = descriptor?.manifest.contributes.windows?.find(
        (entry) => entry.id === windowId
      )
      if (
        !descriptor?.manifest.frontend ||
        !contribution ||
        !descriptor.manifest.permissions.includes('window:manage')
      )
        throw new Error('插件窗口声明或加载代次已失效')
      if (disposed) return
      scope = { descriptor, active: true, events: new Map(), listeners: new Set() }
      const current = scope
      retain(
        await listen<{ pluginId: string; generation: number; event: string; payload: unknown }>(
          'plugin-event',
          ({ payload }) => {
            if (
              disposed ||
              !current.active ||
              payload.pluginId !== id ||
              payload.generation !== generation
            )
              return
            current.events.set(payload.event, payload.payload)
            current.listeners.forEach((listener) => listener())
          }
        )
      )
      retain(
        await listen('plugins-changed', () => {
          void nativeCall<PluginDescriptor[]>('plugin_list')
            .then((plugins) => {
              if (disposed) return
              if (
                !plugins.some(
                  (plugin) =>
                    plugin.manifest.id === id && plugin.generation === generation && plugin.loaded
                )
              ) {
                release()
                setLoaded(undefined)
                setError('插件已停用或重新加载')
              }
            })
            .catch((cause) => reportCaughtError('plugins.window.refresh', cause))
        })
      )
      if (descriptor.manifest.permissions.includes('player:read'))
        retain(await connectPlayer({ observerOnly: true }))
      const module: Record<string, unknown> = await import(
        /* @vite-ignore */ pluginResourceUrl(descriptor, descriptor.manifest.frontend)
      )
      if (disposed || !current.active) return
      // Recheck after an uncancellable import before publishing the component.
      const latest = await nativeCall<PluginDescriptor[]>('plugin_list')
      if (
        !latest.some(
          (plugin) => plugin.manifest.id === id && plugin.generation === generation && plugin.loaded
        )
      )
        throw new Error('插件窗口加载代次已失效')
      if (disposed) return
      const View = module[contribution.export]
      if (typeof View !== 'function' && !(View && typeof View === 'object' && '$$typeof' in View))
        throw new Error(`插件未导出窗口组件 ${contribution.export}`)
      setLoaded({ scope: current, View: View as ComponentType })
    })().catch((cause) => {
      reportCaughtError('plugins.window.load', cause)
      if (!disposed) {
        if (scope?.active) {
          void nativeCall('plugin_fault', {
            id: scope.descriptor.manifest.id,
            generation: scope.descriptor.generation,
            error: errorText(cause)
          }).catch((error) => reportCaughtError('plugins.window.fault', error))
        }
        release()
        cleanups.splice(0).forEach((cleanup) => cleanup())
        setError(errorText(cause))
      }
    })
    return () => {
      disposed = true
      release()
      cleanups.splice(0).forEach((cleanup) => cleanup())
    }
  }, [])
  if (error) return <p role="alert">{error}</p>
  if (!loaded) return null
  return (
    <WindowBoundary scope={loaded.scope}>
      <PluginScope.Provider value={loaded.scope}>
        <loaded.View />
      </PluginScope.Provider>
    </WindowBoundary>
  )
}

export default function PluginWindowHost() {
  return (
    <MusicNavigationStateProvider>
      <WindowContent />
    </MusicNavigationStateProvider>
  )
}
