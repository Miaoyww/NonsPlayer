import { nativeCall } from '@/lib/player'
import { musicClient } from './client'
import { sourcePreferences } from './source-preferences'
import { collectionReference, onlineReference } from './legacy-reference'
import type { MusicReadRequest, MusicResponse } from './types'
import type { Track } from '@/lib/player'
import type { MusicCollection } from '@/features/workspace/music-navigation'
const continuations = new Map<string, Map<number, string | null>>()
export const currentSource = async () => (await sourcePreferences.get()).current
export function clearContinuations() {
  continuations.clear()
}
export async function queryMusic(request: MusicReadRequest, source?: string, refresh = false) {
  source ??= await currentSource()
  return refresh ? musicClient.refresh(source, request) : musicClient.query(source, request)
}
// Bounded continuation state for the existing infinite lists, not a resource cache.
export async function queryPage(
  request: MusicReadRequest,
  offset: number,
  step: number,
  source?: string,
  refresh = false
): Promise<Exclude<MusicResponse, { type: 'write' }>> {
  if (
    !('page' in request) ||
    offset < 0 ||
    !Number.isSafeInteger(offset) ||
    !Number.isSafeInteger(step) ||
    step <= 0
  )
    throw new Error('分页请求无效')
  source ??= await currentSource()
  const scope = await nativeCall<string>('music_context', { source })
  const key = JSON.stringify([source, scope, step, { ...request, page: undefined }])
  let pages = continuations.get(key)
  if (!pages || (refresh && offset === 0)) {
    while (continuations.size >= 16) continuations.delete(continuations.keys().next().value!)
    pages = new Map([[0, null]])
    continuations.set(key, pages)
  }
  if (!pages.has(offset)) throw new Error('分页上下文已过期，请刷新列表')
  const result = await queryMusic(
    { ...request, page: { cursor: pages.get(offset)!, limit: step } },
    source,
    refresh
  )
  const page = result.type === 'tracks' || result.type === 'entities' ? result.data : undefined
  if (!page) throw new Error('音乐来源返回了错误的分页响应')
  if (page.nextCursor && page.nextCursor === pages.get(offset))
    throw new Error('音乐来源返回了重复游标')
  if (scope !== (await nativeCall<string>('music_context', { source })))
    throw new Error('音乐来源上下文已变化')
  while (pages.size >= 128) {
    const first = [...pages.keys()].find((value) => value !== 0)
    if (first === undefined) break
    pages.delete(first)
  }
  if (page.nextCursor) pages.set(offset + step, page.nextCursor)
  return result
}
export async function tracksForCollection(collection: MusicCollection, offset: number) {
  const reference = collectionReference(collection),
    page = { cursor: null, limit: 100 }
  const response = await queryPage(
    reference.kind === 'artist'
      ? { operation: 'artistTracks', artist: reference, page }
      : { operation: 'tracks', entity: reference, page },
    offset,
    100,
    reference.source
  )
  if (response.type !== 'tracks') throw new Error('曲目列表响应无效')
  return {
    tracks: await (
      reference.kind === 'playlist' ? musicClient.storedTracks : musicClient.queueTracks
    )(reference.source, response.data.items),
    total: response.data.total ?? response.data.items.length,
    more: response.data.nextCursor !== null,
    description: response.data.description
  }
}
export async function readTrackInformation(track: Track, refresh = false) {
  const reference = onlineReference(track)
  if (!reference) throw new Error('曲目缺少在线来源引用')
  const response = await queryMusic(
    { operation: 'trackInformation', track: reference },
    reference.source,
    refresh
  )
  if (response.type !== 'information') throw new Error('曲目信息响应无效')
  return response.data
}
