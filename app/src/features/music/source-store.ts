import type { SourcePreferences } from './source-preferences'
import type { SourceDescriptor } from './types'
export interface SourceStatus {
  descriptor: SourceDescriptor
  enabled: boolean
}
export interface SourceSnapshot {
  preferences?: SourcePreferences
  statuses: SourceStatus[]
  loading: boolean
  revision: number
  error: string
}
const events = [
  'music-source-preferences-changed',
  'adapters-changed',
  'music-account-changed',
  'music-changed',
  'adapter-load-failed'
]
export function createSourceStore(
  read: () => Promise<[SourcePreferences | undefined, SourceStatus[]]>,
  watch: (event: string, changed: (payload?: unknown) => void) => Promise<() => void>,
  errorText: (cause: unknown) => string
) {
  let value: SourceSnapshot = { statuses: [], loading: true, revision: 0, error: '' }
  const subscribers = new Set<() => void>()
  let cycle = 0,
    serial = 0,
    pending: Promise<void> | undefined,
    dirty = false
  let stops: (() => void)[] = []
  const publish = (next: SourceSnapshot) => {
    value = next
    subscribers.forEach((notify) => notify())
  }
  const refresh = (): Promise<void> => {
    if (!subscribers.size) return Promise.resolve()
    if (pending) {
      dirty = true
      serial++
      return pending
    }
    const active = cycle
    const task = (async () => {
      do {
        dirty = false
        const request = ++serial
        try {
          const [preferences, statuses] = await read()
          if (active !== cycle) return
          if (request === serial)
            publish({
              preferences,
              statuses,
              loading: false,
              error: '',
              revision: value.revision + 1
            })
        } catch (cause) {
          if (active === cycle && request === serial)
            publish({ ...value, loading: false, error: errorText(cause) })
        }
      } while (dirty && active === cycle && subscribers.size)
    })()
    pending = task
    void task.finally(() => {
      if (pending === task) pending = undefined
    })
    return task
  }
  const subscribe = (notify: () => void) => {
    subscribers.add(notify)
    if (subscribers.size === 1) {
      const active = ++cycle
      publish({ ...value, loading: true })
      // Subscribe before reading, but do not serialize independent IPC registrations.
      void Promise.all(
        events.map(async (event) => {
          const stop = await watch(event, (payload) => {
            if (event === 'adapter-load-failed') {
              if (active === cycle)
                publish({
                  ...value,
                  loading: false,
                  error: typeof payload === 'string' ? payload : '适配器加载失败'
                })
              return
            }
            void refresh()
          })
          if (active !== cycle) stop()
          else stops.push(stop)
        })
      )
        .then(() => {
          if (active === cycle) return refresh()
        })
        .catch((cause) => {
          if (active === cycle) publish({ ...value, loading: false, error: errorText(cause) })
        })
    }
    return () => {
      subscribers.delete(notify)
      if (!subscribers.size) {
        cycle++
        serial++
        dirty = false
        pending = undefined
        stops.splice(0).forEach((stop) => stop())
      }
    }
  }
  return { subscribe, getSnapshot: () => value, refresh }
}
