import { reportCaughtError } from '@/lib/diagnostics'
import { LRUCache } from 'lru-cache'

export const RUNTIME_TTL = 10 * 60 * 1000
interface Entry {
  value: unknown
}
type Loader = { load: () => Promise<unknown>; invalidate: () => void }
interface ReadOptions {
  validate?: () => Promise<void>
  refresh?: boolean
}

export class CacheInvalidatedError extends Error {
  readonly code = 'cacheInvalidated'
  constructor() {
    super('缓存请求已失效，请重试。')
  }
}

// LRUCache owns capacity, TTL and concurrent-fetch coalescing. Wrapping values
// allows valid null responses without treating them as a cache miss.
export class RuntimeCache {
  private cache: LRUCache<string, Entry, Loader>
  private pending = new Map<string, { readers: number; invalidated: boolean; refresh: boolean }>()
  contextRevision = 0
  revision = 0
  constructor(maxBytes = 48 * 1024 * 1024, ttl = RUNTIME_TTL) {
    this.cache = new LRUCache<string, Entry, Loader>({
      max: 512,
      maxSize: maxBytes,
      ttl,
      sizeCalculation: (entry, key) =>
        Math.max(1, (JSON.stringify(entry.value)?.length ?? 0) * 2 + key.length * 2),
      fetchMethod: async (_key, _stale, { context, signal }) => {
        signal.addEventListener('abort', context.invalidate, { once: true })
        try {
          return { value: await context.load() }
        } finally {
          signal.removeEventListener('abort', context.invalidate)
        }
      }
    })
  }
  async get<T>(key: string, load: () => Promise<T>, options: ReadOptions = {}): Promise<T> {
    const active = this.pending.get(key)
    // Concurrent explicit refreshes share the replacement; the first refresh
    // cancels an ordinary read that started before it.
    if (options.refresh && !(active?.refresh && !active.invalidated)) this.delete(key)
    const current = this.pending.get(key)
    const pending =
      current && !current.invalidated
        ? current
        : { readers: 0, invalidated: false, refresh: Boolean(options.refresh) }
    pending.readers++
    this.pending.set(key, pending)
    try {
      const result = await this.cache.fetch(key, {
        context: {
          load,
          invalidate: () => {
            pending.invalidated = true
          }
        }
      })
      if (pending.invalidated || !result) throw new CacheInvalidatedError()
      await options.validate?.()
      if (pending.invalidated) throw new CacheInvalidatedError()
      return result.value as T
    } catch (cause) {
      reportCaughtError('app/src/lib/runtime-cache.ts', cause)

      if (pending.invalidated) throw new CacheInvalidatedError()
      throw cause
    } finally {
      pending.readers--
      if (!pending.readers && this.pending.get(key) === pending) this.pending.delete(key)
    }
  }
  peek<T>(key: string): T | undefined {
    return this.cache.peek(key, { allowStale: true })?.value as T | undefined
  }
  peekFresh<T>(key: string): T | undefined {
    return this.cache.peek(key, { allowStale: false })?.value as T | undefined
  }
  set<T>(key: string, value: T) {
    this.cache.set(key, { value }, { noUpdateTTL: this.cache.has(key) })
  }
  invalidate(prefixes?: readonly string[], reason: 'refresh' | 'context' = 'refresh') {
    this.revision++
    if (reason === 'context') this.contextRevision++
    if (!prefixes) {
      for (const pending of this.pending.values()) pending.invalidated = true
      this.pending.clear()
      this.cache.clear()
      return
    }
    for (const key of new Set([...this.cache.keys(), ...this.pending.keys()]))
      if (prefixes.some((prefix) => key.startsWith(`${prefix}:`))) this.delete(key)
  }
  delete(key: string) {
    const pending = this.pending.get(key)
    if (pending) pending.invalidated = true
    this.pending.delete(key)
    this.cache.delete(key)
  }
  invalidateMatching(matches: (key: string) => boolean) {
    this.revision++
    for (const key of new Set([...this.cache.keys(), ...this.pending.keys()]))
      if (matches(key)) this.delete(key)
  }
}

export function invalidateCollectionCache(entity: { source: string; kind: string; id: string }) {
  const same = (value: unknown) => {
    const candidate = value as typeof entity | undefined
    return (
      candidate?.source === entity.source &&
      candidate.kind === entity.kind &&
      candidate.id === entity.id
    )
  }
  requestCache.invalidateMatching((key) => {
    try {
      if (key.startsWith('music_query:'))
        return same(JSON.parse(key.slice('music_query:'.length)).request?.entity)
      if (key.startsWith('presentation:'))
        return (JSON.parse(key.slice('presentation:'.length)) as unknown[]).some(same)
    } catch {
      return false
    }
    return false
  })
}

// Playlist mutations leave unrelated member pages and online queries reusable.
export function invalidateAggregateCache(id?: string) {
  requestCache.invalidateMatching((key) => {
    const separator = key.indexOf(':')
    const command = key.slice(0, separator)
    try {
      const args = JSON.parse(key.slice(separator + 1))
      if (command === 'aggregate_playlists') return true
      if (command === 'aggregate_tracks' || command === 'aggregate_playlist')
        return !id || args.id === id
      if (command === 'local_entities') return args.kind === 'playlist'
      if (command === 'local_entity_detail' || command === 'local_entity_tracks')
        return args.kind === 'playlist' && (!id || args.id === id)
      if (command === 'presentation')
        return (
          args[0] === 'aggregate-playlist-picker' ||
          (args[0] === 'local-detail' && args[1] === 'playlist' && (!id || args[2] === id))
        )
    } catch {
      return false
    }
    return false
  })
}

export const requestCache = new RuntimeCache()
export const coverCache = new RuntimeCache(48 * 1024 * 1024)
export const cachedCommands = new Set([
  'aggregate_playlists',
  'aggregate_playlist',
  'aggregate_tracks',
  'music_query',
  'music_read_track',
  'discovery_hitokoto',
  'system_fonts',
  'local_music',
  'local_entities',
  'local_entity_detail',
  'local_entity_tracks',
  'local_track_information',
  'track_lyrics'
])
export function requestKey(command: string, args?: Record<string, unknown>) {
  return `${command}:${JSON.stringify(
    Object.fromEntries(
      Object.entries(args ?? {})
        .filter(([key]) => key !== 'refresh')
        .sort(([a], [b]) => a.localeCompare(b))
    )
  )}`
}
export function invalidateNativeCache(
  commands?: readonly string[],
  reason: 'refresh' | 'context' = 'refresh'
) {
  requestCache.invalidate(commands ? [...commands, 'presentation'] : undefined, reason)
}

// Derived first-page display state shares the query cache's capacity and invalidation.
export function presentationKey(...parts: unknown[]) {
  return `presentation:${JSON.stringify(parts)}`
}
