import type { PlayerSnapshot, Progress, Track } from '@/lib/player'

export type PlayerStateEvent = Omit<
  PlayerSnapshot,
  'queue' | 'shuffleOrder' | 'positionMs' | 'durationMs' | 'privateFmError'
> & { sequence: number; queueRevision: number }
export interface PlayerQueueEvent {
  sequence: number
  queueRevision: number
  queue: Track[]
  shuffleOrder: number[]
}
export type PlayerProgressEvent = Omit<Progress, 'receivedAt'> & { sequence: number }
export interface PlayerBootstrap {
  state: PlayerStateEvent
  queue: PlayerQueueEvent
  progress: PlayerProgressEvent
}

// Retain at most one event per channel during bootstrap or cross-channel delivery.
// A state is exposed only with its matching queue version. Progress has its own
// listeners; the legacy imperative snapshot retains the latest sampled position.
export function createPlayerStore(initial: PlayerSnapshot, now = () => performance.now()) {
  let snapshot = initial
  let progress: Progress = {
    revision: initial.revision,
    positionMs: initial.positionMs,
    durationMs: initial.durationMs,
    status: initial.status,
    receivedAt: 0
  }
  let initialized = false
  let state: PlayerStateEvent | undefined
  let queue: PlayerQueueEvent | undefined
  let pendingProgress: PlayerProgressEvent | undefined
  let stateSequence = -1
  let progressSequence = -1
  const stateListeners = new Set<() => void>()
  const progressListeners = new Set<() => void>()
  const subscribe = (listeners: Set<() => void>) => (listener: () => void) => {
    listeners.add(listener)
    return () => {
      listeners.delete(listener)
    }
  }
  function applyProgress() {
    if (!initialized || !pendingProgress || pendingProgress.revision !== snapshot.revision) return
    if (pendingProgress.sequence <= progressSequence) return
    const { sequence, ...value } = pendingProgress
    progressSequence = sequence
    progress = { ...value, receivedAt: now() }
    snapshot = { ...snapshot, positionMs: value.positionMs, durationMs: value.durationMs }
    progressListeners.forEach((notify) => notify())
  }
  function reconcile() {
    if (!initialized || !state || !queue || state.queueRevision !== queue.queueRevision) return
    if (state.sequence > stateSequence) {
      const { sequence, queueRevision: _queueRevision, ...value } = state
      stateSequence = sequence
      snapshot = {
        ...snapshot,
        ...value,
        playbackOptions:
          value.playbackOptions.fades === snapshot.playbackOptions.fades &&
          value.playbackOptions.gapless === snapshot.playbackOptions.gapless
            ? snapshot.playbackOptions
            : value.playbackOptions,
        queue: queue.queue,
        shuffleOrder: queue.shuffleOrder,
        privateFmError:
          value.privateFmSession === snapshot.privateFmSession ? snapshot.privateFmError : undefined
      }
      stateListeners.forEach((notify) => notify())
    }
    applyProgress()
  }
  return {
    getSnapshot: () => snapshot,
    getProgress: () => progress,
    subscribeState: subscribe(stateListeners),
    subscribeProgress: subscribe(progressListeners),
    get needsSnapshot() {
      return initialized && !!state && !!queue && state.queueRevision > queue.queueRevision
    },
    state(value: PlayerStateEvent) {
      if (value.sequence <= (state?.sequence ?? -1)) return
      state = value
      reconcile()
    },
    queue(value: PlayerQueueEvent) {
      if (value.queueRevision <= (queue?.queueRevision ?? -1)) return
      queue = value
      reconcile()
    },
    progress(value: PlayerProgressEvent) {
      if (value.sequence <= (pendingProgress?.sequence ?? -1)) return
      pendingProgress = value
      applyProgress()
    },
    bootstrap(value: PlayerBootstrap) {
      if (value.state.sequence < stateSequence) return
      // Merge the baseline with events received while IPC was in flight.
      if (!queue || value.queue.queueRevision > queue.queueRevision) queue = value.queue
      if (!state || value.state.sequence >= state.sequence) state = value.state
      if (!pendingProgress || value.progress.sequence > pendingProgress.sequence)
        pendingProgress = value.progress
      initialized = true
      // The baseline may predate a buffered replacement queue. Never expose
      // the baseline's index against that queue; wait for its matching state.
      reconcile()
    },
    privateFmError(error: string | undefined) {
      if (snapshot.privateFmError === error) return
      snapshot = { ...snapshot, privateFmError: error }
      stateListeners.forEach((notify) => notify())
    }
  }
}

export function selectPlayerFields<K extends keyof PlayerSnapshot>(
  getSnapshot: () => PlayerSnapshot,
  fields: readonly K[]
) {
  let selected: Pick<PlayerSnapshot, K> | undefined
  return () => {
    const value = getSnapshot()
    if (!selected || fields.some((field) => !Object.is(selected![field], value[field]))) {
      selected = Object.fromEntries(fields.map((field) => [field, value[field]])) as Pick<
        PlayerSnapshot,
        K
      >
    }
    return selected
  }
}
