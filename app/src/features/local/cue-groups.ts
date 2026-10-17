import type { Track } from '@/lib/player'

export function cueSheet(track: Track): string | undefined {
  return track.source.kind === 'local' ? track.source.cue?.sheetPath : undefined
}

// Keep each sheet together in track order without combining different rips of the same album.
export function cueGroups<T extends { track: Track }>(entries: T[]): T[][] {
  const sheets = new Map<string, T[]>()
  const groups: T[][] = []
  for (const entry of entries) {
    const sheet = cueSheet(entry.track)
    if (!sheet) {
      groups.push([entry])
      continue
    }
    let group = sheets.get(sheet)
    if (!group) {
      group = []
      sheets.set(sheet, group)
      groups.push(group)
    }
    group.push(entry)
  }
  for (const group of sheets.values()) {
    group.sort((a, b) => {
      const left = a.track.source
      const right = b.track.source
      return left.kind === 'local' && right.kind === 'local'
        ? (left.cue?.trackNumber ?? 0) - (right.cue?.trackNumber ?? 0)
        : 0
    })
  }
  return groups
}
