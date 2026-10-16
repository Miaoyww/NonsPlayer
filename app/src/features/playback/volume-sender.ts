import { reportCaughtError } from '@/lib/diagnostics'
// One in-flight command and one latest pending value, shared by every volume input.
// Keeping the optimistic value until its state acknowledgement prevents old echoes
// from pulling the sliders backwards during a drag.
export function createVolumeSender(
  send: (volume: number) => Promise<unknown>,
  readVolume: () => number,
  clock = {
    now: () => performance.now(),
    setTimeout: (callback: () => void, delay: number) => globalThis.setTimeout(callback, delay),
    clearTimeout: (timer: ReturnType<typeof globalThis.setTimeout>) =>
      globalThis.clearTimeout(timer)
  },
  interval = 40
) {
  type Request = { volume: number; serial: number; onError: (error: unknown) => void }
  let serial = 0
  let optimistic: number | null = null
  let pending: Request | undefined
  let inFlight = false
  let lastSent = -Infinity
  let urgent = false
  let timer: ReturnType<typeof globalThis.setTimeout> | undefined
  const listeners = new Set<() => void>()
  function notify() {
    listeners.forEach((listener) => listener())
  }
  function cancelTimer() {
    if (timer !== undefined) clock.clearTimeout(timer)
    timer = undefined
  }
  function acknowledge(volume: number) {
    if (!pending && !inFlight && optimistic === volume) {
      optimistic = null
      notify()
    }
  }
  function drain() {
    if (inFlight || !pending) return
    cancelTimer()
    const wait = interval - (clock.now() - lastSent)
    if (!urgent && wait > 0) {
      timer = clock.setTimeout(() => {
        timer = undefined
        drain()
      }, wait)
      return
    }
    const request = pending
    pending = undefined
    urgent = false
    inFlight = true
    lastSent = clock.now()
    void Promise.resolve()
      .then(() => send(request.volume))
      .catch((error) => {
        reportCaughtError('app/src/features/playback/volume-sender.ts', error)

        if (request.serial === serial) {
          optimistic = null
          notify()
          request.onError(error)
        }
      })
      .finally(() => {
        inFlight = false
        drain()
        acknowledge(readVolume())
      })
  }
  return {
    getSnapshot: () => optimistic,
    subscribe(listener: () => void) {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    acknowledge,
    change(volume: number, onError: (error: unknown) => void, immediate = false) {
      if (!Number.isFinite(volume)) return
      volume = Math.max(0, Math.min(1, volume))
      if (volume === (optimistic ?? readVolume())) {
        if (immediate) this.flush()
        return
      }
      optimistic = volume
      pending = { volume, serial: ++serial, onError }
      urgent ||= immediate
      notify()
      drain()
    },
    flush() {
      urgent = true
      cancelTimer()
      drain()
    }
  }
}
