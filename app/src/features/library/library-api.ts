import { requestCache, requestKey, invalidateNativeCache } from '@/lib/runtime-cache'
import { nativeCall, type Track } from '@/lib/player'
import type { AccountProfile } from '@/features/account/account'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { musicClient } from '@/features/music/client'

export type LibraryTab = 'playlist' | 'album' | 'artist' | 'history'
export type PlaylistFilter = 'all' | 'mine' | 'liked'
export interface LibrarySummary {
  profile: AccountProfile
  likedPlaylist: MusicCollection | null
  likedTracks: Track[]
  likedError: string | null
}
export interface CollectionPage {
  items: MusicCollection[]
  more: boolean
}
export interface CollectionTracks {
  description?: string | null
  tracks: Track[]
  total: number
  more: boolean
}
let accountId: number | undefined
export function invalidateMusicLibrary() {
  invalidateNativeCache(['music_library', 'library_tracks', 'library_collections'])
}
export function resetAccountCache() {
  accountId = undefined
  invalidateNativeCache()
}
function selectAccount(userId: number) {
  if (accountId !== undefined && accountId !== userId) resetAccountCache()
  accountId = userId
}
export function peekMusicLibrary(userId: number) {
  selectAccount(userId)
  return requestCache.peek<LibrarySummary>(requestKey('music_library'))
}
export function getMusicLibrary(userId: number): Promise<LibrarySummary> {
  selectAccount(userId)
  return nativeCall<LibrarySummary>('music_library')
}
export const getLibraryCollections = (kind: LibraryTab, offset: number, filter: PlaylistFilter) =>
  nativeCall<CollectionPage>('library_collections', { kind, offset, filter })
export function getLibraryTracks(
  collection: MusicCollection,
  offset: number,
  userId: number
): Promise<CollectionTracks> {
  selectAccount(userId)
  return nativeCall<CollectionTracks>('library_tracks', {
    kind: collection.kind,
    id: collection.id,
    offset
  })
}
export const getLibraryHistory = (week: boolean, offset: number) =>
  nativeCall<CollectionTracks>('library_history', { week, offset })
export async function playMusicCollection(collection: MusicCollection, next = false, key?: string) {
  if (!collection.reference)
    return nativeCall<boolean>(next ? 'append_library_collection' : 'play_library_collection', {
      kind: collection.kind,
      id: collection.id,
      key: key ?? null
    })
  const reference = collection.reference
  const scope = await nativeCall<string>('music_context', { source: reference.source })
  const tracks: Track[] = []
  let cursor: string | null = null
  const seen = new Set<string>()
  let more = false
  let pages = 0
  do {
    pages++
    const response = await musicClient.query(
      reference.source,
      reference.kind === 'artist'
        ? { operation: 'artistTracks', artist: reference, page: { cursor, limit: 100 } }
        : { operation: 'tracks', entity: reference, page: { cursor, limit: 100 } }
    )
    if (response.type !== 'tracks') throw new Error('音乐来源返回了错误的歌曲列表')
    const values = await musicClient.queueTracks(reference.source, response.data.items)
    for (const value of values)
      if (!seen.has(value.key) && tracks.length < 1000) {
        seen.add(value.key)
        tracks.push(value)
      }
    if (response.data.nextCursor !== null && response.data.nextCursor === cursor)
      throw new Error('音乐来源返回了重复游标')
    cursor = response.data.nextCursor
    more = cursor !== null
  } while (more && tracks.length < 1000 && pages < 10)
  if (scope !== (await nativeCall<string>('music_context', { source: reference.source })))
    throw new Error('账号或音乐来源状态已变化，请重试')
  if (!tracks.length) throw new Error('此收藏暂无歌曲')
  await nativeCall(next ? 'append_queue' : 'play_queue', {
    keys: tracks.map((track) => track.key),
    ...(!next
      ? {
          index: Math.max(
            0,
            tracks.findIndex((track) => track.key === key)
          )
        }
      : {})
  })
  return more
}
export const playLibraryCollection = (collection: MusicCollection, key?: string) =>
  playMusicCollection(collection, false, key)
