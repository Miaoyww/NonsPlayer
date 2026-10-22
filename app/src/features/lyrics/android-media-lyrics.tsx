import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { nativeCall, usePlayer } from '@/lib/player'
import { loadLyrics } from './load-lyrics'
import { parseLyrics } from './parse-lyrics'
import { useLyricSources } from './use-lyric-sources'
import { mediaLyricLines } from './media-lyric-lines'
import { offsetLyrics, type LyricCorrection } from './correction'

// Fetch/parse through the existing lyric pipeline once; native code advances lines in the background.
// This component renders nothing and has no animation or progress subscription.
export function AndroidMediaLyrics() {
  const state = usePlayer('index', 'queue')
  const track = state.index === null ? undefined : state.queue[state.index]
  const { sources, ready } = useLyricSources()
  const [revision, setRevision] = useState(0)
  useEffect(() => {
    let disposed = false
    const cleanups: (() => void)[] = []
    const bind = async () => {
      for (const event of ['lyrics-updated', 'lyric-correction-changed']) {
        const cleanup = await listen<{ key: string } | string>(event, ({ payload }) => {
          if (!disposed && (typeof payload === 'string' ? payload : payload.key) === track?.key)
            setRevision((value) => value + 1)
        })
        if (disposed) cleanup()
        else cleanups.push(cleanup)
      }
    }
    void bind().catch((cause) =>
      reportCaughtError('app/src/features/lyrics/android-media-lyrics.tsx', cause)
    )
    return () => {
      disposed = true
      cleanups.forEach((cleanup) => cleanup())
    }
  }, [track?.key])
  useEffect(() => {
    if (!track || !ready) return
    let disposed = false
    void nativeCall('android_media_lyrics', { key: track.key, lines: [] }).catch((cause) => {
      reportCaughtError('app/src/features/lyrics/android-media-lyrics.tsx', cause)
    })
    void nativeCall<LyricCorrection>('lyric_correction', { key: track.key })
      .then((correction) =>
        disposed
          ? null
          : loadLyrics(
              track,
              false,
              sources,
              (value) =>
                value
                  ? mediaLyricLines(
                      offsetLyrics(
                        parseLyrics(
                          value,
                          track.durationMs,
                          track.source.kind === 'local' ? track.source.cue : undefined
                        ),
                        correction.offsetMs
                      )
                    )
                  : [],
              () => !disposed,
              !!correction.candidate
            )
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
