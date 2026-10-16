import { reportCaughtError } from '@/lib/diagnostics'
// Last rendered metadata only. A live host account must be confirmed before looking it up.
import { validCoverInput } from '@/features/music/cover-image'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import type { LibrarySummary, CollectionPage } from './library-api'
const key = 'nons-library-preview-v2'
const limit = 128 * 1024
const ttl = 24 * 60 * 60 * 1000
interface Preview {
  userId: string
  updatedAt: number
  summary?: LibrarySummary
  collections?: CollectionPage
}
let entries: Preview[] | undefined
const owner = (value: unknown): value is string =>
  typeof value === 'string' && value.length > 0 && value.length <= 2048
function validSummary(value: LibrarySummary | undefined, userId: string) {
  return (
    !value ||
    (JSON.stringify(value.profile?.reference) === userId &&
      typeof value.profile.nickname === 'string' &&
      validCoverInput(value.profile.avatarUrl) &&
      Array.isArray(value.likedTracks) &&
      value.likedTracks.length <= 12 &&
      value.likedTracks.every(
        (track) =>
          typeof track.key === 'string' &&
          typeof track.title === 'string' &&
          typeof track.artist === 'string' &&
          typeof track.album === 'string' &&
          Number.isFinite(track.durationMs) &&
          validCoverInput(track.cover) &&
          track.source?.kind === 'online' &&
          owner(track.source.track.reference.id)
      ) &&
      (value.likedPlaylist === null || validCollection(value.likedPlaylist)))
  )
}
function validCollection(value: MusicCollection | null | undefined): boolean {
  return (
    !!value &&
    value.kind === 'playlist' &&
    !!value.reference &&
    owner(value.reference.id) &&
    typeof value.name === 'string' &&
    typeof value.subtitle === 'string' &&
    Number.isFinite(value.trackCount) &&
    value.trackCount >= 0 &&
    value.localId === undefined &&
    validCoverInput(value.cover) &&
    owner(value.reference.source)
  )
}
function valid(value: Preview): boolean {
  let account: { source?: string; id?: string }
  try {
    account = JSON.parse(value.userId)
  } catch (cause) {
    reportCaughtError('app/src/features/library/library-preview.ts', cause)

    return false
  }
  if (!account || !owner(account.source) || !owner(account.id)) return false
  const source = account.source
  if (
    value.summary?.likedTracks.some(
      (track) => track.source.kind !== 'online' || track.source.track.reference.source !== source
    ) ||
    (value.summary?.likedPlaylist && value.summary.likedPlaylist.reference?.source !== source) ||
    value.collections?.items.some((item) => item.reference?.source !== source)
  )
    return false
  return (
    !!value &&
    owner(value.userId) &&
    Number.isFinite(value.updatedAt) &&
    value.updatedAt <= Date.now() &&
    Date.now() - value.updatedAt <= ttl &&
    JSON.stringify(value).length * 2 <= limit &&
    validSummary(value.summary, value.userId) &&
    (!value.collections ||
      (Array.isArray(value.collections.items) &&
        value.collections.items.length <= 30 &&
        value.collections.items.every(validCollection)))
  )
}
function read() {
  if (!entries) {
    try {
      const raw = localStorage.getItem(key)
      const data: unknown = raw && raw.length * 2 <= 2 * limit ? JSON.parse(raw) : []
      entries = Array.isArray(data) ? data.filter(valid).slice(0, 2) : []
    } catch (cause) {
      reportCaughtError('app/src/features/library/library-preview.ts', cause)

      entries = []
    }
  }
  entries = entries.filter(valid)
  return entries
}
export function peekLibraryPreview(userId: string) {
  return read().find((value) => value.userId === userId)
}
export function saveLibraryPreview(
  userId: string,
  patch: Pick<Preview, 'summary' | 'collections'>
) {
  if (!owner(userId)) return
  const value = { ...peekLibraryPreview(userId), ...patch, userId, updatedAt: Date.now() }
  if (value.summary)
    value.summary = { ...value.summary, likedTracks: value.summary.likedTracks.slice(0, 12) }
  if (!valid(value)) return
  entries = [value, ...read().filter((item) => item.userId !== userId)].slice(0, 2)
  try {
    localStorage.setItem(key, JSON.stringify(entries))
  } catch (cause) {
    reportCaughtError('app/src/features/library/library-preview.ts', cause)

    /* Quota/storage failure does not block rendering. */
  }
}
export function clearLibraryPreviews() {
  entries = []
  try {
    localStorage.removeItem(key)
  } catch (cause) {
    reportCaughtError('app/src/features/library/library-preview.ts', cause)

    /* Storage may be unavailable. */
  }
}
