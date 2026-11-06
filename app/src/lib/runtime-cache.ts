import { reportCaughtError } from '@/lib/diagnostics'
import { LRUCache } from 'lru-cache'

export const RUNTIME_TTL = 10 * 60 * 1000
export const RUNTIME_MAX_BYTES = 128 * 1024 * 1024
export const RUNTIME_MAX_ENTRIES = 8192
interface Entry {
  value: unknown
}
type Loader = {
  load: () => Promise<unknown>
  invalidate: () => void
  evicted: () => void
}
export type CacheInvalidationReason =
  'refresh' | 'replace' | 'clear' | 'context' | 'event' | 'access' | 'unknown'
export interface CacheDiagnosticContext {
  command?: string
  source?: string
  operation?: string
  attempt?: number
  attempts?: number
}
interface ReadOptions {
  validate?: () => Promise<void>
  refresh?: boolean
  retryInvalidated?: number
  diagnostic?: CacheDiagnosticContext
  trigger?: string
}
interface PendingRead {
  readers: number
  invalidated: boolean
  refresh: boolean
  evicted: boolean
  promise: Promise<unknown>
  invalidation: Promise<CacheInvalidatedError>
  resolveInvalidation: (reason: CacheInvalidatedError) => void
  diagnostic?: CacheDiagnosticContext
  trigger: string
  error?: CacheInvalidatedError
}

export class CacheInvalidatedError extends Error {
  readonly code = 'cacheInvalidated'
  readonly stage: CacheInvalidationReason
  readonly trigger: string
  attempt?: number
  attempts?: number
  constructor(reason: CacheInvalidationReason = 'unknown', trigger = 'unknown') {
    super('缓存请求已失效，请重试。')
    this.name = 'CacheInvalidatedError'
    this.stage = reason
    this.trigger = trigger
  }
}

// LRUCache owns capacity and TTL. RuntimeCache owns reader coalescing so an
// in-flight loader remains shared after its LRU slot is evicted.
export class RuntimeCache {
  private cache: LRUCache<string, Entry, Loader>
  private pending = new Map<string, PendingRead>()
  contextRevision = 0
  revision = 0
  constructor(
    maxBytes = RUNTIME_MAX_BYTES,
    ttl = RUNTIME_TTL,
    sizeOf: (value: unknown) => number = (value) => (JSON.stringify(value)?.length ?? 0) * 2,
    maxEntries = RUNTIME_MAX_ENTRIES
  ) {
    this.cache = new LRUCache<string, Entry, Loader>({
      max: maxEntries,
      maxSize: maxBytes,
      ttl,
      sizeCalculation: (entry, key) => Math.max(1, sizeOf(entry.value) + key.length * 2),
      ignoreFetchAbort: true,
      fetchMethod: async (_key, _stale, { context, signal }) => {
        const abort = () => {
          // Capacity eviction must not turn an already-started read into a failure.
          if ((signal.reason as Error | undefined)?.message === 'evicted') {
            context.evicted()
            return
          }
          context.invalidate()
        }
        signal.addEventListener('abort', abort, { once: true })
        try {
          return { value: await context.load() }
        } finally {
          signal.removeEventListener('abort', abort)
        }
      }
    })
  }
  private createPending(diagnostic?: CacheDiagnosticContext, trigger = 'unknown'): PendingRead {
    let resolveInvalidation!: (reason: CacheInvalidatedError) => void
    const invalidation = new Promise<CacheInvalidatedError>((resolve) => {
      resolveInvalidation = resolve
    })
    return {
      readers: 0,
      invalidated: false,
      refresh: false,
      evicted: false,
      promise: invalidation,
      invalidation,
      resolveInvalidation,
      diagnostic,
      trigger
    }
  }
  private invalidatePending(
    key: string,
    pending: PendingRead,
    reason: CacheInvalidationReason,
    trigger: string
  ) {
    if (pending.invalidated) return
    pending.invalidated = true
    pending.trigger = trigger
    pending.error = new CacheInvalidatedError(reason, trigger)
    pending.resolveInvalidation(pending.error)
    if (this.pending.get(key) === pending) this.pending.delete(key)
  }
  private reportLateError(pending: PendingRead, cause: unknown) {
    if (
      !pending.invalidated ||
      (cause &&
        typeof cause === 'object' &&
        'message' in cause &&
        ['evicted', 'deleted', 'replaced'].includes(String(cause.message)))
    )
      return
    reportCaughtError('app/src/lib/runtime-cache.ts', cause, {
      command: pending.diagnostic?.command,
      source: pending.diagnostic?.source,
      operation: pending.diagnostic?.operation,
      location: 'app/src/lib/runtime-cache.ts',
      trigger: pending.trigger
    })
  }
  private load(key: string, load: () => Promise<unknown>, pending: PendingRead): Promise<unknown> {
    const cached = this.cache.fetch(key, {
      context: {
        load,
        invalidate: () => this.invalidatePending(key, pending, 'unknown', 'lru-cache:invalidation'),
        evicted: () => {
          pending.evicted = true
        }
      }
    })
    void cached.catch((cause) => this.reportLateError(pending, cause))
    return Promise.race([
      cached,
      pending.invalidation.then((reason) => {
        throw reason
      })
    ]).then((entry) => {
      if (!entry) throw new CacheInvalidatedError('unknown', pending.trigger)
      if (pending.evicted && this.cache.peek(key) === entry) this.cache.delete(key)
      return entry.value
    })
  }
  private async fetch<T>(key: string, load: () => Promise<T>, options: ReadOptions): Promise<T> {
    const active = this.pending.get(key)
    // Concurrent explicit refreshes share the replacement; the first refresh
    // cancels an ordinary read that started before it.
    if (options.refresh && !(active?.refresh && !active.invalidated))
      this.delete(key, 'refresh', options.trigger ?? 'cache.refresh')
    const current = this.pending.get(key)
    const pending: PendingRead =
      current && !current.invalidated
        ? current
        : this.createPending(options.diagnostic, options.trigger)
    if (pending !== current) {
      pending.refresh = Boolean(options.refresh)
      this.pending.set(key, pending)
      pending.promise = this.load(key, load, pending)
    }
    pending.readers++
    try {
      const result = await pending.promise
      if (pending.invalidated) throw pending.error
      if (options.validate) {
        const validation = options.validate()
        void validation.catch((cause) => this.reportLateError(pending, cause))
        await Promise.race([
          validation,
          pending.invalidation.then((reason) => {
            throw reason
          })
        ])
      }
      if (pending.invalidated) throw new CacheInvalidatedError('unknown', pending.trigger)
      return result as T
    } catch (cause) {
      throw pending.invalidated ? (pending.error ??= new CacheInvalidatedError()) : cause
    } finally {
      pending.readers--
      if (!pending.readers && this.pending.get(key) === pending) this.pending.delete(key)
    }
  }
  async get<T>(key: string, load: () => Promise<T>, options: ReadOptions = {}): Promise<T> {
    const retries = options.retryInvalidated ?? 0
    for (let attempt = 0; ; attempt++) {
      try {
        return await this.fetch(key, load, {
          validate: options.validate,
          refresh: attempt === 0 && options.refresh,
          diagnostic: options.diagnostic,
          trigger: options.trigger
        })
      } catch (cause) {
        if (cause instanceof CacheInvalidatedError) {
          cause.attempt = options.diagnostic?.attempt ?? attempt + 1
          cause.attempts = options.diagnostic?.attempts ?? retries + 1
        }
        reportCaughtError('app/src/lib/runtime-cache.ts', cause, {
          command: options.diagnostic?.command,
          source: options.diagnostic?.source,
          operation: options.diagnostic?.operation,
          location: 'app/src/lib/runtime-cache.ts',
          trigger: cause instanceof CacheInvalidatedError ? cause.trigger : options.trigger
        })
        if (cause instanceof CacheInvalidatedError && attempt < retries) continue
        throw cause
      }
    }
  }
  peek<T>(key: string): T | undefined {
    return this.cache.peek(key, { allowStale: true })?.value as T | undefined
  }
  peekFresh<T>(key: string): T | undefined {
    return this.cache.peek(key, { allowStale: false })?.value as T | undefined
  }
  stats(matches: (key: string) => boolean = () => true) {
    let usedBytes = 0
    let entries = 0
    for (const key of this.cache.keys()) {
      if (!matches(key)) continue
      usedBytes += this.cache.info(key)?.size ?? 0
      entries++
    }
    return { usedBytes, entries }
  }
  set<T>(key: string, value: T) {
    const pending = this.pending.get(key)
    if (pending) this.invalidatePending(key, pending, 'replace', 'cache.set')
    this.cache.set(key, { value }, { noUpdateTTL: this.cache.has(key) })
  }
  invalidate(
    prefixes?: readonly string[],
    reason: CacheInvalidationReason = 'refresh',
    trigger = 'runtime-cache.invalidate'
  ) {
    this.revision++
    if (reason === 'context') this.contextRevision++
    if (!prefixes) {
      for (const [key, pending] of this.pending)
        this.invalidatePending(key, pending, reason, trigger)
      this.cache.clear()
      return
    }
    for (const key of new Set([...this.cache.keys(), ...this.pending.keys()]))
      if (prefixes.some((prefix) => key.startsWith(`${prefix}:`))) this.delete(key, reason, trigger)
  }
  delete(
    key: string,
    reason: CacheInvalidationReason = 'unknown',
    trigger = 'runtime-cache.delete'
  ) {
    const pending = this.pending.get(key)
    if (pending) this.invalidatePending(key, pending, reason, trigger)
    this.cache.delete(key)
  }
  invalidateMatching(
    matches: (key: string) => boolean,
    reason: CacheInvalidationReason = 'refresh',
    trigger = 'runtime-cache.invalidateMatching'
  ) {
    this.revision++
    for (const key of new Set([...this.cache.keys(), ...this.pending.keys()]))
      if (matches(key)) this.delete(key, reason, trigger)
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
  requestCache.invalidateMatching(
    (key) => {
      try {
        if (key.startsWith('music_query:'))
          return same(JSON.parse(key.slice('music_query:'.length)).request?.entity)
        if (key.startsWith('presentation:'))
          return (JSON.parse(key.slice('presentation:'.length)) as unknown[]).some(same)
      } catch {
        return false
      }
      return false
    },
    'event',
    'music-collection-updated'
  )
}

// Playlist mutations leave unrelated member pages and online queries reusable.
export function invalidateAggregateCache(
  id?: string,
  reason: CacheInvalidationReason = 'refresh',
  trigger = 'invalidateAggregateCache'
) {
  requestCache.invalidateMatching(
    (key) => {
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
    },
    reason,
    trigger
  )
}

export const requestCache = new RuntimeCache()
export const coverCache = new RuntimeCache(
  RUNTIME_MAX_BYTES,
  RUNTIME_TTL,
  (value) => {
    if (typeof value === 'string') return value.length * 2
    const entry = value as { access: string; data: string }
    return (entry.access.length + entry.data.length) * 2
  },
  RUNTIME_MAX_ENTRIES
)
export const isOtherQueryCacheKey = (key: string) => !key.startsWith('track_lyrics:')
export const cachedCommands = new Set([
  'extension_store_index',
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
  reason: CacheInvalidationReason = 'refresh',
  trigger = 'invalidateNativeCache'
) {
  requestCache.invalidate(commands ? [...commands, 'presentation'] : undefined, reason, trigger)
}

// Derived first-page display state shares the query cache's capacity and invalidation.
export function presentationKey(...parts: unknown[]) {
  return `presentation:${JSON.stringify(parts)}`
}
