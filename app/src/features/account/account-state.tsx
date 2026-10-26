import { reportCaughtError } from '@/lib/diagnostics'
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
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
function reconcileProfile(previous: AccountProfile | null, next: AccountProfile | null) {
  if (
    !previous ||
    !next ||
    previous.reference.source !== next.reference.source ||
    previous.reference.id !== next.reference.id
  )
    return next
  if (
    previous.nickname === next.nickname &&
    JSON.stringify(previous.avatarUrl) === JSON.stringify(next.avatarUrl)
  )
    return previous
  // Display metadata can change without changing the account's query identity.
  return { ...next, reference: previous.reference }
}
interface AccountState {
  profile: AccountProfile | null
  loading: boolean
  error?: string
  setProfile: (profile: AccountProfile | null) => void
  likedIds: ReadonlySet<string>
  likesReady: boolean
  likesRefreshing: boolean
  likesStale: boolean
  favoriteStatus: (id: string) => 'liked' | 'unliked' | 'unknown'
  confirmLikedTracks: (tracks: readonly import('@/lib/player').Track[]) => void
  canToggleLike: boolean
  likesError?: string
  likesRevision: number
  pendingLikes: ReadonlySet<string>
  reloadLikes: () => void
  toggleLike: (id: string) => Promise<void>
}
const AccountContext = createContext<ReadonlyMap<string, AccountState>>(new Map())
const LIKES_TTL = 10 * 60 * 1000
const LIKES_RETRY = 30 * 1000
const likesStorageKey = (source: string) => `nons:music:favorites:v2:${source}`
function readGlobalLikes(source: string): { ids: Set<string>; updatedAt: number } | null {
  try {
    const raw = globalThis.localStorage?.getItem(likesStorageKey(source))
    if (raw === null || raw === undefined || raw.length > 4 * 1024 * 1024) return null
    const parsed = JSON.parse(raw) as { ids?: unknown; updatedAt?: unknown }
    if (
      !parsed ||
      !Array.isArray(parsed.ids) ||
      parsed.ids.length > 100_000 ||
      parsed.ids.some((id) => typeof id !== 'string' || id.length > 2048) ||
      typeof parsed.updatedAt !== 'number' ||
      !Number.isFinite(parsed.updatedAt) ||
      parsed.updatedAt <= 0 ||
      parsed.updatedAt > Date.now()
    )
      return null
    return { ids: new Set(parsed.ids), updatedAt: parsed.updatedAt }
  } catch {
    return null
  }
}
function persistGlobalLikes(source: string, likedIds: ReadonlySet<string>, updatedAt: number) {
  try {
    const value = JSON.stringify({ ids: [...likedIds], updatedAt })
    if (likedIds.size <= 100_000 && value.length <= 4 * 1024 * 1024)
      globalThis.localStorage?.setItem(likesStorageKey(source), value)
  } catch (cause) {
    reportCaughtError('app/src/features/account/account-state.tsx', cause)
  }
}

export function AccountProvider({ children }: { children: ReactNode }) {
  const { statuses } = useMusicSources()
  const sources = [...new Set(statuses.map((item) => item.descriptor.source))].sort()
  return sources.reduceRight<ReactNode>(
    (content, source) => (
      <SourceAccountProvider key={source} source={source}>
        {content}
      </SourceAccountProvider>
    ),
    <AccountLibraryBridge>{children}</AccountLibraryBridge>
  )
}

function AccountLibraryBridge({ children }: { children: ReactNode }) {
  const { profile } = useAccount()
  const reference = profile?.reference
  useLayoutEffect(() => {
    resetAccountCache()
    setLibraryAccount(reference ?? null)
  }, [reference])
  return children
}

function SourceAccountProvider({ children, source }: { children: ReactNode; source: string }) {
  const parent = useContext(AccountContext)
  const { statuses } = useMusicSources()
  const descriptor = statuses.find(
    (value) => value.descriptor.source === source && value.enabled
  )?.descriptor
  const available = !!descriptor
  const canToggleLike = !!descriptor?.capabilities.includes('favorites')
  const hasFavoritesRead = !!descriptor?.capabilities.includes('favoritesRead')
  const canUseLikedPlaylist =
    !!descriptor?.capabilities.includes('userLibrary') &&
    !!descriptor?.capabilities.includes('browse')
  const canReadLikes = canToggleLike || hasFavoritesRead || canUseLikedPlaylist
  const [profile, updateProfile] = useState<AccountProfile | null>(null)
  const reference = profile?.reference
  const [loading, setLoading] = useState(isTauri())
  const [error, setError] = useState<string>()
  const [refresh, setRefresh] = useState(0)
  const [storedLikes] = useState(() => readGlobalLikes(source))
  const globalLikesReady = useRef(storedLikes !== null)
  const [likedIds, setLikedIds] = useState<Set<string>>(storedLikes?.ids ?? new Set())
  const liked = useRef(likedIds)
  const knownUnliked = useRef(new Set<string>())
  const lastSuccess = useRef(storedLikes?.updatedAt ?? 0)
  const lastAttempt = useRef(0)
  const lastReadFailed = useRef(false)
  const activeRead = useRef(false)
  const refreshQueued = useRef(false)
  const favoriteEvents = useRef(0)
  const editRevision = useRef(0)
  const edits = useRef(new Map<string, { revision: number; liked: boolean }>())
  const [pendingLikes, setPendingLikes] = useState<Set<string>>(new Set())
  const [likesReady, setLikesReady] = useState(storedLikes !== null)
  const [likesRefreshing, setLikesRefreshing] = useState(false)
  const [likesStale, setLikesStale] = useState(true)
  const [likesError, setLikesError] = useState<string>()
  const [likesRevision, setLikesRevision] = useState(0)
  const [likesRefresh, setLikesRefresh] = useState(0)
  const forceLikesRefresh = useRef(false)
  const generation = useRef(0),
    likesGeneration = useRef(0),
    pending = useRef(new Set<string>())
  const publishLikes = useCallback((next: Set<string>) => {
    liked.current = next
    setLikedIds(next)
  }, [])
  const confirmLikedTracks = useCallback(
    (tracks: readonly import('@/lib/player').Track[]) => {
      const next = new Set(liked.current)
      for (const track of tracks) {
        if (track.source.kind !== 'online') continue
        const ref = track.source.track.reference
        if (
          ref.source !== source ||
          ref.kind !== 'track' ||
          edits.current.has(ref.id) ||
          knownUnliked.current.has(ref.id)
        )
          continue
        if (next.size < 100_000) next.add(ref.id)
      }
      if (next.size === liked.current.size) return
      publishLikes(next)
      if (globalLikesReady.current) persistGlobalLikes(source, next, lastSuccess.current)
    },
    [source, publishLikes]
  )
  const setProfile = useCallback((value: AccountProfile | null) => {
    invalidateNativeCache(undefined, 'context')
    generation.current++
    likesGeneration.current++
    updateProfile(value)
    setLoading(false)
    setError(undefined)
  }, [])
  useEffect(() => {
    let disposed = false
    const serial = ++generation.current
    updateProfile(null)
    setLoading(Boolean(source && isTauri()))
    setError(undefined)
    if (source && descriptor?.capabilities.includes('account') && isTauri())
      void (async () => {
        const saved = await musicClient.currentAccount(source)
        if (disposed || serial !== generation.current) return
        const snapshot = saved ? accountProfile(saved) : null
        updateProfile((previous) => reconcileProfile(previous, snapshot))
        setLoading(false)
        const result = await musicClient.account(source, { operation: 'profile' })
        if (disposed || serial !== generation.current) return
        if (result.type !== 'profile') throw new Error('账号资料响应无效')
        const current = result.data ? accountProfile(result.data) : null
        updateProfile((previous) => reconcileProfile(previous, current))
      })()
        .catch((cause) => {
          reportCaughtError('app/src/features/account/account-state.tsx', cause)

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
        // Bulk startup completion is reflected by each source's availability.
        // Only explicit changes to this source invalidate its account state.
        if (!disposed && payload === source) setRefresh((value) => value + 1)
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
    setLikesError(undefined)
    if (!reference || !canReadLikes || !isTauri()) {
      activeRead.current = false
      setLikesRefreshing(false)
      setLikesReady(globalLikesReady.current)
      return
    }
    const forceRefresh = forceLikesRefresh.current || globalLikesReady.current
    forceLikesRefresh.current = false
    const revision = editRevision.current
    const eventRevision = favoriteEvents.current
    const currentRead = () =>
      serial === likesGeneration.current && eventRevision === favoriteEvents.current
    refreshQueued.current = false
    activeRead.current = true
    lastAttempt.current = Date.now()
    setLikesRefreshing(true)
    setLikesStale(true)
    const query = (request: import('@/features/music/types').MusicReadRequest) =>
      forceRefresh && (!('page' in request) || !request.page.cursor)
        ? musicClient.refresh(reference.source, request)
        : musicClient.query(reference.source, request)
    const confirmPage = (items: readonly import('@/features/music/types').EntityRef[]) => {
      if (!currentRead()) return
      const next = new Set(liked.current)
      for (const item of items) {
        const edit = edits.current.get(item.id)
        if ((!edit || edit.revision <= revision || edit.liked) && next.size < 100_000)
          next.add(item.id)
      }
      if (next.size !== liked.current.size) publishLikes(next)
    }
    void (async () => {
      if (canToggleLike) {
        const result = await query({ operation: 'favorites' })
        if (result.type !== 'favorites') throw new Error('收藏状态响应无效')
        if (
          result.data.length > 100_000 ||
          result.data.some((item) => item.source !== source || item.kind !== 'track')
        )
          throw new Error('收藏歌曲引用无效或超过数量上限')
        return result.data
      }
      let playlist: import('@/features/music/types').EntityRef | undefined
      if (!hasFavoritesRead) {
        const result = await query({
          operation: 'librarySummary'
        })
        if (!currentRead()) return []
        if (result.type !== 'summary') throw new Error('音乐库摘要响应无效')
        if (result.data.likedError && !result.data.likedPlaylist)
          throw new Error('无法读取喜欢的音乐歌单')
        playlist = result.data.likedPlaylist?.reference
        if (!playlist) return []
        if (playlist.source !== reference.source || playlist.kind !== 'playlist')
          throw new Error('喜欢的音乐歌单引用无效')
      }
      const items: import('@/features/music/types').EntityRef[] = []
      let cursor: string | null = null
      const seen = new Set<string>()
      let pages = 0
      do {
        if (++pages > 4000) throw new Error('收藏状态超过分页上限')
        const result = await query({
          ...(playlist
            ? { operation: 'tracks' as const, entity: playlist }
            : { operation: 'favoritesPage' as const }),
          page: { cursor, limit: 100 }
        })
        if (!currentRead()) return []
        if (result.type !== 'favoritesPage' && result.type !== 'tracks')
          throw new Error('收藏状态响应无效')
        const references =
          result.type === 'tracks'
            ? result.data.items.map((item) => item.reference)
            : result.data.items
        if (references.some((item) => item.source !== reference.source || item.kind !== 'track'))
          throw new Error('收藏歌曲引用无效')
        items.push(...references)
        cursor = result.data.nextCursor
        if (items.length > 100_000 || (cursor && seen.has(cursor)))
          throw new Error('收藏状态超过数量上限或返回重复游标')
        confirmPage(references)
        if (cursor) seen.add(cursor)
      } while (cursor)
      return items
    })()
      .then((items) => {
        if (!currentRead()) return
        const next = new Set(items.map((value) => value.id))
        for (const [id, edit] of edits.current) {
          if (edit.revision > revision) {
            if (edit.liked) next.add(id)
            else next.delete(id)
          } else if (!pending.current.has(id)) edits.current.delete(id)
        }
        for (const id of pending.current) {
          if (liked.current.has(id)) next.add(id)
          else next.delete(id)
        }
        globalLikesReady.current = true
        for (const id of knownUnliked.current) {
          if (next.has(id)) knownUnliked.current.delete(id)
        }
        lastSuccess.current = Date.now()
        lastReadFailed.current = false
        publishLikes(next)
        setLikesReady(true)
        setLikesStale(false)
        persistGlobalLikes(source, next, lastSuccess.current)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/account/account-state.tsx', cause)

        if (currentRead()) {
          lastReadFailed.current = true
          setLikesError(errorText(cause))
          setLikesStale(true)
        }
      })
      .finally(() => {
        if (serial === likesGeneration.current) {
          activeRead.current = false
          setLikesRefreshing(false)
          if (refreshQueued.current) {
            refreshQueued.current = false
            forceLikesRefresh.current = true
            setLikesRefresh((value) => value + 1)
          }
        }
      })
    return () => {
      likesGeneration.current++
      activeRead.current = false
    }
  }, [reference, canReadLikes, canToggleLike, hasFavoritesRead, likesRefresh])
  const reloadLikes = useCallback(() => {
    if (activeRead.current) return
    forceLikesRefresh.current = true
    setLikesRefresh((value) => value + 1)
  }, [])
  useEffect(() => {
    if (!reference || !canReadLikes || !isTauri() || !globalThis.window) return
    const revalidate = () => {
      if (document.visibilityState === 'hidden' || activeRead.current) return
      const now = Date.now()
      const deadline = lastReadFailed.current
        ? lastAttempt.current + LIKES_RETRY
        : lastSuccess.current + LIKES_TTL
      if (now < deadline) return
      reloadLikes()
    }
    window.addEventListener('focus', revalidate)
    document.addEventListener('visibilitychange', revalidate)
    const timer = window.setInterval(revalidate, LIKES_RETRY)
    return () => {
      window.removeEventListener('focus', revalidate)
      document.removeEventListener('visibilitychange', revalidate)
      window.clearInterval(timer)
    }
  }, [reference, canReadLikes, reloadLikes])
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    const stops: (() => void)[] = []
    for (const event of ['media-favorite-changed', 'music-changed'])
      void listen<string>(event, ({ payload }) => {
        if (disposed || payload !== source) return
        invalidateNativeCache()
        invalidateMusicLibrary()
        setLikesRevision((value) => value + 1)
        favoriteEvents.current++
        setLikesStale(true)
        if (activeRead.current) refreshQueued.current = true
        else reloadLikes()
      }).then((value) => {
        if (disposed) value()
        else stops.push(value)
      })
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [source, reloadLikes])
  async function toggleLike(id: string) {
    if (
      !profile ||
      (!globalLikesReady.current && !liked.current.has(id) && !knownUnliked.current.has(id)) ||
      !canToggleLike ||
      pending.current.has(id)
    )
      return
    const nextLiked = !liked.current.has(id)
    pending.current.add(id)
    setPendingLikes(new Set(pending.current))
    try {
      await musicClient.write(profile.reference.source, {
        operation: 'setFavorite',
        track: { source: profile.reference.source, kind: 'track', id },
        liked: nextLiked
      })
      edits.current.set(id, { revision: ++editRevision.current, liked: nextLiked })
      const next = new Set(liked.current)
      if (nextLiked) {
        next.add(id)
        knownUnliked.current.delete(id)
      } else {
        next.delete(id)
        knownUnliked.current.add(id)
      }
      publishLikes(next)
      if (globalLikesReady.current) persistGlobalLikes(source, next, lastSuccess.current)
      invalidateMusicLibrary()
      setLikesRevision((value) => value + 1)
    } finally {
      pending.current.delete(id)
      setPendingLikes(new Set(pending.current))
    }
  }
  return (
    <AccountContext.Provider
      value={
        new Map([
          ...parent,
          [
            source,
            {
              profile,
              loading,
              error,
              setProfile,
              likedIds,
              likesReady,
              likesRefreshing,
              likesStale,
              favoriteStatus: (id: string) =>
                likedIds.has(id)
                  ? 'liked'
                  : likesReady || knownUnliked.current.has(id)
                    ? 'unliked'
                    : 'unknown',
              confirmLikedTracks,
              canToggleLike,
              likesError,
              likesRevision,
              pendingLikes,
              reloadLikes,
              toggleLike
            }
          ]
        ])
      }
    >
      {children}
    </AccountContext.Provider>
  )
}
const unavailableAccount: AccountState = {
  profile: null,
  loading: false,
  setProfile() {},
  likedIds: new Set(),
  likesReady: false,
  likesRefreshing: false,
  likesStale: true,
  favoriteStatus: () => 'unknown',
  confirmLikedTracks() {},
  canToggleLike: false,
  likesRevision: 0,
  pendingLikes: new Set(),
  reloadLikes() {},
  async toggleLike() {}
}

export function useAccount(source?: string) {
  const accounts = useContext(AccountContext)
  const { preferences } = useMusicSources()
  const getAccount = (target: string) => accounts.get(target) ?? unavailableAccount
  return { ...getAccount(source ?? preferences?.current ?? ''), getAccount }
}
