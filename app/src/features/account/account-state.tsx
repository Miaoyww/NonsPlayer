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
import { errorText } from '@/lib/player'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import {
  invalidateMusicLibrary,
  resetAccountCache,
  setLibraryAccount
} from '@/features/library/library-api'
import { musicClient } from '@/features/music/client'
import { useMusicSources } from '@/features/music/use-music-sources'
import type { AccountRecord, AccountRef } from '@/features/music/types'

export interface AccountProfile {
  reference: AccountRef
  userId: string
  nickname: string
  avatarUrl: import('@/features/music/types').CoverInput
}
export const accountProfile = (record: AccountRecord): AccountProfile => ({
  reference: record.reference,
  userId: record.reference.id,
  nickname: record.displayName,
  avatarUrl: record.avatar
})
const AccountContext = createContext<{
  profile: AccountProfile | null
  loading: boolean
  error?: string
  setProfile: (profile: AccountProfile | null) => void
  likedIds: ReadonlySet<string>
  likesReady: boolean
  canToggleLike: boolean
  likesError?: string
  likesRevision: number
  pendingLikes: ReadonlySet<string>
  reloadLikes: () => void
  toggleLike: (id: string) => Promise<void>
} | null>(null)

export function AccountProvider({ children }: { children: ReactNode }) {
  const { preferences, statuses } = useMusicSources()
  const source = preferences?.current
  const descriptor = statuses.find(
    (value) => value.descriptor.source === source && value.enabled
  )?.descriptor
  const available = !!descriptor
  const canToggleLike = !!descriptor?.capabilities.includes('favorites')
  const canReadLikes = canToggleLike || !!descriptor?.capabilities.includes('favoritesRead')
  const [profile, updateProfile] = useState<AccountProfile | null>(null)
  const [loading, setLoading] = useState(isTauri())
  const [error, setError] = useState<string>()
  const [refresh, setRefresh] = useState(0)
  const [likedIds, setLikedIds] = useState<Set<string>>(new Set())
  const [pendingLikes, setPendingLikes] = useState<Set<string>>(new Set())
  const [likesReady, setLikesReady] = useState(false)
  const [likesError, setLikesError] = useState<string>()
  const [likesRevision, setLikesRevision] = useState(0)
  const [likesRefresh, setLikesRefresh] = useState(0)
  const generation = useRef(0),
    likesGeneration = useRef(0),
    pending = useRef(new Set<string>())
  const setProfile = useCallback((value: AccountProfile | null) => {
    resetAccountCache()
    generation.current++
    likesGeneration.current++
    setLibraryAccount(value?.reference ?? null)
    updateProfile(value)
    setLikesReady(false)
    setLikedIds(new Set())
    setLoading(false)
    setError(undefined)
  }, [])
  useEffect(() => {
    let disposed = false
    const serial = ++generation.current
    resetAccountCache()
    setLibraryAccount(null)
    updateProfile(null)
    setLoading(Boolean(source && isTauri()))
    setError(undefined)
    if (source && descriptor && isTauri())
      void (async () => {
        const saved = await musicClient.currentAccount(source)
        if (disposed || serial !== generation.current) return
        const snapshot = saved ? accountProfile(saved) : null
        setLibraryAccount(snapshot?.reference ?? null)
        updateProfile(snapshot)
        setLoading(false)
        const result = await musicClient.account(source, { operation: 'profile' })
        if (disposed || serial !== generation.current) return
        if (result.type !== 'profile') throw new Error('账号资料响应无效')
        const current = result.data ? accountProfile(result.data) : null
        setLibraryAccount(current?.reference ?? null)
        updateProfile(current)
      })()
        .catch((cause) => {
          if (!disposed && serial === generation.current) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed && serial === generation.current) setLoading(false)
        })
    else setLoading(false)
    return () => {
      disposed = true
      generation.current++
    }
  }, [source, available, refresh])
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    const stops: (() => void)[] = []
    for (const event of ['music-account-changed', 'adapters-changed'])
      void listen<string>(event, ({ payload }) => {
        if (!disposed && (event === 'adapters-changed' || payload === source))
          setRefresh((value) => value + 1)
      }).then((stop) => {
        if (disposed) stop()
        else stops.push(stop)
      })
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [source])
  useEffect(() => {
    const serial = ++likesGeneration.current
    pending.current = new Set()
    setPendingLikes(new Set())
    setLikedIds(new Set())
    setLikesReady(false)
    setLikesError(undefined)
    if (!profile || !canReadLikes || !isTauri()) return
    void (async () => {
      if (canToggleLike) {
        const result = await musicClient.query(profile.reference.source, { operation: 'favorites' })
        if (result.type !== 'favorites') throw new Error('收藏状态响应无效')
        return result.data
      }
      const items: import('@/features/music/types').EntityRef[] = []
      let cursor: string | null = null
      const seen = new Set<string>()
      do {
        const result = await musicClient.query(profile.reference.source, {
          operation: 'favoritesPage',
          page: { cursor, limit: 100 }
        })
        if (serial !== likesGeneration.current) return []
        if (result.type !== 'favoritesPage') throw new Error('收藏状态响应无效')
        items.push(...result.data.items)
        cursor = result.data.nextCursor
        if (items.length > 100_000 || (cursor && seen.has(cursor)))
          throw new Error('收藏状态超过数量上限或返回重复游标')
        if (cursor) seen.add(cursor)
      } while (cursor)
      return items
    })()
      .then((items) => {
        if (serial !== likesGeneration.current) return
        setLikedIds(new Set(items.map((value) => value.id)))
        setLikesReady(true)
      })
      .catch((cause) => {
        if (serial === likesGeneration.current) setLikesError(errorText(cause))
      })
    return () => {
      likesGeneration.current++
    }
  }, [profile, canReadLikes, canToggleLike, likesRefresh])
  const reloadLikes = useCallback(() => setLikesRefresh((value) => value + 1), [])
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false,
      stop: (() => void) | undefined
    void listen<string>('media-favorite-changed', ({ payload }) => {
      if (disposed || payload !== source) return
      invalidateNativeCache()
      invalidateMusicLibrary()
      setLikesRevision((value) => value + 1)
      reloadLikes()
    }).then((value) => {
      if (disposed) value()
      else stop = value
    })
    return () => {
      disposed = true
      stop?.()
    }
  }, [source, reloadLikes])
  async function toggleLike(id: string) {
    if (!profile || !likesReady || !canToggleLike || pending.current.has(id)) return
    const serial = likesGeneration.current,
      liked = !likedIds.has(id)
    pending.current.add(id)
    setPendingLikes(new Set(pending.current))
    try {
      await musicClient.write(profile.reference.source, {
        operation: 'setFavorite',
        track: { source: profile.reference.source, kind: 'track', id },
        liked
      })
      if (serial !== likesGeneration.current) return
      setLikedIds((values) => {
        const next = new Set(values)
        if (liked) next.add(id)
        else next.delete(id)
        return next
      })
      invalidateMusicLibrary()
      setLikesRevision((value) => value + 1)
    } finally {
      if (serial === likesGeneration.current) {
        pending.current.delete(id)
        setPendingLikes(new Set(pending.current))
      }
    }
  }
  return (
    <AccountContext.Provider
      value={{
        profile,
        loading,
        error,
        setProfile,
        likedIds,
        likesReady,
        canToggleLike,
        likesError,
        likesRevision,
        pendingLikes,
        reloadLikes,
        toggleLike
      }}
    >
      {children}
    </AccountContext.Provider>
  )
}
export function useAccount() {
  const value = useContext(AccountContext)
  if (!value) throw new Error('AccountProvider is required')
  return value
}
