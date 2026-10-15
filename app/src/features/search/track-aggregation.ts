import { distance } from 'fastest-levenshtein'
import type { Track } from '@/lib/player'

export const aggregationRules = {
  titleWeight: 45,
  artistWeight: 30,
  albumWeight: 10,
  durationWeight: 15,
  threshold: 90,
  titleThreshold: 0.95,
  shortTitleLength: 6,
  fullDurationMs: 2000,
  maxDurationMs: 5000
} as const

export interface SearchTrackMember {
  source: string
  track: Track
}

export interface SearchTrackGroup {
  id: string
  members: SearchTrackMember[]
  representative: Track
  sourceCount: number
}

interface Metadata {
  title: string
  album: string
  artists: string
  version: string
  qualifiers: string
  duration: number
  valid: boolean
}

const versionKinds = [
  ['live', /\blive\b|现场(?:版)?|現場(?:版)?/gu],
  ['studio', /\bstudio(?:\s+version)?\b|录音室(?:版)?|錄音室(?:版)?/gu],
  ['remix', /\bremix(?:ed)?\b|混音(?:版)?/gu],
  ['instrumental', /\binstrumental\b|\bkaraoke\b|伴奏(?:版)?|纯音乐(?:版)?|純音樂(?:版)?/gu],
  ['acoustic', /\bacoustic\b|不插电(?:版)?|不插電(?:版)?/gu],
  ['demo', /\bdemo\b|小样|小樣/gu],
  ['remaster', /\bremaster(?:ed)?\b|重制(?:版)?|重製(?:版)?/gu],
  ['rerecorded', /\bre[ -]?record(?:ed|ing)?\b|重新录制|重新錄製|重录(?:版)?|重錄(?:版)?/gu],
  ['clean', /\bclean\b|洁净版|潔淨版/gu],
  ['explicit', /\bexplicit\b/gu],
  ['sped', /\bsped[ -]?up\b|\bspeed[ -]?up\b|加速(?:版)?/gu],
  ['slowed', /\bslowed(?:[ -]?down)?\b|减速(?:版)?|慢速(?:版)?/gu],
  ['edit', /\bedit\b|剪辑(?:版)?|剪輯(?:版)?/gu],
  ['extended', /\bextended(?:\s+(?:mix|version))?\b|延长(?:版)?|延長(?:版)?/gu]
] as const

export function normalizeMetadata(value: string) {
  return value
    .normalize('NFKC')
    .toLowerCase()
    .replace(/[‘’ʼ]/gu, "'")
    .replace(/[“”]/gu, '"')
    .replace(/[‐‑‒–—−]/gu, '-')
    .replace(/[【\[｛{]/gu, '(')
    .replace(/[】\]｝}]/gu, ')')
    .replace(/\s*([(),:])\s*/gu, '$1')
    .replace(/\s+/gu, ' ')
    .trim()
}

function versionPart(value: string): { kinds: string[]; detail: string } | null {
  const kinds: string[] = []
  let detail = value
  for (const [kind, pattern] of versionKinds) {
    const stripped = detail.replace(pattern, '')
    if (stripped !== detail) kinds.push(kind)
    detail = stripped
  }
  if (!kinds.length) return null
  detail = normalizeMetadata(detail.replace(/\b(?:version|ver)\b|版本|版/gu, ''))
    .replace(/^[\s.,:()\-]+|[\s.,:()\-]+$/gu, '')
    .trim()
  return { kinds, detail }
}

function recordingMetadata(track: Track): Metadata {
  const versions = new Map<string, Set<string>>()
  const qualifiers: string[] = []
  const addVersion = (part: NonNullable<ReturnType<typeof versionPart>>) => {
    for (const kind of part.kinds) {
      if (kind === 'studio' && !part.detail) continue
      const details = versions.get(kind) ?? new Set<string>()
      if (part.detail) details.add(part.detail)
      versions.set(kind, details)
    }
  }
  const extract = (text: string, album = false) => {
    let result = normalizeMetadata(text).replace(/\([^()]*\)/gu, (segment) => {
      const part = versionPart(segment.slice(1, -1))
      if (!part) {
        if (!album) qualifiers.push(segment)
        return segment
      }
      addVersion(part)
      return ' '
    })
    const suffix = result.match(/^(.*?)(?:\s+-\s+|:)([^:]+)$/u)
    if (suffix) {
      const part = versionPart(suffix[2])
      if (part) {
        addVersion(part)
        result = suffix[1]
      }
    }
    const trailing = result.match(
      /^(.*?)\s+((?:live|studio|remix|instrumental|karaoke|acoustic|demo|remaster(?:ed)?|re[ -]?recorded|clean|explicit|sped[ -]?up|slowed|radio edit|extended)\b.*)$/u
    )
    const chinese = result.match(
      /^(.*?)(现场版|現場版|伴奏版|不插电版|不插電版|重制版|重製版|重录版|重錄版|加速版|减速版|剪辑版|延长版)$/u
    )
    if (trailing || chinese) {
      const match = (trailing ?? chinese)!
      const part = versionPart(match[2])!
      addVersion(part)
      result = match[1]
    } else if (album && /^(?:live\b|现场|現場)/u.test(result)) {
      addVersion(versionPart(result)!)
    }
    return normalizeMetadata(result)
  }
  const title = extract(track.title)
  const album = extract(track.album, true)
  const credits = track.artists?.length
    ? track.artists.map((artist) => artist.name)
    : [track.artist]
  const names = credits.map(normalizeMetadata)
  const artists = JSON.stringify([...new Set(names)].sort())
  return {
    title,
    album,
    artists,
    version: JSON.stringify(
      [...versions]
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([kind, details]) => [kind, [...details].sort()])
    ),
    qualifiers: JSON.stringify(qualifiers.sort()),
    duration: track.durationMs,
    valid:
      !!title &&
      names.length > 0 &&
      names.every(Boolean) &&
      Number.isFinite(track.durationMs) &&
      track.durationMs > 0
  }
}

function similarity(left: string, right: string) {
  if (!left || !right) return 0
  return 1 - distance(left, right) / Math.max(left.length, right.length)
}

function scoreMetadata(left: Metadata, right: Metadata): number | null {
  const rules = aggregationRules
  if (
    !left.valid ||
    !right.valid ||
    left.artists !== right.artists ||
    left.version !== right.version ||
    left.qualifiers !== right.qualifiers
  )
    return null
  if (left.version !== '[]' && left.title !== right.title) return null
  const delta = Math.abs(left.duration - right.duration)
  if (delta > rules.maxDurationMs) return null
  const title = similarity(left.title, right.title)
  if (
    Math.min([...left.title].length, [...right.title].length) <= rules.shortTitleLength &&
    left.title !== right.title
  )
    return null
  if (title < rules.titleThreshold) return null
  const duration =
    delta <= rules.fullDurationMs
      ? 1
      : (rules.maxDurationMs - delta) / (rules.maxDurationMs - rules.fullDurationMs)
  const score =
    title * rules.titleWeight +
    rules.artistWeight +
    similarity(left.album, right.album) * rules.albumWeight +
    duration * rules.durationWeight
  return score >= rules.threshold ? score : null
}

export function trackMatchScore(left: Track, right: Track) {
  return scoreMetadata(recordingMetadata(left), recordingMetadata(right))
}

export class TrackAggregation {
  private groups: SearchTrackGroup[] = []
  private metadata: Metadata[][] = []
  private seen = new Set<string>()
  private identityGroups = new Map<string, number>()
  private buckets = new Map<string, Map<number, Set<number>>>()
  private priority: Map<string, number>

  constructor(sources: string[]) {
    this.priority = new Map(sources.map((source, index) => [source, index]))
  }

  getSnapshot = () => this.groups

  append(batch: { source: string; item: Track }[]) {
    const next = [...this.groups]
    for (const { source, item: track } of batch) {
      const id = JSON.stringify([source, track.key])
      if (this.seen.has(id)) {
        const index = this.identityGroups.get(id)!
        const group = next[index]
        const previous = group.members.find(
          (member) => member.source === source && member.track.key === track.key
        )
        // A provider can deliver metadata without duration before the catalog.
        // The same real identity is one song, not an extra platform or recording.
        if (previous && previous.track.durationMs === 0 && track.durationMs > 0) {
          const members = group.members.map((member) =>
            member === previous ? { source, track } : member
          )
          next[index] = { ...group, members, representative: members[0].track }
          this.metadata[index] = members.map((member) => recordingMetadata(member.track))
          const metadata = recordingMetadata(track)
          if (metadata.valid) {
            const buckets = this.buckets.get(metadata.artists) ?? new Map<number, Set<number>>()
            const bucket = Math.floor(metadata.duration / aggregationRules.maxDurationMs)
            const indices = buckets.get(bucket) ?? new Set<number>()
            indices.add(index)
            buckets.set(bucket, indices)
            this.buckets.set(metadata.artists, buckets)
          }
        }
        continue
      }
      this.seen.add(id)
      const metadata = recordingMetadata(track)
      const bucket = Math.floor(metadata.duration / aggregationRules.maxDurationMs)
      const artistBuckets = metadata.valid ? this.buckets.get(metadata.artists) : undefined
      const candidates = new Set<number>()
      for (let offset = -1; offset <= 1; offset++)
        artistBuckets?.get(bucket + offset)?.forEach((index) => candidates.add(index))
      let bestIndex = -1
      let bestScore = -1
      for (const index of [...candidates].sort((left, right) => left - right)) {
        let minimum = Infinity
        for (const member of this.metadata[index]) {
          const score = scoreMetadata(metadata, member)
          if (score === null) {
            minimum = -1
            break
          }
          minimum = Math.min(minimum, score)
        }
        if (minimum > bestScore) {
          bestIndex = index
          bestScore = minimum
        }
      }
      const member = { source, track }
      if (bestIndex >= 0) {
        this.identityGroups.set(id, bestIndex)
        const group = next[bestIndex]
        const members = [...group.members, member].sort(
          (left, right) =>
            (this.priority.get(left.source) ?? Infinity) -
            (this.priority.get(right.source) ?? Infinity)
        )
        next[bestIndex] = {
          ...group,
          members,
          representative: members[0].track,
          sourceCount: new Set(members.map((value) => value.source)).size
        }
        this.metadata[bestIndex].push(metadata)
      } else {
        const index = next.length
        this.identityGroups.set(id, index)
        next.push({ id, members: [member], representative: track, sourceCount: 1 })
        this.metadata.push([metadata])
        if (metadata.valid) {
          const buckets = artistBuckets ?? new Map<number, Set<number>>()
          const indices = buckets.get(bucket) ?? new Set<number>()
          indices.add(index)
          buckets.set(bucket, indices)
          this.buckets.set(metadata.artists, buckets)
        }
      }
    }
    this.groups = next
  }
}
