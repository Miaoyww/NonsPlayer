import { listen } from '@tauri-apps/api/event'
import { coverCache, requestKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'

type Registration = { ready: Promise<void>; stop?: () => void }
const subscribers = new Map<string, Set<() => void>>()
let registration: Registration | undefined

export function subscribeOnlineCover(url: string, changed: () => void) {
  let callbacks = subscribers.get(url)
  if (!callbacks) subscribers.set(url, (callbacks = new Set()))
  callbacks.add(changed)
  if (!registration) {
    const active: Registration = { ready: Promise.resolve() }
    registration = active
    active.ready = listen<{ url: string }>('runtime-cover-updated', ({ payload }) => {
      if (registration !== active || typeof payload?.url !== 'string') return
      coverCache.delete(requestKey('runtime_cover', { url: payload.url }))
      for (const callback of [...(subscribers.get(payload.url) ?? [])]) callback()
    })
      .then((stop) => {
        if (registration !== active) stop()
        else active.stop = stop
      })
      .catch((cause) => {
        reportCaughtError('music.online-cover-updates.listen', cause)
        if (registration === active) registration = undefined
      })
  }
  const active = registration
  return {
    ready: active.ready,
    stop() {
      callbacks.delete(changed)
      if (!callbacks.size && subscribers.get(url) === callbacks) subscribers.delete(url)
      if (!subscribers.size && registration === active) {
        registration = undefined
        active.stop?.()
      }
    }
  }
}
