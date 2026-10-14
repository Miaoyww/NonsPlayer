import type { Track } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import type { EntityRef, SourceDescriptor } from './types'

export function entityShareLink(reference: EntityRef, sources: SourceDescriptor[]): string {
  const template = sources.find((source) => source.source === reference.source)
    ?.shareLinkTemplates?.[reference.kind]
  if (!template || template.split('{id}').length !== 2) return ''
  try {
    const link = template.replace('{id}', encodeURIComponent(reference.id))
    const parsed = new URL(link)
    return ['https:', 'http:'].includes(parsed.protocol) && !parsed.username && !parsed.password
      ? link
      : ''
  } catch {
    return ''
  }
}

// Legacy persisted items are converted to references only at this compatibility boundary.
export function trackShareLink(track: Track, sources: SourceDescriptor[]): string {
  if (track.source.kind === 'local') return ''
  const reference: EntityRef =
    track.source.kind === 'online'
      ? track.source.track.reference
      : { source: 'netease', kind: 'track', id: String(track.source.id) }
  return entityShareLink(reference, sources)
}

export function collectionShareLink(item: MusicCollection, sources: SourceDescriptor[]): string {
  if (item.localId) return ''
  return entityShareLink(
    item.reference ?? {
      source: 'netease',
      kind: item.kind,
      id: String(item.id)
    },
    sources
  )
}
