import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'

export type MusicView =
  | 'accounts'
  | 'library'
  | 'discover'
  | 'local'
  | 'search'
  | 'settings'
  | 'queue'
  | 'collection'
  | 'artist'
  | 'album'
  | 'tools'
  | 'plugin'
  | 'local-artist'
  | 'local-album'
  | 'local-playlist'
export interface MusicCollection {
  reference?: import('@/features/music/types').EntityRef
  localId?: string
  id: number
  kind: 'playlist' | 'album' | 'artist'
  name: string
  cover: import('@/features/music/types').CoverInput
  subtitle: string
  trackCount: number
  creatorId?: number
  liked?: boolean
  playCount?: number | null
  publishedAt?: number | null
  artists?: import('@/lib/player').MusicCredit[]
}
interface Page {
  view: MusicView
  query: string
  collection?: MusicCollection
}
const NavigationContext = createContext<{
  page: Page
  canBack: boolean
  canForward: boolean
  navigate: (view: MusicView, query?: string, collection?: MusicCollection) => void
  back: () => void
  forward: () => void
} | null>(null)

type NavigationHistory = { entries: Page[]; index: number }
const historyKey = 'nons-workspace-history'
function restoreHistory(): NavigationHistory {
  try {
    const saved = JSON.parse(localStorage.getItem(historyKey) ?? 'null') as NavigationHistory | null
    const views: MusicView[] = [
      'accounts',
      'library',
      'discover',
      'local',
      'search',
      'settings',
      'queue',
      'collection',
      'artist',
      'album',
      'tools',
      'plugin',
      'local-artist',
      'local-album',
      'local-playlist'
    ]
    if (
      saved &&
      Array.isArray(saved.entries) &&
      saved.entries.length > 0 &&
      saved.entries.length <= 100 &&
      Number.isInteger(saved.index) &&
      saved.index >= 0 &&
      saved.index < saved.entries.length &&
      saved.entries.every(
        (page) => page && views.includes(page.view) && typeof page.query === 'string'
      )
    )
      return saved
  } catch {
    /* A damaged snapshot starts at the library. */
  }
  return { entries: [{ view: 'library', query: '' }], index: 0 }
}

export function MusicNavigationProvider({ children }: { children: ReactNode }) {
  const [history, setHistory] = useState<NavigationHistory>(restoreHistory)
  useEffect(() => {
    try {
      localStorage.setItem(historyKey, JSON.stringify(history))
    } catch {
      /* Navigation still works without storage. */
    }
  }, [history])
  useEffect(() => {
    // Application history is independent of the WebView's document history.
    // Capture side buttons before card/menu handlers and suppress browser navigation.
    const sideButton = (event: MouseEvent) => {
      if (event.button !== 3 && event.button !== 4) return
      event.preventDefault()
      event.stopPropagation()
      if (event.type !== 'mousedown') return
      const delta = event.button === 3 ? -1 : 1
      setHistory((previous) => {
        const index = Math.max(0, Math.min(previous.entries.length - 1, previous.index + delta))
        return index === previous.index ? previous : { ...previous, index }
      })
    }
    const events = ['mousedown', 'mouseup', 'auxclick'] as const
    events.forEach((type) => window.addEventListener(type, sideButton, true))
    return () => {
      events.forEach((type) => window.removeEventListener(type, sideButton, true))
    }
  }, [])
  function navigate(view: MusicView, query = '', collection?: MusicCollection) {
    if (view === 'collection' && collection?.kind === 'artist') view = 'artist'
    if (view === 'collection' && collection?.kind === 'album') view = 'album'
    setHistory((previous) => {
      const current = previous.entries[previous.index]
      if (
        current.view === view &&
        current.query === query &&
        current.collection?.id === collection?.id &&
        current.collection?.kind === collection?.kind &&
        current.collection?.localId === collection?.localId &&
        JSON.stringify(current.collection?.reference) === JSON.stringify(collection?.reference) &&
        current.collection?.name === collection?.name
      )
        return previous
      // Bound history; navigating after going back discards the forward branch.
      const entries = [
        ...previous.entries.slice(0, previous.index + 1),
        { view, query, collection }
      ].slice(-100)
      return { entries, index: entries.length - 1 }
    })
  }
  const move = (delta: number) =>
    setHistory((previous) => ({
      ...previous,
      index: Math.max(0, Math.min(previous.entries.length - 1, previous.index + delta))
    }))
  return (
    <NavigationContext.Provider
      value={{
        page: history.entries[history.index],
        canBack: history.index > 0,
        canForward: history.index < history.entries.length - 1,
        navigate,
        back: () => move(-1),
        forward: () => move(1)
      }}
    >
      {children}
    </NavigationContext.Provider>
  )
}

export function useMusicNavigation() {
  const value = useContext(NavigationContext)
  if (!value) throw new Error('MusicNavigationProvider is required')
  return value
}
