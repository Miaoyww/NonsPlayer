import { reportCaughtError } from '@/lib/diagnostics'
import { onlineReference, optionalCollectionReference } from './references'
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
  } catch (cause) {
    reportCaughtError('app/src/features/music/share-links.ts', cause)

    return ''
  }
}

export function trackShareLink(track: Track, sources: SourceDescriptor[]): string {
  if (track.source.kind === 'local') return ''
  const reference = onlineReference(track)
  return reference ? entityShareLink(reference, sources) : ''
}

export function collectionShareLink(item: MusicCollection, sources: SourceDescriptor[]): string {
  if (item.localId) return ''
  const reference = optionalCollectionReference(item)
  return reference ? entityShareLink(reference, sources) : ''
}
