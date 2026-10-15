import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { errorText, nativeCall, type Track } from '@/lib/player'
import type { SearchSource } from '@/features/search/aggregate-query'
import type { SourceStatus } from './source-store'

export interface ResourceProviderStatus {
  descriptor: {
    id: string
    displayName: string
    platforms: string[]
    searchPlatforms: string[]
    contractVersion: number
    coverHosts: string[]
  }
  enabled: boolean
  loading?: boolean
  loadError?: string | null
}
export function useResourceProviders() {
  const [providers, setProviders] = useState<ResourceProviderStatus[]>([])
  const [error, setError] = useState<string>()
  useEffect(() => {
    let disposed = false
    let revision = 0
    async function refresh() {
      const request = ++revision
      try {
        const values = await nativeCall<ResourceProviderStatus[]>('resource_provider_list')
        if (!disposed && request === revision) {
          setProviders(values)
          setError(undefined)
        }
      } catch (cause) {
        if (!disposed && request === revision) setError(errorText(cause))
      }
    }
    if (!isTauri()) return
    void refresh()
    const subscription = listen('adapters-changed', () => void refresh())
    return () => {
      disposed = true
      void subscription.then((unlisten) => unlisten())
    }
  }, [])
  return { providers, error }
}
export function providerSearchSources(
  providers: ResourceProviderStatus[],
  statuses: SourceStatus[]
): SearchSource[] {
  return providers
    .filter((p) => p.enabled)
    .flatMap(({ descriptor }) =>
      descriptor.searchPlatforms.map((platform) => ({
        source: `resource/${descriptor.id}/${platform}`,
        name: `${descriptor.displayName} · ${statuses.find((s) => s.descriptor.source === platform)?.descriptor.displayName ?? platform}`,
        provider: descriptor.id,
        platform
      }))
    )
}
export function searchResourceProvider(
  source: SearchSource,
  keyword: string,
  cursor: string | null,
  limit = 50
) {
  return nativeCall<{ items: Track[]; nextCursor: string | null }>('resource_search', {
    id: source.provider,
    request: { platform: source.platform, keyword, page: { cursor, limit } }
  })
}
