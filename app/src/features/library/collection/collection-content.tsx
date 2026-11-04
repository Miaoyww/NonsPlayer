import { createContext, useContext, useId, useState, type ReactNode } from 'react'
import { Search } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { TrackSearchContext } from '@/components/music/track-search-context'
import { isAndroid } from '@/lib/platform'

const SearchControls = createContext<{
  query: string
  setQuery: (query: string) => void
  hasMore: boolean
} | null>(null)

export function CollectionContent({
  children,
  hasMore = false,
  searchable = true
}: {
  children: ReactNode
  hasMore?: boolean
  searchable?: boolean
}) {
  const [query, setQuery] = useState('')
  const enabled = searchable && !isAndroid()
  return (
    <SearchControls.Provider value={enabled ? { query, setQuery, hasMore } : null}>
      <TrackSearchContext.Provider value={enabled ? query : ''}>
        {children}
      </TrackSearchContext.Provider>
    </SearchControls.Provider>
  )
}

export function CollectionSearch() {
  const controls = useContext(SearchControls)
  const hint = useId()
  if (!controls) return null
  return (
    <label className="collection-search">
      <span className="sr-only">搜索列表歌曲</span>
      <Search aria-hidden="true" />
      <Input
        type="search"
        placeholder="搜索歌曲、艺术家、专辑"
        value={controls.query}
        onChange={(event) => controls.setQuery(event.target.value)}
        aria-describedby={controls.hasMore ? hint : undefined}
        title={controls.hasMore ? '搜索已加载歌曲，滚动到底部继续加载' : undefined}
      />
      {controls.hasMore && (
        <span id={hint} className="sr-only">
          搜索已加载歌曲，滚动到底部继续加载
        </span>
      )}
    </label>
  )
}
