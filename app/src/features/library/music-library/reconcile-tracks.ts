import type { Track } from '@/lib/player'

// Preserve unchanged track objects, including repeated occurrences of a song.
// Native track DTOs have a consistent JSON field order.
export function reconcileTracks(previous: Track[], next: Track[]): Track[] {
  const byKey = new Map<string, { tracks: Track[]; index: number }>()
  for (const track of previous) {
    const bucket = byKey.get(track.key)
    if (bucket) bucket.tracks.push(track)
    else byKey.set(track.key, { tracks: [track], index: 0 })
  }
  const result = next.map((track) => {
    const bucket = byKey.get(track.key)
    const old = bucket?.tracks[bucket.index++]
    return old && JSON.stringify(old) === JSON.stringify(track) ? old : track
  })
  return result.length === previous.length &&
    result.every((track, index) => track === previous[index])
    ? previous
    : result
}
