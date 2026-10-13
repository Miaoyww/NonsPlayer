import { CacheInvalidatedError, requestKey, type RuntimeCache } from '@/lib/runtime-cache'

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>
const contextChanged = () => new Error('账号或音乐来源状态已变化，请重试')
class WriteRevisionChangedError extends Error {}

// music_context serializes [SessionContext, adapter instance generation, write revision].
// Unknown formats stay opaque: they are never treated as a safe write-only change.
function sameIdentity(before: string, after: string) {
  if (before === after) return true
  try {
    const left = JSON.parse(before),
      right = JSON.parse(after)
    return (
      Array.isArray(left) &&
      Array.isArray(right) &&
      left.length === 3 &&
      right.length === 3 &&
      JSON.stringify(left.slice(0, 2)) === JSON.stringify(right.slice(0, 2))
    )
  } catch {
    return false
  }
}

export class ScopedMusicQueries {
  private scopes = new Map<string, string>()
  constructor(
    private cache: RuntimeCache,
    private invoke: Invoke
  ) {}

  peek<T>(source: string, command: string, args?: Record<string, unknown>): T | undefined {
    const scope = this.scopes.get(source)
    return scope === undefined
      ? undefined
      : this.cache.peek<T>(requestKey(command, { ...args, scope }))
  }

  async read<T>(source: string, command: string, args?: Record<string, unknown>): Promise<T> {
    const revision = this.cache.contextRevision
    let scope = await this.invoke<string>('music_context', { source })
    const initial = scope
    const check = (current = scope) => {
      if (revision !== this.cache.contextRevision || !sameIdentity(initial, current))
        throw contextChanged()
    }
    const request = args?.request as { page?: { cursor?: unknown } } | undefined
    const paginated = Boolean(request?.page?.cursor) || Number(args?.offset ?? 0) > 0
    const cached = command !== 'music_queue_track' && command !== 'music_queue_tracks'
    for (let attempt = 0; attempt < 3; attempt++) {
      check()
      if ((args?.scope !== undefined && args.scope !== scope) || (paginated && initial !== scope))
        throw contextChanged()
      const key = requestKey(command, { ...args, scope })
      try {
        const load = async () => {
          const result = await this.invoke<T>(command, { ...args, source, scope })
          if (command === 'music_library' && (result as { likedError?: string })?.likedError)
            throw new Error((result as { likedError: string }).likedError)
          return result
        }
        const validate = async () => {
          const next = await this.invoke<string>('music_context', { source })
          check(next)
          if (scope !== next) throw new WriteRevisionChangedError()
        }
        const value = cached
          ? await this.cache.get<T>(key, load, {
              validate,
              refresh: attempt === 0 && Boolean(args?.refresh)
            })
          : await load()
        if (!cached) await validate()
        this.scopes.set(source, scope)
        return value
      } catch (cause) {
        if (
          !(cause instanceof CacheInvalidatedError) &&
          !(cause instanceof WriteRevisionChangedError) &&
          !(
            typeof cause === 'object' &&
            cause !== null &&
            'code' in cause &&
            cause.code === 'staleContext'
          )
        )
          throw cause
      }
      const next = await this.invoke<string>('music_context', { source })
      check(next)
      // Queue mutations must never be replayed. Old pages belong to their original scope.
      if (!cached) throw contextChanged()
      scope = next
    }
    throw new Error('音乐数据频繁变化，请稍后重试')
  }
}
