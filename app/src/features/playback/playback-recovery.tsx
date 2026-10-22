import { useEffect, useRef, useState } from 'react'
import { errorText, getPlayer, nativeCall, usePlayer, type Track } from '@/lib/player'
import { errorNotices, type NoticeAction } from './error-notices'
import { failedPlayback, samePlayback } from './recovery'
import { SongSourceDialog } from './song-source-dialog'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { trackSource } from '@/features/music/source-label'
import { PlaybackNotice } from './playback-notice'

/** One producer for playback failures; existing notice slots render the actions. */
export function PlaybackRecovery({
  onAccount,
  fallback = false
}: {
  onAccount: () => void
  fallback?: boolean
}) {
  const state = usePlayer('revision', 'index', 'queue', 'status', 'error')
  const { navigate } = useMusicNavigation()
  const navigation = useRef({ navigate, onAccount })
  navigation.current = { navigate, onAccount }
  const [sourceTrack, setSourceTrack] = useState<Track>()
  useEffect(() => {
    errorNotices.clearGroup('playback-recovery')
    const failed = failedPlayback(state)
    if (!failed) return
    const { track, revision, message, login } = failed
    const guard = (run: () => void | Promise<void>) => () => {
      if (!samePlayback(getPlayer(), revision, track)) return
      return run()
    }
    const actions: NoticeAction[] = [
      {
        label: '重试',
        run: guard(async () => {
          try {
            await nativeCall('player_retry', { revision, key: track.key })
          } catch (cause) {
            errorNotices.push(errorText(cause))
          }
        })
      }
    ]
    if (track.source.kind !== 'local')
      actions.push({ label: '更换音源', run: guard(() => setSourceTrack(track)) })
    if (login && track.source.kind !== 'local')
      actions.push({
        label: '重新登录',
        run: guard(() => {
          navigation.current.onAccount()
          navigation.current.navigate('accounts', trackSource(track))
        })
      })
    errorNotices.push(`「${track.title}」播放失败：${message}`, actions, 'playback-recovery')
    return () => errorNotices.clearGroup('playback-recovery')
  }, [state.revision, state.index, state.queue, state.status, state.error])
  return (
    <>
      {fallback && (
        <div className="pointer-events-none fixed inset-x-0 bottom-6 z-[80] flex justify-center">
          <div className="relative w-full max-w-sm">
            <PlaybackNotice />
          </div>
        </div>
      )}
      {sourceTrack ? (
        <SongSourceDialog
          key={sourceTrack.key}
          track={sourceTrack}
          onClose={() => setSourceTrack(undefined)}
        />
      ) : null}
    </>
  )
}
