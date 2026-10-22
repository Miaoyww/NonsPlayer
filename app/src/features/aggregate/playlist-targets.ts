import { musicClient } from '@/features/music/client'
import { musicCollection } from '@/features/music/entities'
import { collectionReference, onlineReference } from '@/features/music/legacy-reference'
import { nativeCall, type Track } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import type { AccountRef } from '@/features/music/types'
import { writeAggregate, type AggregatePlaylist, type SyncProgress } from './aggregate-library'

export function mergeLibraryPlaylists(
  remote: MusicCollection[],
  local: MusicCollection[],
  replacement: (item: MusicCollection) => { id: string } | undefined
) {
  const represented = new Set(local.map((item) => item.localId))
  return [...local, ...remote.filter((item) => !represented.has(replacement(item)?.id))]
}

export interface PlaylistTarget {
  item: MusicCollection
  account: AccountRef
  writable: boolean
}

export async function ownedPlaylistTargets(
  source: string,
  writable: boolean
): Promise<PlaylistTarget[]> {
  const account = (await musicClient.currentAccount(source))?.reference
  if (!account) return []
  const targets: PlaylistTarget[] = []
  const seen = new Set<string>()
  let cursor: string | null = null
  for (let page = 0; page < 3334; page++) {
    const result = await musicClient.query(source, {
      operation: 'libraryCollections',
      kind: 'playlist',
      filter: 'created',
      page: { cursor, limit: 30 }
    })
    if (result.type !== 'entities') throw new Error('歌单列表响应无效')
    for (const entity of result.data.items) {
      if (entity.creator?.source === account.source && entity.creator.id === account.id)
        targets.push({ item: musicCollection(entity), account, writable })
    }
    cursor = result.data.nextCursor
    if (!cursor) return targets
    if (seen.has(cursor)) throw new Error('歌单返回重复游标')
    seen.add(cursor)
  }
  throw new Error('歌单分页超过安全上限')
}

export async function addToPlaylist(target: PlaylistTarget, tracks: Track[], localId?: string) {
  const checkAccount = async () => {
    const current = (await musicClient.currentAccount(target.account.source))?.reference
    if (current?.source !== target.account.source || current.id !== target.account.id)
      throw new Error('账号已变化，请重新选择歌单')
  }
  await checkAccount()
  const playlist = collectionReference(target.item)
  if (!localId) {
    // The global presentation state can lag behind the completed native transaction.
    const versions = await nativeCall<AggregatePlaylist[]>('aggregate_playlists', { refresh: true })
    localId = versions.find(
      (p) =>
        p.link?.reference.source === playlist.source &&
        p.link.reference.id === playlist.id &&
        p.link.account.source === target.account.source &&
        p.link.account.id === target.account.id
    )?.id
  }
  if (localId) return addToAggregatePlaylist(localId, tracks)
  const references = tracks.map(onlineReference)
  if (references.every((reference) => reference?.source === playlist.source)) {
    if (!target.writable) throw new Error('此来源不支持修改平台歌单')
    for (const track of references) {
      await checkAccount()
      await musicClient.write(playlist.source, {
        operation: 'playlistTrack',
        playlist,
        track: track!,
        add: true
      })
    }
    return
  }
  if (target.item.liked) throw new Error('我喜欢的音乐不能转为聚合歌单，请选择普通歌单')
  let progress = await nativeCall<SyncProgress>('aggregate_sync', {
    reference: playlist,
    force: true
  })
  while (progress.running) {
    await new Promise((resolve) => setTimeout(resolve, 750))
    const status = await nativeCall<SyncProgress | null>('aggregate_sync_status', {
      key: progress.key
    })
    if (!status) throw new Error('同步任务已结束，请重试')
    progress = status
  }
  if (progress.error || !progress.playlistId) throw new Error(progress.error ?? '未能建立聚合歌单')
  await checkAccount()
  return addToAggregatePlaylist(progress.playlistId, tracks)
}

// Same-platform additions are uploaded before the local batch is committed.
// Other sources and detached playlists remain local.
export async function addToAggregatePlaylist(id: string, tracks: Track[]) {
  const playlist = await nativeCall<AggregatePlaylist>('aggregate_playlist', { id, refresh: true })
  const link = playlist.link
  const references = [
    ...new Map(
      tracks
        .map(onlineReference)
        .filter((reference) => reference && reference.source === link?.reference.source)
        .map((reference) => [reference!.id, reference!])
    ).values()
  ]
  let uploaded = 0
  if (link && references.length) {
    for (const track of references) {
      const current = (await musicClient.currentAccount(link.account.source))?.reference
      if (current?.source !== link.account.source || current.id !== link.account.id)
        throw new Error('请先选择关联账号，再添加此平台的歌曲；本机尚未添加')
      try {
        await musicClient.write(link.reference.source, {
          operation: 'playlistTrack',
          playlist: link.reference,
          track,
          add: true
        })
        uploaded++
      } catch (cause) {
        throw new Error(
          `${uploaded ? `已有 ${uploaded} 首上传成功；` : ''}上传失败，本机尚未添加，可重试：${String(cause)}`
        )
      }
    }
  }
  try {
    return await writeAggregate({ operation: 'add', id, keys: tracks.map((track) => track.key) })
  } catch (cause) {
    if (uploaded) throw new Error(`服务器已保存，但本机添加失败，请重试：${String(cause)}`)
    throw cause
  }
}
