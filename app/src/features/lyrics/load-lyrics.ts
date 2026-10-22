import { reportCaughtError } from '@/lib/diagnostics'
import { nativeCall, type Lyrics, type Track } from '@/lib/player'
import type { LyricSources } from '@/features/lyrics/use-lyric-sources'

// Stable across DTO property ordering; excludes artwork and the selected audio provider.
export function lyricTrackVersion(track?: Track) {
  if (!track) return ''
  const source = track.source
  const online = source?.kind === 'online' ? source.track : undefined
  return JSON.stringify([
    track.key,
    track.title,
    track.aliases ?? [],
    track.artist,
    track.album,
    track.durationMs,
    source?.kind,
    source?.kind === 'local' ? source.cue : null,
    source?.kind === 'local' ? source.neteaseId : source?.kind === 'netease' ? source.id : null,
    online ? [online.reference.source, online.reference.kind, online.reference.id] : null,
    online?.lyricIdentifiers?.map(({ kind, id }) => [kind, id]).sort() ?? [],
    online?.associations?.map(({ source, kind, id }) => [source, kind, id]).sort() ?? []
  ])
}

// The native service selects sources; semantic parsing failures continue from
// the next source without retrying a source already rejected by the renderer.
export async function loadLyrics<T>(
  track: Track,
  refresh: boolean,
  sources: LyricSources,
  parse: (value: Lyrics | null) => T,
  isCurrent: () => boolean = () => true,
  manual = false
) {
  if (!manual && track.source?.kind === 'local' && sources.localSource === 'local') {
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
      // Cache discriminator; the native service still reads its authoritative stored track.
      trackVersion: lyricTrackVersion(track),
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

      if (manual && !(lyrics?.source === 'qq' && lyrics.format === 'qrc' && !skipQrc)) throw cause
      if (lyrics?.source === 'amll') skipAmll = true
      else if (lyrics?.source === 'qq' && lyrics.format === 'qrc' && !skipQrc) skipQrc = true
      else if (lyrics?.source === 'qq') skipQq = true
      else if (track.source.kind === 'local' && lyrics?.source === 'netease') skipNetease = true
      else throw cause
    }
  }
  return null
}
