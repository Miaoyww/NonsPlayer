import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode
} from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { nativeCall, type Track } from '@/lib/player'
import { invalidateNativeCache, invalidateAggregateCache } from '@/lib/runtime-cache'
import { musicClient } from '@/features/music/client'
import type { AccountRef, EntityRef } from '@/features/music/types'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export interface AggregatePlaylist {
  id: string
  name: string
  covers: string[]
  trackCount: number
  link: {
    reference: EntityRef
    account: AccountRef
    lastSuccess: number
    lastAttempt: number
    error: string | null
  } | null
}
export interface AggregateMember {
  track: Track
  remote: boolean
  unavailable: string | null
}
export interface SyncProgress {
  key: string
  running: boolean
  count: number
  playlistId: string | null
  error: string | null
}
export type AggregateWrite =
  | { operation: 'create'; name: string; keys?: string[] }
  | { operation: 'rename'; id: string; name: string }
  | { operation: 'delete' | 'unlink'; id: string }
  | { operation: 'add' | 'remove' | 'reorder'; id: string; keys: string[] }
const Context = createContext<{
  playlists: AggregatePlaylist[]
  accounts: Record<string, AccountRef | null>
  revision: number
  error?: string
  reload: () => Promise<void>
} | null>(null)
export function AggregateLibraryProvider({ children }: { children: ReactNode }) {
  const [playlists, setPlaylists] = useState<AggregatePlaylist[]>([])
  const [accounts, setAccounts] = useState<Record<string, AccountRef | null>>({})
  const [revision, setRevision] = useState(0)
  const [error, setError] = useState<string>()
  const serial = useRef(0)
  const reload = useCallback(async () => {
    if (!isTauri()) return
    const request = ++serial.current
    try {
      const items = await nativeCall<AggregatePlaylist[]>('aggregate_playlists', { refresh: true })
      const sources = [...new Set(items.flatMap((p) => (p.link ? [p.link.reference.source] : [])))]
      const selected = await Promise.all(
        sources.map(
          async (source) =>
            [source, (await musicClient.currentAccount(source))?.reference ?? null] as const
        )
      )
      if (request !== serial.current) return
      setPlaylists(items)
      setAccounts(Object.fromEntries(selected))
      setRevision((v) => v + 1)
      setError(undefined)
    } catch (cause) {
      if (request === serial.current) setError(String(cause))
    }
  }, [])
  useEffect(() => {
    let disposed = false
    const stops: (() => void)[] = []
    void reload()
    if (isTauri())
      for (const event of [
        'aggregate-playlist-updated',
        'local-library-updated',
        'music-account-changed',
        'adapters-changed'
      ]) {
        void listen(event, ({ payload }) => {
          if (disposed) return
          if (event === 'aggregate-playlist-updated')
            invalidateAggregateCache(typeof payload === 'string' ? payload : undefined)
          else
            invalidateNativeCache([
              'aggregate_playlists',
              'aggregate_playlist',
              'aggregate_tracks',
              'local_entities',
              'local_entity_detail',
              'local_entity_tracks'
            ])
          void reload()
        })
          .then((stop) => {
            if (disposed) stop()
            else stops.push(stop)
          })
          .catch((cause) => {
            if (!disposed) setError(String(cause))
          })
      }
    return () => {
      disposed = true
      serial.current++
      stops.forEach((stop) => stop())
    }
  }, [reload])
  return (
    <Context.Provider value={{ playlists, accounts, revision, reload, error }}>
      {children}
    </Context.Provider>
  )
}
export function useAggregateLibrary() {
  const value = useContext(Context)
  if (!value) throw new Error('AggregateLibraryProvider is required')
  return value
}
export function matchesAccount(a?: AccountRef | null, b?: AccountRef | null) {
  return !!a && !!b && a.source === b.source && a.id === b.id
}
export function aggregateCollection(p: AggregatePlaylist): MusicCollection {
  return {
    id: p.id,
    localId: p.id,
    reference: null,
    kind: 'playlist',
    name: p.name,
    cover: p.covers[0] ?? '',
    subtitle: '聚合歌单',
    trackCount: p.trackCount
  }
}
export function useAggregateReplacement() {
  const { playlists, accounts } = useAggregateLibrary()
  return (item: MusicCollection) =>
    item.bypassAggregate
      ? undefined
      : item.localId
        ? playlists.find((p) => p.id === item.localId)
        : playlists.find(
            (p) =>
              p.link &&
              item.reference?.kind === 'playlist' &&
              p.link.reference.source === item.reference.source &&
              p.link.reference.id === item.reference.id &&
              matchesAccount(accounts[item.reference.source], p.link.account)
          )
}
export async function writeAggregate(request: AggregateWrite) {
  const id = await nativeCall<string>('aggregate_write', { request })
  return id
}
export async function allAggregateTracks(id: string, excluded = false) {
  const items: AggregateMember[] = []
  for (let offset = 0; offset <= 10000; offset += 100) {
    const page = await nativeCall<{ items: AggregateMember[]; more: boolean }>('aggregate_tracks', {
      id,
      offset,
      excluded
    })
    items.push(...page.items)
    if (!page.more) return items
  }
  throw new Error('歌单超过容量上限，请重新读取。')
}
export async function playAggregate(id: string, key?: string, next = false) {
  return nativeCall<number>('aggregate_play', { id, key, next })
}
