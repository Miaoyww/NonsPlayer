import type { PlayerSnapshot, Track } from '@/lib/player'

export function failedPlayback(
  state: Pick<PlayerSnapshot, 'error' | 'status' | 'index' | 'queue' | 'revision'>
) {
  const track = state.index === null ? undefined : state.queue[state.index]
  if (!state.error || !track || !['error', 'stopped'].includes(state.status)) return null
  // The legacy playback boundary exposes display messages, not adapter error codes.
  // Keep the original reason and offer login only for an explicit authentication failure.
  const login =
    /未登录|登录.*(?:失效|过期)|(?:账号|会话|凭证).*(?:失效|过期)|需要登录|认证失败/i.test(
      state.error
    )
  return { track, revision: state.revision, message: state.error, login }
}

export function samePlayback(
  state: Pick<PlayerSnapshot, 'revision' | 'index' | 'queue'>,
  revision: number,
  track: Track
) {
  return (
    state.revision === revision &&
    state.index !== null &&
    state.queue[state.index]?.key === track.key
  )
}
