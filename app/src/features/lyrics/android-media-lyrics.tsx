import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { nativeCall, usePlayer } from '@/lib/player'
import { loadLyrics } from './load-lyrics'
import { parseLyrics } from './parse-lyrics'
import { useLyricSources } from './use-lyric-sources'
import { mediaLyricLines } from './media-lyric-lines'

// Fetch/parse through the existing lyric pipeline once; native code advances lines in the background.
// This component renders nothing and has no animation or progress subscription.
export function AndroidMediaLyrics() {
  const state = usePlayer('index', 'queue')
  const track = state.index === null ? undefined : state.queue[state.index]
  const { sources, ready } = useLyricSources()
  const [revision, setRevision] = useState(0)
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined
    void listen<{ key: string }>('lyrics-updated', ({ payload }) => {
      if (!disposed && payload.key === track?.key) setRevision((value) => value + 1)
    }).then((cleanup) => {
      if (disposed) cleanup()
      else unlisten = cleanup
    })
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [track?.key])
  useEffect(() => {
    if (!track || !ready) return
    let disposed = false
    void nativeCall('android_media_lyrics', { key: track.key, lines: [] }).catch((cause) => {
      reportCaughtError('app/src/features/lyrics/android-media-lyrics.tsx', cause)
    })
    void loadLyrics(
      track,
      false,
      sources,
      (value) => (value ? mediaLyricLines(parseLyrics(value, track.durationMs)) : []),
      () => !disposed
    )
      .then((value) => {
        if (!disposed && value)
          return nativeCall('android_media_lyrics', { key: track.key, lines: value.parsed })
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/lyrics/android-media-lyrics.tsx', cause)

        // Missing/invalid lyrics do not interrupt playback; the normal lyric page exposes retry.
      })
    return () => {
      disposed = true
    }
  }, [track, sources, ready, revision])
  return null
}
