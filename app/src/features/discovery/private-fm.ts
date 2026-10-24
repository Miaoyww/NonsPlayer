import { reportCaughtError } from '@/lib/diagnostics'
import type { PlayerSnapshot } from '@/lib/player'
import { discoveryTracks } from './discovery-api'
import { onlineReference } from '@/features/music/references'

// Lives with the global player connection so navigation cannot stop FM.
export function createPrivateFm(
  getPlayer: () => PlayerSnapshot,
  nativeCall: <T>(command: string, args?: Record<string, unknown>) => Promise<T>,
  reportError: (error?: string) => void,
  loadTracks = discoveryTracks
) {
  let pending: { session: number } | undefined
  let attempt: string | undefined
  function update() {
    const state = getPlayer()
    const session = state.privateFmSession
    if (session == null) {
      pending = undefined
      attempt = undefined
      return
    }
    if (pending && pending.session !== session) pending = undefined
    if (state.index === null || !['loading', 'playing', 'buffering'].includes(state.status)) return
    const position = state.shuffle ? state.shuffleOrder.indexOf(state.index) : state.index
    if (position < 0 || position < state.queue.length - 2) return
    const tail = state.queue[state.queue.length - 1]?.key
    const nextAttempt = `${session}:${tail}:${state.index}`
    if (pending || attempt === nextAttempt) return
    const request = { session }
    pending = request
    attempt = nextAttempt
    reportError()
    const queueLen = state.queue.length
    const source = state.queue[0] ? onlineReference(state.queue[0])?.source : undefined
    void (
      source ? loadTracks('fm', true, source) : Promise.reject(new Error('FM 队列缺少来源引用'))
    )
      .then(async (tracks) => {
        if (pending !== request || getPlayer().privateFmSession !== session) return
        const seen = new Set(getPlayer().queue.map((track) => track.key))
        const keys: string[] = []
        for (const track of tracks) {
          if (!seen.has(track.key)) {
            seen.add(track.key)
            keys.push(track.key)
          }
        }
        if (!keys.length) throw new Error('暂时没有新的 FM 歌曲，请重试。')
        await nativeCall('append_private_fm', { session, queueLen, keys })
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/discovery/private-fm.ts', cause)

        if (pending !== request || getPlayer().privateFmSession !== session) return
        reportError(cause instanceof Error ? cause.message : String(cause))
      })
      .finally(() => {
        if (pending !== request) return
        pending = undefined
        update()
      })
  }
  return {
    update,
    retry() {
      attempt = undefined
      update()
    },
    dispose() {
      pending = undefined
      attempt = undefined
    }
  }
}
