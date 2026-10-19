import {
  useAggregateReplacement,
  aggregateCollection
} from '@/features/aggregate/aggregate-library'
import { reportCaughtError } from '@/lib/diagnostics'
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
  bypassAggregate?: boolean

  reference?: import('@/features/music/types').EntityRef
  localId?: string
  id: number | string
  kind: 'playlist' | 'album' | 'artist'
  name: string
  cover: import('@/features/music/types').CoverInput
  subtitle: string
  trackCount: number
  creatorId?: number | string
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
export function MusicNavigationProvider({ children }: { children: ReactNode }) {
  const replacement = useAggregateReplacement()
  const [history, setHistory] = useState<NavigationHistory>({
    entries: [{ view: 'library', query: '' }],
    index: 0
  })
  useEffect(() => {
    try {
      // Remove snapshots written by versions that restored the last workspace.
      localStorage.removeItem('nons-workspace-history')
      localStorage.removeItem('nons-now-playing-open')
    } catch (cause) {
      reportCaughtError('app/src/features/workspace/music-navigation.tsx', cause)
    }
  }, [])
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
    if (collection && view === 'collection') {
      const aggregate = replacement(collection)
      if (aggregate) {
        view = 'local-playlist'
        collection = aggregateCollection(aggregate)
      }
    }
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
        current.collection?.bypassAggregate === collection?.bypassAggregate &&
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
