import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefObject
} from 'react'
import { Search, X } from 'lucide-react'
import { nativeCall, type Track } from '@/lib/player'
import { Input } from '@/components/ui/input'
import { ActionButton } from './action-button'
import { TrackIdentity } from './track-identity'
import { SongContextMenu, useSongActions } from './song-actions'
import { useMusicNavigation } from '@/features/workspace/music-navigation'

type FindEntry = { track: Track; index: number }
type FindSource = {
  element: RefObject<HTMLElement | null>
  entries: FindEntry[]
  hasMore?: boolean
  busy?: boolean
  play: (index: number) => void
  locate: (index: number) => void
  remove?: (index: number) => void | Promise<unknown>
  removeLabel?: string
}
type Registration = FindSource & { id: symbol }
const FindContext = createContext<((id: symbol, source: FindSource | null) => void) | null>(null)

export function useListFind(source: FindSource) {
  const register = useContext(FindContext)
  const id = useRef(Symbol('song-list'))
  useEffect(() => {
    register?.(id.current, source)
  }, [register, source])
  useEffect(() => () => register?.(id.current, null), [register])
}

export function ListFindProvider({
  children,
  playerVisible,
  onGlobalSearch
}: {
  children: ReactNode
  playerVisible: boolean
  onGlobalSearch: () => void
}) {
  const { page } = useMusicNavigation()
  const actions = useSongActions()
  const resultsId = useId()
  const sources = useRef(new Map<symbol, Registration>())
  const [active, setActive] = useState<Registration | null>(null)
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [submitting, setSubmitting] = useState(false)
  const pending = useRef(false)
  const resultsElement = useRef<HTMLDivElement>(null)
  const input = useRef<HTMLInputElement>(null)
  const panel = useRef<HTMLDivElement>(null)
  const previousFocus = useRef<HTMLElement | null>(null)
  const register = useCallback((id: symbol, source: FindSource | null) => {
    if (source) sources.current.set(id, { ...source, id })
    else sources.current.delete(id)
    setActive((current) => (current?.id === id ? (sources.current.get(id) ?? null) : current))
  }, [])
  const close = useCallback(() => {
    setActive(null)
    if (previousFocus.current?.isConnected) previousFocus.current.focus({ preventScroll: true })
  }, [])
  useEffect(() => {
    setActive(null)
  }, [page, playerVisible])
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (
        (event.ctrlKey || event.metaKey) &&
        !event.altKey &&
        !event.shiftKey &&
        event.key.toLowerCase() === 'f'
      ) {
        if ((event.target as HTMLElement)?.closest?.('[data-shortcut-recorder]')) return
        event.preventDefault()
        event.stopPropagation()
        // A hidden tab or the lyrics layer must never contribute its underlying list.
        const visible =
          playerVisible || page.view === 'discover' || page.view === 'queue'
            ? []
            : [...sources.current.values()].filter(
                (source) => source.element.current?.getClientRects().length
              )
        const focused = visible.find((source) =>
          source.element.current?.contains(document.activeElement)
        )
        const source = focused ?? visible[visible.length - 1]
        if (!source) {
          setActive(null)
          onGlobalSearch()
          return
        }
        if (!active) previousFocus.current = document.activeElement as HTMLElement | null
        setActive(source)
        requestAnimationFrame(() => {
          input.current?.focus()
          input.current?.select()
        })
      } else if (event.key === 'Escape' && active && !document.querySelector('[role="menu"]')) {
        event.preventDefault()
        event.stopPropagation()
        close()
      }
    }
    document.addEventListener('keydown', keydown, true)
    return () => document.removeEventListener('keydown', keydown, true)
  }, [active, close, page.view, playerVisible, onGlobalSearch])
  useEffect(() => {
    if (!active) {
      setQuery('')
      return
    }
    const outside = (event: PointerEvent) => {
      const target = event.target as HTMLElement
      if (target.closest('[role="tab"], .library-tab')) {
        close()
        return
      }
      if (!panel.current?.contains(target) && !target.closest('[role="menu"], .song-context-menu'))
        close()
    }
    document.addEventListener('pointerdown', outside)
    return () => document.removeEventListener('pointerdown', outside)
  }, [active, close])
  const results = useMemo(() => {
    const keyword = query.trim().toLocaleLowerCase()
    const matches: FindEntry[] = []
    if (!keyword) return matches
    for (const entry of active?.entries ?? []) {
      const { track } = entry
      if (
        [track.title, track.artist, track.album, ...(track.aliases ?? [])].some((value) =>
          value.toLocaleLowerCase().includes(keyword)
        )
      ) {
        matches.push(entry)
        if (matches.length === 5) break
      }
    }
    return matches
  }, [active, query])
  const selected = Math.min(selectedIndex, results.length - 1)
  useEffect(() => {
    resultsElement.current
      ?.querySelector<HTMLElement>('[aria-selected="true"]')
      ?.scrollIntoView?.({ block: 'nearest' })
  }, [selected, results])
  useEffect(() => {
    if (resultsElement.current) resultsElement.current.scrollTop = 0
  }, [query])
  const locate = (index: number) => {
    setActive(null)
    active?.locate(index)
  }
  return (
    <FindContext.Provider value={register}>
      <div className="relative flex min-h-0 flex-1 flex-col">
        {children}
        {active && (
          <div
            ref={panel}
            role="region"
            aria-label="快捷搜索列表歌曲"
            className="fixed left-1/2 top-[calc(3rem+1.25rem)] z-50 w-[min(30rem,calc(100%-2rem))] -translate-x-1/2 rounded-2xl border border-border bg-background p-3 shadow-xl"
          >
            <div className="flex items-center gap-2">
              <Search aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
              <Input
                ref={input}
                type="search"
                aria-label="快捷搜索列表歌曲"
                role="combobox"
                aria-autocomplete="list"
                aria-expanded={results.length > 0}
                aria-controls={resultsId}
                aria-activedescendant={selected >= 0 ? `${resultsId}-${selected}` : undefined}
                placeholder="搜索歌曲、艺术家、专辑"
                value={query}
                onChange={(event) => {
                  setQuery(event.target.value)
                  setSelectedIndex(0)
                }}
                onKeyDown={(event) => {
                  if (
                    event.nativeEvent.isComposing ||
                    event.keyCode === 229 ||
                    event.ctrlKey ||
                    event.metaKey ||
                    event.altKey ||
                    event.shiftKey ||
                    !results.length
                  )
                    return
                  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
                    event.preventDefault()
                    event.stopPropagation()
                    const delta = event.key === 'ArrowDown' ? 1 : -1
                    setSelectedIndex((selected + delta + results.length) % results.length)
                  } else if (event.key === 'Enter') {
                    event.preventDefault()
                    event.stopPropagation()
                    if (event.repeat || active.busy || pending.current) return
                    const { track } = results[selected]
                    pending.current = true
                    setSubmitting(true)
                    actions.run(async () => {
                      try {
                        await nativeCall('append_queue', { keys: [track.key], play: true })
                        close()
                      } finally {
                        pending.current = false
                        setSubmitting(false)
                      }
                    })
                  }
                }}
              />
              <ActionButton
                variant="ghost"
                size="icon-sm"
                aria-label="关闭快捷搜索"
                onClick={close}
              >
                <X aria-hidden="true" />
              </ActionButton>
            </div>
            <p role="status" className="sr-only">
              {query.trim()
                ? `找到 ${results.length} 首歌曲${active.hasMore ? ' · 仅搜索已加载歌曲' : ''}`
                : ''}
            </p>
            <div
              ref={resultsElement}
              id={resultsId}
              role="listbox"
              aria-label="匹配的歌曲"
              aria-busy={submitting}
              className="max-h-[min(28rem,55vh)] overflow-y-auto overscroll-contain"
            >
              {results.map(({ track, index }, resultIndex) => (
                <SongContextMenu
                  key={`${track.key}:${index}`}
                  track={track}
                  busy={active.busy || submitting}
                  onPlay={() => active.play(index)}
                  onLocate={() => locate(index)}
                  onRemove={active.remove ? () => active.remove!(index) : undefined}
                  removeLabel={active.removeLabel}
                  render={
                    <div
                      id={`${resultsId}-${resultIndex}`}
                      role="option"
                      aria-selected={selected === resultIndex}
                      onPointerMove={() => setSelectedIndex(resultIndex)}
                      className={`rounded-lg px-2 py-2 hover:bg-muted focus-visible:outline-2 focus-visible:outline-primary ${selected === resultIndex ? 'bg-muted ring-1 ring-inset ring-primary/40' : ''}`}
                    />
                  }
                >
                  <TrackIdentity track={track} showSource />
                </SongContextMenu>
              ))}
            </div>
            {results.length > 0 && (
              <p className="mt-2 px-2 text-xs text-muted-foreground">
                ↑↓ 选择歌曲 · Enter 插入下一首并立即播放
              </p>
            )}
          </div>
        )}
      </div>
    </FindContext.Provider>
  )
}

export function FindSongGrid({
  tracks,
  busy,
  onPlay,
  children
}: {
  tracks: Track[]
  busy: boolean
  onPlay: (track: Track) => void
  children: ReactNode
}) {
  const element = useRef<HTMLDivElement>(null)
  const [located, setLocated] = useState<number>()
  useListFind(
    useMemo(
      () => ({
        element,
        entries: tracks.map((track, index) => ({ track, index })),
        busy,
        play: (index: number) => onPlay(tracks[index]),
        locate: (index: number) => {
          const card = element.current?.querySelectorAll<HTMLElement>('.library-song')[index]
          card?.scrollIntoView({ block: 'center' })
          card?.focus({ preventScroll: true })
          setLocated(index)
        }
      }),
      [tracks, busy, onPlay]
    )
  )
  useEffect(() => {
    if (located === undefined) return
    const card = element.current?.querySelectorAll<HTMLElement>('.library-song')[located]
    card?.setAttribute('data-find-highlight', 'true')
    const timer = window.setTimeout(() => setLocated(undefined), 2500)
    return () => {
      window.clearTimeout(timer)
      card?.removeAttribute('data-find-highlight')
    }
  }, [located])
  return (
    <div ref={element} className="library-song-grid">
      {children}
    </div>
  )
}
