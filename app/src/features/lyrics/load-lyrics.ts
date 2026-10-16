import { reportCaughtError } from '@/lib/diagnostics'
import { nativeCall, type Lyrics, type Track } from '@/lib/player'
import type { LyricSources } from '@/features/lyrics/use-lyric-sources'

// The native service selects sources; semantic parsing failures continue from
// the next source without retrying a source already rejected by the renderer.
export async function loadLyrics<T>(
  track: Track,
  refresh: boolean,
  sources: LyricSources,
  parse: (value: Lyrics | null) => T,
  isCurrent: () => boolean = () => true
) {
  if (track.source?.kind === 'local' && sources.localSource === 'local') {
    for (const skipEmbedded of [false, true]) {
      if (!isCurrent()) return null
      const lyrics = await nativeCall<Lyrics | null>('local_track_lyrics', {
        key: track.key,
        skipEmbedded
      })
      if (!isCurrent()) return null
      try {
        return { lyrics, parsed: parse(lyrics) }
      } catch (cause) {
        reportCaughtError('app/src/features/lyrics/load-lyrics.ts', cause)

        if (skipEmbedded) throw cause
      }
    }
    return null
  }
  let skipAmll = !sources.amll,
    skipQq = !sources.qq,
    skipQrc = false,
    skipNetease = false
  for (let attempt = 0; attempt < 5; attempt++) {
    if (!isCurrent()) return null
    const lyrics = await nativeCall<Lyrics | null>('track_lyrics', {
      key: track.key,
      refresh: attempt === 0 && refresh,
      skipAmll,
      skipQq,
      skipQrc,
      skipNetease
    })
    if (!isCurrent()) return null
    try {
      return { lyrics, parsed: parse(lyrics) }
    } catch (cause) {
      reportCaughtError('app/src/features/lyrics/load-lyrics.ts', cause)

      if (lyrics?.source === 'amll') skipAmll = true
      else if (lyrics?.source === 'qq' && lyrics.format === 'qrc' && !skipQrc) skipQrc = true
      else if (lyrics?.source === 'qq') skipQq = true
      else if (track.source.kind === 'local' && lyrics?.source === 'netease') skipNetease = true
      else throw cause
    }
  }
  return null
}
