import { peekLibraryPreview, saveLibraryPreview, clearLibraryPreviews } from './library-preview'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { nativeCall, type Track } from '@/lib/player'
import { accountProfile, type AccountProfile } from '@/features/account/account'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { musicClient } from '@/features/music/client'
import { collectionReference } from '@/features/music/references'
import {
  clearContinuations,
  queryMusic,
  queryPage,
  tracksForCollection
} from '@/features/music/queries'
import { musicCollection } from '@/features/music/entities'
import type { AccountRef } from '@/features/music/types'
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
let account: AccountRef | null = null
export function setLibraryAccount(value: AccountRef | null) {
  account = value
}
export function invalidateMusicLibrary() {
  clearLibraryPreviews()
  clearContinuations()
  invalidateNativeCache(['music_query'])
}
export function resetAccountCache() {
  account = null
  clearContinuations()
  invalidateNativeCache(undefined, 'context')
}
function selectAccount(userId: string) {
  if (!account || account.id !== userId) throw new Error('账号上下文已变化')
  return account
}
export function peekMusicLibrary(userId: string, source?: string) {
  if (!account || account.id !== userId || (source !== undefined && account.source !== source))
    return undefined
  return peekLibraryPreview(JSON.stringify(account))?.summary
}
export async function getMusicLibrary(userId: string): Promise<LibrarySummary> {
  const owner = selectAccount(userId)
  const response = await queryMusic({ operation: 'librarySummary' }, owner.source)
  if (response.type !== 'summary') throw new Error('音乐库摘要响应无效')
  const value = {
    profile: accountProfile(response.data.account),
    likedPlaylist: response.data.likedPlaylist
      ? musicCollection(response.data.likedPlaylist)
      : null,
    likedTracks: await musicClient.queueTracks(owner.source, response.data.likedTracks),
    likedError: response.data.likedError?.code ?? null
  }
  if (!value.likedError && JSON.stringify(account) === JSON.stringify(owner))
    saveLibraryPreview(JSON.stringify(owner), { summary: value })
  return value
}
export function peekLibraryCollections(userId: string, source?: string) {
  if (!account || account.id !== userId || (source !== undefined && account.source !== source))
    return undefined
  return peekLibraryPreview(JSON.stringify(account))?.collections
}
export function getLibraryCollections(
  kind: LibraryTab,
  offset: number,
  filter: PlaylistFilter,
  userId = account?.id
): Promise<CollectionPage> {
  if (!userId || kind === 'history') throw new Error('收藏查询缺少账号或类型无效')
  const owner = selectAccount(userId)
  return queryPage(
    {
      operation: 'libraryCollections',
      kind,
      filter: filter === 'mine' ? 'created' : filter === 'liked' ? 'collected' : filter,
      page: { cursor: null, limit: 30 }
    },
    offset,
    30,
    owner.source
  ).then((response) => {
    if (response.type !== 'entities') throw new Error('收藏列表响应无效')
    const value = {
      items: response.data.items.map(musicCollection),
      more: response.data.nextCursor !== null
    }
    if (
      kind === 'playlist' &&
      offset === 0 &&
      filter === 'all' &&
      JSON.stringify(account) === JSON.stringify(owner)
    )
      saveLibraryPreview(JSON.stringify(owner), { collections: value })
    return value
  })
}
export function getLibraryTracks(
  collection: MusicCollection,
  offset: number,
  userId: string
): Promise<CollectionTracks> {
  selectAccount(userId)
  return tracksForCollection(collection, offset)
}
export async function getLibraryHistory(week: boolean, offset: number): Promise<CollectionTracks> {
  const owner = account
  if (!owner) throw new Error('账号上下文已变化')
  const response = await queryPage(
    { operation: 'history', week, page: { cursor: null, limit: 100 } },
    offset,
    100,
    owner.source
  )
  if (response.type !== 'tracks') throw new Error('播放历史响应无效')
  return {
    tracks: await musicClient.queueTracks(owner.source, response.data.items),
    more: response.data.nextCursor !== null,
    total: response.data.total ?? response.data.items.length
  }
}
export const playMusicCollection = (collection: MusicCollection, next = false, key?: string) =>
  collection.localId
    ? nativeCall<number>('aggregate_play', { id: collection.localId, key, next }).then(() => false)
    : nativeCall<boolean>('music_play_collection', {
        entity: collectionReference(collection),
        next,
        key: key ?? null
      })
export const playLibraryCollection = (collection: MusicCollection, key?: string) =>
  playMusicCollection(collection, false, key)
